---
qq-cli: "patch:refactor"
---

Isolate Windows native capabilities behind platform modules and fix non-Windows compilation while preserving existing Windows behavior. Add native Apple Silicon offline checks and ephemeral TLS fixtures.

The platform foundation does not itself enable managed macOS QQ startup.
