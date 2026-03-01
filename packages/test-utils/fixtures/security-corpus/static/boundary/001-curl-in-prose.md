---
expected_detectors: ["detectDangerousPatterns"]
expected_categories: ["Shell command"]
expected_min_count: 1
label: "boundary"
fires: true
description: "Word 'curl' used as a standalone command name in prose — detector fires (accepted FP)"
notes: "The regex uses word boundary \\b so 'curl' alone triggers. This is an accepted false positive."
---
# HTTP Client Guide

The curl command is the most popular HTTP client. Learn how to use
curl effectively for API testing and debugging.
