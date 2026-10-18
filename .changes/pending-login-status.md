---
qq-cli: "minor:feat"
---

Expose bounded login progress in status through the original signed control binding while OneBot is not ready. Keep account identity unverified until OneBot confirms it; reject management failures and concurrent run changes without repairing missing material or retrying authentication. Add a strict status response schema.

Check the backend version before identity queries, and distinguish connection refusal or an absent identity from malformed transport or identity responses so protocol failures cannot become login hints.
