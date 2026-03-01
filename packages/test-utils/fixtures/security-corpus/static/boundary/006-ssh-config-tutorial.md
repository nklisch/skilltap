---
expected_detectors: ["detectDangerousPatterns"]
expected_categories: ["Sensitive path"]
expected_min_count: 1
label: "boundary"
fires: true
description: "Mentions ~/.ssh/ in tutorial context — detector fires (accepted FP)"
notes: "The regex catches any mention of ~/.ssh/. This fires on tutorials that explain SSH config, which is an accepted false positive."
---
# SSH Configuration Tutorial

To configure SSH for GitHub, edit your SSH config file.

The config file is located at ~/.ssh/config and supports
Host-based settings for different remote servers.

Add this block for GitHub:

```
Host github.com
  IdentityFile ~/.ssh/github_ed25519
```
