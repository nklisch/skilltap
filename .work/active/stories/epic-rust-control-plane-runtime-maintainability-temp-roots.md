---
id: epic-rust-control-plane-runtime-maintainability-temp-roots
kind: story
stage: implementing
tags: [refactor, testing]
parent: epic-rust-control-plane-runtime-maintainability
depends_on: [epic-rust-control-plane-runtime-maintainability-sidecar-tests]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Share Test Temporary Roots

Add a generic unique temporary-root owner to `skilltap-test-support` and consume
it from command, filesystem, and scope tests through a core dev-dependency.
Keep module-specific fixture behavior local, preserve paths/test identities and
assertions, use best-effort cleanup without hiding an active panic, and run the
full locked verification ladder.
