---
expected_detectors: []
expected_categories: []
expected_min_count: 0
label: "true-negative"
description: "Clean database skill with SQL examples"
---
# Database Best Practices

Guidelines for working with PostgreSQL.

## Schema Design

- Use UUIDs for primary keys in distributed systems
- Add `created_at` and `updated_at` timestamps to all tables
- Index columns used in WHERE and JOIN clauses
- Use foreign keys to enforce referential integrity

## Query Patterns

Always use parameterized queries:

```sql
SELECT * FROM users WHERE id = $1;
INSERT INTO orders (user_id, total) VALUES ($1, $2);
```

Never concatenate user input into SQL strings.

## Migrations

Run migrations in a transaction. Test rollback before deploying.
Keep migrations small and reversible.
