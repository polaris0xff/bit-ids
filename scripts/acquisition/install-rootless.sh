#!/bin/sh
# install-rootless.sh - fetch a published build, identify its bytes, and place it
# where the current user already has write access. No privilege, no package
# index, no host state outside one prefix this file derives.
#
# ⛔ THIS EXISTS BECAUSE EVERY INSTALL THIS PROJECT PERFORMED NEEDED `sudo` AND
# WROTE TO `/usr/local/bin`. What such an install does is a property of a host
# this project does not control, and `ACQ-06` is the entry that removes the
# variable rather than measuring it. `TODO/acquisition.md` carries the argument.
#
# -- the prefix ---------------------------------------------------------------
#
# ⛔ THE PREFIX HAS ONE DERIVATION AND `--prefix` IS IT. Every caller asks this
# file rather than composing a path of its own, for the reason
# `assert-disposable --marker` already exists: a second spelling goes on reading
# the old place the day the first one moves. An adapter's `binary()` and its
# `install` route are two such spellings inside one file.
#
# ⚠ `$HOME/.local/share/bit-ids` RATHER THAN `$HOME/.local`. A build placed in a
# user's own prefix would shadow their tools on PATH, and removing what this
# project installed would mean knowing which files were its. One directory
# carries all of it, names itself, and is removed in one command.
#
# ⛔ AND THIS FILE NEVER ESCALATES. A prefix the current user cannot write is a
# refusal naming the prefix, never a `sudo` retry: an installer that reaches for
# privilege when it runs out of permission is the thing being removed.
#
# -- identifying the bytes ----------------------------------------------------
#
# ⛔ NOTHING IS MADE EXECUTABLE BEFORE ITS DIGEST IS SETTLED, and the disposition
# is declared rather than defaulted. At least one of these is required, so a
# caller that forgot to pass a digest is refused rather than served:
#
#   --sha256 <hex>              the SOURCE published this digest of the bytes it
#                               serves. ⚠ Same party as the one serving them, so
#                               it establishes that nothing changed between
#                               reading the index and fetching the artifact
#   --sums <file> --sums-name <name>
#                               the VENDOR published a document; `sha256sum -c`
#                               is the verifier and this project wrote none of it
#   --digest-unpublished <why>  neither exists, and <why> is the measured reason.
#                               The digest of what arrived is still recorded;
#                               what changes is the claim.
#
# ⭐ THE FIRST TWO MAY BE GIVEN TOGETHER AND BOTH ARE THEN CHECKED, because two
# statements about one artifact is strictly stronger than either and they come
# from different parties. ⛔ The third may not be combined with either: a caller
# cannot both have a digest and declare there is none.
#
# ⚠ THE THIRD IS NOT A WAY ROUND THE FIRST TWO. It records that the bytes were
# identified and not verified, which is a different sentence, and a route that
# used it where a digest exists would be saying something untrue about its own
# evidence.
#
# -- ⭐ `--from-resolution` IS HOW THE CAPTURE PATH PASSES ALL THREE -----------
#
# ⛔ FOUR ADAPTERS CHOOSING BETWEEN FOUR DISPOSITIONS IS FOUR COPIES OF ONE
# DECISION, and a gate on one of several paths into the same action is the most
# recurring hole this repository records. So the resolution record is read HERE,
# once: `resolve-release.sh` already wrote which digest it resolved and where,
# and every adapter passes the path instead of the flags.
#
# Usage:
#   sh scripts/acquisition/install-rootless.sh --prefix
#   sh scripts/acquisition/install-rootless.sh --fetch --url <u> --into <f> <disposition>
#   sh scripts/acquisition/install-rootless.sh --install --url <u> --into <f> \
#                                              --as <name> <disposition>
#
# `--fetch` prints the path of the verified file. `--install` does that and then
# places it executable at `<prefix>/bin/<name>`, printing that absolute path.
# ⭐ Callers take the path this prints; nothing here touches PATH.
#
# Exit codes: 0 done, 1 the fetch or the digest refused, 2 could not run.
#
# ⛔ Read the exit code from this process, unpiped.

set -u

MODE=""
URL=""
INTO=""
AS=""
SHA256=""
SUMS=""
SUMS_NAME=""
UNPUBLISHED=""
REPORT=""
RESOLUTION=""

usage() {
  awk 'NR>1 { if (/^#/) { sub(/^# ?/, ""); print } else exit }' "$0"
}

refuse() {
  printf 'install-rootless: %s\n' "$1" >&2
  exit 1
}

cannot() {
  printf 'install-rootless: %s\n' "$1" >&2
  exit 2
}

set_mode() {
  [ -z "$MODE" ] || cannot "--prefix, --fetch and --install are three modes; name one"
  MODE="$1"
}

while [ $# -gt 0 ]; do
  case "$1" in
    --prefix) set_mode prefix ;;
    --fetch) set_mode fetch ;;
    --install) set_mode install ;;
    --url)
      [ $# -ge 2 ] || cannot "--url takes a value"
      URL="$2"
      shift
      ;;
    --into)
      [ $# -ge 2 ] || cannot "--into takes a value"
      INTO="$2"
      shift
      ;;
    --as)
      [ $# -ge 2 ] || cannot "--as takes a value"
      AS="$2"
      shift
      ;;
    --sha256)
      [ $# -ge 2 ] || cannot "--sha256 takes a value"
      SHA256="$2"
      shift
      ;;
    --sums)
      [ $# -ge 2 ] || cannot "--sums takes a value"
      SUMS="$2"
      shift
      ;;
    --sums-name)
      [ $# -ge 2 ] || cannot "--sums-name takes a value"
      SUMS_NAME="$2"
      shift
      ;;
    --digest-unpublished)
      [ $# -ge 2 ] || cannot "--digest-unpublished takes the measured reason"
      UNPUBLISHED="$2"
      shift
      ;;
    --from-resolution)
      [ $# -ge 2 ] || cannot "--from-resolution takes a value"
      RESOLUTION="$2"
      shift
      ;;
    --report)
      [ $# -ge 2 ] || cannot "--report takes a value"
      REPORT="$2"
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      printf 'install-rootless: unknown argument: %s\n' "$1" >&2
      exit 2
      ;;
  esac
  shift
done

[ -n "$MODE" ] || cannot "name a mode: --prefix, --fetch or --install"

# -- the one derivation of the prefix -----------------------------------------
#
# ⚠ `$HOME` IS REQUIRED TO BE ABSOLUTE RATHER THAN MERELY SET. An empty or
# relative value composes into a path relative to whatever directory the caller
# happened to be in, so two callers in two directories would install into two
# prefixes and the second would report the first's build as preexisting.
resolved_prefix() {
  if [ -n "${BIT_IDS_PREFIX:-}" ]; then
    case "$BIT_IDS_PREFIX" in
      /*) printf '%s' "$BIT_IDS_PREFIX" ;;
      *) cannot "BIT_IDS_PREFIX is [$BIT_IDS_PREFIX], which is not an absolute path" ;;
    esac
    return 0
  fi
  case "${HOME:-}" in
    /*) printf '%s/.local/share/bit-ids' "$HOME" ;;
    *) cannot "HOME is [${HOME:-}], so there is no prefix this user is known to own" ;;
  esac
}

PREFIX=$(resolved_prefix) || exit $?

if [ "$MODE" = prefix ]; then
  printf '%s\n' "$PREFIX"
  exit 0
fi

[ -n "$URL" ] || cannot "--url is required"
[ -n "$INTO" ] || cannot "--into is required"
[ "$MODE" != install ] || [ -n "$AS" ] || cannot "--install needs --as <name>"

# ⛔ A NAME, NEVER A PATH. `--as ../../bin/sh` would place a build outside the
# prefix this file exists to confine it to, and the caller is an adapter reading
# a value out of a document.
case "$AS" in
  */* | .. | .) cannot "--as takes a file name, not a path: [$AS]" ;;
  *) ;;
esac

# -- the resolution, read here so four adapters do not each read it ------------
#
# ⛔ THE RECORD SAYS WHICH DIGEST IT RESOLVED AND WHERE, and this turns that into
# the flags below. ⚠ A record naming a `digest_source` this file does not know is
# could-not-run rather than an install: a vocabulary that grew on one side and
# not the other is how a route ends up installing under a disposition nobody
# implemented.
if [ -n "$RESOLUTION" ]; then
  [ -f "$RESOLUTION" ] || cannot "the resolution $RESOLUTION is not present"
  [ -z "$SHA256$SUMS$UNPUBLISHED" ] ||
    cannot "--from-resolution carries the disposition; do not pass one beside it"
  _src=$(sed -n 's/^digest_source=//p' "$RESOLUTION")
  case "$_src" in
    vendor-document | source-listing | both)
      if [ "$_src" != source-listing ]; then
        SUMS=$(sed -n 's/^digest_document=//p' "$RESOLUTION")
        SUMS_NAME=$(sed -n 's/^asset=//p' "$RESOLUTION")
        if [ -z "$SUMS" ] || [ -z "$SUMS_NAME" ]; then
          cannot "the resolution says $_src and names no document or asset"
        fi
      fi
      if [ "$_src" != vendor-document ]; then
        SHA256=$(sed -n 's/^asset_sha256=//p' "$RESOLUTION")
        # ⚠ THE CANONICAL FORM CARRIES ITS ALGORITHM AND THIS FLAG TAKES THE HEX.
        # The record spells `sha256:<hex>` because a bare digest cannot say what
        # produced it; the prefix is stripped here rather than in the record.
        SHA256=${SHA256#sha256:}
        [ -n "$SHA256" ] || cannot "the resolution says $_src and names no asset_sha256"
      fi
      ;;
    unpublished)
      UNPUBLISHED=$(sed -n 's/^digest_unpublished=//p' "$RESOLUTION")
      [ -n "$UNPUBLISHED" ] ||
        UNPUBLISHED="the resolution records no published digest and no reason"
      ;;
    "") cannot "the resolution $RESOLUTION names no digest_source" ;;
    *) cannot "the resolution names digest_source=[$_src], which this installer does not know" ;;
  esac
fi

# ⛔ AT LEAST ONE DISPOSITION, AND `--digest-unpublished` ALONE. Zero is a caller
# that forgot, which is the case this refuses. ⭐ A digest and a document
# together are two statements about one artifact from two parties, and both are
# checked. ⛔ But a caller cannot both hold a digest and declare there is none.
DISPOSITIONS=0
[ -z "$SHA256" ] || DISPOSITIONS=$((DISPOSITIONS + 1))
[ -z "$SUMS" ] || DISPOSITIONS=$((DISPOSITIONS + 1))
[ -z "$UNPUBLISHED" ] || DISPOSITIONS=$((DISPOSITIONS + 1))
[ "$DISPOSITIONS" != 0 ] ||
  cannot "name a digest disposition: --sha256, --sums or --digest-unpublished"
[ -z "$UNPUBLISHED" ] || [ "$DISPOSITIONS" = 1 ] ||
  cannot "--digest-unpublished says there is no digest; it cannot be given beside one"

if [ -n "$SHA256" ]; then
  # ⚠ SIXTY-FOUR LOWERCASE HEX DIGITS, CHECKED AS A SHAPE. A truncated or
  # upper-case value would compare unequal against every artifact and read as a
  # vendor whose bytes changed, which sends a reader to the wrong question.
  case "$SHA256" in
    *[!0-9a-f]* | "") cannot "--sha256 [$SHA256] is not lowercase hex" ;;
    *) [ ${#SHA256} -eq 64 ] || cannot "--sha256 [$SHA256] is not 64 digits" ;;
  esac
fi
if [ -n "$SUMS" ]; then
  [ -n "$SUMS_NAME" ] || cannot "--sums needs --sums-name, the name the document uses"
  [ -f "$SUMS" ] || cannot "the checksums document $SUMS is not present"
  case "$SUMS_NAME" in
    */* | .. | .) cannot "--sums-name takes a file name, not a path: [$SUMS_NAME]" ;;
    *) ;;
  esac
fi

for _tool in curl sha256sum install; do
  command -v "$_tool" >/dev/null 2>&1 || cannot "$_tool is not on this host"
done

# -- the fetch ----------------------------------------------------------------
#
# ⛔ BOUNDED IN BOTH DIRECTIONS, AND THE SPEED FLOOR IS THE ONE THAT CATCHES A
# STALL. `--max-time` alone has to be large enough for a slow link to finish a
# fourteen-megabyte artifact, which is large enough to sit in a dead transfer for
# minutes; a transfer under 1 KB/s for a minute is stopped whatever its size.
# `docs/conventions/shell.md` section 9 carries the rule and `bit-check
# check-adapters` enforces it on the adapters.
#
# ⭐ AND THE OUTPUT GOES TO FILES. Nothing this spawns holds a descriptor the
# caller is reading, which is the property `install-step.sh` is built on one
# directory up.
FETCH_SECONDS=${BIT_IDS_FETCH_TIMEOUT:-300}
INTO_DIR=$(dirname -- "$INTO")
mkdir -p "$INTO_DIR" || cannot "cannot create $INTO_DIR"
INTO_DIR=$(CDPATH='' cd -- "$INTO_DIR" && pwd) || cannot "cannot resolve $INTO_DIR"
INTO="$INTO_DIR/$(basename -- "$INTO")"
FETCH_LOG="$INTO.fetch.log"

curl -fsSL --retry 2 --connect-timeout 20 --max-time "$FETCH_SECONDS" \
  --speed-limit 1024 --speed-time 60 \
  -o "$INTO" "$URL" </dev/null >"$FETCH_LOG" 2>&1 ||
  refuse "the artifact could not be fetched from $URL"
[ -s "$INTO" ] || refuse "the fetch left an empty file at $INTO"

ARRIVED=$(sha256sum "$INTO" | cut -d' ' -f1)
[ -n "$ARRIVED" ] || cannot "sha256sum reported no digest for $INTO"

# -- identifying what arrived -------------------------------------------------
#
# ⛔ BEFORE ANY MODE BIT IS SET. A file that is executable is a file something can
# run, so the order here is the guarantee rather than a tidy sequence: an
# artifact whose digest does not settle is removed and never made runnable.
DIGEST_SOURCE=""
if [ -n "$SHA256" ]; then
  if [ "$ARRIVED" != "$SHA256" ]; then
    rm -f "$INTO"
    refuse "the artifact from $URL digests as $ARRIVED and the source published $SHA256"
  fi
  DIGEST_SOURCE=source-listing
fi
if [ -n "$SUMS" ]; then
  # ⭐ THE VERIFIER IS `sha256sum -c` OVER THE VENDOR'S OWN DOCUMENT, so neither
  # the digest nor the comparison is this project's code. ⚠ It is run in a
  # directory holding the artifact under the name that document uses, because a
  # checksums file names assets and this workdir names files.
  #
  # ⛔ `--ignore-missing` IS WHAT LETS A DOCUMENT COVERING SEVEN PLATFORMS VERIFY
  # THE ONE ASSET PRESENT, and it does NOT pass over nothing: measured on
  # 2026-09-17, a document naming only absent files exits 1 with `no file was
  # verified`, which is the refusal a reader would otherwise have to add.
  VERIFY_DIR="$INTO_DIR/.verify-$$"
  rm -rf "$VERIFY_DIR"
  mkdir -p "$VERIFY_DIR" || cannot "cannot create $VERIFY_DIR"
  cp "$INTO" "$VERIFY_DIR/$SUMS_NAME" || cannot "cannot stage $INTO for verification"
  cp "$SUMS" "$VERIFY_DIR/sums.txt" || cannot "cannot stage $SUMS for verification"
  (cd "$VERIFY_DIR" && sha256sum -c --ignore-missing sums.txt) \
    </dev/null >"$INTO.verify.log" 2>&1
  VERIFY_RC=$?
  rm -rf "$VERIFY_DIR"
  if [ "$VERIFY_RC" != 0 ]; then
    sed 's/^/          /' "$INTO.verify.log" >&2
    rm -f "$INTO"
    refuse "sha256sum -c refused $SUMS_NAME against the vendor's document (exit $VERIFY_RC)"
  fi
  # ⭐ BOTH, WHERE BOTH WERE GIVEN. Two parties stated a digest of these bytes
  # and both were checked, which is a different and stronger sentence than
  # either alone - so the record says so rather than naming whichever ran last.
  if [ "$DIGEST_SOURCE" = source-listing ]; then
    DIGEST_SOURCE=both
  else
    DIGEST_SOURCE=vendor-document
  fi
fi
if [ -z "$DIGEST_SOURCE" ]; then
  # ⚠ SAID OUT LOUD, ON stderr. A route whose sources publish no digest has
  # identified its bytes and not verified them, and a reader of the step log is
  # who needs to know which of the two happened.
  printf 'install-rootless: %s publishes no digest for this asset (%s); the artifact is recorded as %s and was not verified\n' \
    "$URL" "$UNPUBLISHED" "$ARRIVED" >&2
  DIGEST_SOURCE=unpublished
fi

# ⚠ WRITTEN WHERE THE INSTALL RECORD CAN READ IT, and empty is never a value
# here: every field is printed on every path.
if [ -n "$REPORT" ]; then
  {
    printf 'bit-ids/rootless-install/1\n'
    printf 'url=%s\n' "$URL"
    printf 'artifact=%s\n' "$INTO"
    printf 'artifact_sha256=%s\n' "$ARRIVED"
    printf 'digest_source=%s\n' "$DIGEST_SOURCE"
    printf 'digest_note=%s\n' "$UNPUBLISHED"
    printf 'prefix=%s\n' "$PREFIX"
  } >"$REPORT" || cannot "the install report could not be written to $REPORT"
fi

if [ "$MODE" = fetch ]; then
  printf '%s\n' "$INTO"
  exit 0
fi

# -- placing it ---------------------------------------------------------------
#
# ⛔ THE PREFIX IS CREATED BY THIS USER OR THE ROUTE REFUSES. A `mkdir` that
# fails here is a host where this install needs a privilege it is not going to
# ask for, and the message names the prefix so the caller can point somewhere it
# owns rather than wonder which directory refused.
mkdir -p "$PREFIX/bin" 2>"$INTO.prefix.log" || {
  sed 's/^/          /' "$INTO.prefix.log" >&2
  refuse "this user cannot create $PREFIX/bin; a rootless install writes nowhere else"
}
[ -w "$PREFIX/bin" ] ||
  refuse "$PREFIX/bin is not writable by this user; a rootless install writes nowhere else"

# ⚠ INTO PLACE AS ONE STEP. A copy that is interrupted leaves a truncated
# executable somewhere an adapter's `binary()` will happily find; a rename within
# one directory either happened or did not. ⭐ `install -m755` sets the mode as it
# writes, so the staged file is never both present and non-executable.
STAGED="$PREFIX/bin/.$AS.new"
install -m755 "$INTO" "$STAGED" ||
  refuse "the artifact could not be staged in $PREFIX/bin"
mv "$STAGED" "$PREFIX/bin/$AS" || {
  rm -f "$STAGED"
  refuse "the artifact could not be installed as $PREFIX/bin/$AS"
}
[ -x "$PREFIX/bin/$AS" ] ||
  refuse "the install reported success and left no executable $AS in $PREFIX/bin"

printf '%s\n' "$PREFIX/bin/$AS"
printf 'install-rootless: %s -> %s (sha256:%s, %s)\n' \
  "$URL" "$PREFIX/bin/$AS" "$ARRIVED" "$DIGEST_SOURCE" >&2
exit 0
