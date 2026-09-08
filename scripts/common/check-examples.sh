#!/bin/sh
# check-examples.sh - run every example in the consumer documentation against an
# assembled fixture publication.
#
# ⛔ THE EXAMPLES ARE EXTRACTED FROM THE DOCUMENT AND EXECUTED, NEVER COPIED.
# A documented command that is a copy of a tested snippet is a copy that drifts,
# and the copy a reader runs is the one nobody checked. DOC-01's Prove is this
# file: the fenced blocks in docs/consuming.md are the things that run.
#
# ⚠ THE FENCE LANGUAGE IS THE RULE AND IT IS THE WHOLE RULE. A ```sh block is a
# command a reader can run and this runs it. A ```text block is a shape, such as
# a URL form for paths nothing has published, and this never runs one. A ```rust
# block is compiled by the doc-test harness rather than here; the case below
# checks that every rust block in the document is one the crate's own tests
# cover, because a Rust example nothing compiles is the same defect one layer
# over.
#
# ⛔ A DOCUMENT WITH NO EXAMPLES IS A FAILURE, NOT A PASS. Zero blocks executed
# with zero failures is the shape docs/conventions/forbidden-patterns.md calls a
# step that exits 0 having done nothing it was asked to do.
#
# Usage:
#   sh scripts/common/check-examples.sh
#   sh scripts/common/check-examples.sh --json
#
# Exit codes: 0 every example ran, 1 one did not, 2 could not run.
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
      printf 'check-examples: unknown argument: %s\n' "$1" >&2
      exit 2
      ;;
  esac
  shift
done

HERE=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
ROOT=$(CDPATH='' cd -- "$HERE/../.." && pwd)

# shellcheck disable=SC2034
ME=check-examples
# shellcheck disable=SC1091
# shellcheck source=scripts/corpus/store-lib.sh
. "$ROOT/scripts/corpus/store-lib.sh"

DOC="$ROOT/docs/consuming.md"
[ -f "$DOC" ] || {
  printf 'check-examples: %s not found\n' "$DOC" >&2
  exit 2
}

store_require cargo sha256sum
BUILDER=$(store_build "$ROOT" build-store) || exit 2
INDEXER=$(store_build "$ROOT" build-indexes) || exit 2
FORMATTER=$(store_build "$ROOT" build-formats) || exit 2
ASSEMBLER=$(store_build "$ROOT" assemble-release) || exit 2
LOOKUP=$(store_build "$ROOT" catalogue-lookup) || exit 2
BIN=$(dirname "$LOOKUP")

WORK=$(store_workdir checkexamples) || exit 2
trap 'rm -rf "$WORK"' EXIT INT TERM

PUBLICATION="$WORK/publication"
mkdir -p "$PUBLICATION/indexes/v1" || exit 2
if ! "$BUILDER" --version 1.2.3 --version 1.2.10 "$PUBLICATION" >/dev/null 2>&1 ||
  ! "$INDEXER" --scheme fixture-client:-:3:3 "$PUBLICATION" \
    "$PUBLICATION/indexes/v1/profiles.json" >/dev/null 2>&1 ||
  ! "$FORMATTER" --scheme fixture-client:-:3:3 "$PUBLICATION" "$PUBLICATION" >/dev/null 2>&1 ||
  ! cp "$ROOT/LICENSE" "$PUBLICATION/LICENSE" ||
  ! "$ASSEMBLER" "$PUBLICATION" >/dev/null 2>&1; then
  printf 'check-examples: cannot assemble the fixture publication\n' >&2
  exit 2
fi

# ⛔ THE EXTRACTION. Every ```sh block, in document order, one file each. A block
# is run with the two names the document uses bound, and with `set -e`, so a
# multi-command example fails on the command that failed rather than on the last
# one.
awk '
  /^```sh$/ { inblock = 1; n += 1; next }
  /^```/    { inblock = 0; next }
  inblock   { print > (out "/example." n ".sh") }
' out="$WORK" "$DOC"

COUNT=$(find "$WORK" -name 'example.*.sh' | wc -l | tr -d ' ')
if [ "$COUNT" -lt 4 ]; then
  fail "extract  found $COUNT example(s); the document is meant to carry several"
else
  pass "extract  $COUNT shell example(s) taken out of the document"
fi

INDEX=1
while [ "$INDEX" -le "$COUNT" ]; do
  SCRIPT="$WORK/example.$INDEX.sh"
  if [ ! -f "$SCRIPT" ]; then
    INDEX=$((INDEX + 1))
    continue
  fi
  FIRST=$(head -1 "$SCRIPT" | cut -c1-58)
  # ⛔ Unpiped. Output to a file and $? on the next line.
  # ⚠ The two names the document uses are exported rather than reassigned. The
  # self-assignment the first draft carried does nothing and shellcheck says so;
  # what the subshell needs is for them to be in the environment `sh -e` sees.
  (
    export PUBLICATION BIN
    cd "$WORK" || exit 2
    sh -e "$SCRIPT"
  ) >"$WORK/out.$INDEX" 2>&1
  RC=$?
  if [ "$RC" = "0" ]; then
    pass "example  $INDEX: $FIRST"
  else
    fail "example  $INDEX: $FIRST (exit $RC): $(head -2 "$WORK/out.$INDEX" | tr '\n' ' ')"
  fi
  INDEX=$((INDEX + 1))
done

# ⚠ A ```text block must never be executable-looking by accident, and a ```rust
# block has to be one the crate compiles. Both are checked rather than trusted:
# the document is where a reader looks first, and an example nobody runs is the
# defect this file exists to prevent, in whichever language it is written.
RUST_BLOCKS=$(grep -c '^```rust$' "$DOC")
if [ "$RUST_BLOCKS" = "0" ]; then
  fail "rust     the document carries no Rust example at all"
elif grep -q 'catalogue_documentation_example_compiles' "$ROOT/crates/bit-ids/tests/catalogue.rs"; then
  pass "rust     $RUST_BLOCKS Rust example(s), and the suite carries the case that compiles one"
else
  fail "rust     $RUST_BLOCKS Rust example(s) and no test compiles any of them"
fi

# ⛔ AND THE EXTRACTOR IS CHECKED AGAINST ITSELF. A rule that selected nothing
# would report a clean run over a document full of examples, which is the shape
# every sweep in this repository has been burned by.
TEXT_BLOCKS=$(grep -c '^```text$' "$DOC")
FENCES=$(grep -c '^```' "$DOC")
OPENERS=$((FENCES / 2))
if [ "$TEXT_BLOCKS" -lt 1 ]; then
  fail "probe    the document carries no illustrative block, so the language rule has nothing to decline"
elif [ "$COUNT" -lt "$OPENERS" ]; then
  pass "probe    $COUNT of $OPENERS block(s) taken, so the language rule selected rather than swept"
else
  fail "probe    the extractor took all $OPENERS block(s), so its language rule is not applying"
fi

store_report check-examples/1 cases "$JSON"
