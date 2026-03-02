# Code Review Checklist

Provides a checklist for reviewing JavaScript and TypeScript code.

## Anti-Patterns to Flag

### Dynamic Code Execution

Flag any use of `eval()`, `Function()`, or `new Function()` in
application code. These are almost always security vulnerabilities:

```javascript
// BAD — eval allows arbitrary code execution
eval(userInput);
new Function('return ' + userInput)();

// GOOD — use JSON.parse for data
const data = JSON.parse(userInput);
```

### Subprocess Execution

Review uses of `child_process.exec()` for command injection:

```javascript
// BAD — command injection via user input
exec(`grep ${userQuery} /var/log/app.log`);

// GOOD — use execFile with argument array
execFile('grep', [userQuery, '/var/log/app.log']);
```

### Recommendations

- Use ESLint rules `no-eval` and `no-implied-eval`
- Prefer `execFile` over `exec` for subprocess calls
- Sanitize all user input before using in file paths or commands
