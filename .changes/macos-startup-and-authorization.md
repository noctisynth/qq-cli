---
qq-cli: "patch:fix"
---

Resolve Electron entries relative to the application, restore QQ launcher metadata after initialization, and audit physical adapter paths through Electron original-fs. Resolve private workers from the running kernel image so command-link upgrades cannot redirect them.

Check authenticated pending runs before reporting login status and verify private-state and lifecycle-lock ACLs. Add explicit bounded Keychain system authorization while keeping default credential access noninteractive.
