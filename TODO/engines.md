# Engine entries

Engine results are labeled harness observations. They must never be silently
substituted for the behavior of an application that embeds the engine.

## ENGINE-01: libtorrent engine matrix

Source: operator scope and libtorrent upstream
Priority: P1 | Effort: L | Status: OPEN

Problem: libtorrent defaults and compile-time options influence many clients,
but package and source builds can differ.

Approach: Build a minimal stock harness, compare official/source and package
routes, record build features, and observe through the Rust lab plus alerts or
a packet oracle.

Prove: same-version route records and two-connector captures exist for Linux
and Windows without claiming that an embedding client shares the profile.

## ENGINE-02: anacrolix/torrent engine matrix

Source: operator scope and anacrolix/torrent upstream
Priority: P1 | Effort: L | Status: OPEN

Problem: A Go module version is not directly an installable client release and
needs a reproducible harness definition.

Approach: Pin a minimal harness source, compare module-proxy and source-checkout
routes at one tag, record Go toolchain facts, and actively capture behavior.

Prove: two independently resolved harness builds at the same module version
produce corroborated profiles and reproducible build manifests.

## ENGINE-03: rqbit engine matrix

Source: operator scope and rqbit upstream
Priority: P1 | Effort: L | Status: OPEN

Problem: rqbit is available as application artifacts and Rust packages, which
may encode different features or revisions.

Approach: Compare upstream release and Cargo/source routes, record enabled
features, and observe the same synthetic torrent through independent connectors.

Prove: a two-route run proves build equivalence and publishes a harness-labeled
profile with raw evidence.
