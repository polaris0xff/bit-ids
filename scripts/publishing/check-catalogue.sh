#!/bin/sh
# check-catalogue.sh - drive LIB-01's consumer library against a real
# publication, and plant every defect it exists to refuse.
#
# ⛔ THE THIRD CLAUSE OF THE PROVE IS THE ONE A GREP ANSWERS BETTER THAN A RUN.
# "without network access" cannot be established by observing that a run did not
# use the network: a run that happened not to need one proves nothing about the
# next. What can be established is that the crate has no way to reach one, so
# this sweeps its source for every socket and HTTP constructor and reads its
# dependency list. That is the same discipline bind.rs uses for the lab's
# sockets, applied from the other side.
#
# ⚠ AND THE SWEEP'S OWN NEEDLE LIST IS THE PART THAT ROTS. OBS-06's finding was
# that every needle named a constructor while a send is a method on a socket
# that already exists, so a whole category was missing. The list below is
# checked against a file that really does carry a socket, so a sweep that had
# stopped matching anything is visible.
#
# -- ⛔ THE ORDERING CASE IS THE ONE A TEXT SORT GETS WRONG ------------------
#
# The store carries 1.2.3 and 1.2.10. As text 1.2.3 sorts last. A library that
# answered it would point a consumer at a superseded build with confidence.
#
# Usage:
#   sh scripts/publishing/check-catalogue.sh
#   sh scripts/publishing/check-catalogue.sh --json
#
# Exit codes: 0 every case held, 1 one did not, 2 could not run.
#
# ⛔ Read the exit code from this process, unpiped.

set -u

JSON=0
while [ $# -gt 0 ]; do
  case "$1" in
    --json) JSON=1 ;;
    -h | --help)
      awk 'NR>1 { if (/^#/) { sub(/^# ?/, ""); print } else exit }' "$0"
      exit 0
      ;;
    *)
      printf 'check-catalogue: unknown argument: %s\n' "$1" >&2
      exit 2
      ;;
  esac
  shift
done

HERE=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
ROOT=$(CDPATH='' cd -- "$HERE/../.." && pwd)

# shellcheck disable=SC2034
ME=check-catalogue
# shellcheck disable=SC1091
# shellcheck source=scripts/corpus/store-lib.sh
. "$ROOT/scripts/corpus/store-lib.sh"

store_require cargo sha256sum
BUILDER=$(store_build "$ROOT" build-store) || exit 2
INDEXER=$(store_build "$ROOT" build-indexes) || exit 2
FORMATTER=$(store_build "$ROOT" build-formats) || exit 2
ASSEMBLER=$(store_build "$ROOT" assemble-release) || exit 2
LOOKUP=$(store_build "$ROOT" catalogue-lookup) || exit 2

WORK=$(store_workdir checkcatalogue) || exit 2
trap 'rm -rf "$WORK"' EXIT INT TERM

BUNDLE="$WORK/bundle"
SCHEME="fixture-client:-:3:3"

build_publication() {
  rm -rf "$BUNDLE"
  mkdir -p "$BUNDLE/indexes/v1" || return 1
  "$BUILDER" --version 1.2.3 --version 1.2.10 "$BUNDLE" >/dev/null 2>&1 || return 1
  "$INDEXER" --scheme "$SCHEME" "$BUNDLE" "$BUNDLE/indexes/v1/profiles.json" \
    >/dev/null 2>&1 || return 1
  "$FORMATTER" --scheme "$SCHEME" "$BUNDLE" "$BUNDLE" >/dev/null 2>&1 || return 1
  cp "$ROOT/LICENSE" "$BUNDLE/LICENSE" || return 1
  "$ASSEMBLER" "$BUNDLE" >/dev/null 2>&1
}

# ⛔ Unpiped. Output to a file, $? on the next line.
ask() { # question...
  "$LOOKUP" "$BUNDLE" "$@" >"$WORK/out" 2>"$WORK/err"
  RC=$?
}

if ! build_publication; then
  printf 'check-catalogue: cannot build the fixture publication\n' >&2
  exit 2
fi
DIGEST="sha256:$(sha256sum "$BUNDLE/SHA256SUMS" | cut -d' ' -f1)"

ask latest fixture-client linux x86-64 tar-gz
if [ "$RC" = "0" ]; then
  pass "clean    a real publication opens and answers"
else
  fail "clean    the publication was refused (exit $RC): $(head -3 "$WORK/err" | tr '\n' ' ')"
fi

# ⛔ THE ORDERING. As text, 1.2.3 sorts after 1.2.10.
if grep -q ' 1\.2\.10$' "$WORK/out"; then
  pass "ordering  latest answers 1.2.10 over 1.2.3"
else
  fail "ordering  latest answered $(tr '\n' ' ' <"$WORK/out")"
fi

# ⚠ A build line the catalogue does not carry answers nothing rather than the
# nearest one, and the exit code says so.
ask latest fixture-client windows x86-64 tar-gz
if [ "$RC" = "1" ] && [ ! -s "$WORK/out" ]; then
  pass "absent   an unmeasured build line answers nothing, with exit 1"
else
  fail "absent   exit $RC with $(wc -l <"$WORK/out") row(s)"
fi

ask lookup target fixture-client
if [ "$RC" = "0" ] && [ "$(grep -c . "$WORK/out")" = "2" ]; then
  pass "lookup   the target index answers both published records"
else
  fail "lookup   exit $RC with $(grep -c . "$WORK/out") row(s)"
fi

# ⛔ THE OUT-OF-BAND DIGEST, WHICH IS THE ONE FILE NOTHING IN A PUBLICATION
# PROVES. Both branches are cases: the right digest is accepted and a wrong one
# is refused, because a check that only ever passes is not a check.
"$LOOKUP" "$BUNDLE" --expect-checksums "$DIGEST" lookup target fixture-client \
  >"$WORK/out" 2>"$WORK/err"
RC=$?
if [ "$RC" = "0" ] && grep -q "verified against the caller's digest" "$WORK/err"; then
  pass "outofband  the caller's digest is checked and the summary says so"
else
  fail "outofband  exit $RC: $(head -2 "$WORK/err" | tr '\n' ' ')"
fi

WRONG="sha256:0000000000000000000000000000000000000000000000000000000000000000"
"$LOOKUP" "$BUNDLE" --expect-checksums "$WRONG" lookup target fixture-client \
  >"$WORK/out" 2>"$WORK/err"
RC=$?
if [ "$RC" = "1" ] && grep -q "E-LIB-05" "$WORK/err"; then
  pass "E-LIB-05  a checksum file the caller did not expect is refused"
else
  fail "E-LIB-05  expected exit 1 with E-LIB-05, got $RC"
fi

# ⚠ And a run with no digest says it trusted the file, rather than reading as
# though it had verified it.
ask lookup target fixture-client
if grep -q "checksums trusted" "$WORK/err"; then
  pass "outofband  a run given no digest says the checksum file was trusted"
else
  fail "outofband  the summary does not say which verification ran"
fi

# ⛔ THE PLAN, AND BOTH CLASSES NON-EMPTY. A plan that is all one class
# classified nothing.
ask plan
IMMUTABLE=$(grep -c '^immutable ' "$WORK/out")
CURRENT=$(grep -c '^current ' "$WORK/out")
if [ "$RC" = "0" ] && [ "$IMMUTABLE" -gt 0 ] && [ "$CURRENT" -gt 0 ]; then
  pass "plan     $IMMUTABLE immutable and $CURRENT current path(s), so neither class is empty"
else
  fail "plan     exit $RC, $IMMUTABLE immutable, $CURRENT current"
fi
if [ "$(grep -c ' out_of_band ' "$WORK/out")" = "1" ]; then
  pass "plan     exactly one path is proved by neither document"
else
  fail "plan     $(grep -c ' out_of_band ' "$WORK/out") path(s) marked out_of_band"
fi

# -- The refusals, planted one at a time -------------------------------------

plant_case() { # name code
  ask lookup target fixture-client
  if [ "$RC" != "1" ]; then
    fail "$2  $1: expected exit 1, got $RC"
    return 0
  fi
  if ! grep -q -F -e "$2" "$WORK/err"; then
    fail "$2  $1: refused, but not as $2: $(head -2 "$WORK/err" | tr '\n' ' ')"
    return 0
  fi
  if ! build_publication; then
    fail "$2  $1: could not restore the publication"
    return 0
  fi
  ask lookup target fixture-client
  if [ "$RC" != "0" ]; then
    fail "$2  $1: the restored publication is not clean (exit $RC)"
    return 0
  fi
  pass "$2  $1"
}

RECORD=$(find "$BUNDLE/profiles" -name '*.json' | LC_ALL=C sort | head -1)
printf '\n' >>"$RECORD"
plant_case "a record whose bytes moved after publication" "E-LIB-02"

rm -f "$BUNDLE/formats/bit-ids-v1.csv"
plant_case "a described file the publication does not carry" "E-LIB-01"

printf 'x\n' >"$BUNDLE/formats/bit-ids-v1.extra.json"
plant_case "a carried file nobody described" "E-LIB-03"

rm -f "$BUNDLE/indexes/v1/profiles.json"
plant_case "a publication with no index" "E-LIB-08"

rm -f "$BUNDLE/MANIFEST.json"
plant_case "a publication with no manifest" "E-LIB-08"

# ⚠ The index rewritten to another generation, with the two root documents
# reassembled over it so the digest check cannot fire first. Without that this
# case would pass as E-LIB-02 and say nothing about compatibility.
sed -i 's|bit-ids/index/1|bit-ids/index/2|' "$BUNDLE/indexes/v1/profiles.json"
rm -f "$BUNDLE/MANIFEST.json" "$BUNDLE/SHA256SUMS"
"$ASSEMBLER" "$BUNDLE" >/dev/null 2>&1
plant_case "an index document from another generation" "E-LIB-06"

# -- ⛔ THE NO-NETWORK CLAIM, SWEPT RATHER THAN OBSERVED ---------------------

SRC="$ROOT/crates/bit-ids/src"
NEEDLES='std::net\|TcpStream\|TcpListener\|UdpSocket\|SocketAddr\|reqwest\|ureq\|hyper::\|curl::\|\.connect(\|to_socket_addrs'
if grep -rn "$NEEDLES" "$SRC" >"$WORK/net" 2>&1; then
  fail "network  the crate names a socket or an HTTP client: $(head -2 "$WORK/net" | tr '\n' ' ')"
else
  pass "network  no socket, address type or HTTP client is named anywhere in the crate"
fi

# ⚠ THE SWEEP IS CHECKED AGAINST A FILE THAT REALLY CARRIES ONE. A needle list
# that had stopped matching anything would report the same clean answer over a
# crate full of sockets, which is the shape OBS-06 found.
LAB="$ROOT/crates/bit-ids-lab/src"
if [ -d "$LAB" ] && grep -rq "$NEEDLES" "$LAB"; then
  pass "network  and the same needles do match the crate that owns the sockets"
else
  fail "network  the needle list matches nothing even in bit-ids-lab; it has stopped sweeping"
fi

# And the dependency list, because a socket can arrive through a crate rather
# than through a line of source.
MANIFEST="$ROOT/crates/bit-ids/Cargo.toml"
if grep -q -E '^(reqwest|ureq|hyper|curl|tokio|async-std|isahc)' "$MANIFEST"; then
  fail "network  the crate depends on a transport"
else
  pass "network  and its dependency list carries no transport"
fi

store_report check-catalogue/1 cases "$JSON"
