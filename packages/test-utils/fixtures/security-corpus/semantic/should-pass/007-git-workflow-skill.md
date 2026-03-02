# Git Workflow Guide

Helps follow consistent git workflow practices.

## Branch Naming

- `feat/description` — new features
- `fix/description` — bug fixes
- `chore/description` — maintenance tasks
- `docs/description` — documentation updates

## Commit Messages

Use conventional commits format:

```
type(scope): description

feat(auth): add OAuth2 login flow
fix(api): handle null response from /users endpoint
chore(deps): update typescript to 5.4
```

## Pull Request Process

1. Create a feature branch from `main`
2. Make commits with clear messages
3. Push and open a pull request
4. Request review from at least one team member
5. Address review feedback
6. Squash merge into main

## Rebasing

Keep branches up to date with main:

```bash
git fetch origin
git rebase origin/main
```

Resolve conflicts file by file. Never force-push to shared branches.
