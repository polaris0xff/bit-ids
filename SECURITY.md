# Security

## Reporting

Report a vulnerability through GitHub private vulnerability reporting when it
is enabled for this repository. If that route is unavailable, contact the
repository owner through their public GitHub profile without including exploit
details in a public issue.

Do not report a client fingerprint correction as a vulnerability. Open a
normal data-correction issue and attach only evidence that is safe to publish.

## Threat model

The capture system downloads and runs third-party BitTorrent software. Some
targets are proprietary. Treat every downloaded artifact and every client
process as untrusted.

- Clients run only on disposable GitHub runners or disposable virtual
  machines/containers.
- Capture networks are isolated and use project-owned fixtures. A capture must
  not join an unrelated public swarm or collect another person's address.
- A capture job receives no repository write token. A separate collector
  validates artifacts before any append to the `data` branch.
- The acquisition record keeps URLs, versions, signatures and digests. It does
  not keep installers or redistributable copies of proprietary binaries.
- Raw captures are scanned for addresses, credentials, tracker passkeys and
  machine-specific paths before publication.
- Test torrents contain generated, non-sensitive bytes and never copyrighted
  payloads obtained from elsewhere.

## Secrets

No project secret is required for ordinary capture. GitHub workflows use the
run-scoped token with least privilege. A personal access token, tracker passkey,
private torrent, signing key or vendor credential must never enter the tree,
an artifact, a log, a profile, a commit message or a session record.

See [`docs/security/secrets.md`](docs/security/secrets.md) and
[`docs/security/remote-ops.md`](docs/security/remote-ops.md) for the operating
rules.

## Data corrections

Published records are immutable. A parser error or mislabelled capture is
corrected by adding a replacement record that identifies the superseded
record. Published bytes and branch history are never rewritten.
