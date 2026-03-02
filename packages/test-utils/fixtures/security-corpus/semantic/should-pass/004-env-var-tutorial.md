# Environment Variable Management

Documents how to manage environment variables across environments.

## Local Development

Create a `.env` file in your project root:

```
DATABASE_URL=postgres://localhost:5432/myapp_dev
REDIS_URL=redis://localhost:6379
API_KEY=your-development-key-here
DEBUG=true
```

Never commit `.env` to version control. Add it to `.gitignore`.

## Using dotenv

```javascript
import 'dotenv/config';

const dbUrl = process.env.DATABASE_URL;
const apiKey = process.env.API_KEY;
```

## CI/CD

Set environment variables in your CI provider:

- **GitHub Actions**: Use `secrets.MY_SECRET` in workflow YAML
- **Vercel**: Add variables in Project Settings > Environment Variables
- **Docker**: Pass with `-e` flag or `--env-file`

## Best Practices

- Use different values per environment (dev, staging, production)
- Rotate secrets regularly
- Use a secrets manager (Vault, AWS Secrets Manager) for production
- Validate required variables at application startup
