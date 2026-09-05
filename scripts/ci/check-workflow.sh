#!/bin/sh
# check-workflow.sh - does the workflow actually go red for each class of
# defect it is supposed to catch?
#
# CI-01's Prove has two halves. One is that every required check appears as a
# non-skipped CI result, which is what check-gate's --strict answers now that a
# declared unavailability and an observed skip are counted apart. This is the
# other half: an injected failure in docs, schema, fixtures or Rust turns the
# workflow red.
#
# -- ⛔ EVERY COMMAND IT RUNS IS READ OUT OF THE WORKFLOW -------------------
#
# Nothing here spells a build command. A harness carrying its own copy of
# `cargo test --workspace --locked --all-targets` proves that command refuses a
# defect and says nothing whatever about the one CI runs, and the two would
# drift in the direction that keeps this file green. So a case names a JOB and a
# STEP, the command is pulled from .github/workflows/ci.yml, and ⭐ A STEP THIS
# FILE NAMES AND THE WORKFLOW NO LONGER HAS IS A FAILURE, never a silent pass.
# That is the same defect docs/methodology/reviews.md calls two spellings of one
# layout, and CORPUS-01 paid for it twice.
#
# -- ⛔ IT IS NOT IN check-gate.sh, AND THAT IS A SHARED CONTRACT -----------
#
# Two of the cases below run the workflow's own *Repository gate* step, which is
# check-gate.sh. A runner that appears in the gate and also runs the gate
# recurses, which is precisely why check-gate.sh keeps check-twins out of its
# own pair list. ⚠ Removing this exclusion reintroduces the hang. The workflow
# runs this file as its own step instead, so it still runs on every push, and
# scripts/README.md carries the reasoning beside check-twins'.
#
# -- ⛔ THE TREE IT PLANTS IN IS THE TREE ON DISK ---------------------------
#
# It copies the tracked and untracked working tree into a scratch directory and
# commits it there, rather than checking out HEAD. A harness that tested the
# last commit would report green over a defect somebody had just introduced and
# not yet committed, which is the state every pre-push run is in.
#
# ⚠ THE PLANTS AND THE CONTROLS ARE BOTH REQUIRED. A command that refuses the
# planted tree proves nothing unless it accepts the clean one: a step that is
# broken for an unrelated reason refuses everything, and reads here as a guard
# working perfectly.
#
# Usage:
#   sh scripts/ci/check-workflow.sh
#   sh scripts/ci/check-workflow.sh --json
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
      printf 'check-workflow: unknown argument: %s\n' "$1" >&2
      exit 2
      ;;
  esac
  shift
done

# ⛔ Resolved from this script's own location, never from the working directory.
HERE=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
ROOT=$(CDPATH='' cd -- "$HERE/.." && pwd)
ROOT=$(CDPATH='' cd -- "$ROOT/.." && pwd)

# ⚠ ME is set here and read by store-lib.sh, which this sources on the next
# line. shellcheck cannot see across a source it is not told to follow, so it
# reads as unused unless every file is handed to one invocation.
# shellcheck disable=SC2034
ME=check-workflow
# shellcheck disable=SC1091
# shellcheck source=scripts/corpus/store-lib.sh
. "$ROOT/scripts/corpus/store-lib.sh"

store_require git cargo awk tar sha256sum

WORKFLOW="$ROOT/.github/workflows/ci.yml"
[ -f "$WORKFLOW" ] || {
  printf 'check-workflow: %s is missing\n' "$WORKFLOW" >&2
  exit 2
}

WORK=$(store_workdir checkworkflow) || exit 2
trap 'rm -rf "$WORK"' EXIT INT TERM

TREE="$WORK/tree"
LOG="$WORK/log"
ORIG="$WORK/orig"

# ⭐ ONE TARGET DIRECTORY, SHARED BY EVERY CASE. A fresh one per plant would
# rebuild the workspace each time and turn a two-minute check into twenty.
CARGOTGT="$WORK/target"

# -- the scratch tree ---------------------------------------------------------
#
# ⚠ Tracked AND untracked-but-not-ignored, because an uncommitted new file is
# part of the tree the next push carries. target/ is ignored, so the 1G build
# directory is not copied.
mkdir -p "$TREE" || exit 2
(
  cd "$ROOT" || exit 1
  {
    git ls-files -z
    git ls-files -zo --exclude-standard
  } | tar --null -T - -cf -
) | (cd "$TREE" && tar -xf -) || {
  printf 'check-workflow: cannot copy the working tree\n' >&2
  exit 2
}

# ⛔ IT IS A REAL REPOSITORY, because half the checks the gate runs ask git what
# is tracked. A plain directory would make every one of them exit 2, and a run
# of nothing but skips is what --strict exists to refuse.
#
# ⚠ AND IT CARRIES THE ORIGIN, which is not decoration either. check-remote-items
# resolves the repository from that remote, so a scratch tree without one made
# that check answer 2 wherever it ran, including on a CI runner that could
# perfectly well have run it. The gate cases then never saw a clean tree exit 0
# and their exit codes did no work anywhere. Measured on run 37.
ORIGIN=$(cd "$ROOT" && git remote get-url origin 2>/dev/null) || ORIGIN=""
(
  cd "$TREE" || exit 1
  # ⚠ The identity carries no at-sign on purpose. check-no-secrets --public
  # reads one as an email address and refused this file over its own scratch
  # commit; git accepts a bare word here and the commit is thrown away anyway.
  git init -q -b main &&
    git add -A &&
    git -c user.email=gate -c user.name=gate commit -qm scratch &&
    { [ -z "$ORIGIN" ] || git remote add origin "$ORIGIN"; }
) >/dev/null 2>&1 || {
  printf 'check-workflow: cannot make the scratch tree a repository\n' >&2
  exit 2
}

# -- reading a command out of the workflow ------------------------------------
#
# ⚠ Indents are derived from the lines themselves rather than assumed. A step
# list sets the key indent and a block scalar keeps the indent of its body, so
# re-indenting the workflow does not silently stop this matching.
step_command() { # workflow job step
  awk -v WANTJOB="$2" -v WANTSTEP="$3" '
    function indent(s,   i) { i = match(s, /[^ ]/); return i ? i - 1 : -1 }
    {
      line = $0
      sub(/\r$/, "", line)
      ind = indent(line)

      if (inrun) {
        if (ind < 0) { print ""; next }
        if (ind >= runind) { print substr(line, runind + 1); next }
        inrun = 0
      }
      if (ind < 0) next

      if (ind == 2 && line ~ /^ *[A-Za-z0-9_-]+:[ \t]*$/) {
        job = line; sub(/^ +/, "", job); sub(/:.*$/, "", job)
        step = ""; keyind = -1; next
      }
      if (line ~ /^ *- name:/) {
        step = line; sub(/^ *- name:[ \t]*/, "", step)
        gsub(/^["'"'"']|["'"'"']$/, "", step)
        keyind = ind + 2; next
      }
      if (job == WANTJOB && step == WANTSTEP && ind == keyind && line ~ /^ *run:/) {
        v = line; sub(/^ *run:[ \t]*/, "", v)
        if (v == "|" || v == ">" || v == "|-" || v == ">-") { inrun = 1; runind = keyind + 2; next }
        print v
      }
    }
  ' "$1"
}

# Every job that does not declare KEY at the job level, one per line.
#
# ⛔ BOTH DIRECTIONS MATTER AND ONLY ONE IS OBVIOUS. Reporting the jobs that
# lack the key is the rule; what makes the rule hold as jobs are added is that
# it enumerates the jobs itself rather than being handed a list. A rule given
# the two job names it knows about would pass forever over a third.
jobs_missing() { # workflow key
  awk -v KEY="$2" '
    function indent(s,   i) { i = match(s, /[^ ]/); return i ? i - 1 : -1 }
    /^jobs:[ \t]*$/ { injobs = 1; next }
    !injobs { next }
    {
      line = $0
      sub(/\r$/, "", line)
      ind = indent(line)
      if (ind < 0) next
      if (ind == 0) {
        if (job != "" && !seen) print job
        job = ""; injobs = 0; next
      }
      if (ind == 2 && line ~ /^ *[A-Za-z0-9_-]+:[ \t]*$/) {
        if (job != "" && !seen) print job
        job = line; sub(/^ +/, "", job); sub(/:.*$/, "", job)
        seen = 0; next
      }
      if (ind == 4 && line ~ ("^ *" KEY ":")) seen = 1
    }
    END { if (job != "" && !seen) print job }
  ' "$1"
}

# ⭐ ONE READER PER RULE, CALLED BY THE CASE AND BY THE PROBE. A rule the probe
# re-implements is a rule the probe cannot refute.
gate_asks_strictly() { # workflow job flag
  case "$(step_command "$1" "$2" 'Repository gate')" in
    *"$3"*) return 0 ;;
    *) return 1 ;;
  esac
}

declares_key() { # workflow key
  _missing=$(jobs_missing "$1" "$2")
  [ -z "$_missing" ]
}

# The top-level trigger keys of a workflow, one per line.
triggers() { # workflow
  awk '
    function indent(s,   i) { i = match(s, /[^ ]/); return i ? i - 1 : -1 }
    /^on:[ \t]*$/ { inon = 1; next }
    /^on:/ { print "inline"; next }
    !inon { next }
    {
      line = $0
      sub(/\r$/, "", line)
      ind = indent(line)
      if (ind < 0) next
      if (ind == 0) { inon = 0; next }
      if (ind == 2 && line ~ /^ *[A-Za-z_][A-Za-z0-9_]*:/) {
        k = line; sub(/^ +/, "", k); sub(/:.*$/, "", k); print k
      }
    }
  ' "$1"
}

# The declared default of one workflow_dispatch input.
input_default() { # workflow input
  awk -v WANT="$2" '
    function indent(s,   i) { i = match(s, /[^ ]/); return i ? i - 1 : -1 }
    {
      line = $0
      sub(/\r$/, "", line)
      ind = indent(line)
    }
    ind == 6 && line ~ /^ *[A-Za-z_][A-Za-z0-9_]*:[ \t]*$/ {
      k = line; sub(/^ +/, "", k); sub(/:.*$/, "", k); cur = k; next
    }
    cur == WANT && ind == 8 && line ~ /^ *default:/ {
      v = line; sub(/^ *default:[ \t]*/, "", v); print v; exit
    }
  ' "$1"
}

# ⛔ 127 MEANS THE WORKFLOW HAS NO SUCH STEP, and it is reported separately from
# every other status. A missing step read as a refusal would let deleting a
# whole CI step register as the guard working.
run_step() { # job step
  _cmd=$(step_command "$WORKFLOW" "$1" "$2")
  [ -n "$_cmd" ] || return 127
  (
    cd "$TREE" || exit 2
    CARGO_TARGET_DIR="$CARGOTGT" sh -c "$_cmd"
  ) >"$LOG" 2>&1
}

# -- the two verdicts ---------------------------------------------------------

control() { # job step label
  run_step "$1" "$2"
  _rc=$?
  case "$_rc" in
    0) pass "control  $3 accepts the clean tree" ;;
    127) fail "control  $3: the workflow has no step named that" ;;
    *) fail "control  $3 refused the clean tree (exit $_rc)" ;;
  esac
}

# ⛔ THE GATE STEP NEEDS ITS OWN CONTROL, because its exit code on a developer
# host is not the one CI reads. The workflow runs it with --strict, and --strict
# refuses an observed skip: this host has no authenticated gh, so
# check-remote-items answers 2 and the clean tree exits 1 here and 0 on the
# Linux lane. Reporting that as a broken control would be false, and treating it
# as a pass would hide a genuinely failing check.
#
# ⭐ So the control reads the summary line the runner prints, which counts
# FAILURES, and records which of the two hosts this is. The plants below still
# read the exit code from the process that produced it; what this establishes is
# whether that code is discriminating here.
GATE_NOTE=""
gate_control() { # label
  run_step linux "Repository gate"
  _rc=$?
  if [ "$_rc" = 127 ]; then
    fail "control  $1: the workflow has no step named that"
    return
  fi
  if ! grep -q -E '^[0-9]+ checks: .*, 0 failed,' "$LOG"; then
    fail "control  $1: the clean tree failed a check"
    return
  fi
  if [ "$_rc" = 0 ]; then
    pass "control  $1 accepts the clean tree (exit 0)"
  else
    GATE_NOTE=" (this host cannot reach exit 0; see the header)"
    pass "control  $1 fails no check, and exits $_rc on a host with an observed skip"
  fi
}

# ⚠ The pattern argument is what stops a plant passing for the wrong reason. A
# step can go red because the scratch tree is broken, and without asking WHICH
# check refused, that reads here as the planted guard firing.
refuses() { # job step label pattern
  run_step "$1" "$2"
  _rc=$?
  if [ "$_rc" = 127 ]; then
    fail "$3: the workflow has no step named that"
    return
  fi
  if [ "$_rc" = 0 ]; then
    fail "$3 was NOT refused; the step exited 0"
    return
  fi
  if [ -n "$4" ] && ! grep -q -F -e "$4" "$LOG"; then
    fail "$3 was refused (exit $_rc) but not by $4"
    return
  fi
  pass "$3 turns the workflow red (exit $_rc)"
}

# ⚠ The gate plants carry the control's note, so a reader of this report can see
# whether the exit code was doing any of the work on the host it ran on.
gate_refuses() { # label pattern
  refuses linux "Repository gate" "$1$GATE_NOTE" "$2"
}

# -- planting -----------------------------------------------------------------

keep() { # relative path
  cp "$TREE/$1" "$ORIG" || return 1
}
restore() { # relative path
  cp "$ORIG" "$TREE/$1" || return 1
}

# ⛔ AN APPEND IS VERIFIED LIKE A SUBSTITUTION. store-lib's replace_once refuses
# a literal that matched no times or twice; an append cannot miss that way, so
# what it owes instead is proof the file changed at all. A read-only file would
# otherwise leave the case reporting a guard that failed to fire.
append_once() { # file text
  _before=$(sha256sum "$1" | cut -d' ' -f1)
  printf '%s\n' "$2" >>"$1" || return 1
  _after=$(sha256sum "$1" | cut -d' ' -f1)
  [ "$_before" != "$_after" ]
}

# -- the probe's own guards ---------------------------------------------------
store_probe_guards "$TREE/README.md" \
  "A dated, provenance-carrying catalogue" "bit-ids"

# -- 0. the flag the whole first half of the Prove rests on -------------------
#
# ⛔ READ OUT OF THE WORKFLOW, NOT ASSERTED ABOUT IT. Every planted case below
# proves that a defect makes some check answer non-zero. None of them can prove
# that CI is asking strictly, because a skip is only a failure when the flag is
# there, and on a host with an observed skip of its own the exit code cannot
# tell the two apart. This case is what closes that, and it is why the two are
# written as separate rows rather than one.
if gate_asks_strictly "$WORKFLOW" linux '--strict'; then
  pass "workflow  the Linux gate step asks strictly"
else
  fail "workflow  the Linux gate step does not pass --strict"
fi

if gate_asks_strictly "$WORKFLOW" windows '-Strict'; then
  pass "workflow  the Windows gate step asks strictly"
else
  fail "workflow  the Windows gate step does not pass -Strict"
fi

# -- 0b. the job-scoped properties the entry's Approach names -----------------
#
# ⚠ Declared PER JOB rather than inherited. A job that states no permissions
# runs with whatever the workflow's floor happens to be that week, and the
# person who widens that floor for one job widens it for every job that never
# said anything. The same argument holds for a timeout: the default is six
# hours, so a job with none is a job that hangs for an afternoon.
#
# ⛔ EVERY WORKFLOW, NOT THE ONE THIS FILE CARES ABOUT. A rule that read ci.yml
# alone would be a gate on one of two doors the moment a second workflow landed,
# and a second workflow did land: the publisher is the one job in this
# repository that may write, so it is exactly the file a permissions rule must
# not skip. check-project.sh's action-pin rule generalises the same way.
for wf in "$ROOT"/.github/workflows/*.yml; do
  [ -f "$wf" ] || continue
  wfname=${wf##*/}
  for key in permissions timeout-minutes; do
    if declares_key "$wf" "$key"; then
      pass "workflow  $wfname: every job declares $key"
    else
      fail "workflow  $wfname: jobs with no $key: $(jobs_missing "$wf" "$key" | tr '\n' ' ')"
    fi
  done
  if grep -q '^concurrency:' "$wf" && grep -q 'cancel-in-progress:' "$wf"; then
    pass "workflow  $wfname declares a concurrency group"
  else
    fail "workflow  $wfname declares no concurrency group"
  fi
done

# -- 0c. the publisher cannot fire on its own ---------------------------------
#
# ⛔ NOTHING HAS EVER BEEN PUBLISHED AND NOTHING MAY BE until a measured record
# exists, because everything in the tree is synthetic. The publisher's workflow
# is therefore dispatch-only, and its dry run is the default so that dispatching
# it by accident still pushes nothing. ⚠ Both are properties a reader can check,
# which is why they are cases here rather than a sentence in the file.
PUBWF="$ROOT/.github/workflows/publish-data.yml"
if [ -f "$PUBWF" ]; then
  PUBTRIG=$(triggers "$PUBWF" | tr '\n' ' ' | sed 's/ *$//')
  if [ "$PUBTRIG" = "workflow_dispatch" ]; then
    pass "workflow  the publisher can only be dispatched by hand"
  else
    fail "workflow  the publisher has other triggers: $PUBTRIG"
  fi

  PUBDRY=$(input_default "$PUBWF" dry_run)
  if [ "$PUBDRY" = "true" ]; then
    pass "workflow  the publisher dry-runs unless told otherwise"
  else
    fail "workflow  the publisher's dry_run default is [$PUBDRY], not true"
  fi
else
  fail "workflow  there is no publisher workflow to check"
fi

# -- 0d. the static readers, refuted ------------------------------------------
#
# ⛔ EVERY RULE ABOVE IS A READER THAT HAS NEVER BEEN SEEN TO REFUSE ANYTHING.
# Each is checked against a copy of the workflow with the property removed,
# because a reader that answers "present" over a file that does not have it
# would pass all four cases above on any workflow at all.
MUTWF="$WORK/workflow-without.yml"

sed 's/ --strict//' "$WORKFLOW" >"$MUTWF"
if gate_asks_strictly "$MUTWF" linux '--strict'; then
  fail "probe    the strict-flag reader passed a workflow with no --strict"
else
  pass "probe    the strict-flag reader refuses a workflow with no --strict"
fi

awk '!/^    permissions:$/ || n++' "$WORKFLOW" >"$MUTWF"
if declares_key "$MUTWF" permissions; then
  fail "probe    the job-key reader passed a job with no permissions"
else
  pass "probe    the job-key reader refuses a job with no permissions"
fi

awk '!/^    timeout-minutes:/ || n++' "$WORKFLOW" >"$MUTWF"
if declares_key "$MUTWF" timeout-minutes; then
  fail "probe    the job-key reader passed a job with no timeout"
else
  pass "probe    the job-key reader refuses a job with no timeout"
fi

# ⚠ The trigger reader is checked against a workflow that HAS an automatic
# trigger rather than against one with none, because the direction that matters
# is the one where the publisher gains a way to fire by itself.
if [ -f "$PUBWF" ]; then
  awk '/^on:$/ { print; print "  push:"; next } { print }' "$PUBWF" >"$MUTWF"
  case "$(triggers "$MUTWF" | tr '\n' ' ')" in
    *push*) pass "probe    the trigger reader sees an added push trigger" ;;
    *) fail "probe    the trigger reader missed an added push trigger" ;;
  esac

  sed 's/        default: true/        default: false/' "$PUBWF" >"$MUTWF"
  if [ "$(input_default "$MUTWF" dry_run)" = "true" ]; then
    fail "probe    the input-default reader still reports true after the flip"
  else
    pass "probe    the input-default reader follows the declared default"
  fi
fi

# -- the controls -------------------------------------------------------------
#
# ⚠ Run first and in this order, because the first cargo command warms the
# shared target directory that every later case reuses.
control linux "Rust check" "cargo check"
control linux "Rust formatting" "cargo fmt"
control linux "Rust tests" "cargo test"
control linux "Rust lints" "cargo clippy"
control linux "Shell syntax and style" "shellcheck and shfmt"
gate_control "the repository gate"

# -- 1. a Rust defect that does not compile -----------------------------------
LIB="crates/bit-ids/src/lib.rs"
if keep "$LIB" && append_once "$TREE/$LIB" "fn workflow_plant() -> u8 { \"not a u8\" }"; then
  refuses linux "Rust check" "an uncompilable Rust defect" "error"
else
  fail "plant    could not plant the Rust compile defect"
fi
restore "$LIB" || exit 2

# -- 2. a Rust defect clippy refuses ------------------------------------------
#
# ⚠ It compiles and it is correctly formatted on purpose. A plant that also
# broke the build or the formatting would be refused by an earlier step, and the
# case would report the lint step working when it had never been reached.
if keep "$LIB" && append_once "$TREE/$LIB" "pub fn workflow_plant() -> u8 { return 1; }"; then
  refuses linux "Rust lints" "a lint the workspace denies" "needless_return"
else
  fail "plant    could not plant the clippy defect"
fi
restore "$LIB" || exit 2

# -- 3. a Rust file that is not formatted -------------------------------------
if keep "$LIB" && append_once "$TREE/$LIB" "pub  fn   workflow_plant ( )  ->  u8  {  1  }"; then
  refuses linux "Rust formatting" "an unformatted Rust file" "Diff in"
else
  fail "plant    could not plant the formatting defect"
fi
restore "$LIB" || exit 2

# -- 4. a schema fixture whose identity no longer derives ----------------------
#
# ⭐ The platform is in the tuple the record identifier digests, so changing it
# makes the declared id disagree with the record's own contents. That is the
# rule docs/architecture.md section 4 states, planted rather than read.
PROFILE="crates/bit-ids/tests/fixtures/valid-profile.json"
if keep "$PROFILE" &&
  replace_once "$TREE/$PROFILE" '"platform": "linux"' '"platform": "windows"'; then
  refuses linux "Rust tests" "a schema fixture that no longer validates" "FAILED"
else
  fail "plant    could not plant the schema defect"
fi
restore "$PROFILE" || exit 2

# -- 5. a wire fixture whose bytes moved --------------------------------------
#
# ⚠ Not a byte of the frame. index.json digests each fixture FILE, so any change
# to one is what the corpus digest is there to catch, and the summary is a field
# no decoder reads: a plant in the frame bytes could be refused by a codec
# instead, which is a different guard than the one this case names.
FIXTURE="crates/bit-ids-wire/tests/fixtures/tracker-http-announce-started.json"
if keep "$FIXTURE" &&
  replace_once "$TREE/$FIXTURE" 'An ordinary started announce' 'An ordinary drifted announce'; then
  refuses linux "Rust tests" "a wire fixture whose digest moved" "FAILED"
else
  fail "plant    could not plant the fixture defect"
fi
restore "$FIXTURE" || exit 2

# -- 6. a shell script that does not parse ------------------------------------
# ⚠ The pattern is shellcheck's own parse-error code rather than nothing at all.
# This case accepted any non-zero status at first, which `shfmt` would also have
# produced for a reformatting difference, so the case would have passed while
# the syntax checker said nothing.
SCRIPT="scripts/doctor/doctor.sh"
if keep "$SCRIPT" && append_once "$TREE/$SCRIPT" "if [ 1 = 1 ; then :"; then
  refuses linux "Shell syntax and style" "a shell script that does not parse" "SC1073"
else
  fail "plant    could not plant the shell defect"
fi
restore "$SCRIPT" || exit 2

# -- 7. a document whose link resolves to nothing -----------------------------
#
# ⚠ The pattern is the runner's FAILURE row and not the check's name. The name
# alone also matches the row that reads `ok    check-docs`, so a plant that was
# never refused would have matched the control's own output.
README="README.md"
if keep "$README" &&
  append_once "$TREE/$README" "A link to [nothing](docs/this-file-does-not-exist.md)."; then
  gate_refuses "a document with a dead link" "FAIL  check-docs"
else
  fail "plant    could not plant the docs defect"
fi
restore "$README" || exit 2

# -- 8. a check that stopped running ------------------------------------------
#
# ⛔ THIS IS THE ONE THE LANE COULD NOT SEE. A check that answers 2 is counted as
# a skip, and the workflow's gate step ran without --strict on the Windows lane
# because six of its rows are genuinely unavailable there. Measured on
# 2026-09-06: with this same plant, that invocation exited 0. The declared and
# observed counts are separate now, so --strict permits the six and refuses
# this. ⚠ Case 0 above is the other half: this shows the runner reports the
# skip, and that one shows CI is asking with the flag that makes it fatal.
BROKEN="scripts/common/check-project.sh"
if keep "$BROKEN" && printf 'exit 2\n' >"$TREE/$BROKEN"; then
  gate_refuses "a check that can no longer run" "SKIP  check-project"
else
  fail "plant    could not plant the unrunnable check"
fi
restore "$BROKEN" || exit 2

# -- 9. the clean tree again --------------------------------------------------
#
# ⛔ THE LAST CASE IS A CONTROL, NOT A REPEAT. Every plant above restores the
# file it touched, and nothing so far has checked that any of them did. A
# restore that silently failed would leave every later case running against a
# defective tree, and the run would still report every guard refusing.
gate_control "the repository gate, after every restore"

store_report "check-workflow/1" cases "$JSON"
