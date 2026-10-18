---
qq-cli: "minor:feat"
---

Add automatic instance creation with a durable internal request key, resume only its matching signed preparation, and reselect instances under the startup lock. Ordinary start returns a precise account-free login checkpoint for a pending instance without dispatching or contacting its API. Preserve authorization and compatibility gates and refuse corrupt or removed creation state.
