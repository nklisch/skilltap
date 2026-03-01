---
expected_detectors: []
expected_categories: []
expected_min_count: 0
label: "true-negative"
description: "Clean Docker skill with legitimate container commands"
---
# Docker Guide

Best practices for containerizing applications.

## Dockerfile

- Use multi-stage builds to reduce image size
- Pin base image versions for reproducibility
- Copy dependency files before source code for better caching
- Run as non-root user in production

## Compose

Define services in `docker-compose.yml`. Use named volumes
for persistent data. Set resource limits on all services.

## Common Commands

```
docker build -t myapp .
docker run -p 3000:3000 myapp
docker compose up -d
docker compose logs -f
```

Keep images small. Scan for vulnerabilities before deploying.
