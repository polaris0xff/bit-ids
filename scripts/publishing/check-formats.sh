#!/bin/sh
# check-formats.sh - do the published renderings agree with each other, with the
# views they came from, and with the release that describes them?
#
# `PUB-03`'s Prove is that release asset digests match the assembled manifest and
# that the formats reconstruct equivalent records. Both halves are here.
#
# -- ⛔ THE RECORD SET IS THE VIEWS' AND NOT A SECOND FILTER ----------------
#
# A corrected record leaves the lookups under `CORPUS-04`, and a renderer that
# selected records on its own would keep publishing it in the tabular view,
# which is the one a reader is least likely to cross-check. So the store below
# carries a correction and the cases assert the corrected record is not
# published as a record anywhere.
#
# ⚠ NOT "its identifier appears nowhere". A correction names what it corrects,
# so the corrected identifier is legitimately in the published bytes as the
# value of `supersedes`. The identifiers are compared rather than the text
# searched, which is the distinction the Rust suite's first version of this
# assertion got wrong.
#
# -- ⭐ THE INDEPENDENT READER HERE IS sha256sum ----------------------------
#
# The release's checksum file is handed to a reader this project did not write,
# so a run that agreed with itself about what it wrote is still caught.
#
# ⚠ THE CBOR IS NOT DECODED HERE, and that is a stated gap rather than an
# oversight. This project has no CBOR reader, and decoding the file with its own
# encoder would be checking the writer against itself. `cbor2` is what read it,
# in `PUB-03`'s driven pass, the same way `libtorrent` and `torf` read `OBS-08`'s
# torrent: a third-party reader belongs in an entry's evidence rather than in a
# gate that would then need the package index on every run.
#
# Usage:
#   sh scripts/publishing/check-formats.sh
#   sh scripts/publishing/check-formats.sh --json
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
      printf 'check-formats: unknown argument: %s\n' "$1" >&2
      exit 2
      ;;
  esac
  shift
done

# ⛔ Resolved from this script's own location, never from the working directory.
HERE=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
ROOT=$(CDPATH='' cd -- "$HERE/.." && pwd)
ROOT=$(CDPATH='' cd -- "$ROOT/.." && pwd)

# ⚠ ME is set here and read by store-lib.sh, which this sources on the next
# line. shellcheck cannot see across a source it is not told to follow.
# shellcheck disable=SC2034
ME=check-formats
# shellcheck disable=SC1091
# shellcheck source=scripts/corpus/store-lib.sh
. "$ROOT/scripts/corpus/store-lib.sh"

# ⛔ python3 IS REQUIRED RATHER THAN OPTIONAL, and its sqlite3 module is why.
# The queryable rendering has to be opened by a reader this project did not
# write, and Python's is in the standard library - unlike `cbor2`, which needs
# the package index and therefore stayed out of the gate. A host without
# python3 exits 2 here, which the gate reads as a skip and never as a pass;
# ubuntu-24.04 carries one and the Linux lane runs this with --strict.
store_require cargo sha256sum python3
BUILDER=$(store_build "$ROOT" build-store) || exit 2
RENDERER=$(store_build "$ROOT" build-formats) || exit 2
ASSEMBLER=$(store_build "$ROOT" assemble-release) || exit 2

WORK=$(store_workdir checkformats) || exit 2
trap 'rm -rf "$WORK"' EXIT INT TERM

STORE="$WORK/store"
SCHEME="fixture-client:-:3:3"
CSV="formats/bit-ids-v1.csv"
SQLITE="formats/bit-ids-v1.sqlite3"
COMBINED="formats/bit-ids-v1.json"

mkdir -p "$STORE" "$WORK/first" "$WORK/second" || exit 2
if ! "$BUILDER" --version 1.2.3 --version 1.3.0 --correct 1.3.0 "$STORE" >/dev/null 2>&1; then
  printf 'check-formats: cannot build the fixture store\n' >&2
  exit 2
fi

# ⛔ Unpiped, both of them.
"$RENDERER" --scheme "$SCHEME" "$STORE" "$WORK/first" >"$WORK/first.out" 2>&1
FIRST_RC=$?
"$RENDERER" --scheme "$SCHEME" "$STORE" "$WORK/second" >"$WORK/second.out" 2>&1
SECOND_RC=$?

if [ "$FIRST_RC" = "0" ]; then
  pass "clean    the store renders and every file reads back"
else
  fail "clean    the render was refused (exit $FIRST_RC): $(head -3 "$WORK/first.out" | tr '\n' ' ')"
fi

if [ "$SECOND_RC" != "0" ]; then
  fail "determinism  the second render failed (exit $SECOND_RC)"
elif [ "$(tree_digest "$WORK/first")" = "$(tree_digest "$WORK/second")" ]; then
  pass "determinism  two renders of one store are byte-identical"
else
  fail "determinism  two renders of one store differ"
fi

RENDERED=$(tree_files "$WORK/first")
if [ "$RENDERED" = "6" ]; then
  pass "determinism  six files were rendered, so the comparison had something to compare"
else
  fail "determinism  $RENDERED file(s) rendered, expected 6"
fi

# ⭐ THE DATABASE IS COMPARED ON ITS OWN, not only inside the tree digest.
# It is the one rendering whose bytes are an encoder's choice rather than this
# project's, so a page order or a freelist that moved between two builds would
# be lost in a digest over five other files that did not.
if cmp -s "$WORK/first/$SQLITE" "$WORK/second/$SQLITE"; then
  pass "determinism  two builds of the database are byte-identical"
else
  fail "determinism  the database differs between two builds of one store"
fi

# -- the record set is the views' ---------------------------------------------
#
# ⚠ The two identifiers are read out of the store rather than spelled here, so a
# change to how build-store names a capture moves them with it.
ORIGINAL=$(find "$STORE/profiles" -name 'cap-1-3-0.json' | head -1)
FIXED=$(find "$STORE/profiles" -name 'cap-1-3-0-fix.json' | head -1)
id_of() { grep -o '"id": "record:sha256:[0-9a-f]*"' "$1" | head -1 | sed 's/.*"\(record:sha256:[0-9a-f]*\)"/\1/'; }
ORIGINAL_ID=$(id_of "$ORIGINAL")
FIXED_ID=$(id_of "$FIXED")

if [ -z "$ORIGINAL_ID" ] || [ -z "$FIXED_ID" ] || [ "$ORIGINAL_ID" = "$FIXED_ID" ]; then
  fail "records  could not read two distinct identifiers out of the store"
else
  # The first cell of every data row is the record identifier.
  PUBLISHED=$(tail -n +2 "$WORK/first/$CSV" | cut -d, -f1 | tr -d '\r')
  if printf '%s\n' "$PUBLISHED" | grep -q -F -x -e "$ORIGINAL_ID"; then
    fail "records  the table publishes the corrected record"
  else
    pass "records  the table does not publish the corrected record"
  fi
  if printf '%s\n' "$PUBLISHED" | grep -q -F -x -e "$FIXED_ID"; then
    pass "records  the table publishes the correction"
  else
    fail "records  the table does not publish the correction"
  fi
  if grep -q -F -e "\"id\": \"$ORIGINAL_ID\"" "$WORK/first/$COMBINED"; then
    fail "records  the combined document publishes the corrected record"
  else
    pass "records  the combined document does not publish it either"
  fi
  # ⚠ And it is in there, as the value of supersedes. A case asserting the
  # identifier is absent entirely would pass only over a correction that had
  # forgotten to say what it corrects.
  if grep -q -F -e "\"supersedes\": \"$ORIGINAL_ID\"" "$WORK/first/$COMBINED"; then
    pass "records  and the correction still names what it corrects"
  else
    fail "records  the correction does not name what it corrects"
  fi
fi

# -- the tabular view omits what it says it omits ------------------------------
OMITTED=0
for section in acquisition observations corroboration normalizations evidence; do
  if grep -q -F -e "$section" "$WORK/first/$CSV"; then
    OMITTED=$((OMITTED + 1))
  fi
done
if [ "$OMITTED" = "0" ]; then
  pass "tabular  the table carries none of the sections it declares it omits"
else
  fail "tabular  $OMITTED declared-omitted section(s) appear in the table"
fi

# -- the release describes every rendering -------------------------------------
#
# ⭐ THE PROVE'S FIRST HALF, AND sha256sum IS WHAT ANSWERS IT. The assembler
# describes what it was given; the checksum file is then read by a program this
# project did not write, so a manifest that agreed with its own writer is still
# caught.
cp "$ROOT/LICENSE" "$WORK/first/LICENSE" || exit 2
"$ASSEMBLER" "$WORK/first" >"$WORK/assemble.out" 2>&1
ASSEMBLE_RC=$?
if [ "$ASSEMBLE_RC" != "0" ]; then
  fail "release  the rendered tree would not assemble (exit $ASSEMBLE_RC): $(head -3 "$WORK/assemble.out" | tr '\n' ' ')"
else
  pass "release  the rendered tree assembles and the manifest covers it"

  MISSING=0
  for name in bit-ids-v1.cbor bit-ids-v1.columns.json bit-ids-v1.csv \
    bit-ids-v1.json bit-ids-v1.jsonl bit-ids-v1.sqlite3; do
    grep -q -F -e "formats/$name" "$WORK/first/MANIFEST.json" || MISSING=$((MISSING + 1))
  done
  if [ "$MISSING" = "0" ]; then
    pass "release  every rendering has a manifest row"
  else
    fail "release  $MISSING rendering(s) have no manifest row"
  fi

  (cd "$WORK/first" && sha256sum -c SHA256SUMS) >"$WORK/verify.out" 2>&1
  VERIFY_RC=$?
  if [ "$VERIFY_RC" = "0" ]; then
    pass "release  sha256sum -c agrees with every published digest"
  else
    fail "release  sha256sum -c disagrees (exit $VERIFY_RC): $(head -3 "$WORK/verify.out" | tr '\n' ' ')"
  fi

  # ⛔ AND IT HAS BEEN SEEN TO REFUSE. A verifier nobody has watched fail is a
  # verifier nobody knows works, and this one is the case's whole strength.
  printf 'x' >>"$WORK/first/$CSV"
  (cd "$WORK/first" && sha256sum -c SHA256SUMS) >"$WORK/verify2.out" 2>&1
  BROKEN_RC=$?
  if [ "$BROKEN_RC" != "0" ]; then
    pass "release  and refuses a rendering whose bytes moved"
  else
    fail "release  sha256sum -c accepted a file that had changed"
  fi
fi

# -- ⭐ THE QUERYABLE RENDERING, READ BY SOMEBODY ELSE'S SQLITE ----------------
#
# `PUB-05` writes this file with a vendored SQLite 3.50.2 compiled into the
# crate. Python's standard library carries its own, at whatever version this
# host has, so opening the file here is two implementations agreeing rather than
# one agreeing with itself - and the reader is older than the writer on the host
# this was written on, which is the direction that matters for a published file.
#
# ⛔ THE ROUND TRIP IS AGAINST THE JSON RENDERING AND NOT AGAINST THE DATABASE.
# A `document` column compared with the rows derived from it would agree with
# itself; the combined JSON is a separate published file, so a database that
# described a different record set is caught.
#
# ⚠ The script is written to a file and its path is passed. A prose payload does
# not go through a shell - `docs/conventions/shell.md` section 1.
cat >"$WORK/read-db.py" <<'PYTHON'
"""Read a rendered formats tree with a SQLite this project did not write."""

import json
import sqlite3
import sys

tree = sys.argv[1]
out = []


def case(ok, text):
    out.append(("PASS" if ok else "FAIL", text))


con = sqlite3.connect(tree + "/formats/bit-ids-v1.sqlite3")
case(
    con.execute("PRAGMA integrity_check").fetchone()[0] == "ok",
    "sqlite    integrity_check passes under python's sqlite3 %s" % sqlite3.sqlite_version,
)
case(not con.execute("PRAGMA foreign_key_check").fetchall(), "sqlite    every foreign key resolves")

meta = dict(con.execute("SELECT key, value FROM meta"))
combined = json.load(open(tree + "/formats/bit-ids-v1.json", encoding="utf-8"))
case(
    meta.get("records") == str(len(combined)),
    "sqlite    meta names the same record count the combined JSON has",
)
case(con.execute("SELECT count(*) FROM omission").fetchone()[0] > 0,
     "sqlite    the file says what it does not carry, in a table")

rows = dict(con.execute("SELECT record, canonical FROM document"))
by_id = {record["id"]: record for record in combined}
case(set(rows) == set(by_id), "sqlite    it publishes exactly the record set the JSON rendering does")
case(
    all(json.loads(rows[key]) == by_id[key] for key in by_id if key in rows),
    "sqlite    every stored document round-trips to the JSON rendering's record",
)

# ⛔ The columns are compared against the document they were derived from,
# because a schema that dropped a section would still round-trip `document`.
agree = True
counted = 0
for key, canonical in rows.items():
    record = json.loads(canonical)
    row = con.execute(
        "SELECT schema, target, target_kind, version, platform, arch, package,"
        " executable, capture, captured_at, supersedes FROM record WHERE id = ?",
        (key,),
    ).fetchone()
    want = (
        record["schema"],
        record["target"]["id"],
        record["target"]["kind"],
        record["build"]["version"],
        record["build"]["platform"],
        record["build"]["arch"],
        record["build"]["package"],
        record["build"]["executable"],
        record["capture"]["id"],
        record["capture"]["captured_at"],
        record.get("supersedes"),
    )
    agree = agree and row == want
    for table, section in (
        ("acquisition", "acquisition"),
        ("observation", "observations"),
        ("corroboration", "corroboration"),
        ("normalization", "normalizations"),
        ("evidence", "evidence"),
    ):
        held = con.execute(
            "SELECT count(*) FROM %s WHERE record = ?" % table, (key,)
        ).fetchone()[0]
        agree = agree and held == len(record[section])
        counted += held
case(agree, "sqlite    every column and every list agrees with the document it came from")

# ⚠ A count nothing compares is a count worth nothing: the tables the CSV cannot
# hold have to be non-empty, or the case above passes over a database of zeroes.
case(counted > 0, "sqlite    the sections the table cannot hold are tabulated here, and are not empty")

for status, text in out:
    print("%s %s" % (status, text))
sys.exit(0 if all(status == "PASS" for status, _ in out) else 1)
PYTHON

python3 "$WORK/read-db.py" "$WORK/first" >"$WORK/read-db.out" 2>&1
READ_RC=$?
if [ "$READ_RC" = "2" ] || ! [ -s "$WORK/read-db.out" ]; then
  fail "sqlite    the independent reader could not run (exit $READ_RC): $(head -3 "$WORK/read-db.out" | tr '\n' ' ')"
else
  # ⚠ Each of the reader's own cases is reported as a row here rather than as
  # one pass, so a reader that answered six questions and a reader that answered
  # one are not the same green.
  while IFS= read -r line; do
    case "$line" in
      PASS\ *) pass "${line#PASS }" ;;
      *) fail "${line#FAIL }" ;;
    esac
  done <"$WORK/read-db.out"
fi

# ⛔ AND THE READER HAS BEEN SEEN TO REFUSE. A verifier nobody has watched fail
# is a verifier nobody knows works, and this one carries six of the cases above.
# ⚠ The plant is a truncation rather than a byte flipped in the middle: SQLite
# reads a page at a time, so a change inside an unread page is invisible to
# `integrity_check` on a file this small.
cp "$WORK/first/$SQLITE" "$WORK/plant.sqlite3" || exit 2
mkdir -p "$WORK/planted/formats" || exit 2
cp "$WORK/first/formats/bit-ids-v1.json" "$WORK/planted/formats/" || exit 2
dd if="$WORK/plant.sqlite3" of="$WORK/planted/$SQLITE" bs=1 \
  count=$(($(wc -c <"$WORK/plant.sqlite3") - 4096)) >/dev/null 2>&1
python3 "$WORK/read-db.py" "$WORK/planted" >"$WORK/read-db2.out" 2>&1
PLANT_RC=$?
if [ "$PLANT_RC" = "0" ]; then
  fail "sqlite    the independent reader accepted a truncated database"
else
  pass "sqlite    and it refuses a database whose bytes were truncated (exit $PLANT_RC)"
fi

PROBE=$(find "$STORE/profiles" -name '*.json' | LC_ALL=C sort | head -1)
store_probe_guards "$PROBE" "fixture-client" "sha256:"

store_report check-formats/1 cases "$JSON"
