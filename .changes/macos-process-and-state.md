---
qq-cli: "patch:feat"
---

Add Apple Silicon process identity, executable discovery, generation-bound stop/suspend/resume, and exact backend socket ownership checks. Recover authenticated services into guarded HTTP and WebSocket transports.

Store credentials in macOS Keychain and durably publish state through pinned directories, revalidating original state and lifecycle locks around startup dispatch.
