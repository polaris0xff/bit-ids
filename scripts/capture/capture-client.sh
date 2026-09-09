#!/bin/sh
# capture-client.sh - a capture of an INSTALLED BUILD, on a host that has
# already been claimed and already had its route off the machine deleted.
#
# ⛔ THE DIFFERENCE FROM capture-run.sh IS THE DRIVER, AND IT IS THE WHOLE
# PRODUCT. `capture-run` drives this project's observer with `curl` and writes
# `kind=fixture`, `measured_build=none`, `stock_client=false`, because nothing
# is installed. Here an ADAPTER stands for one target: it answers what the
# installed build's exact version is, and it starts that build on the torrent
# the observer generated. Those two facts are what turn an attestation about a
# containment into an attestation about a build.
#
# ⛔ IT INSTALLS NOTHING AND BUILDS NOTHING. By the time this runs there is no
# network: the workflow deleted the default route before calling it. Acquisition
# and installation happen in an earlier step, while the network still exists,
# through `scripts/acquisition/install-client.sh`. A capture that discovered a
# missing package here would have to restore egress to fix it, on the host that
# exists to have none.
#
# -- ⛔ WHAT MAKES THIS A MEASUREMENT OF A BUILD -----------------------------
#
# Two things, and both are checked rather than declared:
#
#  1. The adapter answers `version` from the INSTALLED EXECUTABLE. A version
#     read from a filename or a package index is not the build speaking, which
#     is the rule `E-ACQ-10` states for a record and this restates for a run.
#  2. An announce arrived carrying a peer ID THIS OBSERVER DID NOT GENERATE,
#     and the raw transcript on disk carries those exact bytes. Everything else
#     here is satisfied by a bundle of empty artifacts that verify against their
#     own empty digests, or by the observer announcing to itself.
#
# ⚠ The adapter DECLARES what it is, and the attestation copies that rather
# than assuming. A stub adapter yields `stock_client=false`, because a run
# driven by a stand-in is not a run driven by a product, and a field that said
# otherwise would be the one sentence in the record nothing backs.
#
# Usage:
#   sh scripts/capture/capture-client.sh --run-id <id> --out <dir>
#                                        --observer <path> --adapter <path>
#                                        [--seconds <n>] [--peer-port <n>]
#                                        [--route-table <path>]
#
# Exit codes: 0 the capture ran and its evidence verifies, 1 a guard refuses,
# 2 could not run.
#
# ⛔ Read the exit code from this process, unpiped.

set -u

RUN_ID=""
OUT=""
OBSERVER=""
ADAPTER=""
SECONDS_TO_SERVE=25
PEER_PORT=""
ROUTE_TABLE=""

usage() {
  awk 'NR>1 { if (/^#/) { sub(/^# ?/, ""); print } else exit }' "$0"
}

# ⛔ ONE MESSAGE PREFIX, so a refusal is attributable to this script rather than
# to the guard or the adapter it delegated to. Each of those prints its own name
# as well, which is how a log says which of the three refused.
refuse() { # message
  printf 'capture-client: %s\n' "$1" >&2
  exit 1
}

cannot() { # message
  printf 'capture-client: %s\n' "$1" >&2
  exit 2
}

while [ $# -gt 0 ]; do
  case "$1" in
    --run-id)
      [ $# -ge 2 ] || cannot "--run-id takes a value"
      RUN_ID="$2"
      shift
      ;;
    --out)
      [ $# -ge 2 ] || cannot "--out takes a value"
      OUT="$2"
      shift
      ;;
    --observer)
      [ $# -ge 2 ] || cannot "--observer takes a value"
      OBSERVER="$2"
      shift
      ;;
    --adapter)
      [ $# -ge 2 ] || cannot "--adapter takes a value"
      ADAPTER="$2"
      shift
      ;;
    --seconds)
      [ $# -ge 2 ] || cannot "--seconds takes a value"
      SECONDS_TO_SERVE="$2"
      shift
      ;;
    --peer-port)
      [ $# -ge 2 ] || cannot "--peer-port takes a value"
      PEER_PORT="$2"
      shift
      ;;
    --route-table)
      [ $# -ge 2 ] || cannot "--route-table takes a value"
      ROUTE_TABLE="$2"
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      printf 'capture-client: unknown argument: %s\n' "$1" >&2
      exit 2
      ;;
  esac
  shift
done

[ -n "$RUN_ID" ] || cannot "--run-id is required"
[ -n "$OUT" ] || cannot "--out is required"
[ -n "$OBSERVER" ] || cannot "--observer is required"
[ -n "$ADAPTER" ] || cannot "--adapter is required"

# ⚠ The same slug rule the guard applies, restated here rather than delegated,
# because this value is a key in a document before the guard is ever consulted.
case "$RUN_ID" in
  '' | *[!a-z0-9-]*) cannot "run id must be lowercase a-z0-9-: $RUN_ID" ;;
esac

case "$SECONDS_TO_SERVE" in
  '' | *[!0-9]*) cannot "--seconds must be a whole number: $SECONDS_TO_SERVE" ;;
esac

if [ -n "$PEER_PORT" ]; then
  case "$PEER_PORT" in
    '' | *[!0-9]*) cannot "--peer-port must be a whole number: $PEER_PORT" ;;
  esac
fi

HERE=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
ROOT=$(CDPATH='' cd -- "$HERE/.." && pwd)
ROOT=$(CDPATH='' cd -- "$ROOT/.." && pwd)
GUARD="$ROOT/scripts/acquisition/assert-disposable.sh"
[ -f "$GUARD" ] || cannot "$GUARD is not present"

for tool in sha256sum awk od grep timeout; do
  command -v "$tool" >/dev/null 2>&1 || cannot "$tool not found"
done

# ⛔ EVERY ADAPTER CALL IS BOUNDED HERE AS WELL AS IN install-client, and for the
# same measured reason: an adapter shells out to a product this project did not
# write, and a product that waits on a prompt waits forever. ⚠ Under containment
# it is worse than during an install, because the observer's own deadline expires
# meanwhile and the run reports "the build announced nothing" over a build that
# was never asked anything.
#
# ⭐ AND STDIN IS /dev/null on every one, so a prompt gets end-of-file.
ADAPTER_SECONDS=${BIT_IDS_ADAPTER_TIMEOUT:-120}
# ⛔ -k, because `timeout` sends TERM and then waits: a child that blocks or
# ignores it is never killed and the bound reports nothing.
ADAPTER_KILL_AFTER=${BIT_IDS_KILL_AFTER:-20}

# ⛔ THE BUILD RAN BEFORE THE NETWORK WAS CUT, OR IT DID NOT RUN. `could not
# run` rather than a guard refusing: nothing about the host is wrong.
[ -x "$OBSERVER" ] ||
  cannot "the observer $OBSERVER is not executable; it is built before the route is deleted, never here"

[ -f "$ADAPTER" ] ||
  cannot "the adapter $ADAPTER is not present"

# ⛔ A SECOND CAPTURE MUST NOT WRITE OVER THE FIRST ONE'S EVIDENCE.
[ -e "$OUT" ] && refuse "$OUT already exists; a capture never writes into another run's evidence"

# -- the host, as this run finds it -------------------------------------------
#
# ⛔ THE MARKER'S PATH IS ASKED FOR, NEVER COMPOSED. `--marker` is the one
# derivation; a second spelling would go on reading the old location the day the
# state directory moves and report an unclaimed host as claimed.
MARKER=$(sh "$GUARD" --marker) || cannot "the guard could not report its marker path"
[ -n "$MARKER" ] || cannot "the guard reported an empty marker path"

[ -f "$MARKER" ] ||
  refuse "the host was never claimed; $MARKER does not exist"

CLAIMED_RUN=$(awk -F= '$1 == "run" { print $2; exit }' "$MARKER")
[ "$CLAIMED_RUN" = "$RUN_ID" ] ||
  refuse "the host is claimed by run [$CLAIMED_RUN], not [$RUN_ID]"

# ⛔ AND THE ROUTE IS READ AGAIN AT THE MOMENT OF CAPTURE. What this adds over
# the workflow's own step is that nothing between the two put the route back.
if [ -n "$ROUTE_TABLE" ]; then
  sh "$GUARD" --egress "$ROUTE_TABLE"
  EGRESS_RC=$?
  EGRESS_SOURCE="$ROUTE_TABLE"
else
  sh "$GUARD" --egress
  EGRESS_RC=$?
  EGRESS_SOURCE=/proc/net/route
fi
case "$EGRESS_RC" in
  0) : ;;
  1) refuse "egress is open; the default route is still there and a capture would leak" ;;
  *) cannot "egress could not be established from $EGRESS_SOURCE" ;;
esac

FINGERPRINT=$(sh "$GUARD" --fingerprint) || cannot "the host fingerprint could not be read"

mkdir -p "$OUT" || cannot "cannot create $OUT"
OUT=$(CDPATH='' cd -- "$OUT" && pwd)
BUNDLE="$OUT/bundle"
LOG="$OUT/observer.log"
SUMS="$OUT/SHA256SUMS"
ATTEST="$OUT/attestation.txt"
WORKDIR="$OUT/client"
TORRENT="$OUT/run.torrent"
mkdir -p "$WORKDIR" || cannot "cannot create $WORKDIR"

# -- what the adapter says it is ----------------------------------------------
#
# ⛔ ASKED BEFORE ANYTHING IS STARTED, so a run whose adapter cannot describe
# itself never launches a process at all. `describe` prints key=value lines: the
# target it drives and whether it is a stock product or a stand-in.
ADAPTER_DESC="$OUT/adapter.describe"
timeout -k "$ADAPTER_KILL_AFTER" "$ADAPTER_SECONDS" sh "$ADAPTER" describe </dev/null \
  >"$ADAPTER_DESC" 2>"$OUT/adapter.err" ||
  cannot "the adapter could not describe itself"

TARGET=$(awk -F= '$1 == "target" { sub(/^[^=]*=/, ""); print; exit }' "$ADAPTER_DESC")
ADAPTER_KIND=$(awk -F= '$1 == "kind" { sub(/^[^=]*=/, ""); print; exit }' "$ADAPTER_DESC")
[ -n "$TARGET" ] || cannot "the adapter named no target"
case "$ADAPTER_KIND" in
  stock | stub) : ;;
  '') cannot "the adapter declared no kind" ;;
  *) cannot "the adapter declared an unknown kind: $ADAPTER_KIND" ;;
esac

# ⛔ THE VERSION COMES OUT OF THE INSTALLED EXECUTABLE. An adapter that cannot
# ask it is `could not run`: there is no fixture fallback here, because a run
# reporting `measured_build=none` beside `kind=client` would be the record
# claiming a build it never identified.
MEASURED_BUILD=$(timeout -k "$ADAPTER_KILL_AFTER" "$ADAPTER_SECONDS" sh "$ADAPTER" version </dev/null 2>>"$OUT/adapter.err")
VERSION_RC=$?
[ "$VERSION_RC" != 124 ] ||
  cannot "the installed build did not answer --version within ${ADAPTER_SECONDS}s"
[ "$VERSION_RC" = 0 ] ||
  cannot "the adapter could not ask the installed build its version"
[ -n "$MEASURED_BUILD" ] ||
  cannot "the adapter reported an empty version for the installed build"
case "$MEASURED_BUILD" in
  *[!\ -~]*) cannot "the reported version is not printable text: $MEASURED_BUILD" ;;
esac

STARTED=$(date -u +%Y-%m-%dT%H:%M:%SZ)

# -- the run ------------------------------------------------------------------
#
# ⚠ The observer prints its endpoints, writes the torrent, and then serves until
# its own deadline. Waiting on its line rather than on a delay is what makes
# this work on a slow runner.
"$OBSERVER" "$BUNDLE" "$SECONDS_TO_SERVE" "$TORRENT" ${PEER_PORT:+"$PEER_PORT"} >"$LOG" 2>&1 &
OBSERVER_PID=$!

WAITED=0
while [ "$WAITED" -lt 60 ]; do
  grep -q '^serving for ' "$LOG" 2>/dev/null && break
  kill -0 "$OBSERVER_PID" 2>/dev/null || break
  sleep 1
  WAITED=$((WAITED + 1))
done

ANNOUNCE_URL=$(awk '$1 == "announce-url" { print $2; exit }' "$LOG")
if [ -z "$ANNOUNCE_URL" ]; then
  wait "$OBSERVER_PID" 2>/dev/null
  sed 's/^/          /' "$LOG" >&2
  refuse "the observer never reported an announce URL"
fi

# ⛔ THE TORRENT IS A FILE THE CLIENT READS, AND IT MUST EXIST BEFORE THE CLIENT
# IS STARTED. An adapter handed a path to nothing would report its own failure,
# which is a different fact from the observer never having written one.
[ -s "$TORRENT" ] || {
  wait "$OBSERVER_PID" 2>/dev/null
  refuse "the observer wrote no torrent at $TORRENT"
}

# ⭐ THE CLIENT IS STARTED WITH THE TORRENT AND NOTHING ELSE. It learns the
# tracker's address by reading the file, which is what a stock build does with
# any torrent, so nothing here is a control the product does not already have.
timeout -k "$ADAPTER_KILL_AFTER" "$ADAPTER_SECONDS" sh "$ADAPTER" start "$TORRENT" "$WORKDIR" "${PEER_PORT:-0}" \
  </dev/null >"$OUT/client.log" 2>&1
START_RC=$?
if [ "$START_RC" != 0 ]; then
  timeout -k "$ADAPTER_KILL_AFTER" "$ADAPTER_SECONDS" sh "$ADAPTER" stop "$WORKDIR" </dev/null >/dev/null 2>&1
  wait "$OBSERVER_PID" 2>/dev/null
  sed 's/^/          /' "$OUT/client.log" >&2
  if [ "$START_RC" = 124 ]; then
    refuse "the adapter did not return from start within ${ADAPTER_SECONDS}s; it launches and returns"
  fi
  refuse "the adapter could not start the build (exit $START_RC)"
fi

wait "$OBSERVER_PID"
OBSERVER_RC=$?

# ⚠ STOPPED WHATEVER HAPPENED ABOVE. A client left running holds a port and its
# own executable open, and on a host that is about to upload evidence that is a
# process writing into the directory being uploaded.
timeout -k "$ADAPTER_KILL_AFTER" "$ADAPTER_SECONDS" sh "$ADAPTER" stop "$WORKDIR" </dev/null >>"$OUT/client.log" 2>&1
STOP_RC=$?

[ "$OBSERVER_RC" = "0" ] || {
  sed 's/^/          /' "$LOG" >&2
  refuse "the observer exited $OBSERVER_RC"
}

# ⚠ THE SEGMENT GUARD IS EARLIER THAN THE ANNOUNCE GUARD AND MASKS IT ON THE
# EASY INPUT. A build that never started leaves no bytes at all, so `segments`
# is what refuses that run and `announces` is never reached; the announce guard
# only fires where something DID reach the socket without announcing, which is a
# connection that opened and said nothing a tracker could read. Both are real
# and they are separate refusals, which is why the harness provokes the second
# one through a request the codec refuses rather than through silence.
SEGMENTS=$(awk '$1 == "segments:" { print $2; exit }' "$LOG")
[ -n "$SEGMENTS" ] || refuse "the observer reported no segment count"
[ "$SEGMENTS" -gt 0 ] || refuse "the run recorded no bytes at all"

ANNOUNCES=$(awk '$1 == "announces" { print $2; exit }' "$LOG")
[ -n "$ANNOUNCES" ] || refuse "the observer reported no announce count"
[ "$ANNOUNCES" -gt 0 ] || refuse "the build announced nothing; there is no measurement here"

# -- the evidence, checked by something else ----------------------------------
#
# ⭐ THE ROWS ARE THE OBSERVER'S OWN CLAIMS AND sha256sum IS WHAT TESTS THEM.
awk '$1 == "evidence" {
       digest = $5
       sub(/^sha256:/, "", digest)
       printf "%s  bundle/%s\n", digest, $3
     }' "$LOG" >"$SUMS"

EVIDENCE=$(grep -c . "$SUMS")
[ "$EVIDENCE" -gt 0 ] || refuse "the observer described no evidence"

(cd "$OUT" && sha256sum -c SHA256SUMS) >"$OUT/verify.log" 2>&1 ||
  refuse "the evidence does not verify against the digests the observer declared"

# -- and the transcript must carry a peer ID this observer did not generate ----
#
# ⛔ THIS IS THE CASE THAT SAYS A BUILD WAS MEASURED. Everything above is
# satisfied by a bundle of empty artifacts that verify against their own empty
# digests, or by an observer announcing to itself. Two separate refusals,
# because the fixes differ: an announce carrying the observer's own fixture peer
# ID means nothing external ran, and a peer ID the transcript does not hold
# means the observer reported an announce the bytes do not show.
OBSERVED_ONLY=$(awk '$1 == "announce" && $3 == "is-observer-peer-id" && $4 == "false" { found = 1 }
                     END { print (found ? "no" : "yes") }' "$LOG")
[ "$OBSERVED_ONLY" = "no" ] ||
  refuse "every announce carried this observer's own peer ID; no build was measured"

# ⛔ A THIRD REFUSAL, BECAUSE AN ANNOUNCE CAN CARRY NO PEER ID AT ALL. The
# observer prints `absent` or `undecodable: ...` for those, and an earlier
# version of this took `$4` unconditionally and then searched the transcript for
# the literal word `absent` - which is a guard whose verdict depends on whether
# an unrelated English word happens to appear in a JSON document. The hex is
# required to BE hex, and it is taken from the first announce that was not this
# observer's own, because those are the bytes a build chose.
#
# ⚠ The peer-id line precedes the is-observer-peer-id line for the same index,
# which is what makes one pass in printed order enough.
PEER_ID_HEX=$(awk '
  $1 == "announce" && $3 == "peer-id" { pending = $4; index_of = $2 }
  $1 == "announce" && $3 == "is-observer-peer-id" && $2 == index_of && $4 == "false" {
    if (!found && pending ~ /^[0-9a-f]+$/) { print pending; found = 1 }
  }' "$LOG")
[ -n "$PEER_ID_HEX" ] ||
  refuse "no announce carried a readable peer ID; no build was identified"
grep -q "$PEER_ID_HEX" "$BUNDLE/tracker-http.transcript.json" 2>/dev/null ||
  refuse "the transcript does not carry the peer ID the observer reported"

FINISHED=$(date -u +%Y-%m-%dT%H:%M:%SZ)
OBSERVER_DIGEST=$(sha256sum "$OBSERVER" | cut -d' ' -f1)
ADAPTER_DIGEST=$(sha256sum "$ADAPTER" | cut -d' ' -f1)
INFO_HASH=$(awk '$1 == "info-hash" { print $2; exit }' "$LOG")
FIXTURE=$(awk '$1 == "fixture-sha256" { print $2; exit }' "$LOG")
PEER_STREAMS=$(awk '$1 == "peer-streams" { print $2; exit }' "$LOG")
PEER_DIALLED=$(awk '$1 == "peer-dialled" { $1 = ""; sub(/^ /, ""); print; exit }' "$LOG")

# -- the attestation ----------------------------------------------------------
#
# ⚠ Key=value, one per line, the shape assert-disposable's own marker uses and
# `capture-run`'s attestation shares. ⛔ `stock_client` is the adapter's own
# declaration rather than an assumption: a stub adapter drives every guard above
# and is not a product, so a run it drove says so in a field.
{
  printf 'bit-ids/capture-attestation/1\n'
  printf 'run=%s\n' "$RUN_ID"
  printf 'kind=client\n'
  printf 'target=%s\n' "$TARGET"
  printf 'measured_build=%s\n' "$MEASURED_BUILD"
  printf 'stock_client=%s\n' "$([ "$ADAPTER_KIND" = stock ] && printf true || printf false)"
  printf 'adapter=%s\n' "$ADAPTER"
  printf 'adapter_kind=%s\n' "$ADAPTER_KIND"
  printf 'adapter_sha256=%s\n' "$ADAPTER_DIGEST"
  printf 'adapter_stop_status=%s\n' "$STOP_RC"
  printf 'platform=%s\n' "$(uname -srm)"
  printf 'started_at=%s\n' "$STARTED"
  printf 'finished_at=%s\n' "$FINISHED"
  printf 'host_fingerprint=%s\n' "$FINGERPRINT"
  printf 'claim_marker=%s\n' "$MARKER"
  printf 'claimed_run=%s\n' "$CLAIMED_RUN"
  printf 'egress=closed\n'
  printf 'egress_source=%s\n' "$EGRESS_SOURCE"
  printf 'observer=%s\n' "$OBSERVER"
  printf 'observer_sha256=%s\n' "$OBSERVER_DIGEST"
  printf 'announce_url=%s\n' "$ANNOUNCE_URL"
  printf 'info_hash=%s\n' "$INFO_HASH"
  printf 'fixture=%s\n' "$FIXTURE"
  printf 'announces=%s\n' "$ANNOUNCES"
  printf 'measured_peer_id=%s\n' "$PEER_ID_HEX"
  printf 'peer_streams=%s\n' "${PEER_STREAMS:-0}"
  printf 'peer_dialled=%s\n' "${PEER_DIALLED:-none}"
  printf 'segments=%s\n' "$SEGMENTS"
  printf 'evidence=%s\n' "$EVIDENCE"
  printf 'evidence_verified_by=sha256sum\n'
} >"$ATTEST"

# ⛔ READ BACK, NOT ASSUMED.
grep -q "^run=$RUN_ID\$" "$ATTEST" ||
  refuse "the attestation was not written"

printf '%s\n' "$FINGERPRINT"
printf 'capture-client: %s %s, %s announce(s), peer id %s\n' \
  "$TARGET" "$MEASURED_BUILD" "$ANNOUNCES" "$PEER_ID_HEX"
printf 'capture-client: %s segment(s), %s artifact(s), verified by sha256sum\n' \
  "$SEGMENTS" "$EVIDENCE"
printf 'capture-client: attestation %s\n' "$ATTEST"
exit 0
