#!/bin/sh
# check-handbook.sh - walk the contributor handbook, in order, in one directory,
# and hand the result to the validator.
#
# ⛔ THIS IS DOC-02'S PROVE AND THE CLAUSE THAT MATTERS IS "WITHOUT UNDOCUMENTED
# STEPS". A walkthrough that a harness helps along proves that the harness knows
# how to build a submission, not that the page does. So this runs the page's own
# blocks and nothing else: three names are bound before the first one, and after
# the last one the result is validated. A step the page does not carry is a step
# the walkthrough does not have, and the validation at the end is what notices.
#
# ⚠ THE STEPS SHARE ONE DIRECTORY AND ONE SHELL STATE, WHICH IS WHAT MAKES IT A
# WALKTHROUGH RATHER THAN A LIST OF EXAMPLES. check-examples.sh runs each block
# on its own because a reader can run any one of those alone; here step four
# depends on step three having happened.
#
# -- ⛔ THE THREE BOUND NAMES ARE THE HARNESS'S WHOLE CONTRIBUTION ------------
#
#   REPO        where the checkout is
#   BIN         where cargo put the examples
#   SUBMISSION  the directory being built
#   EMPTY       an empty tree, which the page's own block creates
#
# A contributor supplies the same four by knowing where they cloned to. Binding
# anything else here would be the harness doing a step the page does not.
#
# Usage:
#   sh scripts/common/check-handbook.sh
#   sh scripts/common/check-handbook.sh --json
#
# Exit codes: 0 the walkthrough ran and validated, 1 it did not, 2 could not run.
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
      printf 'check-handbook: unknown argument: %s\n' "$1" >&2
      exit 2
      ;;
  esac
  shift
done

HERE=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
ROOT=$(CDPATH='' cd -- "$HERE/../.." && pwd)

# shellcheck disable=SC2034
ME=check-handbook
# shellcheck disable=SC1091
# shellcheck source=scripts/corpus/store-lib.sh
. "$ROOT/scripts/corpus/store-lib.sh"

DOC="$ROOT/docs/contributing-captures.md"
[ -f "$DOC" ] || {
  printf 'check-handbook: %s not found\n' "$DOC" >&2
  exit 2
}

store_require cargo sha256sum

WORK=$(store_workdir checkhandbook) || exit 2
trap 'rm -rf "$WORK"' EXIT INT TERM

# ⛔ THE EXTRACTION, IN DOCUMENT ORDER, INTO ONE SCRIPT. Concatenated rather than
# run one at a time, because the page's steps depend on each other and a reader
# following it types them into one shell.
awk '
  /^```sh$/ { inblock = 1; n += 1; print "### step " n; next }
  /^```/    { inblock = 0; next }
  inblock   { print }
' "$DOC" >"$WORK/walkthrough.sh"

STEPS=$(grep -c '^### step ' "$WORK/walkthrough.sh")
if [ "$STEPS" -lt 4 ]; then
  fail "extract  the handbook carries $STEPS runnable step(s); a walkthrough needs several"
else
  pass "extract  $STEPS step(s) taken out of the handbook, in document order"
fi

REPO="$ROOT"
SUBMISSION="$WORK/submission"
EMPTY="$WORK/empty"
BIN="${CARGO_TARGET_DIR:-$ROOT/target}/debug/examples"
export REPO SUBMISSION EMPTY BIN

# ⛔ Unpiped, and with -e so the walkthrough stops on the step that failed rather
# than on the last one.
(cd "$WORK" && sh -e "$WORK/walkthrough.sh") >"$WORK/out" 2>&1
RC=$?
if [ "$RC" = "0" ]; then
  pass "walk     every step ran, in order, with nothing the page does not carry"
else
  fail "walk     a step failed (exit $RC): $(tail -3 "$WORK/out" | tr '\n' ' ')"
fi

# ⛔ AND THE RESULT IS VALIDATED SEPARATELY. The page's own last steps validate,
# so this would pass on their exit code alone; asking again here, from outside
# the walkthrough, is what distinguishes "the page ran" from "the page produced
# something acceptable".
#
# ⚠ THE VERDICT IS READ FROM WHAT THE VALIDATOR SAID, NOT ONLY FROM ITS EXIT
# CODE. A mutation that replaced this call with `true` left the whole harness
# green, because an exit code of 0 is what `true` produces too. Requiring the
# validator's own line means the case cannot pass unless it ran.
if [ -d "$SUBMISSION" ] && [ -x "$BIN/validate-corpus" ]; then
  "$BIN/validate-corpus" "$SUBMISSION" >"$WORK/validate" 2>&1
  VRC=$?
  if [ "$VRC" = "0" ] && grep -q 'valid store:' "$WORK/validate"; then
    pass "accepted  the submission is validator-accepted: $(head -1 "$WORK/validate")"
  elif [ "$VRC" = "0" ]; then
    fail "accepted  exit 0 with no verdict line; the validator did not run"
  else
    fail "accepted  the validator refused it (exit $VRC): $(head -3 "$WORK/validate" | tr '\n' ' ')"
  fi
else
  fail "accepted  the walkthrough produced no submission directory"
fi

# ⚠ AND IT IS NOT EMPTY. A validator accepts an empty store, so a walkthrough
# that created a directory and nothing else would pass every case above.
DOCUMENTS=$(find "$SUBMISSION" -name '*.json' 2>/dev/null | wc -l | tr -d ' ')
if [ "$DOCUMENTS" -ge 2 ]; then
  pass "content  the submission carries $DOCUMENTS document(s), so the validator had something to accept"
else
  fail "content  the submission carries $DOCUMENTS document(s)"
fi

# ⛔ THE NEGATIVE CONTROL. A submission the validator would refuse must be
# refused, or the case above says nothing. One evidence artifact is removed,
# which is E-CRP-03: a citation the store cannot resolve.
BROKEN="$WORK/broken"
rm -rf "$BROKEN"
if cp -r "$SUBMISSION" "$BROKEN" 2>/dev/null &&
  ARTIFACT=$(find "$BROKEN/raw" -type f ! -name 'manifest.json' 2>/dev/null | head -1) &&
  [ -n "$ARTIFACT" ] && rm -f "$ARTIFACT"; then
  "$BIN/validate-corpus" "$BROKEN" >"$WORK/broken.out" 2>&1
  BRC=$?
  if [ "$BRC" = "1" ]; then
    pass "control  a submission missing a cited artifact is refused"
  else
    fail "control  expected exit 1 over a missing artifact, got $BRC"
  fi
else
  fail "control  could not build the negative control"
fi

# ⛔ AND THE PAGE MUST TEACH THE CHECK, NOT ONLY PASS IT. The harness validates
# the result independently, so a page that dropped its own validation step would
# still produce an acceptable submission and every case above would stay green.
# ⚠ Measured: removing `validate-corpus` from the page left this harness green
# until this case existed. What a contributor is told to run before submitting is
# the point of the page, so it is asserted rather than inferred.
# ⚠ THE INVOCATION FORM, NOT THE BARE NAME. The first version of this grepped
# for `validate-corpus` and matched the build step's own `--example
# validate-corpus` argument, so deleting the invocation left the case green: a
# check that passes because a different line happens to satisfy it. `$BIN/` is
# what makes it the call.
MISSING=""
for TAUGHT in validate-corpus check-store build-store; do
  grep -q -F -e "\$BIN/$TAUGHT" "$WORK/walkthrough.sh" || MISSING="$MISSING $TAUGHT"
done
if [ -z "$MISSING" ]; then
  pass "teaches  the page's own steps build, validate and check the submission"
else
  fail "teaches  the walkthrough does not run:$MISSING"
fi

# ⚠ THE EXTRACTOR'S OWN RULE, CHECKED. The page carries an illustrative block
# showing the two host guards, which must never be executed: running --egress on
# a session host is the refusal the boundary exists for, and running --claim
# would write a marker onto this machine.
TEXT_BLOCKS=$(grep -c '^```text$' "$DOC")
if [ "$TEXT_BLOCKS" -lt 1 ]; then
  fail "probe    the handbook carries no illustrative block, so the language rule declines nothing"
elif grep -q 'assert-disposable' "$WORK/walkthrough.sh"; then
  fail "probe    the extractor took the host guards into the walkthrough"
else
  pass "probe    the host-guard block was shown and not run"
fi

store_report check-handbook/1 cases "$JSON"
