# Catalogue

[`clients.toml`](clients.toml) is the machine-readable inventory of required
products, engines, supported host families, upstream locations, and candidate
acquisition routes. A route is only a research lead until live acquisition
records prove that it resolves to the same stable build as another route.

[`licences.toml`](licences.toml) is the register `FOUND-04` owns: one row per
catalogue target and one per third-party package, each recording what was
measured about its licence and who answered. ⛔ **Every row refuses
redistribution and that is the policy rather than a consequence of the
licences**, because this project publishes measurements and never artifacts.

⚠ `unverified` is a disposition and not a gap. Six of the nine targets with a
GitHub upstream answer `NOASSERTION` when their licence endpoint is asked, so
naming one anyway would be inventing it. `check-licences` refuses a row with no
disposition, a row the catalogue or the lockfile does not have, and any
installer-shaped file in the tree.

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
