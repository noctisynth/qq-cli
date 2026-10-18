---
qq-cli: "patch:fix"
---
Reject stale successor-preparation retries after the successor was explicitly stopped, instead of starting another run through restart or update recovery.
