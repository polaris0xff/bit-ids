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

# ⚠ THE MODE FLAGS GO TO THE GO HALF VERBATIM, and that is a property of the
# binary rather than a convenience here: `--public` is spelled the same way the
# `sh` half spells it, so a case states one flag and two of the three
# implementations take it unchanged. Only PowerShell needs a translation, and
# `agree` does that one below rather than deriving all three from a label.
run_go() { # check flags...
  _c=$1
  shift
  (cd "$TREE" && "$BIN" "$_c" --json "$@") >"$OUTF" 2>&1
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

  _g=$(run_go "$_check" "$@")
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

  # ⛔ A RULE THAT NEVER HAD A SHELL HALF IS NOT A RULE WHOSE HALF AGREED, and it
  # is not one whose half was deleted either. `-` says so explicitly rather than
  # letting a new Go-only rule borrow the wording of a completed port: there is
  # no predecessor to compare, which is a different fact from having outlived one.
  if [ "$_rel" = "-" ]; then
    pass "$_label: go (exit $_want); a Go-only rule, so there is no half to compare"
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

# ⛔ SOME PLANTS HAVE TO BE TRACKED, AND .gitignore IS WHY. check-no-secrets'
# first rule fires on a credential FILE, and this repository ignores most of the
# names it knows - `*.pem`, `*.key`, `id_rsa` - so a plant left untracked is
# ignored, out of scope, and the case would report a rule holding over a file it
# never saw. ⭐ `git add -f` is also the truer fixture: the rule's own title is
# *a credential file is TRACKED*, which is what happens when a name is force-added
# or was added before the ignore rule existed.
plant_tracked() { # relative-path
  (cd "$TREE" && git add -f -- "$1")
}

# ⛔ AND IT LEAVES THE INDEX AS IT FOUND IT. Deleting the file alone leaves a
# tracked-but-deleted path, which `git ls-files` still reports - and rule 1 reads
# a PATH rather than a file, so the finding would survive the unplant and every
# case after it would be measuring a tree nobody chose.
unplant_tracked() { # relative-path
  (cd "$TREE" && git rm -q --cached --ignore-unmatch -- "$1")
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
# proved with a fixture. The lesson comes from check-markers' own deleted shell
# header: a `py` scope dropped from one half was invisible to check-twins because
# the tree held no .py file at all. TODO/ci.md carries it under CI-10.
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
# check-licences
# ============================================================================

agree "licences clean tree" check-licences common/check-licences 0

REG="$TREE/catalogue/licences.toml"
CAT="$TREE/catalogue/clients.toml"
cp "$REG" "$WORK/licences.orig" || exit 2
cp "$CAT" "$WORK/clients.orig" || exit 2
restore_licences() {
  cp "$WORK/licences.orig" "$REG"
  cp "$WORK/clients.orig" "$CAT"
}

# ⛔ A CATALOGUE TARGET WITH NO REGISTER ROW. The comparison runs in both
# directions and this is the direction that matters: a target added and its
# licence forgotten is a build nobody decided the disposition of.
{
  cat "$WORK/clients.orig"
  printf '\n[[targets]]\nid = "zz-plant"\nopen_source = true\n'
} >"$CAT"
agree "licences a catalogue target with no register row is refused" check-licences common/check-licences 1
restore_licences

# ⚠ AND THE OTHER DIRECTION, which is a different branch and would survive a
# check that only walked the catalogue.
{
  cat "$WORK/licences.orig"
  printf '\n[[targets]]\nid = "zz-plant"\nlicence = "MIT"\nlicence_source = "x"\nredistribute = "refused"\n'
} >"$REG"
agree "licences a register row naming no catalogue target is refused" check-licences common/check-licences 1
restore_licences

# ⛔ PERMITTED IS THE EXPENSIVE VALUE AND IT HAS TO BE EARNED. `unverified` plus
# `permitted` is the combination that would publish somebody's bytes on nobody's
# authority, and it is a separate rule from "has a disposition at all".
sed 's/^redistribute = "refused"$/redistribute = "permitted"/' "$WORK/licences.orig" >"$REG"
agree "licences permitted without a verified licence and a notice is refused" check-licences common/check-licences 1
restore_licences

# ⛔ A REGISTER OF NOTHING IS REFUSED - AND THIS CASE DOES NOT ESTABLISH BY WHICH
# RULE, WHICH IS A FINDING RATHER THAN A DETAIL. Measured on 2026-09-10 by
# planting: with the empty-register refusal removed outright, every case here
# still passed. The plant is still refused, because a register with no rows also
# has no row for any catalogue target and rule 1 fires on that.
#
# ⚠ SO THE EMPTY-REGISTER GUARD IS DEFENCE IN DEPTH AND ITS REFUSAL IS NEVER
# ALONE. It is unreachable as a sole cause over a real catalogue: any register
# empty enough to trigger it has already failed the two comparisons. It is kept
# and recorded rather than removed, the way CI-03 keeps a marker's exclusive
# create - a guard nothing can refute is still the guard that matters if a parser
# ever stops matching. ⛔ What this case is NOT is proof that it works, and the
# label says so instead of claiming it.
printf 'schema = "bit-ids/licences/1"\n' >"$REG"
agree "licences an empty register is refused, by rule 1 rather than by the empty guard" check-licences common/check-licences 1
restore_licences

agree "licences clean tree again" check-licences common/check-licences 0

# ============================================================================
# check-placeholders
# ============================================================================

# ⛔ EVERY NEEDLE BELOW IS ASSEMBLED RATHER THAN WRITTEN, and that is not style.
# A harness that spelled a stand-in value or a double brace as a literal would BE
# the finding: check-placeholders reads every tracked file, this file is one, and
# its first version turned the clean tree red for exactly that reason. ⚠ The three
# implementations of the check are exempt from themselves; a harness is not, and
# exempting one more file is a worse answer than not planting the literal.
# ⭐ It is the same rule the marker harness already followed, by building its
# marker bytes with printf instead of typing them.
BR=$(printf '%s%s' '{' '{')
BRC=$(printf '%s%s' '}' '}')

agree "placeholders clean tree" check-placeholders common/check-placeholders 0

# ⛔ THE FOUR CATEGORIES ARE FOUR RULES and each is planted on its own, because a
# harness that planted them together could not tell which one fired.
printf 'a survived %sPLACEHOLDER%s here\n' "$BR" "$BRC" >"$TREE/tools/check/plant.md"
agree "placeholders a double-brace placeholder is refused" check-placeholders common/check-placeholders 1
unplant tools/check/plant.md

printf 'text\n<!-- %s: fill this in -->\n' 'TEMPLATE' >"$TREE/tools/check/plant.md"
agree "placeholders a template guidance comment is refused" check-placeholders common/check-placeholders 1
unplant tools/check/plant.md

printf 'contact %s%s for details\n' 'CHANGE' 'ME' >"$TREE/tools/check/plant.md"
agree "placeholders a stand-in value is refused" check-placeholders common/check-placeholders 1
unplant tools/check/plant.md

# ⚠ THE OWNER-SLASH-REPO GENERIC IS RECOMMENDED IN A PUBLIC DOCUMENT, so it is a
# defect only where it is configuration rather than prose. Both are planted,
# because they are different branches and a rule that fired on both would fire on
# correct writing.
printf 'clone %s/%s to begin\n' 'OWNER' 'REPO' >"$TREE/tools/check/plant.md"
agree "placeholders the generic in a .md is accepted" check-placeholders common/check-placeholders 0
unplant tools/check/plant.md

printf 'repo: %s/%s\n' 'OWNER' 'REPO' >"$TREE/tools/check/plant.yml"
agree "placeholders the generic in a config file is refused" check-placeholders common/check-placeholders 1
unplant tools/check/plant.yml

# ⭐ THE TWO SHAPES THAT MUST NOT FIRE, and they are why the brace rule is narrow
# rather than absent. GitHub Actions expression syntax and a Go template both
# carry a double brace, and a rule that refused either would refuse every correct
# workflow file and every container format string in the tree.
printf 'run: echo $%s github.sha %s\n' "$BR" "$BRC" >"$TREE/tools/check/plant.yml"
agree "placeholders an Actions expression is accepted" check-placeholders common/check-placeholders 0
unplant tools/check/plant.yml

printf 'info --format %sjson .Host.Arch%s\n' "$BR" "$BRC" >"$TREE/tools/check/plant.yml"
agree "placeholders a Go template is accepted" check-placeholders common/check-placeholders 0
unplant tools/check/plant.yml

agree "placeholders clean tree again" check-placeholders common/check-placeholders 0

# ============================================================================
# check-no-secrets
# ============================================================================
#
# ⛔ EVERY NEEDLE BELOW IS ASSEMBLED AND NONE OF THEM IS WRITTEN, for the reason
# the placeholder needles are: check-no-secrets reads every file in this tree,
# this harness is one of them, and a token shape or a forty-digit run typed here
# as a literal would make THIS file the finding. ⚠ A credential's SHAPE is
# exactly what it looks for, so the rule bites harder here than anywhere else -
# a bare `AKIA` and sixteen characters would turn the clean-tree control red and
# the harness would be refusing itself.
AKID=$(printf '%s%s%s' 'AKI' 'A' 'IOSFODNN7EXAMPLE')
KEYBLOCK=$(printf '%s %s' 'BEGIN' 'PRIVATE KEY')
URLCRED=$(printf '%s//%s:%s@%s' 'https:' 'bob' 'hunter22x' 'example.invalid')
MAIL=$(printf '%s@%s' 'someone' 'example.invalid')
HOMEP=$(printf '%s/%s/work' '/home' 'alice')
HOMEG=$(printf '%s/%s/work' '/home' 'runner')

# ⚠ SIXTEEN HEX DIGITS IS UNDER THE TWENTY-FOUR FLOOR, which is what lets the
# pieces be written and the assembled runs not be.
HX=0123456789abcdef
HEX40=$(printf '%s%s%s' "$HX" "$HX" '01234567')
HEX64=$(printf '%s%s%s%s' "$HX" "$HX" "$HX" "$HX")
HEX46=$(printf '%s%s' "$HEX40" '012345')

agree "no-secrets clean tree" check-no-secrets common/check-no-secrets 0

# ⛔ A CREDENTIAL FILE, TRACKED. `id_ecdsa` and `*.jks` are the two names the
# check knows that .gitignore does not carry, so an untracked plant of either is
# genuinely in scope - but it is force-added anyway, because the rule is about a
# file that reached the index and a fixture that only works while an ignore list
# has a hole is a fixture that breaks when the hole is closed.
printf 'not a real key\n' >"$TREE/tools/check/id_ecdsa"
plant_tracked tools/check/id_ecdsa
agree "no-secrets a tracked credential file is refused" check-no-secrets common/check-no-secrets 1
unplant_tracked tools/check/id_ecdsa

# ⭐ AND THE WAIVER, WHICH IS A DIFFERENT BRANCH. `.env.example` matches the same
# name rule and is then dropped by the `.example` exclusion; without that branch
# the rule fires on the file a project is SUPPOSED to commit. ⚠ .gitignore
# un-ignores this one name explicitly, so it reaches the scan on its own.
printf 'TOKEN=\n' >"$TREE/tools/check/.env.example"
agree "no-secrets an .example credential template is accepted" check-no-secrets common/check-no-secrets 0
unplant tools/check/.env.example

printf 'key %s here\n' "$AKID" >"$TREE/tools/check/plant.md"
agree "no-secrets an aws access key id is refused" check-no-secrets common/check-no-secrets 1
unplant tools/check/plant.md

printf -- '-----%s-----\n' "$KEYBLOCK" >"$TREE/tools/check/plant.md"
agree "no-secrets a private key block is refused" check-no-secrets common/check-no-secrets 1
unplant tools/check/plant.md

printf 'clone %s now\n' "$URLCRED" >"$TREE/tools/check/plant.md"
agree "no-secrets a password in a url is refused" check-no-secrets common/check-no-secrets 1
unplant tools/check/plant.md

# ⛔ AN EMAIL IS NOT A DEFAULT-MODE FINDING, and this is the case that separates
# the two modes rather than treating --public as "the same rules, stricter". In a
# private project an address is legitimate content.
printf 'write to %s\n' "$MAIL" >"$TREE/tools/check/plant.md"
agree "no-secrets an email address is accepted without --public" check-no-secrets common/check-no-secrets 0
unplant tools/check/plant.md

# ⚠ A BINARY FILE IS SKIPPED, which is what `grep -I` does, and the plant carries
# a real key so the case cannot pass by the file being uninteresting. ⛔ The byte
# is written by printf rather than as a literal, this repository's own rule.
printf 'key %s\000here\n' "$AKID" >"$TREE/tools/check/plant.md"
agree "no-secrets a key inside a binary file is out of scope" check-no-secrets common/check-no-secrets 0
unplant tools/check/plant.md

agree "no-secrets clean tree again" check-no-secrets common/check-no-secrets 0

# -- the public rules ---------------------------------------------------------
#
# ⚠ --public IS A SECOND QUESTION AND THE FLAG IS PASSED TO ALL THREE. The Go
# binary and the sh half spell it the same way; `agree` translates it for the
# PowerShell half, because only that one differs.

agree "no-secrets clean tree, public" check-no-secrets common/check-no-secrets 0 --public

printf 'write to %s\n' "$MAIL" >"$TREE/tools/check/plant.md"
agree "no-secrets an email address is refused with --public" check-no-secrets common/check-no-secrets 1 --public
unplant tools/check/plant.md

printf 'built in %s\n' "$HOMEP" >"$TREE/tools/check/plant.md"
agree "no-secrets an absolute home path is refused" check-no-secrets common/check-no-secrets 1 --public
unplant tools/check/plant.md

# ⚠ Narrowed rather than switched off: /home/runner/ is a well-known generic
# path, not a fingerprint of anybody's machine.
printf 'built in %s\n' "$HOMEG" >"$TREE/tools/check/plant.md"
agree "no-secrets a generic runner home path is accepted" check-no-secrets common/check-no-secrets 0 --public
unplant tools/check/plant.md

printf 'a bare %s here\n' "$HEX40" >"$TREE/tools/check/plant.md"
agree "no-secrets a bare long hex run is refused" check-no-secrets common/check-no-secrets 1 --public
unplant tools/check/plant.md

# ⚠ A PINNED ACTION IS THE SAFE PRACTICE and a rule that fired on correct
# hardening is a rule somebody disables.
printf 'uses: owner/repo@%s\n' "$HEX40" >"$TREE/tools/check/plant.yml"
agree "no-secrets a pinned action commit is accepted" check-no-secrets common/check-no-secrets 0 --public
unplant tools/check/plant.yml

# ⛔ THE CASE THAT SAYS THE ALLOW EXPRESSIONS READ THE OUTPUT LINE AND NOT THE
# TEXT. The lockfile allowance is anchored `^(.*Cargo\.lock:[0-9]+:checksum = )`,
# so it can only match with the `path:lineno:` prefix in front of it. A port that
# ran its allowances over the file's own line would refuse every registry digest
# in the tree, and every case above would still be green.
printf '%s "%s"\n' 'checksum =' "$HEX64" >"$TREE/tools/check/plant-Cargo.lock"
agree "no-secrets a registry lockfile digest is accepted" check-no-secrets common/check-no-secrets 0 --public
unplant tools/check/plant-Cargo.lock

# ⛔ AND THE CASE THAT SAYS AN ALLOWED ITEM IS DELETED FROM THE LINE RATHER THAN
# THE LINE BEING DROPPED. `grep -v` drops lines, not characters, so an allowed
# digest sitting beside a real one would take the real one out of the report with
# it. This plants exactly that pair on one line: an allowed `sha256:` digest and
# a bare run. A port that dropped the line answers 0 and every other case here
# still passes.
printf 'sha256:%s and %s\n' "$HEX64" "$HEX40" >"$TREE/tools/check/plant.md"
agree "no-secrets an allowed digest beside a bare one is still refused" check-no-secrets common/check-no-secrets 1 --public
unplant tools/check/plant.md

printf 'Infohash: %s\n' "$HEX40" >"$TREE/tools/check/plant.md"
agree "no-secrets an announce infohash is accepted" check-no-secrets common/check-no-secrets 0 --public
unplant tools/check/plant.md

# ⛔ THE TRAILING CLASS IS THE ANCHOR AND THIS IS WHAT IT IS FOR. Without it
# `{40}` blanks the first forty characters of a LONGER run and leaves a remainder
# too short to reach the threshold, so a forty-six digit value after the field
# name goes unreported. Both shell twins agreed on that and both were wrong.
printf 'Infohash: %s\n' "$HEX46" >"$TREE/tools/check/plant.md"
agree "no-secrets a longer run after the infohash field is refused" check-no-secrets common/check-no-secrets 1 --public
unplant tools/check/plant.md

agree "no-secrets clean tree again, public" check-no-secrets common/check-no-secrets 0 --public

# ============================================================================
# check-ignores
# ============================================================================
#
# ⭐ THE FIRST RULE HERE THAT WAS NEVER A SHELL CHECK, so its cases name `-` for
# the predecessor rather than a path: there is nothing to compare, which is a
# different fact from a half that has been deleted, and the row says which.
#
# ⚠ WHAT THESE CASES CANNOT REACH is the specimen self-check - the branch that
# fires when a shape is narrowed OUT of check-no-secrets while its specimen stays
# behind. That plant is a change to Go source, and this harness builds the binary
# once before the first case, so a case cannot rebuild it. ⛔ It is mutation-proved
# in TODO/ci.md against a plant verified to have changed the file, and recorded as
# a measurement rather than claimed as a case here.

agree "ignores clean tree" check-ignores - 0

# ⛔ A HOLE IN THE IGNORE LIST. This is the defect the rule exists for, and it is
# exactly the shape found by hand on 2026-09-15: a name check-no-secrets refuses
# that `git add -A` would stage anyway.
GI="$TREE/.gitignore"
cp "$GI" "$WORK/gitignore.orig" || exit 2
grep -v '^\*\.pem$' "$WORK/gitignore.orig" >"$GI"
agree "ignores a credential shape missing from .gitignore is refused" check-ignores - 1
cp "$WORK/gitignore.orig" "$GI"

# ⚠ AND MORE THAN ONE HOLE IS STILL ONE REFUSAL, which is worth a case because
# the report counts problems and the verdict is a single code. A port or a rewrite
# that stopped after the first finding would pass the case above and lose the rest
# of the report.
grep -vE '^(\*\.pem|id_rsa|credentials\.json)$' "$WORK/gitignore.orig" >"$GI"
agree "ignores several holes at once are still one refusal" check-ignores - 1
cp "$WORK/gitignore.orig" "$GI"

agree "ignores clean tree again" check-ignores - 0

# ============================================================================
# check-adapters
# ============================================================================
#
# ⭐ THE SECOND RULE HERE THAT WAS NEVER A SHELL CHECK, so its cases name `-` for
# the predecessor. ⛔ It exists because `capture-client` runs 21 and 22 both hung
# in *Install the client* on the RELEASE lane - six seconds on runs 19 and 20 -
# and an unbounded fetch is the only thing in that route that can wait forever.
# `docs/conventions/shell.md` section 9 had stated the rule for as long as four
# adapters had been breaking it.

agree "adapters clean tree" check-adapters - 0

# ⛔ A RELEASE FETCH WITH NO TIME LIMIT, which is the exact shape found on
# 2026-09-15 in four adapters at once.
#
# ⚠ IT IS PLANTED IN THE INSTALLER NOW, BECAUSE THAT IS WHERE THE FETCH WENT.
# `ACQ-06` moved every release retrieval out of the four adapters and into one
# rootless installer, so this plant found nothing to change there - and the rule
# answered 2 over a scope that no longer held a single `curl -o`. ⭐ The rule's
# own floor is what said so; the scope followed the subject and so does this.
RL="$TREE/scripts/acquisition/install-rootless.sh"
cp "$RL" "$WORK/rootless.orig" || exit 2
# ⚠ THE FLAG ALONE IS DROPPED AND ITS VALUE IS LEFT, which keeps this sed free of
# a `$` the shell would have to be told not to expand. ⛔ And it must drop the
# flag rather than rename it: the rule asks whether the line CONTAINS
# `--max-time`, so a `--no-max-time` would still satisfy it and the plant would
# report a guard that had not been tested.
sed 's/--connect-timeout 20 --max-time/--connect-timeout 20/' \
  "$WORK/rootless.orig" >"$RL"
agree "adapters a curl that writes a file with no --max-time is refused" check-adapters - 1
cp "$WORK/rootless.orig" "$RL"

# ⛔ AND A CLONE THAT NOTHING BOUNDS. `git` has no flag of its own, so the rule
# is a `timeout` wrapper, and a clone is the other way a capture host hangs.
AD="$TREE/scripts/capture/adapters/aria2-next.sh"
cp "$AD" "$WORK/adapter.orig" || exit 2
sed 's/timeout 600 git clone/git clone/' "$WORK/adapter.orig" >"$AD"
agree "adapters a git clone with no timeout wrapper is refused" check-adapters - 1
cp "$WORK/adapter.orig" "$AD"

# ⚠ AND A BOUND THAT BOUNDS SOMETHING ELSE IS NOT A BOUND. A `timeout` after the
# command it is supposed to wrap satisfies a rule that only asks whether the word
# appears on the line.
sed 's/timeout 600 git clone \(.*\)$/git clone \1 \&\& timeout 5 true/' \
  "$WORK/adapter.orig" >"$AD"
agree "adapters a timeout after the command it should wrap is refused" check-adapters - 1
cp "$WORK/adapter.orig" "$AD"

# ⭐ AND THE ACCEPTING HALF THAT MATTERS MOST: a curl that only ASKS is out of
# scope, so a rule that demanded a bound on every invocation would fire on the
# probe this adapter already carries and would be a rule somebody switches off.
agree "adapters clean tree again" check-adapters - 0

# ============================================================================
# check-docs
# ============================================================================
#
# ⛔ EVERY PLANT THAT MUST BE ACCEPTED SITS AT THE REPOSITORY ROOT, and that is
# forced rather than chosen. A new `.md` in a subdirectory is an ORPHAN unless
# something links to it, so a plant placed there would be refused by the orphan
# rule whatever the branch under test did - and a case expecting 0 would fail for
# a reason it was not written to ask about. A root file is an entry point and the
# orphan rule skips it, which leaves exactly one rule looking at the plant.
P=plant.md

agree "docs clean tree" check-docs common/check-docs 0

printf 'see [the thing](nope-does-not-exist.md) here\n' >"$TREE/$P"
agree "docs a broken relative link is refused" check-docs common/check-docs 1
unplant "$P"

# ⭐ THE TWO READERS ARE NOT THE SAME READER, and these two cases are the whole
# of why. The broken-link pass strips inline code spans, because markdown does
# not linkify one; the orphan pass did NOT, in the `sh` half alone. A port that
# unified them passes the clean tree and fails here.
# shellcheck disable=SC2016
# The backticks are the SUBJECT: this case exists to put a link inside an inline
# code span, so nothing here is meant to expand. The directive is in this file
# rather than in the invocation, because shellcheck answers differently depending
# on how files were grouped on its command line.
printf 'see `[the thing](nope-does-not-exist.md)` here\n' >"$TREE/$P"
agree "docs a broken link inside a code span is accepted" check-docs common/check-docs 0
unplant "$P"

{
  printf 'text\n\n'
  printf '```\n'
  printf '[the thing](nope-does-not-exist.md)\n'
  printf '```\n'
} >"$TREE/$P"
agree "docs a broken link inside a fenced block is accepted" check-docs common/check-docs 0
unplant "$P"

# ⚠ AN ABSOLUTE URL IS NOT A PATH and nothing here resolves one. A rule that
# tried would need the network, and a check that needs the network is a check
# that goes red when somebody else's server does.
printf 'see [the thing](https://example.invalid/nope) here\n' >"$TREE/$P"
agree "docs an unresolvable http link is out of scope" check-docs common/check-docs 0
unplant "$P"

{
  printf 'text\n\n'
  printf '```sh\n'
  printf 'if [ 1 = 1 ; then echo broken\n'
  printf '```\n'
} >"$TREE/$P"
agree "docs a shell block that does not parse is refused" check-docs common/check-docs 1
unplant "$P"

# ⛔ THE PLACEHOLDER A HUMAN READS AS *fill this in* AND bash READS AS A
# REDIRECT. The block parses; that is the point, and it is why this is a separate
# rule rather than a consequence of the one above.
#
# ⛔ AND THE NEEDLE IS ASSEMBLED, WHICH THIS FILE HAD TO LEARN TWICE. The first
# spelling used the possessive stand-in word check-placeholders also looks for,
# so the harness became THAT rule's finding and five of its cases went red over a
# clean tree. ⚠ Then the comment explaining the mistake spelled the same literal
# and did it again. ⭐ The angle brackets are check-docs' subject and the word
# inside them is not, so the word is one no other rule looks for and it is built
# rather than typed - and this sentence names the class instead of quoting it.
ANGLE=$(printf '%s%s%s' '<' 'endpoint' '>')
{
  printf 'text\n\n'
  printf '```sh\n'
  printf 'curl -sS %s\n' "$ANGLE"
  printf '```\n'
} >"$TREE/$P"
agree "docs an angle-bracket placeholder in a shell block is refused" check-docs common/check-docs 1
unplant "$P"

# ⭐ AND A BLOCK THAT IS SIMPLY CORRECT, because a rule that refused every fenced
# block would pass both cases above.
{
  printf 'text\n\n'
  printf '```sh\n'
  printf 'set -eu\nprintf "%%s\\n" "ok"\n'
  printf '```\n'
} >"$TREE/$P"
agree "docs a shell block that parses is accepted" check-docs common/check-docs 0
unplant "$P"

# -- the orphan rule ----------------------------------------------------------

mkdir -p "$TREE/docs"
printf 'a page nothing points at\n' >"$TREE/docs/plant-orphan.md"
agree "docs a page nothing links to is refused" check-docs common/check-docs 1

# ⛔ AND THE SAME PAGE, CITED ONLY INSIDE BACKTICKS, IS STILL AN ORPHAN. ⭐ THIS
# CASE IS WHY --compare EXISTS: on its first run the two shell halves ANSWERED
# DIFFERENTLY here, and nothing in this repository could have told anybody.
#
# The `sh` half read links with two awk programs and only the broken-link one
# stripped code spans, so a backticked citation counted as a link; the PowerShell
# twin has a single extractor that strips them and feeds both passes. ⚠ No page
# in this tree is cited only that way, so `check-twins` saw the two agree on
# every run for as long as both existed - its own documented blind spot, a rule
# differing only on a shape the tree does not contain.
#
# ⭐ The twin is correct on the rule's own reasoning: a code span is not a
# hyperlink and a reader following links never arrives. The `sh` half and the
# port were both changed to match, deliberately and in the same change, so this
# case asserts a REFUSAL rather than freezing the defect it found.
# shellcheck disable=SC2016
# The backticks are the subject here too, for the reason given above.
printf 'cited as `[the page](docs/plant-orphan.md)` above\n' >"$TREE/$P"
agree "docs a page cited only inside backticks is still an orphan" check-docs common/check-docs 1
unplant "$P"
unplant docs/plant-orphan.md

# ⚠ A LINK THAT CLIMBS MORE THAN TWO LEVELS, and this case is a GUARD rather than
# a proof - which is worth saying, because it was written believing it was a
# proof. Normalising `.../fixtures/../../../../docs/plant-orphan.md` is where
# both shell halves hand-rolled a `segment/../` collapse, and
# docs/conventions/forbidden-patterns.md records a GLOBAL replace eating a real
# segment and a `../..` pair together, answering `crates/bit-ids/docs/...` and
# leaving the real page an orphan. The Go port wrote that same global spelling
# first.
#
# ⛔ BUT THIS CASE CANNOT REFUTE IT, MEASURED BY PLANTING: with the global
# spelling restored and the plant verified to have changed the file, the case
# still PASSED. `path.Join` normalises before any collapse runs, so the branch
# that differs is unreachable - the port is correct for a reason this case does
# not test. ⭐ The hand-rolled collapse is deleted rather than fixed, and the case
# is kept as a regression guard on the ANSWER with its label saying so, the way
# the empty-register case above names the rule that really refuses it.
#
# ⚠ The README carrying the link is orphan-exempt, so the only thing this case
# can report is the page it points at.
printf 'a page nothing else points at\n' >"$TREE/docs/plant-orphan.md"
printf 'see [the page](../../../../docs/plant-orphan.md)\n' \
  >"$TREE/crates/bit-ids/tests/fixtures/README.md"
agree "docs a link climbing four levels resolves to the same page" check-docs common/check-docs 0
unplant crates/bit-ids/tests/fixtures/README.md
unplant docs/plant-orphan.md

# ⚠ A README IS AN ENTRY POINT and the rule skips it by name, so a subdirectory
# README nothing links to is accepted. This tree has no such file, so the branch
# is proved with a fixture rather than assumed.
mkdir -p "$TREE/docs/plantdir"
printf 'an entry point\n' >"$TREE/docs/plantdir/README.md"
agree "docs an unlinked subdirectory README is accepted" check-docs common/check-docs 0
rm -rf "$TREE/docs/plantdir"

# -- the history index --------------------------------------------------------
#
# ⛔ A RECORD LINKED FROM ANYWHERE IS NOT AN ORPHAN AND IS STILL MISSING FROM THE
# INDEX, which is the stronger rule. ⚠ The plant is LINKED on purpose, so the
# orphan rule is satisfied and this case can only fail for the index reason.
printf 'an old record\n' >"$TREE/docs/history/SESSION-PLANT.md"
printf 'see [the record](docs/history/SESSION-PLANT.md)\n' >"$TREE/$P"
agree "docs a session record the history index omits is refused" check-docs common/check-docs 1
unplant docs/history/SESSION-PLANT.md
unplant "$P"

# -- the template exemption ---------------------------------------------------
#
# ⛔ docs/templates/ IS EXEMPT FROM LINK RESOLUTION AND THIS TREE HAS NO SUCH
# DIRECTORY, so the branch is invisible to any comparison over it - the `py`
# scope lesson from check-markers, arriving in a different rule. ⚠ A template's
# links are written relative to where the file will live in a PROJECT, not where
# it lives here.
#
# ⚠ The plant is linked from the root file, because the orphan rule has NO
# template exemption and would otherwise refuse it for a different reason.
mkdir -p "$TREE/docs/templates"
printf 'see [gate](methodology/gate.md), which is not here\n' >"$TREE/docs/templates/plant.md"
printf 'see [the template](docs/templates/plant.md)\n' >"$TREE/$P"
agree "docs a template's unresolvable link is exempt" check-docs common/check-docs 0
rm -rf "$TREE/docs/templates"
unplant "$P"

agree "docs clean tree again" check-docs common/check-docs 0

# ============================================================================

store_report check-bitcheck/1 cases \
  "$([ "$JSON" = "1" ] && printf 1 || printf 0)"
