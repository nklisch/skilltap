---
expected_detectors: []
expected_categories: []
expected_min_count: 0
label: "boundary"
fires: false
description: "Word 'evaluation' in prose — should NOT trigger shell command detector"
notes: "Regex uses \\b(eval)\\b so 'evaluation' should not match."
---
# Model Evaluation Guide

The evaluation of model performance requires a structured approach.
Run the evaluation suite after each training epoch. Compare the
evaluation metrics against the baseline to detect regressions.
