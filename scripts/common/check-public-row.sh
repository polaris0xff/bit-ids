#!/bin/sh
# check-public-row.sh - does each gate runner really ask the PUBLIC question?
#
# ⛔ A GATE ROW CAN ASK THE WRONG QUESTION AND STAY GREEN. The gate runs
# `check-no-secrets` twice and the second row exists only because `--public` is a
# different question, not a stricter one. If either runner stopped passing that
# flag the row would still pass, because the DEFAULT question also holds on a
# clean tree - so the lane would keep a row that verified nothing about emails,
# absolute home paths or long hex.
#
# ⚠ NEITHER EXISTING CHECK COVERS IT. `check-gate-rows` compares row NAMES, and
# both runners would go on naming this row. `check-bitcheck` proves the BINARY
# honours the flag and says nothing about what the runner passes it.
# `TODO/ci.md` filed it as a residual under `CI-10` with this acceptance.
#
# -- ⛔ THE FLAGS ARE READ OUT OF THE RUNNERS, NOT WRITTEN HERE ---------------
#
# A harness holding its own copy of a command proves that command behaves and
# says nothing about the one the gate runs. That is the rule
# `scripts/ci/check-workflow.sh` is built on, and it is the whole point here: the
# defect being guarded against is a runner that stopped passing a flag, which is
# invisible to anything that spells the flag itself.
#
# -- ⚠ AND IT RUNS THEM, RATHER THAN READING THEM ----------------------------
#
# The extracted flags are handed to the binary and the answer is read back.
# `"public_rules":true` is the binary's own statement about which rules it ran,
# so this asserts the question that was actually asked rather than the text of a
# line in a script. ⭐ The control is the half that matters: the default
# invocation must answer `"public_rules":false`, or the field is not one that
# moves and the assertion above would hold over anything.
#
# Usage:
#   sh scripts/common/check-public-row.sh
#   sh scripts/common/check-public-row.sh --json
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
      printf 'check-public-row: unknown argument: %s\n' "$1" >&2
      exit 2
      ;;
  esac
  shift
done

HERE=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
ROOT=$(CDPATH='' cd -- "$HERE/.." && pwd)
ROOT=$(CDPATH='' cd -- "$ROOT/.." && pwd)

# shellcheck disable=SC2034
ME=check-public-row
# shellcheck disable=SC1091
# shellcheck source=scripts/corpus/store-lib.sh
. "$ROOT/scripts/corpus/store-lib.sh"

store_require go awk

WORK=$(store_workdir checkpublicrow) || exit 2
trap 'rm -rf "$WORK"' EXIT INT TERM

SH_GATE="$ROOT/scripts/common/check-gate.sh"
PS_GATE="$ROOT/scripts/common/check-gate.ps1"
for _need in "$SH_GATE" "$PS_GATE"; do
  [ -f "$_need" ] || {
    printf 'check-public-row: %s is missing\n' "$_need" >&2
    exit 2
  }
done

BIN="$WORK/bit-check"
(cd "$ROOT/tools/check" && go build -o "$BIN" .) || {
  printf 'check-public-row: tools/check did not build\n' >&2
  exit 2
}

# ⚠ The row's LABEL is what both runners agree on, and it is what a reader looks
# for. The flags are whatever follows the check name on that line.
ROW='check-no-secrets (public)'

trim() { # text
  printf '%s' "$1" | sed 's/^[[:space:]]*//; s/[[:space:]]*$//'
}

# ⛔ NO awk PROGRAM IS BUILT OUT OF A SHELL STRING HERE, and the first version of
# this file was. `docs/conventions/shell.md` section 1 is exactly this: a payload
# that crosses a shell loses its quoting, silently. That draft produced
# `awk: syntax error` on one half and matched the label rather than the
# invocation on the other, reporting the sh runner's flags as
# `(public)" "$GOBIN" check-no-secrets --public`. Parameter expansion crosses no
# boundary at all.

# The `sh` runner queues a row as:  queue "<label>" "$GOBIN" <check> <flags...>
flags_sh() {
  _line=$(grep -F "queue \"$ROW\"" "$SH_GATE" | head -1 | tr -d '\r')
  [ -n "$_line" ] || return 0
  _after=${_line#*\""$ROW"\"}
  case "$_after" in
    *check-no-secrets*) trim "${_after#*check-no-secrets}" ;;
    *) printf '' ;;
  esac
}

# The PowerShell runner declares a row as:
#   Invoke-Ported '<label>' 'check-no-secrets' @('<flag>', ...)
flags_ps() {
  _line=$(grep -F "Invoke-Ported '$ROW'" "$PS_GATE" | head -1 | tr -d '\r')
  [ -n "$_line" ] || return 0
  case "$_line" in
    *@\(*) ;;
    *) return 0 ;;
  esac
  _inner=${_line#*@\(}
  _inner=${_inner%%\)*}
  trim "$(printf '%s' "$_inner" | tr -d "'" | tr ',' ' ')"
}

# ⛔ A READER THAT FOUND NOTHING MUST NOT REPORT CLEAN. An empty extraction is
# the shape this whole file exists to refuse, arriving in its own plumbing: a
# runner whose row was renamed or deleted would hand back nothing, and nothing
# passes every assertion below.
check_found() { # label flags
  if [ -z "$2" ]; then
    fail "$1 names no flags for '$ROW': the row is gone, renamed, or passes none"
    return 1
  fi
  return 0
}

# Runs the binary with the flags a runner would pass and reports its answer.
answer() { # flags...
  "$BIN" check-no-secrets "$@" --json 2>"$WORK/err" | tr -d '\r'
}

SH_FLAGS=$(flags_sh)
PS_FLAGS=$(flags_ps)

if check_found "the sh runner" "$SH_FLAGS"; then
  pass "the sh runner passes [$SH_FLAGS]"
fi
if check_found "the PowerShell runner" "$PS_FLAGS"; then
  pass "the PowerShell runner passes [$PS_FLAGS]"
fi

# ⛔ THE CONTROL FIRST. If the default invocation also reported public rules, the
# assertions below would hold over a runner that passed nothing at all, and this
# whole check would be theatre.
DEFAULT=$(answer)
case "$DEFAULT" in
  *'"public_rules":false'*)
    pass "the default question answers public_rules:false, so the field moves"
    ;;
  *)
    fail "the default question did not answer public_rules:false: $DEFAULT"
    ;;
esac

# ⚠ Each runner's own flags, unquoted on purpose: they are a flag list read out
# of a script, and a quoted expansion would hand the binary one argument spelled
# with a space in it.
for _half in sh ps; do
  case "$_half" in
    sh)
      _label="the sh runner"
      _flags=$SH_FLAGS
      ;;
    *)
      _label="the PowerShell runner"
      _flags=$PS_FLAGS
      ;;
  esac
  [ -n "$_flags" ] || continue
  # shellcheck disable=SC2086
  _answer=$(answer $_flags)
  case "$_answer" in
    *'"public_rules":true'*)
      pass "$_label really asks the public question"
      ;;
    *)
      fail "$_label ran [$_flags] and the binary answered: $_answer"
      ;;
  esac
done

store_report check-public-row/1 cases "$JSON"
