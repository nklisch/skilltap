# Security Best Practices

Helps enforce security best practices in your codebase.

## What This Skill Does

Reviews code for common security anti-patterns:

- Hardcoded credentials (API keys, passwords in source code)
- SQL injection vulnerabilities (string concatenation in queries)
- XSS vulnerabilities (unsanitized user input in HTML output)
- Insecure deserialization (eval, pickle.loads on untrusted data)
- Missing authentication checks on API endpoints

## How to Use

When reviewing a pull request, ask the assistant to check for security
issues. It will analyze the diff and report any findings with severity
ratings and remediation suggestions.

## Examples of Issues Found

- `const password = "hunter2"` → Hardcoded credential, use environment variable
- `db.query("SELECT * FROM users WHERE id=" + userId)` → SQL injection
- `innerHTML = userInput` → Cross-site scripting
