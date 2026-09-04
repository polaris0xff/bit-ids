# Catalogue

[`clients.toml`](clients.toml) is the machine-readable inventory of required
products, engines, supported host families, upstream locations, and candidate
acquisition routes. A route is only a research lead until live acquisition
records prove that it resolves to the same stable build as another route.

The catalogue deliberately does not pin a current version. Stable-version
resolution is time-dependent and belongs in a signed capture run, not in a
hand-maintained claim. [`../TODO/acquisition.md`](../TODO/acquisition.md)
defines that work.

Every listed target must eventually have:

1. one exact stable version acquired by at least two independent routes;
2. version and artifact identity verified after installation;
3. active observation through the first-party Rust observer and an
   independent connector;
4. raw, replayable evidence and a provenance record; and
5. no published conflict on overlapping observed fields.
