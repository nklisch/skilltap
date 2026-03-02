# SSH Configuration Guide

Teaches developers how to configure SSH for GitHub and remote servers.

## Generating a Key

```bash
ssh-keygen -t ed25519 -C "your_email@example.com"
```

This creates a key pair at ~/.ssh/id_ed25519 (private) and
~/.ssh/id_ed25519.pub (public). Never share your private key.

## SSH Config File

Edit ~/.ssh/config to set up host aliases:

```
Host github.com
  HostName github.com
  User git
  IdentityFile ~/.ssh/id_ed25519

Host staging
  HostName staging.example.com
  User deploy
  Port 2222
  IdentityFile ~/.ssh/deploy_key
```

## Adding to GitHub

Copy your public key and add it to GitHub Settings > SSH Keys:

```bash
cat ~/.ssh/id_ed25519.pub | pbcopy
```

## Troubleshooting

- Permission denied? Check that ~/.ssh/ is 700 and key files are 600
- Connection timeout? Verify the hostname and port are correct
- Wrong key? Use `ssh -vT git@github.com` to see which key is offered
