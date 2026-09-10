#!/bin/sh
# check-bitcheck.sh - does the Go checking binary refuse what it must, and does
# it answer exactly what the shell halves answered over the same planted defect?
#
# CI-10 ports the checking layer to one Go binary and DELETES the twin layer
# rather than translating it. ⛔ The entry's own bound on that is why this file
# exists: *a ported check refuses exactly what its shell half refused, over the
# same plants, with the same 0/1/2 vocabulary - and is compared case for case
# against those halves BEFORE they are deleted.*
#
# -- ⭐ ONE CASE LIST, TWO MODES, AND THAT IS THE WHOLE DESIGN ---------------
#
# The case list is the value here and it is written once. What changes is how
# many implementations each case is asked of:
#
#   default     the plants are run against the Go binary alone. Milliseconds.
#               ⭐ THIS IS THE PERMANENT GATE ROW, and it goes on being one after
#               every shell half has been deleted, because a plant does not need
#               a second implementation to be a plant.
#   --compare   each case additionally runs the .sh half and the .ps1 half and
#               refuses any difference in exit code or in the --json line.
#               ⛔ THIS IS THE PRE-DELETION PROOF and it is deliberately NOT in
#               the gate: check-control-bytes.ps1 alone is 67.9 seconds, so
#               running it once per case would cost more than the twin layer this
#               entry exists to remove.
#
# ⚠ A Go harness compared only against itself is the self-consistency OBS-07 was
# opened for, arriving in the checking layer. --compare is the answer to that,
# and TODO/ci.md records the run rather than leaving it to a session to repeat.
#
# -- ⛔ WHY A CLEAN TREE PROVES ALMOST NOTHING HERE --------------------------
#
# check-twins.sh compares two halves' ANSWERS on the tree it runs against, and
# its own header records what that misses. Measured on 2026-08-28 with three
# divergences planted in a .ps1 half: a changed ceiling was caught, a dropped
# `md` scope was caught, and a dropped `py` scope was INVISIBLE, because this
# tree holds no .py file and the smaller scope produced an identical number.
#
# ⭐ So every case below is a PLANT, and the clean tree is a control either side
# rather than the measurement. A rule that differs only on a defect the tree does
# not contain is exactly what this is built to see.
#
# ⚠ AND THAT BLIND SPOT REPRODUCED HERE ON ITS FIRST MUTATION PASS. Adding DEL to
# the Go control class left every case green, because nothing in the tree and
# nothing planted carried the byte. The repair is the fixture below, not a
# reading, and the case says so where it sits.
#
# -- ⚠ A PLANT THE CHECKS ALL ACCEPT IS STILL A CASE ------------------------
#
# Half the cases here plant something the rule must NOT fire on: a specimen
# inside a fenced block, a leading byte-order mark, a control byte in a file that
# is not text. Those are where a port is likeliest to be over-strict, and a
# harness carrying only refusals would never look at them.
#
# Usage:
#   sh scripts/common/check-bitcheck.sh
#   sh scripts/common/check-bitcheck.sh --compare
#   sh scripts/common/check-bitcheck.sh --json
#
# Exit codes: 0 every case held, 1 one did not, 2 could not run.
#
# ⛔ Read the exit code from this process, unpiped.

set -u

JSON=0
COMPARE=0
while [ $# -gt 0 ]; do
  case "$1" in
    --json) JSON=1 ;;
    --compare) COMPARE=1 ;;
    -h | --help)
      awk 'NR>1 { if (/^#/) { sub(/^# ?/, ""); print } else exit }' "$0"
      exit 0
      ;;
    *)
      printf 'check-bitcheck: unknown argument: %s\n' "$1" >&2
      exit 2
      ;;
  esac
  shift
done

# ⛔ Resolved from this script's own location, never from the working directory.
HERE=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
ROOT=$(CDPATH='' cd -- "$HERE/../.." && pwd)

# shellcheck disable=SC2034
ME=check-bitcheck
# shellcheck disable=SC1091
# shellcheck source=scripts/corpus/store-lib.sh
. "$ROOT/scripts/corpus/store-lib.sh"

store_require git go tar

WORK=$(store_workdir checkbitcheck) || exit 2
trap 'rm -rf "$WORK"' EXIT INT TERM

TREE="$WORK/tree"
BIN="$WORK/bit-check"

# ⭐ THE BINARY IS BUILT FROM THE TREE ON DISK, not from a release. A harness
# comparing a stale binary against fresh shell halves would report a drift that
# the tree does not have, or miss one it does.
if ! (cd "$ROOT/tools/check" && go build -o "$BIN" .) >"$WORK/build.log" 2>&1; then
  printf 'check-bitcheck: cannot build tools/check\n' >&2
  sed 's/^/  /' "$WORK/build.log" >&2
  exit 2
fi

# ⚠ pwsh IS OPTIONAL AND ITS ABSENCE IS SAID RATHER THAN ASSUMED. On a host
# without it the PowerShell column simply is not compared, and the row says so,
# because a comparison that silently dropped a participant is a comparison
# reporting agreement between one implementation and itself.
HAVE_PWSH=0
command -v pwsh >/dev/null 2>&1 && HAVE_PWSH=1

# -- the scratch tree ---------------------------------------------------------
#
# ⚠ Tracked AND untracked-but-not-ignored, because an uncommitted new file is
# part of the tree the next push carries, and it is in scope for every rule here.
mkdir -p "$TREE" || exit 2
(
  cd "$ROOT" || exit 1
  {
    git ls-files -z
    git ls-files -zo --exclude-standard
  } | tar --null -T - -cf -
) | (cd "$TREE" && tar -xf -) || {
  printf 'check-bitcheck: cannot copy the working tree\n' >&2
  exit 2
}

# ⛔ IT IS A REAL REPOSITORY, because every check here asks git what is in the
# tree. A plain directory would make all three implementations exit 2 and the
# whole run would be three harnesses agreeing that they could not run.
(
  cd "$TREE" || exit 1
  git init -q -b main &&
    git add -A &&
    git -c user.email=gate -c user.name=gate commit -qm scratch
) >/dev/null 2>&1 || {
  printf 'check-bitcheck: cannot make the scratch tree a repository\n' >&2
  exit 2
}

# -- running one implementation ----------------------------------------------
#
# ⛔ EACH EXIT CODE IS READ FROM THE PROCESS THAT PRODUCED IT, UNPIPED. The
# output goes to a file and the status is read on the next line. This whole
# harness is about comparing statuses, so a pipeline's status here would compare
# three zeroes and call it agreement.
OUTF="$WORK/out"

run_go() { # check
  (cd "$TREE" && "$BIN" "$1" --json) >"$OUTF" 2>&1
  printf '%s\t%s\n' "$?" "$(cat "$OUTF")"
}

run_sh() { # relative-path flags...
  _p=$1
  shift
  (cd "$TREE" && sh "$TREE/scripts/$_p.sh" "$@") >"$OUTF" 2>&1
  printf '%s\t%s\n' "$?" "$(cat "$OUTF")"
}

run_ps() { # relative-path flags...
  _p=$1
  shift
  (cd "$TREE" && pwsh -NoProfile -File "$TREE/scripts/$_p.ps1" "$@") >"$OUTF" 2>&1
  printf '%s\t%s\n' "$?" "$(cat "$OUTF")"
}

# -- the comparison -----------------------------------------------------------
#
# ⛔ THE EXPECTED CODE IS AN ARGUMENT, AND THAT IS THE HALF THAT MAKES THIS A
# PROOF. Three implementations that all answer 0 over a plant agree perfectly and
# have all missed it. So a case states what the plant is supposed to do, and a
# unanimous wrong answer is a failure rather than agreement.
agree() { # label check sh-rel expected-code [sh-flags...]
  _label=$1
  _check=$2
  _rel=$3
  _want=$4
  shift 4

  _g=$(run_go "$_check")
  _gc=${_g%%	*}
  _gj=${_g#*	}

  if [ "$_gc" != "$_want" ]; then
    fail "$_label: go answered $_gc, expected $_want"
    return
  fi

  # ⭐ THE DEFAULT MODE STOPS HERE, and it is still a proof. The plant landed, the
  # Go binary answered the code the case named, and no shell process was started.
  # ⚠ What it does NOT establish is equivalence to the halves; --compare is the
  # only thing that does, and the row says which of the two it ran.
  if [ "$COMPARE" = "0" ]; then
    pass "$_label: go (exit $_want)"
    return
  fi

  # ⛔ A HALF THAT IS GONE IS NOT A HALF THAT AGREED. Once a twin is deleted this
  # case has one fewer implementation to compare, and saying so in the row is the
  # difference between a comparison that shrank and one that passed.
  if [ ! -f "$TREE/scripts/$_rel.sh" ]; then
    pass "$_label: go (exit $_want); the sh half is gone, not compared"
    return
  fi

  _s=$(run_sh "$_rel" --json "$@")
  _sc=${_s%%	*}
  _sj=${_s#*	}

  if [ "$_sc" != "$_want" ]; then
    fail "$_label: sh answered $_sc, expected $_want"
    return
  fi
  if [ "$_gj" != "$_sj" ]; then
    fail "$_label: go and sh disagree
        go: $_gj
        sh: $_sj"
    return
  fi

  if [ "$HAVE_PWSH" = "0" ] || [ ! -f "$TREE/scripts/$_rel.ps1" ]; then
    pass "$_label: go and sh agree (exit $_want); no PowerShell half compared"
    return
  fi

  # ⚠ The PowerShell flag spelling differs from the sh one, so it is passed
  # rather than derived. A label built from a flag made two runners' row lists
  # differ on a row they both have, which check-gate.sh records.
  _pflags=""
  for _f in "$@"; do
    case "$_f" in
      --public) _pflags="-Public" ;;
    esac
  done
  # shellcheck disable=SC2086
  _p=$(run_ps "$_rel" -Json $_pflags)
  _pc=${_p%%	*}
  _pj=${_p#*	}

  if [ "$_pc" != "$_want" ]; then
    fail "$_label: pwsh answered $_pc, expected $_want"
    return
  fi
  if [ "$_gj" != "$_pj" ]; then
    fail "$_label: go and pwsh disagree
        go:   $_gj
        pwsh: $_pj"
    return
  fi
  pass "$_label: all three agree (exit $_want)"
}

# -- planting -----------------------------------------------------------------
#
# ⛔ A PLANT IS REMOVED BY THE CASE THAT MADE IT, and the control after it is what
# says the removal worked. A harness whose plants accumulate measures the sum of
# everything it has done so far, and every case after the first is then reporting
# a tree nobody chose.
unplant() { # relative-path
  rm -f "$TREE/$1"
}

# ============================================================================
# check-control-bytes
# ============================================================================

agree "control-bytes clean tree" check-control-bytes common/check-control-bytes 0

# ⛔ A C0 CONTROL BYTE IN A TEXT FILE. The byte is written by printf rather than
# as a literal, which is this repository's own rule: a literal control byte in
# this harness would make THIS file the thing it exists to refuse.
printf 'plant\001here\n' >"$TREE/tools/check/plant.txt"
agree "control-bytes a C0 byte is refused" check-control-bytes common/check-control-bytes 1
unplant tools/check/plant.txt

# ⛔ NUL IS A SEPARATE TEST, because it cannot live in a shell variable and the
# shell half therefore found it by a different route. A port that folded the two
# would pass every case above and miss the commonest offender there is.
printf 'plant\000here\n' >"$TREE/tools/check/plant.txt"
agree "control-bytes a NUL byte is refused" check-control-bytes common/check-control-bytes 1
unplant tools/check/plant.txt

# ⭐ THE THREE THAT ARE LEGITIMATELY IN TEXT. This is the case a port is likeliest
# to get wrong in the strict direction, and a harness of refusals alone would
# never look at it.
printf 'tab\there\r\nand a return\n' >"$TREE/tools/check/plant.txt"
agree "control-bytes tab, CR and LF are accepted" check-control-bytes common/check-control-bytes 0
unplant tools/check/plant.txt

# ⛔ DEL IS OUTSIDE THE CLASS, AND THIS CASE EXISTS BECAUSE A MUTATION SURVIVED
# WITHOUT IT. Measured on 2026-09-10 while proving this harness: adding 0x7f to
# the Go class left every case above green and the run exited 0, because nothing
# in the tree and nothing planted carried the byte. ⚠ That is check-twins' own
# documented blind spot arriving in its replacement - a rule that differs only on
# a defect the tree does not contain is invisible to any comparison over it - and
# the repair is a fixture rather than a reading.
printf 'plant\177here\n' >"$TREE/tools/check/plant.txt"
agree "control-bytes DEL is outside the class and accepted" check-control-bytes common/check-control-bytes 0
unplant tools/check/plant.txt

# ⚠ SCOPE IS PROVED WITH A FIXTURE RATHER THAN TRUSTED. The same byte in a file
# the extension list does not call text must be accepted, and a scope rule is
# invisible to any comparison over a tree that lacks the shape.
printf 'plant\001here\n' >"$TREE/tools/check/plant.bin"
agree "control-bytes a byte outside the text extensions is out of scope" check-control-bytes common/check-control-bytes 0
unplant tools/check/plant.bin

agree "control-bytes clean tree again" check-control-bytes common/check-control-bytes 0

# ============================================================================
# check-markers
# ============================================================================

agree "markers clean tree" check-markers common/check-markers 0

# An em dash, U+2014, which is outside the five and is the character the rule
# was originally written about.
DASH=$(printf '\342\200\224')
STOP=$(printf '\342\233\224')

printf 'a plant %s here\n' "$DASH" >"$TREE/tools/check/plant.md"
agree "markers a character outside the five is refused" check-markers common/check-markers 1
unplant tools/check/plant.md

# ⭐ THE SPECIMEN EXEMPTION, and it is markdown only. Without it the rule is
# unwritable: a page that bans a character cannot show a reader which character
# it means.
{
  printf 'text\n\n'
  printf '```\n'
  printf 'a specimen %s inside a fence\n' "$DASH"
  printf '```\n'
} >"$TREE/tools/check/plant.md"
agree "markers a specimen inside a fenced block is accepted" check-markers common/check-markers 0
unplant tools/check/plant.md

# shellcheck disable=SC2016
# The backticks are the SUBJECT here rather than a substitution: this case exists
# to put a specimen inside an inline code span. ⚠ The directive is in this file
# rather than in the invocation, because shellcheck answers differently depending
# on how files were grouped on its command line and a contributor checking one
# file does not group them the way CI does.
printf 'an inline `%s` span\n' "$DASH" >"$TREE/tools/check/plant.md"
agree "markers a specimen inside a code span is accepted" check-markers common/check-markers 0
unplant tools/check/plant.md

# ⛔ AND THERE IS NO EXEMPTION OUTSIDE MARKDOWN. A source file has no reader who
# needs a specimen, so the same bytes in a .sh are refused - which is the branch
# that separates the two scopes.
printf '# a plant %s here\n' "$DASH" >"$TREE/tools/check/plant.sh"
agree "markers the same character in a .sh is refused" check-markers common/check-markers 1
unplant tools/check/plant.sh

# ⚠ A LEADING BYTE-ORDER MARK IS EXEMPT AND ONLY A LEADING ONE. Both directions
# are planted, because they are different branches and a port that stripped every
# BOM would pass the first and lose the second.
printf '\357\273\277ascii only\n' >"$TREE/tools/check/plant.md"
agree "markers a leading BOM is accepted" check-markers common/check-markers 0
unplant tools/check/plant.md

printf 'first line\n\357\273\277second line\n' >"$TREE/tools/check/plant.md"
agree "markers a BOM anywhere else is refused" check-markers common/check-markers 1
unplant tools/check/plant.md

# ⛔ THE DENSITY CEILING IS A SEPARATE RULE OVER THE SAME FILES, and a file made
# entirely of permitted characters is how it is reached: every byte here is one
# of the five, so the character rule holds and only the count refuses it.
{
  printf 'a line with markers %s%s%s%s%s%s%s%s%s%s\n' "$STOP" "$STOP" "$STOP" \
    "$STOP" "$STOP" "$STOP" "$STOP" "$STOP" "$STOP" "$STOP"
} >"$TREE/tools/check/plant.md"
agree "markers a file over the density ceiling is refused" check-markers common/check-markers 1
unplant tools/check/plant.md

# ⛔ LICENSES/*.txt IS EXEMPT AND THIS TREE HAS NO SUCH FILE, so the exemption is
# proved with a fixture. check-markers.sh's own header is where the lesson comes
# from: a `py` scope dropped from one half was invisible to check-twins because
# the tree held no .py file at all.
mkdir -p "$TREE/LICENSES"
printf 'a canonical text with a dash %s in it\n' "$DASH" >"$TREE/LICENSES/PLANT.txt"
agree "markers LICENSES/*.txt is exempt" check-markers common/check-markers 0
rm -rf "$TREE/LICENSES"

# ⚠ AND THE EXEMPTION IS THE DIRECTORY AND THE EXTENSION TOGETHER. The same bytes
# under LICENSES with a different extension are in scope, which is what says the
# rule is anchored rather than matching the word anywhere in a path.
mkdir -p "$TREE/LICENSES"
printf 'a dash %s in a markdown file\n' "$DASH" >"$TREE/LICENSES/PLANT.md"
agree "markers LICENSES/*.md is NOT exempt" check-markers common/check-markers 1
rm -rf "$TREE/LICENSES"

agree "markers clean tree again" check-markers common/check-markers 0

# ============================================================================
# check-one-home
# ============================================================================

agree "one-home clean tree" check-one-home common/check-one-home 0

# A sentence of twelve or more words, which is the length below which a repeated
# phrase is not a fact.
LONG='the capture host is claimed before anything else writes to it and the route is cut afterwards'
SHORT='the capture host is claimed first'

printf '%s\n' "$LONG" >"$TREE/tools/check/plant-a.md"
printf '%s\n' "$LONG" >"$TREE/tools/check/plant-b.md"
agree "one-home one sentence in two documents is refused" check-one-home common/check-one-home 1
unplant tools/check/plant-a.md
unplant tools/check/plant-b.md

# ⚠ THE SAME SENTENCE TWICE IN ONE DOCUMENT IS NOT A DUPLICATE. The shell half
# ran `sort -u` over (sentence, file) pairs before counting, so this is the case
# that separates "in two documents" from "twice". A port that counted occurrences
# rather than distinct files would refuse every document that repeats a heading.
printf '%s\n\n%s\n' "$LONG" "$LONG" >"$TREE/tools/check/plant-a.md"
agree "one-home the same sentence twice in ONE document is accepted" check-one-home common/check-one-home 0
unplant tools/check/plant-a.md

# ⚠ UNDER THE WORD FLOOR, SHARED. A short phrase in two documents is a phrase,
# not a fact, and refusing it would fire on every correct tree.
printf '%s\n' "$SHORT" >"$TREE/tools/check/plant-a.md"
printf '%s\n' "$SHORT" >"$TREE/tools/check/plant-b.md"
agree "one-home a sentence under the word floor is accepted" check-one-home common/check-one-home 0
unplant tools/check/plant-a.md
unplant tools/check/plant-b.md

# ⛔ ROUTERS ARE EXEMPT AS A SET AND THIS TREE CARRIES NEITHER OF THEM at the
# paths the rule names - it has docs/AGENTS.md, which is NOT in that set - so the
# exemption is proved with a fixture rather than assumed from the tree.
printf '%s\n' "$LONG" >"$TREE/AGENTS.md"
printf '%s\n' "$LONG" >"$TREE/ROUTE.md"
agree "one-home a sentence shared only between routers is accepted" check-one-home common/check-one-home 0
unplant AGENTS.md

# ⛔ AND THE EXEMPTION IS *EVERY* FILE, NOT *ANY*. With one router and one
# ordinary document the same sentence is the defect, which is the branch that
# says the test is over the whole set.
printf '%s\n' "$LONG" >"$TREE/tools/check/plant-a.md"
agree "one-home a router sharing with an ordinary document is refused" check-one-home common/check-one-home 1
unplant tools/check/plant-a.md
unplant ROUTE.md

# ⚠ docs/history/ IS OUT OF SCOPE, because superseded reasoning is where a
# repeated sentence is correct: the history says what was believed then and the
# current document says what is true now.
printf '%s\n' "$LONG" >"$TREE/tools/check/plant-a.md"
mkdir -p "$TREE/docs/history"
printf '%s\n' "$LONG" >"$TREE/docs/history/PLANT.md"
agree "one-home docs/history is out of scope" check-one-home common/check-one-home 0
unplant tools/check/plant-a.md
unplant docs/history/PLANT.md

# ⭐ A SHARED SENTENCE INSIDE A FENCED BLOCK IS NOT A SHARED SENTENCE. Two
# documents quoting one command are quoting it, and the fence is what says so.
for p in plant-a plant-b; do
  {
    printf '```\n'
    printf '%s\n' "$LONG"
    printf '```\n'
  } >"$TREE/tools/check/$p.md"
done
agree "one-home a sentence shared inside fenced blocks is accepted" check-one-home common/check-one-home 0
unplant tools/check/plant-a.md
unplant tools/check/plant-b.md

agree "one-home clean tree again" check-one-home common/check-one-home 0

# ============================================================================
# check-changelog
# ============================================================================

agree "changelog clean tree" check-changelog common/check-changelog 0

CL="$TREE/CHANGELOG.md"
cp "$CL" "$WORK/CHANGELOG.orig" || exit 2
restore_changelog() { cp "$WORK/CHANGELOG.orig" "$CL"; }

# ⛔ THE ORDERING RESETS AT EVERY SECTION, and this is the control for that: an
# Unreleased section above a versioned one is CORRECT even though its date is
# older, so a port comparing across the boundary refuses a right answer.
{
  printf '# Changelog\n\n## Unreleased\n\n'
  printf '### 2026-01-01T00:00:00Z\n\n- Record: TODO/ci.md. No deploy.\n\n'
  printf '## 1.0.0\n\n'
  printf '### 2026-06-01T00:00:00Z\n\n- Record: TODO/ci.md. No deploy.\n'
} >"$CL"
agree "changelog an older date in a later section is accepted" check-changelog common/check-changelog 0

{
  printf '# Changelog\n\n## Unreleased\n\n'
  printf '### 2026-01-01T00:00:00Z\n\n- Record: TODO/ci.md. No deploy.\n\n'
  printf '### 2026-06-01T00:00:00Z\n\n- Record: TODO/ci.md. No deploy.\n'
} >"$CL"
agree "changelog two entries out of order in ONE section is refused" check-changelog common/check-changelog 1

{
  printf '# Changelog\n\n## Unreleased\n\n'
  printf '### the day it happened\n\n- Record: TODO/ci.md. No deploy.\n'
} >"$CL"
agree "changelog an entry with no date is refused" check-changelog common/check-changelog 1

{
  printf '# Changelog\n\n## Unreleased\n\n'
  printf '### 2026-06-01T00:00:00Z\n\n- Something happened. No deploy.\n'
} >"$CL"
agree "changelog an entry naming no record is refused" check-changelog common/check-changelog 1

{
  printf '# Changelog\n\n## Unreleased\n\n'
  printf '### 2026-06-01T00:00:00Z\n\n- Record: TODO/ci.md.\n'
} >"$CL"
agree "changelog an entry silent about deployment is refused" check-changelog common/check-changelog 1

# ⛔ A FILE THAT PARSES TO NO ENTRIES IS A FAILURE, NOT A CLEAN RUN, and this
# repository shipped exactly that shape for as long as it took somebody to
# notice. ⚠ It is the one refusal here that emits no JSON at all - there is no
# `problems` count to report - so it is also the case that says the port carried
# the stderr path rather than inventing an answer.
{
  printf '# Changelog\n\n## 2026-06-01T00:00:00Z\n\n- Record: TODO/ci.md. No deploy.\n'
} >"$CL"
agree "changelog a file with no entries the parser knows is refused" check-changelog common/check-changelog 1

restore_changelog
agree "changelog clean tree again" check-changelog common/check-changelog 0

# ============================================================================

store_report check-bitcheck/1 cases \
  "$([ "$JSON" = "1" ] && printf 1 || printf 0)"
