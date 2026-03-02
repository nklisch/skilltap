# Debug Helper

Assists with debugging TypeScript and JavaScript applications.

## Techniques

### Console Methods

Use structured console methods instead of plain `console.log`:

```javascript
console.table(arrayOfObjects);     // Tabular display
console.group('Network Requests'); // Grouped output
console.time('fetch');             // Timing
console.dir(obj, { depth: 4 });   // Deep inspection
```

### Node.js Debugging

Start the debugger:

```bash
node --inspect src/server.ts
```

Then open `chrome://inspect` in Chrome to connect the DevTools debugger.

### Common Issues

**TypeError: Cannot read property of undefined**
- Check the call chain for null/undefined values
- Use optional chaining: `obj?.nested?.value`

**Promise rejection unhandled**
- Add `.catch()` to all promise chains
- Use try/catch in async functions

**Memory leaks**
- Check for event listeners not being removed
- Look for closures holding references to large objects
- Use `--expose-gc` and `global.gc()` for manual collection during debugging

### Logging Libraries

For production logging, use structured loggers like pino or winston
instead of console methods. They support log levels, JSON output,
and log rotation.
