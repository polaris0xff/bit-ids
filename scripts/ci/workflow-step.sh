#!/bin/sh
# workflow-step.sh - read one workflow step: its ordered siblings, its `run:`
# body, and the shell it declares.
#
# ⛔ ONE HOME FOR LIFTING A STEP OUT OF A WORKFLOW. Two harnesses now execute
# workflow step bodies - `check-workflow.sh` runs `ci.yml`'s steps against
# planted trees, and `check-step-bodies.sh` runs the capture workflows' bodies
# under the runner's own semantics - and a second copy of this reader is a
# second answer to "what does that step actually run". ⚠ The copies would drift
# in the direction that matters: a reader that stopped matching after a
# re-indent reports an empty body, and an empty body reads exactly like a step
# the workflow no longer has.
#
# ⛔ THE THREE ANSWERS ARE KEPT APART, which is the whole reason this is a
# script with exit codes rather than a function returning a string. A step the
# workflow does not have, a step that runs an action rather than a command, and
# a step whose body is empty are three different facts, and a caller that read
# them all as "" would report a deleted step as a rule that passed.
#
# ⚠ INDENTS ARE DERIVED FROM THE LINES THEMSELVES rather than assumed. A step
# list sets the key indent and a block scalar keeps the indent of its body, so
# re-indenting a workflow does not silently stop this matching.
#
# Usage:
#   sh scripts/ci/workflow-step.sh --workflow <path> --job <name> --step <name>
#   sh scripts/ci/workflow-step.sh --workflow <path> --job <name> --step <name> --shell
#   sh scripts/ci/workflow-step.sh --workflow <path> --job <name> --steps
#
# Exit codes: 0 answered, 1 the step exists and has no `run:` body, 2 could not
# run, 3 the workflow has no such job or step.
#
# ⛔ Read the exit code from this process, unpiped.

set -u

WORKFLOW=""
JOB=""
STEP=""
MODE=run

usage() {
  awk 'NR>1 { if (/^#/) { sub(/^# ?/, ""); print } else exit }' "$0"
}

cannot() {
  printf 'workflow-step: %s\n' "$1" >&2
  exit 2
}

while [ $# -gt 0 ]; do
  case "$1" in
    --workflow)
      [ $# -ge 2 ] || cannot "--workflow takes a value"
      WORKFLOW="$2"
      shift
      ;;
    --job)
      [ $# -ge 2 ] || cannot "--job takes a value"
      JOB="$2"
      shift
      ;;
    --step)
      [ $# -ge 2 ] || cannot "--step takes a value"
      STEP="$2"
      shift
      ;;
    --steps) MODE=steps ;;
    --shell) MODE=shell ;;
    -h | --help)
      usage
      exit 0
      ;;
    *) cannot "unknown argument: $1" ;;
  esac
  shift
done

[ -n "$WORKFLOW" ] || cannot "--workflow is required"
[ -n "$JOB" ] || cannot "--job is required"
[ -f "$WORKFLOW" ] || cannot "the workflow $WORKFLOW is not present"
[ "$MODE" = steps ] || [ -n "$STEP" ] || cannot "--step is required"

# The ordered step names of one job, one per line.
#
# ⚠ Bounded to the `jobs:` block. Without that, the `workflow_dispatch:` key
# under `on:` reads as a job at the same indent.
steps_of() { # workflow job
  awk -v WANTJOB="$2" '
    function indent(s,   i) { i = match(s, /[^ ]/); return i ? i - 1 : -1 }
    /^jobs:[ \t]*$/ { injobs = 1; next }
    !injobs { next }
    {
      line = $0
      sub(/\r$/, "", line)
      ind = indent(line)
      if (ind < 0) next
      if (ind == 0) { injobs = 0; next }
      if (ind == 2 && line ~ /^ *[A-Za-z0-9_-]+:[ \t]*$/) {
        job = line; sub(/^ +/, "", job); sub(/:.*$/, "", job); next
      }
      if (job == WANTJOB && line ~ /^ *- name:/) {
        s = line
        sub(/^ *- name:[ \t]*/, "", s)
        gsub(/^["'"'"']|["'"'"']$/, "", s)
        print s
      }
    }
  ' "$1"
}

if [ "$MODE" = steps ]; then
  steps_of "$WORKFLOW" "$JOB"
  exit 0
fi

# ⛔ EXISTENCE IS ASKED BEFORE THE BODY IS READ, because an absent step and a
# step that runs an action both produce no command and only one of them is rot.
steps_of "$WORKFLOW" "$JOB" | grep -q -x -F -e "$STEP" || exit 3

# One scalar key of one step - `run:` as a block or a single line, or any other
# key such as `shell:`.
step_key() { # workflow job step key
  awk -v WANTJOB="$2" -v WANTSTEP="$3" -v WANTKEY="$4" '
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
      if (job == WANTJOB && step == WANTSTEP && ind == keyind && line ~ ("^ *" WANTKEY ":")) {
        v = line; sub("^ *" WANTKEY ":[ \t]*", "", v)
        if (v == "|" || v == ">" || v == "|-" || v == ">-") { inrun = 1; runind = keyind + 2; next }
        print v
      }
    }
  ' "$1"
}

if [ "$MODE" = shell ]; then
  # ⚠ `default` IS AN ANSWER RATHER THAN AN ABSENCE. GitHub runs a `run:` step
  # with no `shell:` under `bash -e` on Linux, so a caller told "" would have to
  # decide what that means and two callers would decide differently.
  DECLARED=$(step_key "$WORKFLOW" "$JOB" "$STEP" shell)
  [ -n "$DECLARED" ] || DECLARED=default
  printf '%s\n' "$DECLARED"
  exit 0
fi

# ⚠ READ TWICE ON PURPOSE, AND THE SECOND READ IS THE ANSWER. A body reaching
# stdout through `$( )` loses its trailing blank lines, so a caller writing it
# to a file would get a body this reader had edited. The substitution is used
# for the emptiness test alone, where trailing newlines are exactly what does
# not matter.
BODY=$(step_key "$WORKFLOW" "$JOB" "$STEP" run)
[ -n "$BODY" ] || exit 1
step_key "$WORKFLOW" "$JOB" "$STEP" run
exit 0
