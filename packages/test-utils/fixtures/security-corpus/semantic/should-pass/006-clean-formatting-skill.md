# Prettier Configuration

Configures and runs Prettier for consistent code formatting.

## Supported Languages

- TypeScript / JavaScript
- CSS / SCSS / Less
- HTML / JSX / TSX
- JSON / YAML / Markdown
- GraphQL

## Default Configuration

```json
{
  "semi": true,
  "singleQuote": true,
  "tabWidth": 2,
  "trailingComma": "all",
  "printWidth": 80,
  "arrowParens": "always",
  "endOfLine": "lf"
}
```

## Usage

Format all files: `npx prettier --write .`
Check without writing: `npx prettier --check .`
Format staged files only: `npx pretty-quick --staged`

## Editor Integration

Install the Prettier extension for your editor. Enable "Format on Save"
for automatic formatting. The extension reads `.prettierrc` from the
project root.
