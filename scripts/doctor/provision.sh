#!/bin/sh
# provision.sh - give this host the tools the gate needs, at the versions this
# project pins, by running something rather than by reading a document.
#
# ⛔ THE GAP THIS CLOSES IS A GATE THAT IS QUIETLY SMALLER THAN CI's. `pwsh`,
# `shellcheck` and `shfmt` are absent on a fresh container. Without `pwsh` every
# paired check loses its PowerShell half; without the other two, shell syntax and
# style are not checked here at all. ⚠ Both gaps turned CI red, once each, on
# defects a local run would have caught in seconds - and the commands lived in a
# document, so every session either transcribed them or ran the smaller gate
# without noticing which.
#
# ⛔ A DOWNLOAD IS VERIFIED BEFORE IT IS EXECUTED, AND BY `sha256sum -c` RATHER
# THAN BY A COMPARISON WRITTEN HERE. That is a reader this project did not write,
# it reports per file, and its exit code is read from the process that produced
# it.
#
# -- ⛔ WHERE EACH PIN CAME FROM, WHICH IS NOT THE SAME FOR ALL THREE ----------
#
# ⭐ `pwsh` is corroborated. PowerShell publishes `hashes.sha256` beside the
# release, and the digest below is the line that file carries for this artifact;
# measured on 2026-09-08, the vendor's value and the bytes this project
# downloaded agree.
#
# ⚠ `shellcheck` and `shfmt` publish no checksum file, so their digests are the
# bytes this project observed on 2026-09-08, fetched twice and identical both
# times. ⛔ That makes them IMMUTABILITY pins and not authenticity ones: they
# guarantee the artifact cannot change under this project from now on, and they
# say nothing about who built it. Writing them as though they were the same kind
# of evidence as the first would be the claim this repository refuses.
#
# ⭐ `shfmt` HAS A STRONGER ROUTE WHEN `go` IS PRESENT, and it is the route CI
# uses: `go install mvdan.cc/sh/v3/cmd/shfmt@v3.14.0` verifies the module against
# Go's public checksum database, so a re-tagged release fails to install rather
# than installing different code. ⚠ Taking it also makes the two hosts run the
# same build rather than two builds of one version. The pinned binary is the
# fallback for a host with no Go.
#
# Usage:
#   sh scripts/doctor/provision.sh            install whatever is missing or wrong
#   sh scripts/doctor/provision.sh --check    report only; install nothing
#   sh scripts/doctor/provision.sh --json
#
# Exit codes: 0 every tool is present at its pinned version, 1 one is not and
# could not be made so, 2 could not run.
#
# ⛔ Read the exit code from this process, unpiped.

set -u

ME=provision
CHECK=0
JSON=0

while [ $# -gt 0 ]; do
  case "$1" in
    --check) CHECK=1 ;;
    --json) JSON=1 ;;
    -h | --help)
      awk 'NR>1 { if (/^#/) { sub(/^# ?/, ""); print } else exit }' "$0"
      exit 0
      ;;
    *)
      printf '%s: unknown argument: %s\n' "$ME" "$1" >&2
      exit 2
      ;;
  esac
  shift
done

for _tool in curl tar sha256sum awk; do
  command -v "$_tool" >/dev/null 2>&1 || {
    printf '%s: %s not found\n' "$ME" "$_tool" >&2
    exit 2
  }
done

BIN=${BIT_IDS_PROVISION_BIN:-/usr/local/bin}
WORK=${TMPDIR:-/tmp}/.$ME.$$
mkdir -p "$WORK" || {
  printf '%s: cannot write to %s\n' "$ME" "$WORK" >&2
  exit 2
}
trap 'rm -rf "$WORK"' EXIT INT TERM

# -- the pins -----------------------------------------------------------------
#
# ⚠ The URL is composed from the version and the digest is not: the digest is
# what identifies the bytes, so a version bumped without a new digest fails to
# verify rather than installing something nobody pinned.
PWSH_VERSION=7.4.6
PWSH_URL="https://github.com/PowerShell/PowerShell/releases/download/v$PWSH_VERSION/powershell-$PWSH_VERSION-linux-x64.tar.gz"
PWSH_PINNED_SHA256=6f6015203c47806c5cc444c19d8ed019695e610fbd948154264bf9ca8e157561

SHELLCHECK_VERSION=0.10.0
SHELLCHECK_URL="https://github.com/koalaman/shellcheck/releases/download/v$SHELLCHECK_VERSION/shellcheck-v$SHELLCHECK_VERSION.linux.x86_64.tar.xz"
SHELLCHECK_PINNED_SHA256=6c881ab0698e4e6ea235245f22832860544f17ba386442fe7e9d629f8cbedf87

SHFMT_VERSION=3.14.0
SHFMT_URL="https://github.com/mvdan/sh/releases/download/v$SHFMT_VERSION/shfmt_v${SHFMT_VERSION}_linux_amd64"
SHFMT_PINNED_SHA256=fe42021c7272ef2d67ea36cbc3031683c625d0badec733ef3a57b567246a0b66
SHFMT_MODULE="mvdan.cc/sh/v3/cmd/shfmt@v$SHFMT_VERSION"

ROWS=0
BAD=0
row() {
  ROWS=$((ROWS + 1))
  [ "$JSON" = 1 ] || printf '  %s\n' "$1"
}
bad() {
  BAD=$((BAD + 1))
  ROWS=$((ROWS + 1))
  [ "$JSON" = 1 ] || printf '  %s\n' "$1"
}

# ⛔ THE VERSION IS ASKED OF THE INSTALLED EXECUTABLE, UNPIPED, and the field is
# taken from the answer afterwards. A version read from a path or a package
# index is not the tool speaking.
installed_version() { # tool
  case "$1" in
    pwsh)
      command -v pwsh >/dev/null 2>&1 || return 1
      _out=$(pwsh --version 2>/dev/null </dev/null) || return 1
      printf '%s' "$_out" | awk '{ sub(/^v/, "", $2); print $2; exit }'
      ;;
    shellcheck)
      command -v shellcheck >/dev/null 2>&1 || return 1
      _out=$(shellcheck --version 2>/dev/null </dev/null) || return 1
      printf '%s\n' "$_out" | awk '$1 == "version:" { print $2; exit }'
      ;;
    shfmt)
      command -v shfmt >/dev/null 2>&1 || return 1
      _out=$(shfmt --version 2>/dev/null </dev/null) || return 1
      printf '%s' "$_out" | awk '{ sub(/^v/, "", $1); print $1; exit }'
      ;;
    *) return 1 ;;
  esac
}

# ⛔ VERIFIED BY sha256sum -c, WHICH IS A READER THIS PROJECT DID NOT WRITE. The
# checksum file is generated here in the format that tool defines, and its exit
# code is read from the process that produced it.
# ⛔ EVERY NAME HERE IS PREFIXED, AND THAT IS NOT STYLE. A POSIX shell function
# has no locals: `_want` in here and `_want` in `provide` are ONE variable, so
# the first version reported `pwsh answered [7.4.6] after installing
# 6f6015...` - the caller comparing a version against a digest this function had
# left in its own accumulator, and exiting 1 over two tools it had installed
# correctly. ⚠ Invisible to shellcheck, and invisible to a reading that checks
# each function on its own. ⭐ `shfmt` was the row that reported correctly,
# because `go install` is the one route that never calls this.
fetch_verified() { # url  sha256  destination
  _fv_url="$1"
  _fv_want="$2"
  _fv_dest="$3"
  curl -fsSL --retry 2 -o "$_fv_dest" "$_fv_url" </dev/null || {
    printf '%s: could not fetch %s\n' "$ME" "$_fv_url" >&2
    return 1
  }
  printf '%s  %s\n' "$_fv_want" "$(basename "$_fv_dest")" >"$WORK/sums"
  (cd "$(dirname "$_fv_dest")" && sha256sum -c "$WORK/sums" >/dev/null 2>&1)
  _fv_rc=$?
  [ "$_fv_rc" = 0 ] || {
    printf '%s: %s does not match its pinned digest; nothing was installed\n' \
      "$ME" "$_fv_url" >&2
    return 1
  }
  return 0
}

install_pwsh() {
  fetch_verified "$PWSH_URL" "$PWSH_PINNED_SHA256" "$WORK/pwsh.tar.gz" || return 1
  mkdir -p /opt/pwsh || return 1
  tar -xzf "$WORK/pwsh.tar.gz" -C /opt/pwsh || return 1
  # ⛔ THE chmod IS NOT OPTIONAL. The tarball extracts `pwsh` without the
  # executable bit on at least one image, and the failure then reads as
  # `Permission denied` rather than as a missing file - which is the step a
  # prose instruction loses.
  chmod +x /opt/pwsh/pwsh || return 1
  ln -sf /opt/pwsh/pwsh "$BIN/pwsh" || return 1
}

install_shellcheck() {
  fetch_verified "$SHELLCHECK_URL" "$SHELLCHECK_PINNED_SHA256" "$WORK/shellcheck.tar.xz" || return 1
  tar -xJf "$WORK/shellcheck.tar.xz" -C "$WORK" || return 1
  install -m755 "$WORK/shellcheck-v$SHELLCHECK_VERSION/shellcheck" "$BIN/shellcheck" || return 1
}

install_shfmt() {
  # ⭐ The route CI uses, when it is available: Go verifies the module against
  # its public checksum database, so this is an authenticity check rather than
  # an immutability pin, and both hosts end up running one build.
  if command -v go >/dev/null 2>&1; then
    if go install "$SHFMT_MODULE" >"$WORK/go.log" 2>&1; then
      _gopath=$(go env GOPATH 2>/dev/null)
      if [ -n "$_gopath" ] && [ -x "$_gopath/bin/shfmt" ]; then
        ln -sf "$_gopath/bin/shfmt" "$BIN/shfmt" || return 1
        return 0
      fi
    fi
    printf '%s: go install did not produce shfmt; falling back to the pinned binary\n' "$ME" >&2
  fi
  fetch_verified "$SHFMT_URL" "$SHFMT_PINNED_SHA256" "$WORK/shfmt" || return 1
  install -m755 "$WORK/shfmt" "$BIN/shfmt" || return 1
}

# ⚠ THE INSTALLER IS DISPATCHED BY NAME IN A `case`, NOT CALLED THROUGH A
# VARIABLE. An indirect call is invisible to shellcheck, which then reports every
# line of all three installers as unreachable - a page of noise over a file whose
# whole point is that the checks run here.
provide() { # tool  wanted-version
  _pv_tool="$1"
  _pv_want="$2"
  _pv_have=$(installed_version "$_pv_tool") || _pv_have=""
  if [ "$_pv_have" = "$_pv_want" ]; then
    row "ok      $_pv_tool $_pv_have"
    return 0
  fi
  if [ "$CHECK" = 1 ]; then
    if [ -z "$_pv_have" ]; then
      bad "absent  $_pv_tool (pinned $_pv_want)"
    else
      bad "differs $_pv_tool $_pv_have (pinned $_pv_want)"
    fi
    return 0
  fi
  _pv_ok=0
  case "$_pv_tool" in
    pwsh) install_pwsh || _pv_ok=1 ;;
    shellcheck) install_shellcheck || _pv_ok=1 ;;
    shfmt) install_shfmt || _pv_ok=1 ;;
    *) _pv_ok=1 ;;
  esac
  if [ "$_pv_ok" != 0 ]; then
    bad "failed  $_pv_tool (pinned $_pv_want)"
    return 0
  fi
  # ⛔ READ BACK. An installer that reported success and left nothing runnable is
  # the failure this whole file exists to remove, and the only thing that turns
  # `installed` into a fact is asking the tool.
  _pv_now=$(installed_version "$_pv_tool") || _pv_now=""
  if [ "$_pv_now" = "$_pv_want" ]; then
    row "installed $_pv_tool $_pv_now"
  else
    bad "wrong   $_pv_tool answered [$_pv_now] after installing $_pv_want"
  fi
}

if [ "$JSON" != 1 ]; then
  if [ "$CHECK" = 1 ]; then
    printf '%s: checking\n' "$ME"
  else
    printf '%s: installing into %s\n' "$ME" "$BIN"
  fi
fi

provide pwsh "$PWSH_VERSION"
provide shellcheck "$SHELLCHECK_VERSION"
provide shfmt "$SHFMT_VERSION"

if [ "$JSON" = 1 ]; then
  printf '{"schema":"provision/1","tools":%s,"wrong":%s}\n' "$ROWS" "$BAD"
elif [ "$BAD" = 0 ]; then
  printf '%s: every pinned tool is present.\n' "$ME"
else
  printf '%s: %s of %s tools are not at their pinned version.\n' "$ME" "$BAD" "$ROWS" >&2
fi
[ "$BAD" = 0 ] || exit 1
exit 0
