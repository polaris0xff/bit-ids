#!/bin/sh
# store-lib.sh - what the two corpus mutation harnesses both need.
#
# ⛔ IT IS SOURCED, NEVER RUN. It defines functions and sets nothing else, so
# sourcing it twice is harmless and running it does nothing. `check-store.sh`
# and `check-corpus.sh` both plant defects in a disposable tree and read the
# refusal, and the machinery for that is identical: build the example, make a
# scratch tree, digest a whole directory, verify a plant landed, count a row.
#
# The alternative was a copy in each, which is the divergent-copies row in
# docs/conventions/forbidden-patterns.md. ⚠ The copies would have diverged in the
# place that matters most: `replace_once` grew a rule about multi-line literals
# only after one of them was written.
#
# A caller sets ME to its own name for diagnostics, then calls store_require,
# store_build and store_workdir before anything else.

# Every tool the harnesses need, checked once and named when absent.
# ⛔ Exit 2, never 1. A machine that cannot run a check has not failed it, and
# the gate runner reads 2 as a skip.
store_require() { # tool...
  for _tool in "$@"; do
    command -v "$_tool" >/dev/null 2>&1 || {
      printf '%s: %s not found\n' "$ME" "$_tool" >&2
      exit 2
    }
  done
}

# Builds one example and reports the path to it.
#
# ⛔ It checks the binary is there afterwards. A build that exits 0 having
# produced nothing is the "step that exits 0 having done nothing" row, and every
# case downstream of it would report a guard that failed to fire.
store_build() { # root example
  if ! cargo build --manifest-path "$1/Cargo.toml" -p bit-ids --locked \
    --example "$2" >/dev/null 2>&1; then
    printf '%s: cannot build the %s example\n' "$ME" "$2" >&2
    exit 2
  fi
  _bin="$1/target/debug/examples/$2"
  [ -x "$_bin" ] || {
    printf '%s: %s is not executable after a successful build\n' "$ME" "$_bin" >&2
    exit 2
  }
  printf '%s\n' "$_bin"
}

# A scratch directory that removes itself. The caller installs the trap, because
# a trap set inside a function is the caller's anyway and hiding that is worse
# than writing it twice.
store_workdir() { # tag
  _dir="${TMPDIR:-/tmp}/.$1.$$"
  mkdir -p "$_dir" || {
    printf '%s: cannot write to %s\n' "$ME" "$_dir" >&2
    exit 2
  }
  printf '%s\n' "$_dir"
}

# ⛔ PREFIXED, BECAUSE A SOURCED LIBRARY SHARES ONE NAMESPACE WITH ITS CALLER.
# These were PASS, FAIL and ROWS, and check-indexes.sh assigned its own ROWS for
# a row count. The accumulator was overwritten, two rows vanished from the
# report, and the run still printed "10 passed" over eight lines: a report that
# lies about itself, produced by nothing more than a variable name.
#
# ⭐ The prefix makes the collision unlikely and store_report makes it visible,
# which is the half that matters: a naming convention is a rule nobody checks.
STORE_PASS=0
STORE_FAIL=0
STORE_ROWS=""

row() {
  STORE_ROWS="$STORE_ROWS  $1
"
}

fail() {
  row "❌ $1"
  STORE_FAIL=$((STORE_FAIL + 1))
}

pass() {
  row "✅ $1"
  STORE_PASS=$((STORE_PASS + 1))
}

# ⛔ THE LAYOUT IS ASKED FOR, NEVER SPELLED IN SHELL. A second copy of the path
# derivation here is the drift check-twins.sh exists to catch, and it would
# drift in the direction that makes every case pass over a tree the store would
# never have written. `check-store --where` is the one derivation.
place() { # locator tree record
  _rel=$("$1" --where "$3")
  [ -n "$_rel" ] || return 1
  mkdir -p "$2/$(dirname "$_rel")" || return 1
  cp "$3" "$2/$_rel" || return 1
  printf '%s\n' "$_rel"
}

# A digest over every path in a tree and what sits at it, so a plant that did not
# land is visible as a digest that did not move.
tree_digest() { # dir
  (
    cd "$1" 2>/dev/null || exit 1
    find . \( -type f -o -type l -o -type p \) | LC_ALL=C sort | while read -r p; do
      if [ -L "$p" ]; then
        printf '%s symlink\n' "$p"
      elif [ -p "$p" ]; then
        printf '%s fifo\n' "$p"
      else
        printf '%s %s\n' "$p" "$(sha256sum "$p" | cut -d' ' -f1)"
      fi
    done
  ) | sha256sum | cut -d' ' -f1
}

tree_files() { # dir
  find "$1" \( -type f -o -type l -o -type p \) | wc -l | tr -d ' '
}

# ⛔ EXACTLY ONCE, OR NOT AT ALL. A literal that matches twice edits something
# other than what the case names, and one that matches nothing edits nothing
# while the case still reports a guard that failed to fire.
#
# ⚠ SINGLE-LINE LITERALS ONLY, AND THAT IS CHECKED RATHER THAN ASSUMED. grep -F
# splits a pattern containing a newline into separate alternatives, so a unique
# multi-line literal counts as the sum of its lines and this function would
# report it ambiguous. Measured on 2026-09-05, where exactly that miscounted
# three plants as NOT-PLANTED. Refusing one outright is honest; counting one
# wrongly is the defect this function exists to prevent.
replace_once() { # file literal replacement
  case "$2" in
    *"
"*)
      return 1
      ;;
  esac
  _hits=$(grep -o -F -e "$2" "$1" 2>/dev/null | wc -l | tr -d ' ')
  [ "$_hits" = "1" ] || return 1
  _before=$(sha256sum "$1" | cut -d' ' -f1)
  sed -i "s/$2/$3/" "$1" || return 1
  _after=$(sha256sum "$1" | cut -d' ' -f1)
  [ "$_before" != "$_after" ] || return 1
  return 0
}

# The three self-guards every harness that plants defects owes. ⭐ A probe's
# guard is a guard like any other, and this project has been burned by an
# unverified plant three times.
store_probe_guards() { # file present-literal ambiguous-literal
  if replace_once "$1" "a literal this file does not carry" "x"; then
    fail "probe    an absent literal was reported as planted"
  else
    pass "probe    an absent literal is refused"
  fi

  if replace_once "$1" "$3" "x"; then
    fail "probe    an ambiguous literal was reported as planted"
  else
    pass "probe    an ambiguous literal is refused"
  fi

  if replace_once "$1" "$2" "$2"; then
    fail "probe    a no-op edit was reported as planted"
  else
    pass "probe    a no-op edit is refused"
  fi

  if replace_once "$1" "$2
" "x"; then
    fail "probe    a multi-line literal was reported as planted"
  else
    pass "probe    a multi-line literal is refused"
  fi
}

# The shared verdict. ⛔ A run that passed nothing is red whatever else it says:
# zero failures out of zero cases executed is the shape these runners exist to
# refuse.
#
# ⚠ The json flag is an argument rather than a variable this reads out of the
# caller. Ambient state across a source boundary is invisible to shellcheck, and
# silencing the resulting diagnostic teaches the next reader to silence it again.
store_report() { # schema noun json
  _total=$((STORE_PASS + STORE_FAIL))

  # ⛔ THE REPORT CHECKS ITSELF. The row list and the counters are two records of
  # one fact, and a value in two places with nothing comparing them is the copy
  # a reader trusts being the wrong one. Measured: a caller's own variable
  # overwrote the accumulator and the summary went on claiming a count the rows
  # did not support.
  _rows=$(printf '%s' "$STORE_ROWS" | grep -c .)
  if [ "$_rows" != "$_total" ]; then
    printf '%s: %s rows recorded, %s counted; the report does not describe itself\n' \
      "$ME" "$_rows" "$_total" >&2
    return 1
  fi

  if [ "$STORE_PASS" -eq 0 ] || [ "$STORE_FAIL" -gt 0 ]; then
    _rc=1
  else
    _rc=0
  fi

  if [ "${3:-0}" = "1" ]; then
    printf '{"schema":"%s","total":%s,"passed":%s,"failed":%s}\n' \
      "$1" "$_total" "$STORE_PASS" "$STORE_FAIL"
    return "$_rc"
  fi

  printf '\n%s\n' "$STORE_ROWS"
  printf '%s %s: %s passed, %s failed\n' "$_total" "$2" "$STORE_PASS" "$STORE_FAIL"
  if [ "$STORE_PASS" -eq 0 ]; then
    printf -- '❌ NOTHING RAN. Zero cases passed, so this is red whatever else it says.\n'
  elif [ "$STORE_FAIL" -gt 0 ]; then
    printf -- '❌ a guard did not refuse its defect.\n'
  else
    printf -- '✅ every planted defect was refused, and the clean tree was not.\n'
  fi
  return "$_rc"
}
