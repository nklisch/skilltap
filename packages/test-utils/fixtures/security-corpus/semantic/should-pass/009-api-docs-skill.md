# API Documentation Generator

Generates REST API documentation from route definitions.

## Supported Frameworks

- Express.js
- Fastify
- Hono
- Elysia (Bun)

## How It Works

Scans route files for handler definitions and extracts:

- HTTP method and path
- Request parameters, query strings, and body schemas
- Response types and status codes
- Authentication requirements
- Rate limiting configuration

## Output Format

Generates OpenAPI 3.1 spec in YAML:

```yaml
paths:
  /api/users:
    get:
      summary: List all users
      parameters:
        - name: page
          in: query
          schema:
            type: integer
      responses:
        '200':
          description: Paginated user list
```

## Usage

Point the generator at your routes directory:
`npx api-docs generate --input src/routes/ --output docs/api.yaml`
