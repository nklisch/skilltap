---
expected_detectors: []
expected_categories: []
expected_min_count: 0
label: "true-negative"
description: "Clean REST API design skill with URLs"
---
# REST API Design

Guidelines for designing RESTful APIs.

## Endpoints

Follow these conventions:

- `GET /api/users` — list users
- `POST /api/users` — create user
- `GET /api/users/:id` — get single user
- `PATCH /api/users/:id` — update user
- `DELETE /api/users/:id` — delete user

## Response Format

Always return JSON with consistent structure:

```json
{
  "data": { "id": 1, "name": "Alice" },
  "meta": { "requestId": "abc-123" }
}
```

## Error Handling

Return appropriate HTTP status codes. Include a human-readable message
and a machine-readable error code in the response body.
