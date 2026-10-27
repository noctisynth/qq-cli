---
qq-cli: "patch:refactor"
---

Isolate Windows native capabilities behind platform modules and fix non-Windows compilation while preserving existing Windows behavior. Add native Apple Silicon offline checks and ephemeral TLS fixtures.

Keep test-only creation helpers out of production builds and remove an unused Windows startup method so native warning-as-error checks remain clean.

Give ephemeral TLS test certificates distinct CA/server identities and an explicit authority key identifier so Windows can verify the synthetic certificate chain without changing system trust or production validation.
