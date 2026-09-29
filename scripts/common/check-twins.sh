#!/bin/sh
# check-twins.sh - do the two probe implementations still answer the same way?
#
# The defect this exists to catch is DRIFT between two scripts that are supposed
# to do the same job. One gets a fix, a field, a flag; the other does not; and
# six months later one is polished and the other is a barebones copy that nobody
# noticed had fallen behind. The failure is silent, because each one works fine
# on its own host and nobody runs both.
#
# -- ⭐ WHEREVER A TWIN EXISTS, THIS FILE COVERS IT, AND ONE PAIR IS LEFT -----
#
# ⛔ A POSIX sh check cannot be assumed to run on Windows. Measured on one
# Windows 11 machine on 2026-08-25, from a native PowerShell session with Git
# Bash off PATH: `sed` was not installed at all, and `sort` resolved to
# PowerShell's own `Sort-Object` alias, which accepts `-u`, compares
# case-insensitively, and over the five values `b A a B a` returned `A b`
# where coreutils returns `A B a b`. ⚠ A missing tool fails loudly; an ALIASED
# one succeeds and answers differently.
#
# ⛔ So the rule is: wherever a twin exists, THIS CHECK covers it. Adding a
# twin without adding it here is how drift starts.
#
# ⭐ AND THE TWINS ARE ALMOST GONE. `CI-10` ported every `common/` check into one
# Go binary and deleted both halves of each; the doctor's probe pair, sections 1
# to 6 below, is the one this file still compares, and section 7 says where the
# rest went.
#
# ⚠ THIS HEADER USED TO SAY THE OPPOSITE, that only the probe needed a twin.
# It did not go stale: it shipped in the SAME COMMIT as the section 7 below
# that says the current rule, and it survived a later maintenance pass over
# this very file. So the file whose job is stopping two implementations
# drifting spent its whole life telling its next maintainer not to write the
# second one, with the correction two hundred lines further down. The retired
# wording is in docs/history/twins-and-scripts.md.
#
# -- WHAT DIFFERENCE IS CORRECT ----------------------------------------------
#
# ⚠ Some disagreement is honest and must not be flattened away. Each twin
# reports what ITS OWN host can reach, and on a Windows machine with msys
# installed those genuinely differ: `bash` and `tar` resolve to different
# binaries, `zsh` is on the msys PATH and not the native one, and
# PSScriptAnalyzer is a PowerShell module invisible to sh.
#
# ⭐ So this compares the SHAPE and the FACTS, not the tool-by-tool verdicts:
# the schema, the keys, the host and repo values that describe one machine.
# A machine cannot have two architectures.
#
# Usage:
#   sh scripts/common/check-twins.sh
#   sh scripts/common/check-twins.sh --json
#   sh scripts/common/check-twins.sh --verbose      also list per-tool differences
#
# Exit codes: 0 they agree, 1 they have drifted, 2 could not run.
#
# ⛔ Read the exit code from this process, unpiped.

set -u

JSON=0
VERBOSE=0

while [ $# -gt 0 ]; do
  case "$1" in
    --json) JSON=1 ;;
    --verbose) VERBOSE=1 ;;
    -h | --help)
      awk 'NR>1 { if (/^#/) { sub(/^# ?/, ""); print } else exit }' "$0"
      exit 0
      ;;
    *)
      printf 'check-twins: unknown argument: %s\n' "$1" >&2
      exit 2
      ;;
  esac
  shift
done

CDPATH=''
export CDPATH
SCRIPT_DIR=$(cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(cd -- "$SCRIPT_DIR/../.." && pwd)

SH_PROBE="$REPO_ROOT/scripts/doctor/doctor.sh"
PS_PROBE="$REPO_ROOT/scripts/doctor/doctor.ps1"

[ -f "$SH_PROBE" ] || {
  printf 'check-twins: missing %s\n' "$SH_PROBE" >&2
  exit 2
}
[ -f "$PS_PROBE" ] || {
  printf 'check-twins: missing %s\n' "$PS_PROBE" >&2
  exit 2
}

# ⚠ A missing interpreter is "could not run", exit 2, not "they disagree",
# exit 1. Those are different facts and a caller has to be able to tell them
# apart: one blocks a merge and the other is a note about the runner.
PWSH=""
for c in pwsh powershell; do
  if command -v "$c" >/dev/null 2>&1; then
    PWSH="$c"
    break
  fi
done
[ -n "$PWSH" ] || {
  printf 'check-twins: no pwsh or powershell on PATH; cannot compare\n' >&2
  exit 2
}
command -v jq >/dev/null 2>&1 || {
  printf 'check-twins: jq not found; cannot compare json\n' >&2
  exit 2
}

TMP="${TMPDIR:-/tmp}/.checktwins.$$"
mkdir -p "$TMP" || {
  printf 'check-twins: cannot write to %s\n' "$TMP" >&2
  exit 2
}
trap 'rm -rf "$TMP"' EXIT INT TERM

# ⚠ Run both from the repo root, so `repo.*` describes the same tree.
(cd "$REPO_ROOT" && sh "$SH_PROBE" --json >"$TMP/sh.json" 2>"$TMP/sh.err") || {
  printf 'check-twins: doctor.sh failed\n' >&2
  cat "$TMP/sh.err" >&2
  exit 2
}
(cd "$REPO_ROOT" && "$PWSH" -NoProfile -File "$PS_PROBE" -Json >"$TMP/ps.json" 2>"$TMP/ps.err") || {
  printf 'check-twins: doctor.ps1 failed\n' >&2
  cat "$TMP/ps.err" >&2
  exit 2
}

jq -e . "$TMP/sh.json" >/dev/null 2>&1 || {
  printf 'check-twins: doctor.sh emitted invalid json\n' >&2
  exit 2
}
jq -e . "$TMP/ps.json" >/dev/null 2>&1 || {
  printf 'check-twins: doctor.ps1 emitted invalid json\n' >&2
  exit 2
}

DRIFT=0
note() {
  printf '  DRIFT  %s\n' "$1"
  DRIFT=$((DRIFT + 1))
}
ok() { [ "$JSON" = "1" ] || printf '  ok     %s\n' "$1"; }

j() { jq -r "$1" "$2" 2>/dev/null; }

# --- 1. the schema string -----------------------------------------------------
s_schema=$(j '.schema' "$TMP/sh.json")
p_schema=$(j '.schema' "$TMP/ps.json")
if [ "$s_schema" = "$p_schema" ]; then
  ok "schema: $s_schema"
else note "schema: sh=$s_schema ps=$p_schema"; fi

# --- 2. the key sets, per section --------------------------------------------
# ⛔ A field added to one and not the other is the commonest shape of drift,
# and it is invisible to anything that only compares values.
for sec in host repo summary probe; do
  a=$(jq -r --arg s "$sec" '.[$s] | keys_unsorted | sort | join(",")' "$TMP/sh.json" 2>/dev/null)
  b=$(jq -r --arg s "$sec" '.[$s] | keys_unsorted | sort | join(",")' "$TMP/ps.json" 2>/dev/null)
  if [ "$a" = "$b" ]; then
    ok "$sec keys match"
  else
    note "$sec keys differ"
    printf '           sh: %s\n' "$a"
    printf '           ps: %s\n' "$b"
  fi
done

t=$(jq -r 'keys_unsorted | sort | join(",")' "$TMP/sh.json" 2>/dev/null)
u=$(jq -r 'keys_unsorted | sort | join(",")' "$TMP/ps.json" 2>/dev/null)
if [ "$t" = "$u" ]; then
  ok "top-level keys match"
else note "top-level keys differ: sh=[$t] ps=[$u]"; fi

# --- 3. the facts about THIS machine -----------------------------------------
# ⚠ These describe one host, so they cannot honestly differ. `flavor` is
# excluded on purpose: it reports the SHELL environment, so msys and native are
# both correct answers from the same machine.
for f in os wsl container arch distro distro_version; do
  a=$(j ".host.$f" "$TMP/sh.json")
  b=$(j ".host.$f" "$TMP/ps.json")
  if [ "$a" = "$b" ]; then
    ok "host.$f = $a"
  else note "host.$f: sh=[$a] ps=[$b]"; fi
done

for f in is_git dirty commits remote_looks_like_template has_codegraph; do
  a=$(j ".repo.$f" "$TMP/sh.json")
  b=$(j ".repo.$f" "$TMP/ps.json")
  if [ "$a" = "$b" ]; then
    ok "repo.$f = $a"
  else note "repo.$f: sh=[$a] ps=[$b]"; fi
done

a=$(jq -r '.repo.ecosystems | sort | join(",")' "$TMP/sh.json" 2>/dev/null)
b=$(jq -r '.repo.ecosystems | sort | join(",")' "$TMP/ps.json" 2>/dev/null)
if [ "$a" = "$b" ]; then
  ok "repo.ecosystems = [${a:-none}]"
else note "repo.ecosystems: sh=[$a] ps=[$b]"; fi

# --- 4. the tool object's shape ----------------------------------------------
a=$(jq -r '[.tools[0] | keys_unsorted | sort | .[]] | join(",")' "$TMP/sh.json" 2>/dev/null)
b=$(jq -r '[.tools[0] | keys_unsorted | sort | .[]] | join(",")' "$TMP/ps.json" 2>/dev/null)
if [ "$a" = "$b" ]; then
  ok "tool object shape: $a"
else note "tool object shape: sh=[$a] ps=[$b]"; fi

# --- 5. the probed-tool sets --------------------------------------------------
# ⚠ Not the verdicts. A tool one host can reach and the other cannot is an
# honest difference. What is NOT honest is one twin having forgotten to probe
# something the other does, so the ID SETS are compared and the known
# host-specific extras are named rather than silently tolerated.
jq -r '.tools[].id' "$TMP/sh.json" | sort >"$TMP/sh.ids"
jq -r '.tools[].id' "$TMP/ps.json" | sort >"$TMP/ps.ids"

# Documented, host-specific, and each with a reason. Anything outside this list
# is drift.
cat >"$TMP/allowed" <<'ALLOWED'
psscriptanalyzer
ALLOWED

only_ps=$(comm -13 "$TMP/sh.ids" "$TMP/ps.ids" | grep -vxF -f "$TMP/allowed" || true)
only_sh=$(comm -23 "$TMP/sh.ids" "$TMP/ps.ids" || true)

if [ -z "$only_ps" ] && [ -z "$only_sh" ]; then
  ok "both probe the same $(wc -l <"$TMP/sh.ids" | tr -d ' ') tools"
else
  [ -n "$only_sh" ] && note "probed by doctor.sh only: $(printf '%s' "$only_sh" | tr '\n' ' ')"
  [ -n "$only_ps" ] && note "probed by doctor.ps1 only: $(printf '%s' "$only_ps" | tr '\n' ' ')"
  printf '           If a difference is correct, add the id to the allowed list\n'
  printf '           in this script WITH THE REASON. An unexplained one is drift.\n'
fi

# --- 6. the CLI surface -------------------------------------------------------
# ⛔ A FLAG IN ONE TWIN AND NOT THE OTHER IS DRIFT THE JSON CANNOT SHOW.
# Everything above compares what the probes OUTPUT. Nothing above compares what
# they ACCEPT, and the two are independent: `doctor.sh --text` exited 0 while
# `doctor.ps1 -Text` exited 1 with a parameter-binding error, and every
# comparison in this file passed while that was true. The README's flag table
# listed four flags and the sh probe had five.
#
# ⚠ The names are compared, not the spellings. `--fast` and `-Fast` are the
# same flag: POSIX and PowerShell conventions differ and neither is wrong.
# Help is excluded: PowerShell supplies `-?` itself and there is nothing to
# match it against.
sh_flags=$(awk '
  /^while \[ \$# -gt 0 \]/ { inloop = 1 }
  inloop && /^done/        { exit }
  inloop && match($0, /^[[:space:]]*-[^)]*\)/) {
    s = substr($0, RSTART, RLENGTH - 1)
    gsub(/[[:space:]]/, "", s)
    n = split(s, parts, "|")
    for (i = 1; i <= n; i++) {
      f = parts[i]
      sub(/^-+/, "", f)
      if (f != "h" && f != "help") print tolower(f)
    }
  }
' "$SH_PROBE" | sort -u)

ps_flags=$(awk '
  /^param\(/ { inparam = 1; next }
  inparam && /^\)/ { exit }
  inparam && match($0, /\$[A-Za-z][A-Za-z0-9]*/) {
    f = substr($0, RSTART + 1, RLENGTH - 1)
    print tolower(f)
  }
' "$PS_PROBE" | sort -u)

if [ "$sh_flags" = "$ps_flags" ]; then
  ok "cli flags match: $(printf '%s' "$sh_flags" | tr '\n' ' ')"
else
  note "cli flags differ"
  printf '           sh: %s\n' "$(printf '%s' "$sh_flags" | tr '\n' ' ')"
  printf '           ps: %s\n' "$(printf '%s' "$ps_flags" | tr '\n' ' ')"
  printf '           A flag one twin accepts and the other rejects is drift a\n'
  printf '           schema comparison cannot see. Add it to BOTH.\n'
fi

# --- 7. the other twin pairs: none is left ------------------------------------
# ⭐ CI-10 DELETED EVERY ONE, and a pair left this file only after
# `check-bitcheck.sh --compare` had run both of its halves against the Go port
# over the same plants. The last, `mine-repo`, went on 2026-09-29, and with it
# the machinery that ran the pairs at once and read each verdict in list order,
# which nothing called any more. What each comparison found - and why this file
# could not have found it, since it compares answers on the tree it runs
# against and a drift that only a planted shape shows is invisible to that - is
# in TODO/ci.md under CI-10.
#
# ⛔ SO A NEW TWIN IS NOT ADDED HERE. A new rule goes into tools/check/, which is
# one implementation on both platforms and cannot drift from itself.

# --- 8. per-tool verdicts, on request ----------------------------------------
if [ "$VERBOSE" = "1" ]; then
  printf '\n  per-tool differences (informational, not drift):\n'
  jq -r '.tools[] | [.id, (.found|tostring), .version] | @tsv' "$TMP/sh.json" | sort >"$TMP/sh.t"
  jq -r '.tools[] | [.id, (.found|tostring), .version] | @tsv' "$TMP/ps.json" | sort >"$TMP/ps.t"
  join -t"$(printf '\t')" "$TMP/sh.t" "$TMP/ps.t" 2>/dev/null |
    awk -F'\t' '$2 != $4 || $3 != $5 { printf "    %-16s sh=%s/%s  ps=%s/%s\n", $1, $2, $3, $4, $5 }'
fi

# --- report -------------------------------------------------------------------
if [ "$JSON" = "1" ]; then
  printf '{"schema":"check-twins/1","drift":%s}\n' "$DRIFT"
  [ "$DRIFT" -gt 0 ] && exit 1
  exit 0
fi

printf '\n'
if [ "$DRIFT" -gt 0 ]; then
  printf '⛔ the twins have drifted in %s place(s).\n\n' "$DRIFT"
  printf 'A field, a flag or a fact in one and not the other. Fix BOTH, or, if the\n'
  printf 'difference is genuinely host-specific, record it in this script with the\n'
  printf 'reason. ⛔ Do not widen the comparison to make a failure go away: that is\n'
  printf 'how the check stops checking.\n'
  exit 1
fi
printf '✅ every twin pair agrees on this tree.\n'
exit 0
