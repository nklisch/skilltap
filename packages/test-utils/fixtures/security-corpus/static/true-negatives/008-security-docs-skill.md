---
expected_detectors: []
expected_categories: []
expected_min_count: 0
label: "true-negative"
description: "Skill about security best practices (meta — discusses attacks without being one)"
---
# Security Review Checklist

Use this checklist when reviewing code for security issues.

## Input Validation

- Validate all user input at the API boundary
- Sanitize HTML output to prevent XSS
- Use parameterized queries to prevent SQL injection
- Validate file upload types and sizes

## Authentication

- Store passwords with bcrypt (cost factor 12+)
- Use short-lived JWTs for API authentication
- Implement rate limiting on login endpoints
- Support multi-factor authentication

## Secrets Management

- Never commit secrets to version control
- Use environment variables or a secrets manager
- Rotate keys regularly
- Audit access to production credentials

## Common Vulnerabilities

Review OWASP Top 10 before each release. Pay special attention
to injection, broken authentication, and sensitive data exposure.
