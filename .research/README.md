# Research Substrate

This directory contains skilltap's source-grounded research record.

The authoritative flow is:

```text
reference → attestation → precis → analysis
```

Start with `.research/CONVENTIONS.md`. Research claims cite source-direct
attestations using `[handle]{N}`. The numbered entry resolves through the
corpus index and `.research/references.md`.

Raw source fetches may be excluded from Git when their license, size, or format
makes committing them inappropriate. Each corpus `INDEX.md` records how to
re-fetch those sources from its attested URL and version. Attestations and
analysis remain tracked.

`.import-holding/` contains retained legacy lenses. They are not authoritative
research and must never be cited as sources.
