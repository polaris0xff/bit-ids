#!/bin/sh
# check-assemble.sh - can a dispatched capture's artifacts become a record, and
# does the assembler refuse every way they can fall short?
#
# ⛔ WHAT THIS EXISTS FOR IS A SET OF REFUSALS NOBODY HAD SEEN. `CI-09` waited on
# a two-route capture for weeks; `capture-client` run 14 produced one, and
# assembling it on 2026-09-09 refused for reasons no reading had found. None of
# them is visible in an attestation and all of them are visible the moment
# something tries to write the record.
#
# -- ⭐ IT RUNS OVER A SYNTHETIC PAIR AND SAYS SO -----------------------------
#
# The lanes here are written by this harness, not downloaded. A gate case that
# fetched two artifacts from a run would go red the day that run expired, and
# would be measuring GitHub's retention rather than this project's assembler.
# ⚠ SO THIS PROVES THE ASSEMBLER AND NOTHING ABOUT RUN 14. What run 14 answered
# is a driven pass, recorded in `TODO/ci.md` under `CI-09`.
#
# ⚠ NO VALUE HERE IS ANY PRODUCT'S. The peer-ID prefix is `-XX0000-`, which no
# vendor uses, so nothing this harness writes can be mistaken for a measurement.
# `docs/architecture.md` section 5 says why a fixture is never evidence.
#
# ⛔ NOTHING HERE PLANTS IN A TRACKED FILE. Every lane a case mutates is written
# into scratch state, because a gate check that edits a tracked file leaves a
# dirty tree behind when it is interrupted.
#
# Usage:
#   sh scripts/capture/check-assemble.sh
#   sh scripts/capture/check-assemble.sh --json
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
      printf 'check-assemble: unknown argument: %s\n' "$1" >&2
      exit 2
      ;;
  esac
  shift
done

HERE=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
ROOT=$(CDPATH='' cd -- "$HERE/.." && pwd)
ROOT=$(CDPATH='' cd -- "$ROOT/.." && pwd)

# shellcheck disable=SC2034
ME=check-assemble
# shellcheck disable=SC1091
# shellcheck source=scripts/corpus/store-lib.sh
. "$ROOT/scripts/corpus/store-lib.sh"

store_require cargo sha256sum od
ASSEMBLE=$(store_build "$ROOT" assemble-capture bit-ids-probe) || exit 2

WORK=$(store_workdir checkassemble) || exit 2
trap 'rm -rf "$WORK"' EXIT INT TERM

# ⛔ EVERY HEX RUN HERE IS COMPUTED, NEVER TYPED, and that is a rule rather than
# a preference. An info hash and a peer ID are both forty hex digits, which is
# the shape `check-no-secrets --public` refuses because it is also the shape of
# a credential. `docs/security/secrets.md` says to narrow a pattern rather than
# switch a rule off; computing the value narrows nothing and needs no exclusion.
hexof() { # text
  printf '%s' "$1" | od -An -tx1 -v | tr -d ' \n'
}
repeat_byte() { # hex-pair count
  _out=""
  _n=0
  while [ "$_n" -lt "$2" ]; do
    _out="$_out$1"
    _n=$((_n + 1))
  done
  printf '%s' "$_out"
}

PROTOCOL_HEX=$(hexof 'BitTorrent protocol')
INFO_HASH_HEX=$(repeat_byte aa 20)
RESERVED_HEX=0000000000100005

# One announce, in the shape a stock client sends and `tracker_http` parses.
announce_hex() { # peer-id
  hexof "GET /announce?info_hash=$(printf 'X%.0s' 1)&peer_id=$1&port=51413 HTTP/1.1
Host: 127.0.0.1:1
User-Agent: fixture-client/1.2.3

"
}

# One peer handshake, in the shape `peer_wire` parses: a length byte, the
# protocol name, eight reserved bytes, the info hash and the peer ID.
handshake_hex() { # peer-id
  printf '13%s%s%s%s' \
    "$PROTOCOL_HEX" "$RESERVED_HEX" "$INFO_HASH_HEX" "$(hexof "$1")"
}

# One transcript document, in the exact shape `evidence.rs` writes and its
# reader accepts.
#
# ⭐ That reader is the writer's inverse and refuses anything the writer does not
# emit, so a document typed here that it accepts is a document the lab would
# have written. A harness whose fixtures the real reader rejected would prove
# nothing about a real bundle.
transcript() { # path endpoint hex
  {
    printf '{\n'
    printf '  "schema": "bit-ids/transcript/1",\n'
    printf '  "endpoint": "%s",\n' "$2"
    printf '  "segments": [\n'
    printf '    {\n'
    printf '      "connection": 1,\n'
    printf '      "offset_ms": 2075,\n'
    printf '      "direction": "from_target",\n'
    printf '      "bytes": "%s"\n' "$3"
    printf '    }\n'
    printf '  ]\n}\n'
  } >"$1"
}

# Writes one lane's two artifact directories.
#
# ⚠ Every value a lane carries is a parameter, so a case can move exactly one
# and the rest stay fixed. A helper that hardcoded the fields would need a copy
# per case, and the copies are what drift.
write_lane() { # tag route resolver-url commit binary-digest version peer-id [connector-peer-id]
  _cap="$WORK/$1-capture"
  _ins="$WORK/$1-install"
  rm -rf "$_cap" "$_ins"
  mkdir -p "$_cap/capture/bundle/fixture" "$_ins/install-$2" "$_ins/release" || return 1

  {
    printf 'bit-ids/capture-attestation/1\nrun=cap-%s\nkind=client\n' "$1"
    printf 'target=fixture-client\nmeasured_build=%s\nstock_client=true\n' "$6"
    printf 'platform=Linux 0.0.0-fixture x86_64\n'
    printf 'started_at=2026-09-09T15:08:24Z\nfinished_at=2026-09-09T15:09:14Z\n'
    printf 'fixture=sha256:%s\n' "$(printf 'fixture-%s' "$1" | sha256sum | cut -d' ' -f1)"
    printf 'egress=closed\n'
    # ⛔ `E-CAP-01` NEEDS TWO CONNECTORS AT THE VALIDITY GATE, so a lane with no
    # eighth argument is run 14's shape and cannot become a record at all.
    [ -z "${8:-}" ] || printf 'connectors=fixture-oracle\n'
  } >"$_cap/capture/attestation.txt"

  # The second connector's own report, which is the contract `OBS-07` fills.
  if [ -n "${8:-}" ]; then
    mkdir -p "$_cap/capture/bundle/connector" || return 1
    {
      printf 'tracker_http/peer_id=%s\n' "$(hexof "$8")"
      printf 'tracker_http/user_agent=%s\n' "$(hexof 'fixture-client/1.2.3')"
      printf 'peer_wire/peer_id=%s\n' "$(hexof "$8")"
      printf 'peer_wire/reserved=%s\n' "$RESERVED_HEX"
    } >"$_cap/capture/bundle/connector/fixture-oracle.txt"
  fi

  {
    printf 'bit-ids/install-record/1\ntarget=fixture-client\nroute=%s\n' "$2"
    printf 'reported_version=%s\npreexisting_version=\n' "$6"
    printf 'preexisting_binary=\npreexisting_binary_sha256=\n'
    printf 'installed_binary=/usr/local/bin/fixture-client\n'
    printf 'installed_binary_sha256=%s\n' "$5"
    printf 'acquired=yes\nsource_commit=%s\n' "$4"
    printf 'adapter=scripts/capture/adapters/fixture.sh\n'
    printf 'started_at=2026-09-09T15:08:17Z\nfinished_at=2026-09-09T15:08:18Z\n'
  } >"$_cap/install-$2.txt"

  {
    printf 'bit-ids/release-resolution/1\ntarget=fixture-client\n'
    printf 'repository=fixture-owner/fixture-client\n'
    printf 'source_url=%s\n' "$3"
    printf 'selected_version=%s\nselected_tag=v%s\n' "$6" "$6"
    printf 'asset=fixture-client-%s-linux-x86-64\n' "$6"
    printf 'asset_url=https://example.invalid/fixture-client-%s\n' "$6"
  } >"$_ins/release/resolution.txt"

  # ⛔ `E-ACQ-10` NEEDS WHAT THE BUILD PRINTED, AND IT IS IN THE INSTALL
  # ARTIFACT rather than in the capture bundle. That split is the first thing
  # assembling run 14 measured, and the layout here is run 14's.
  printf 'fixture-client %s\ncompiled for the %s route\n' "$6" "$2" \
    >"$_ins/install-$2/version.err"

  printf 'd4:infod4:name7:fixtureee' >"$_cap/capture/bundle/fixture/generated.torrent"
  transcript "$_cap/capture/bundle/tracker-http.transcript.json" \
    tracker-http "$(announce_hex "$7")"
  transcript "$_cap/capture/bundle/peer-wire-dialled.transcript.json" \
    peer-wire-dialled "$(handshake_hex "$7")"
}

# Runs the assembler over two lanes into a fresh store, keeping the streams
# apart. ⛔ The exit code is read from the process that produced it.
assemble() { # tag lane...
  _tag=$1
  shift
  rm -rf "$WORK/store-$_tag"
  mkdir -p "$WORK/store-$_tag" || return 2
  _args=""
  for _lane in "$@"; do
    _args="$_args --lane $WORK/$_lane-capture,$WORK/$_lane-install"
  done
  # shellcheck disable=SC2086
  "$ASSEMBLE" $_args "$WORK/store-$_tag" >"$WORK/$_tag.out" 2>"$WORK/$_tag.err"
  return $?
}

said() { # tag literal
  grep -q -F -e "$2" "$WORK/$1.out" "$WORK/$1.err" 2>/dev/null
}

# A case: run the assembler over some lanes and compare the exit code and one
# literal. ⛔ Both, because several refusals here share an exit code and a case
# asserting the code alone passes when a different refusal fired - which is the
# defect `CI-03`'s Windows harness found in itself.
case_is() { # expected-rc literal name tag lane...
  _want=$1
  _needle=$2
  _name=$3
  shift 3
  assemble "$@"
  _rc=$?
  if [ "$_rc" != "$_want" ]; then
    fail "$_name (exit $_rc, expected $_want)"
    [ "$JSON" = "1" ] || sed 's/^/          /' "$WORK/$1.err" | head -4
    return
  fi
  if [ -n "$_needle" ] && ! said "$1" "$_needle"; then
    fail "$_name (exit $_want, and it did not say '$_needle')"
    [ "$JSON" = "1" ] || sed 's/^/          /' "$WORK/$1.err" | head -4
    return
  fi
  pass "$_name"
}

GH=https://api.example.invalid/repos/fixture-owner/fixture-client/releases
REFS=https://git.example.invalid/fixture-owner/fixture-client/refs
COMMIT=$(printf 'fixture-commit' | sha256sum | cut -c1-40)
DIGEST_A=$(printf 'build-a' | sha256sum | cut -d' ' -f1)
DIGEST_B=$(printf 'build-b' | sha256sum | cut -d' ' -f1)
PEER_A='-XX0000-aaaaaaaaaaaa'
PEER_B='-XX0000-bbbbbbbbbbbb'

# -- ⭐ THE CONTROL, AND IT IS THE HALF THAT MATTERS MOST ---------------------
#
# Every refusal below passes equally over an assembler that refuses everything.
# This is the pair that has to be accepted: two routes, two resolvers, two
# connectors, one version, one identity on the wire.
write_lane good-release release "$GH" "" "$DIGEST_A" 1.2.3 "$PEER_A" "$PEER_A" || exit 2
write_lane good-source source "$REFS" "$COMMIT" "$DIGEST_B" 1.2.3 "$PEER_A" "$PEER_A" || exit 2
case_is 0 "" "two routes with independent resolvers assemble into records" \
  good good-release good-source
if said good build_equivalent; then
  pass "a pair whose observations agree reaches build_equivalent"
else
  fail "a pair whose observations agree reaches build_equivalent"
  [ "$JSON" = "1" ] || sed 's/^/          /' "$WORK/good.out" | head -6
fi
# ⛔ TWO RECORDS, NOT ONE. `E-ACQ-01` needs two routes in each and
# `observed_route` says which install each watched, so a lane per route is two
# documents over one route list. A store with one profile is an assembler that
# wrote the capture it was handed last.
# ⚠ UNDER `profiles/`, not every `.json` in the store. The transcripts this
# copies in are `.json` too, so a count over the whole tree answered six and
# read as a defect in the assembler. Found by running it.
if [ "$(find "$WORK/store-good/profiles" -name '*.json' -type f | grep -c .)" = "2" ]; then
  pass "one record per lane is written, over the same two routes"
else
  fail "one record per lane is written, over the same two routes"
fi
if [ "$(find "$WORK/store-good" -name 'version.err' -type f | grep -c .)" -ge 1 ]; then
  pass "the process output the installed version cites is copied into the store"
else
  fail "the process output the installed version cites is copied into the store"
fi

# -- ⛔ AND THE ONE THAT REFUTES THE WORK ORDER -------------------------------
#
# ⛔ A PEER ID CARRIES A PER-CONNECTION RANDOM TAIL, SO TWO CAPTURES OF ONE
# BUILD NECESSARILY DISAGREE ON IT, and `classify_across` reads a disagreement
# on any field both records measured as `divergent`. `constant` with one sample
# is the only state a single capture can produce for such a field, so
# `build_equivalent` is unreachable for any real client through this path -
# which is what the work order said run 14 had made reachable.
# ⭐ `SCHEMA-04`'s sampling model is where several captures become a `patterned`
# field; it sits above the record, and nothing has run it.
write_lane vary-source source "$REFS" "$COMMIT" "$DIGEST_B" 1.2.3 "$PEER_B" "$PEER_B" || exit 2
case_is 0 "divergent" \
  "two captures whose peer IDs differ are divergent rather than build_equivalent" \
  vary good-release vary-source

# -- The refusals ------------------------------------------------------------

# ⛔ ONE CONNECTOR IS REFUSED AT THE VALIDITY GATE, NOT AT PUBLICATION, and this
# is the case that refutes the standing handoff. `CI-09` recorded that a
# single-connector capture "validates, then `E-PUB-02` refuses to publish it";
# measured on 2026-09-09 by stripping the golden fixture two ways, that is true
# of a record declaring TWO connectors where only one observed each field, and
# false of one declaring one. Every capture this project has run declares one.
write_lane lonely-source source "$REFS" "$COMMIT" "$DIGEST_B" 1.2.3 "$PEER_A" || exit 2
write_lane lonely-release release "$GH" "" "$DIGEST_A" 1.2.3 "$PEER_A" || exit 2
case_is 1 "E-CAP-01" "a capture declaring one connector cannot become a record" \
  lonely lonely-release lonely-source

# ⛔ AND A DECLARED CONNECTOR THAT REPORTS NOTHING ABOUT A FIELD IS REFUSED
# rather than recorded as silent. `E-COR-07` already says silence is not the
# same as not_corroborated, and an assembler that filled the gap in would be
# writing corroboration the run never produced.
write_lane partial-source source "$REFS" "$COMMIT" "$DIGEST_B" 1.2.3 "$PEER_A" "$PEER_A" || exit 2
grep -v '^peer_wire/reserved=' \
  "$WORK/partial-source-capture/capture/bundle/connector/fixture-oracle.txt" \
  >"$WORK/partial.txt"
cp "$WORK/partial.txt" \
  "$WORK/partial-source-capture/capture/bundle/connector/fixture-oracle.txt"
case_is 1 "reports nothing about peer_wire/reserved" \
  "a declared connector silent on a field is refused" \
  partial good-release partial-source

# ⛔ ONE LISTING, TWO LANES. This is run 14's shape: `capture-client.yml` resolves
# once and hands the release lane an asset URL and the source lane a tag, and
# argues for it in a comment. `E-ACQ-07` calls that one route.
write_lane same-source source "$GH" "$COMMIT" "$DIGEST_B" 1.2.3 "$PEER_A" "$PEER_A" || exit 2
case_is 1 "E-ACQ-07" "two lanes that read one listing are refused as one route" \
  same good-release same-source

write_lane no-commit source "$REFS" "" "$DIGEST_B" 1.2.3 "$PEER_A" "$PEER_A" || exit 2
case_is 1 "E-ACQ-06" "a source route with no commit is refused" \
  nocommit good-release no-commit

write_lane short-commit source "$REFS" abbrev1 "$DIGEST_B" 1.2.3 "$PEER_A" "$PEER_A" || exit 2
case_is 1 "" "a source route with an abbreviated commit is refused" \
  short good-release short-commit

# ⛔ TWO VERSIONS ARE TWO BUILDS. Absolute 4 wants two routes resolving ONE
# stable version, and a record over a pair that reported two would be a claim no
# capture made.
write_lane other-version source "$REFS" "$COMMIT" "$DIGEST_B" 1.2.4 "$PEER_A" "$PEER_A" || exit 2
case_is 1 "must resolve one stable version" \
  "two lanes reporting two versions are refused" \
  versions good-release other-version

# ⛔ THE PROCESS OUTPUT LIVES IN THE INSTALL ARTIFACT, and a lane handed only its
# capture bundle cannot produce a record at all. That is could-not-run rather
# than a refusal: nothing was judged.
rm -f "$WORK/good-source-install/install-source/version.err"
case_is 2 "E-ACQ-10" "a lane with no process output for its version cannot run" \
  novers good-release good-source
write_lane good-source source "$REFS" "$COMMIT" "$DIGEST_B" 1.2.3 "$PEER_A" "$PEER_A" || exit 2

# ⛔ ONE LANE IS ONE ROUTE. `E-ACQ-01` refuses a record with one, so the
# assembler refuses before reading anything.
rm -rf "$WORK/store-one"
mkdir -p "$WORK/store-one"
"$ASSEMBLE" --lane "$WORK/good-release-capture,$WORK/good-release-install" \
  "$WORK/store-one" >"$WORK/one.out" 2>"$WORK/one.err"
_rc=$?
if [ "$_rc" = 2 ] && said one "E-ACQ-01"; then
  pass "a single lane is refused before anything is read"
else
  fail "a single lane is refused before anything is read (exit $_rc)"
fi

# ⛔ A TRANSCRIPT THIS PROJECT DID NOT WRITE IS NOT EVIDENCE. The reader is the
# writer's inverse, so a document with uppercase hex - which digests differently
# and would have been written as a different artifact - is refused rather than
# decoded.
sed 's/"bytes": "13/"bytes": "13A/' \
  "$WORK/good-source-capture/capture/bundle/peer-wire-dialled.transcript.json" \
  >"$WORK/bad-transcript.json"
if cmp -s "$WORK/bad-transcript.json" \
  "$WORK/good-source-capture/capture/bundle/peer-wire-dialled.transcript.json"; then
  fail "the transcript plant applied"
else
  pass "the transcript plant applied"
  cp "$WORK/bad-transcript.json" \
    "$WORK/good-source-capture/capture/bundle/peer-wire-dialled.transcript.json"
  case_is 1 "transcript" "a transcript this project's writer would not emit is refused" \
    badtrans good-release good-source
  write_lane good-source source "$REFS" "$COMMIT" "$DIGEST_B" 1.2.3 "$PEER_A" "$PEER_A" || exit 2
fi

store_report check-assemble/1 cases "$JSON"
