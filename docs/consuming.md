# Consuming the catalogue

What a reader outside this project needs in order to use a published record, and
how to check that what arrived is what was published. `DOC-01` owns this page.

⛔ **Nothing has been published yet.** Every command below runs against a
publication a reader assembles locally, which is what
[`../scripts/common/check-examples.sh`](../scripts/common/check-examples.sh)
does on every gate. When a first capture exists, the same commands run against a
checkout of the `data` branch with no edit; what will change is where the
directory came from.

---

## 1. What a record is

One record describes one target, at one exact stable version, on one
platform/architecture/package, from one capture run. It is a measurement of what
a build put on the wire, not a copy of what its source code says.

The record shape, its six field states and its canonical spellings are in
[`architecture.md`](architecture.md) section 4, which is the authority. The two
things a consumer has to know before reading one:

⛔ **A field that asserts anything cites bytes.** `unknown` is the only state
that asserts nothing and the only one that may cite no evidence, so a value with
no recoverable bytes behind it does not exist in a published record.

⛔ **A published value never came from a peer-ID table.** The lookup by measured
peer prefix resolves a prefix *this project observed* to the record that observed
it, which is the opposite of a decoder guessing a client name from bytes.

---

## 2. Schema compatibility

Every document carries its own schema identifier and is refused by any build that
reads a different one. A consumer checks the identifier before the content, which
is what turns a later generation into a clear refusal instead of a missing field.

| document | identifier |
| --- | --- |
| a record | `bit-ids/profile/1` |
| a capture run | `bit-ids/manifest/1` |
| the lookups and the latest view | `bit-ids/index/1` |
| a release's file list | `bit-ids/release/1` |
| the access contract | `bit-ids/access/1` |

⚠ **A new generation gets a new identifier and a new type.** This build reads
exactly the documents it was written for and says so by name, so a consumer
pinned to a version never silently receives a shape it does not understand.

---

## 3. Integrity, and the one file nothing proves

⛔ **Verify before parsing.** `MANIFEST.json` carries a digest for every
published file except itself and `SHA256SUMS`; `SHA256SUMS` covers every file
except itself. Between them each published byte is covered once, and exactly one
file is covered by neither.

That file is `SHA256SUMS`. A consumer establishes it from outside the
publication, from a release asset listing or from a digest it recorded on an
earlier fetch, and everything else follows from it.
[`publishing.md`](publishing.md) carries the per-path table and the stability
classes.

The check with a reader this project did not write:

```sh
cd "$PUBLICATION"
sha256sum -c SHA256SUMS --quiet
```

⚠ That verifies every file the checksum list names, `MANIFEST.json` included, and
says nothing about the checksum list itself. The library does the same comparison
plus the one `sha256sum` cannot make, which is whether the manifest describes the
bytes actually present:

```sh
"$BIN/catalogue-lookup" "$PUBLICATION" \
  --expect-checksums "sha256:$(sha256sum "$PUBLICATION/SHA256SUMS" | cut -d' ' -f1)" \
  lookup target fixture-client
```

⛔ The `--expect-checksums` argument is the out-of-band digest. Without it the
run says `checksums trusted` rather than `verified`, because a run that trusted
that file and one that was handed its digest are different results.

---

## 4. Selecting a build

A consumer asking "what is the newest measured build of this target on this
platform" reads the latest view rather than sorting versions itself:

```sh
"$BIN/catalogue-lookup" "$PUBLICATION" latest fixture-client linux x86-64 tar-gz
```

⚠ **Do not sort version strings.** `1.2.3` sorts after `1.2.10` as text, and a
consumer that ordered them itself would confidently answer with a superseded
build. The ordering a latest row rests on is the target's declared version
scheme, which the publication does not carry, so the row is the answer.

⛔ **A build line with no measurement answers nothing** rather than the nearest
one, and the command exits 1:

```sh
if "$BIN/catalogue-lookup" "$PUBLICATION" latest fixture-client windows x86-64 tar-gz; then
  echo "an unmeasured build line answered, which it must not"
  exit 1
fi
```

---

## 5. Looking a measurement up

Six lookups are published: by target, by measured peer prefix, by measured BEP 10
client string, by platform, by version and by capture instant.

```sh
"$BIN/catalogue-lookup" "$PUBLICATION" lookup target fixture-client
```

⛔ **Every row names the record it came from.** A reader who doubts a row opens
the measurement; a derived file that answered a question the records could not
would be a file that invented one.

⚠ A record something corrects leaves every view and keeps its path and its bytes.
A consumer holding an identifier from last month finds what answers now in the
`corrections` list rather than by fetching records one at a time.

---

## 6. Fetching only what you need

A consumer that does not want the whole publication asks what each path is worth
before fetching it:

```sh
"$BIN/catalogue-lookup" "$PUBLICATION" plan
```

Each row is a stability class, which document proves the bytes, the digest, and
the path:

```text
immutable manifest    sha256:...  profiles/v1/<target>/<version>/.../<capture>.json
current   checksums   sha256:...  MANIFEST.json
current   out_of_band sha256:...  SHA256SUMS
```

⚠ That block is a shape rather than a command, and the checker never runs one:
`sh` fences are examples and `text` fences are illustrations. A digest is elided
above because it moves with the publication, which is exactly why an illustration
is not a thing to run. ⭐ **An `immutable` path may be cached forever**: it carries a
measurement, it never changes and it never disappears, because a correction
appends a new path rather than editing this one. A `current` path is refetched
and its digest compared.

⚠ The URL forms are in [`publishing.md`](publishing.md) and none has been
fetched, because nothing has been published.

---

## 7. Using the library from Rust

The `bit-ids` crate is the consumer library and `LIB-01` owns it. It parses, it
verifies, and it opens no socket: a caller supplies bytes, and a caller that
wants to fetch implements `Retrieval` so the fetching stays in the consumer and
the verifying stays in the library.

```rust
use bit_ids::catalogue::{Bundle, Catalogue};
use bit_ids::canonical::Slug;

/// Opens a publication a caller has already read into memory, and answers the
/// newest measured build for one line.
fn newest(bundle: &Bundle) -> Option<String> {
    let catalogue = Catalogue::open(bundle, None).ok()?;
    let slug = |t: &str| Slug::parse(t).ok();
    let profile = catalogue.latest(
        &slug("fixture-client")?,
        &slug("linux")?,
        &slug("x86-64")?,
        &slug("tar-gz")?,
    )?;
    Some(profile.build.version.as_str().to_owned())
}
```

⛔ **There is no faster path that skips validation.** A record reaches a consumer
through `Profile::from_json`, and the generic serde route validates too, so a
caller cannot hold a record this project would refuse to publish.

---

## 8. What is not here

- ⚠ **`formats/bit-ids-v1.sqlite3` is written now**, by `PUB-05`, and it is the
  one rendering a consumer needs no example for: any SQLite reads it, and
  `sqlite_master` says what it carries. ⭐ It is also the only rendering that is
  both queryable and lossless - the tables index the records and a `document`
  table holds each record's canonical bytes, so a value the schema does not
  tabulate is still there and an `omission` table says where.
  ⛔ **The cost lands on you and is stated rather than hidden**: depending on the
  `bit-ids` crate now compiles a vendored SQLite. It is not behind a feature,
  because a build that could omit a published path would assemble a manifest
  with a hole in it; [`../TODO/publishing.md`](../TODO/publishing.md) carries
  that argument and the rejected alternative.
- ⚠ **No fetched URL.** `PUB-04` proves the path set and every digest against a
  publication pushed to a bare repository and fetched back over git. The GitHub
  forms are unexercised until a first real publication exists.
- ⛔ **No measured profile.** Every record in this repository is synthetic and
  says so. `OBS-07` and `CI-03` are what stand between here and a first
  measurement.
