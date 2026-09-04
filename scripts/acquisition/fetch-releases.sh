#!/bin/sh
# fetch-releases.sh - retrieve one release listing and keep the exact bytes.
#
# ⭐ IT FETCHES AND NOTHING ELSE. It does not parse, sort, filter or decide.
# scripts/README.md puts the line where this repository puts it: shell
# orchestrates, Rust parses. A shell script that also picked the newest version
# would be a second implementation of the rule in
# crates/bit-ids/src/resolution.rs, with no test behind it and no record of what
# it read.
#
# The bytes land in a file and the caller hands that file to
# `cargo run --example resolve-stable`. That split is what makes the digest in a
# resolution mean something: it is of what arrived, not of what a parser
# reconstructed afterwards.
#
# -- ⛔ THE ROUTE IS NOT A PREFERENCE ----------------------------------------
#
# docs/AGENTS.md rule 8: prefer authenticated `gh` for GitHub reads, and use
# https://api.gh.pkgforge.dev/<PATH> for a read-only REST path when it is not
# available. ⚠ An unauthenticated api.github.com is rate limited per address and
# answers 403 from a shared runner, so falling back to it silently would make
# this script work locally and fail in CI. The route that answered is printed,
# because a resolution that cannot say where its bytes came from is not
# replayable.
#
# Usage:
#   sh scripts/acquisition/fetch-releases.sh <owner/repo> <output-file>
#   sh scripts/acquisition/fetch-releases.sh --url <https url> <output-file>
#
# Prints the URL that answered on stdout. Exit codes: 0 fetched, 1 the source
# refused or answered with nothing, 2 could not run.
#
# ⛔ Read the exit code from this process, unpiped.

set -u

PER_PAGE=100

usage() {
  awk 'NR>1 { if (/^#/) { sub(/^# ?/, ""); print } else exit }' "$0"
}

case "${1:-}" in
  -h | --help)
    usage
    exit 0
    ;;
esac

if [ "${1:-}" = "--url" ]; then
  [ $# -eq 3 ] || {
    usage >&2
    exit 2
  }
  URL="$2"
  OUT="$3"
else
  [ $# -eq 2 ] || {
    usage >&2
    exit 2
  }
  case "$1" in
    */*) ;;
    *)
      printf 'fetch-releases: expected owner/repo, got %s\n' "$1" >&2
      exit 2
      ;;
  esac
  URL="https://api.gh.pkgforge.dev/repos/$1/releases?per_page=$PER_PAGE"
  OUT="$2"
fi

case "$URL" in
  https://*) ;;
  *)
    printf 'fetch-releases: a retrieval location is https: %s\n' "$URL" >&2
    exit 2
    ;;
esac

command -v curl >/dev/null 2>&1 || {
  printf 'fetch-releases: curl not found\n' >&2
  exit 2
}

DIR=$(dirname -- "$OUT")
[ -d "$DIR" ] || mkdir -p "$DIR" || {
  printf 'fetch-releases: cannot create %s\n' "$DIR" >&2
  exit 2
}

# ⛔ THE STATUS IS READ FROM curl, UNPIPED, and the body goes to a file rather
# than through a pipeline. `curl ... | tee` would report tee's status, so a 404
# body would be written and reported as a successful fetch.
STATUS=$(curl -sS -L --max-time 60 -o "$OUT" -w '%{http_code}' "$URL")
RC=$?
if [ "$RC" -ne 0 ]; then
  printf 'fetch-releases: curl exited %s for %s\n' "$RC" "$URL" >&2
  rm -f "$OUT"
  exit 1
fi
if [ "$STATUS" != "200" ]; then
  printf 'fetch-releases: %s answered %s\n' "$URL" "$STATUS" >&2
  rm -f "$OUT"
  exit 1
fi
if [ ! -s "$OUT" ]; then
  printf 'fetch-releases: %s answered with an empty body\n' "$URL" >&2
  rm -f "$OUT"
  exit 1
fi

# ⛔ THE RETRIEVAL INSTANT IS WRITTEN BY THE FETCH, not inferred later from the
# file's modification time. An mtime survives a copy, an archive restore and a
# checkout, so a resolution reading one would publish a retrieval time that is
# not one. The sidecar is what the resolver reads.
printf '%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" >"$OUT.fetched-at" || {
  printf 'fetch-releases: cannot record the retrieval instant\n' >&2
  rm -f "$OUT"
  exit 2
}

printf '%s\n' "$URL"
