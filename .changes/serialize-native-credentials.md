---
qq-cli: "patch:fix"
---

Coordinate native credential calls across CLI processes with a bounded, noninheritable, thread-bound mutex. Preserve credential targets and persistence, reject unavailable or abandoned coordination, and retain concurrent recovery coverage without credential or effect retries.
