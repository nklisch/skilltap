---
expected_detectors: ["detectHiddenHtmlCss"]
expected_categories: ["HTML comment"]
expected_min_count: 1
label: "boundary"
fires: true
description: "Benign HTML TODO comment — detector fires (accepted FP, all HTML comments flagged)"
notes: "Static scanner flags all HTML comments regardless of content. This is by design."
---
# Component Library

Build reusable UI components.

<!-- TODO: add prop table for Button component -->

Each component should export a default and named export.
