---
qq-cli: "patch:refactor"
---

Organize Rust code into explicit subsystems and focused application use cases. Separate startup presentation from lifecycle capabilities, share ready-result rendering, isolate native run journals/creation/service retention, move concrete diagnostic adapters into application composition, and keep media byte decoding below messaging use cases. Preserve CLI behavior and the narrow unsafe boundary.

Scope the runtime-material ignore rule to the project root so lifecycle source modules remain tracked.
