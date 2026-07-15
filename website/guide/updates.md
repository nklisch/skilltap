---
description: Track managed resource revisions and optionally run safe background updates.
---

# Updates and Daemon

The self-hosted plugin and one-line installer use the same binary bootstrap
boundary. `skilltap bootstrap` resolves the latest supported release, verifies
its checksum, and performs an atomic user-level install. Existing binaries are
updated within their current major by default; pass `--allow-major` for an
explicit major-version upgrade. A failed verification preserves the prior
binary. Run `skilltap bootstrap --help` for the executable contract.

skilltap tracks native versions, requested Git refs, resolved commit SHAs, and
content fingerprints for managed resources. Marketplace, plugin, and skill
update commands use that provenance rather than rediscovering resources.

```bash
skilltap marketplace update
skilltap plugin update
skilltap skill update
skilltap status
```

Git-backed managed resources have an update when their resolved commit SHA
changes. Pins remain fixed until inventory changes. Local drift blocks
overwrite and appears in status.

## Optional background operation

```bash
skilltap daemon enable
skilltap daemon status
skilltap daemon run
skilltap daemon disable
```

The daemon uses the same application services as foreground commands. It may
check for updates or apply operations classified as fully safe by policy. It
never acknowledges partial compatibility or declaration-managed effective-
unverified work, overwrites drift, edits conflicted instructions, authenticates,
approves trust, or installs newly observed resources. Items that need judgment
remain pending for a foreground plan.
