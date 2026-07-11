---
id: epic-rust-control-plane-storage-maintainability-codecs
kind: story
stage: implementing
tags: [refactor, testing]
parent: epic-rust-control-plane-storage-maintainability
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Separate Document Codecs from Repository IO

Extract private TOML/JSON codecs, schema probes, and duplicate-key validation
from repository filesystem orchestration. Add focused codec tests while
retaining repository integration coverage and exact bytes, classification,
ordering, public identities, and test list. Run the full locked ladder.
