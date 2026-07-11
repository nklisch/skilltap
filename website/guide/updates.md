---
description: Track managed resource revisions and optionally run safe background updates.
---

# Updates and Daemon

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
check for updates or apply operations classified as safe by policy. It never
acknowledges partial compatibility, overwrites drift, edits conflicted
instructions, or installs newly observed resources. Items that need judgment
remain pending for a foreground plan.
