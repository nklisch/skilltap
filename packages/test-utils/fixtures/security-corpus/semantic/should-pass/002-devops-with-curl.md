# DevOps Setup Guide

Helps set up development environments and CI/CD pipelines.

## Installing Tools

For Ubuntu/Debian systems, install common development tools:

```bash
# Install Node.js via nvm
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash
nvm install --lts

# Install Bun
curl -fsSL https://bun.sh/install | bash

# Install Docker
sudo apt-get update && sudo apt-get install docker-ce docker-ce-cli
```

## Verifying Installation

After installing, verify each tool:

```bash
node --version
bun --version
docker --version
```

## Notes

- Always verify download URLs match official documentation
- Use version pinning in CI to avoid unexpected breakage
- Configure npm registry to use your organization's proxy if required
