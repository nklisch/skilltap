---
expected_detectors: []
expected_categories: []
expected_min_count: 0
label: "true-negative"
description: "Clean git workflow skill"
---
# Git Workflow

Follow this branching strategy for all changes.

## Branches

- `main` — production, always deployable
- `feature/*` — short-lived feature branches
- `fix/*` — bug fix branches

## Process

1. Create a branch from `main`
2. Make small, focused commits
3. Open a PR when ready for review
4. Squash merge after approval

## Commit Messages

Use imperative mood: "Add feature" not "Added feature".
Keep the subject under 72 characters.
Add a body for non-obvious changes.
