# Schema fixtures

⛔ **Nothing here is a measurement.** These records describe a target called
`fixture-client` that does not exist, at a version called `0.0.0-fixture` that
was never released. Every digest is the SHA-256 of a short descriptive string
rather than of an artifact, and every byte string was written by hand.

They exist to prove that the schema in [`../../src/`](../../src/) can express a
complete record and can refuse an incomplete one. They are not evidence, they
never reach the catalogue, and no value in them may be copied into a published
profile. [`../../../../docs/capture-methodology.md`](../../../../docs/capture-methodology.md)
says what may.

The synthetic target identifier is the guard against exactly that mistake. A
fixture named after a real client is one search away from being read as a
result about that client.

| file | what it is |
| --- | --- |
| `valid-profile.json` | a complete original record exercising all six field states |
| `valid-correction.json` | the same build, a second capture run, superseding the first |
| `unsupported-schema.json` | the golden record with a schema identifier from another generation |
| `unproven-field.json` | the golden record with the peer ID field stripped of its evidence |

The remaining refusals are planted into a copy of `valid-profile.json` by
[`../profile_schema.rs`](../profile_schema.rs), one per diagnostic code, so a
new invariant cannot be added without a defect that proves it fires.

## Regenerating

⚠ **The golden record is byte-exact.** `profile_schema_writes_the_canonical_form_it_read`
compares `Profile::to_json` against the file, so a hand edit that is valid but
not canonical fails the suite. Rewrite it through the library rather than by
hand:

```sh
cargo run --quiet --example validate-profile -- crates/bit-ids/tests/fixtures/valid-profile.json
```

That command validates without writing. To change a record, edit it, run the
command to see what is refused, and let `cargo test --workspace profile_schema`
report any deviation from the canonical form.
