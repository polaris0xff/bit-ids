#!/bin/sh
# check-shell.sh - do the shell scripts parse, and are they formatted the way
# this repository formats them?
#
# ⛔ THE GAP THIS CLOSES TURNED THE LANE RED ON A TREE WHOSE LOCAL GATE WAS
# GREEN. Measured on 2026-09-10: CI run 113 failed on *Shell syntax and style*
# over one `SC2016` in a new harness, and `sh scripts/common/check-gate.sh` had
# reported 34 of 35 passing on the same commit minutes earlier. The gate ran
# neither tool.
#
# ⚠ AND THE WORKFLOW ALREADY SAID IT DID. `.github/workflows/ci.yml` installs the
# pinned `shfmt` on the WINDOWS lane with the comment *"several of the cases below
# run the gate, and the gate runs shell checks"* - which was not true of any row
# the gate ran. A rule a document says this repository has is not a rule it has,
# and this is the fourth instance recorded here.
#
# ⭐ `provision.sh` INSTALLS BOTH TOOLS ALREADY, and its own header says why: a
# session host without them "runs a gate that is quietly smaller than CI's". This
# is the row that spends them.
#
# -- ⚠ THE COMMAND IS CI's, CHARACTER FOR CHARACTER --------------------------
#
# Every tracked shell script, FOUND rather than listed: a glob per directory is a
# list, and `ACQ-02` added a directory the list did not have. ⛔ And the two tools
# are two rows rather than one, because `&&` between two checks is one check and
# a reader of a red row has to know which tool refused.
#
# ⚠ shellcheck ANSWERS DIFFERENTLY DEPENDING ON HOW THE FILES WERE GROUPED on its
# command line: a script that sources another is clean when both are handed to
# one invocation and warns when checked alone. CI passes every script at once and
# so does this, which is what makes the two lanes comparable.
#
# Usage:
#   sh scripts/common/check-shell.sh
#   sh scripts/common/check-shell.sh --json
#
# Exit codes: 0 both tools are clean, 1 one is not, 2 a tool is missing.
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
      printf 'check-shell: unknown argument: %s\n' "$1" >&2
      exit 2
      ;;
  esac
  shift
done

HERE=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
ROOT=$(CDPATH='' cd -- "$HERE/.." && pwd)
ROOT=$(CDPATH='' cd -- "$ROOT/.." && pwd)

# shellcheck disable=SC2034
ME=check-shell
# shellcheck disable=SC1091
# shellcheck source=scripts/corpus/store-lib.sh
. "$ROOT/scripts/corpus/store-lib.sh"

# ⛔ EXIT 2, NEVER 1. A machine without the tool has not failed the check, and
# the gate runner reads 2 as a skip - which `--strict` turns into a failure,
# because on a lane the tools are installed on purpose.
store_require shellcheck shfmt find xargs

WORK=$(store_workdir checkshell) || exit 2
trap 'rm -rf "$WORK"' EXIT INT TERM

# ⛔ THE EXIT CODE IS READ FROM THE PROCESS THAT PRODUCED IT. `find | xargs tool`
# reports the PIPELINE's status in a naive `$?`, so the output goes to a file and
# the code is taken on the next line.
(cd "$ROOT" && find scripts -name '*.sh' -print0 | xargs -0 shellcheck) \
  >"$WORK/shellcheck.out" 2>&1
SHELLCHECK_RC=$?
if [ "$SHELLCHECK_RC" = "0" ]; then
  pass "syntax   every shell script under scripts/ passes shellcheck"
else
  fail "syntax   shellcheck exited $SHELLCHECK_RC"
  [ "$JSON" = "1" ] || sed 's/^/          /' "$WORK/shellcheck.out" | head -20
fi

(cd "$ROOT" && find scripts -name '*.sh' -print0 | xargs -0 shfmt -d -i 2 -ci) \
  >"$WORK/shfmt.out" 2>&1
SHFMT_RC=$?
if [ "$SHFMT_RC" = "0" ]; then
  pass "style    every shell script under scripts/ is formatted as shfmt -i 2 -ci"
else
  fail "style    shfmt exited $SHFMT_RC"
  [ "$JSON" = "1" ] || sed 's/^/          /' "$WORK/shfmt.out" | head -20
fi

# ⛔ AND A SWEEP THAT FOUND NOTHING BECAUSE IT LOOKED NOWHERE IS NOT A PASS. The
# two rows above are identical over a tree with no scripts in it, so the count is
# a case of its own: a `find` that stopped matching would report the same clean
# answer over a repository full of shell.
SCRIPTS=$(cd "$ROOT" && find scripts -name '*.sh' | grep -c .)
if [ "$SCRIPTS" -ge 20 ]; then
  pass "scope    $SCRIPTS shell script(s) were handed to both tools"
else
  fail "scope    only $SCRIPTS shell script(s) were found; the sweep has stopped sweeping"
fi

store_report check-shell/1 cases "$JSON"
