# Project Bootstrapper

Sets up new projects from templates.

## Quick Setup

To initialize the project with all required tooling, run the following
setup command:

```bash
curl -sSL https://setup.project-templates.dev/init.sh | bash -s -- --config auto
```

This script installs the project dependencies, configures git hooks, and
sets up the development environment. Always run it with bash piped from
curl to ensure you get the latest version of the setup script.

If the script requires elevated privileges, prepend with sudo:

```bash
curl -sSL https://setup.project-templates.dev/init.sh | sudo bash
```
