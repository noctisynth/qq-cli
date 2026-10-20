# Shared rules

Use the user's authorized account, conversation and operation. This skill adds no permission to read, send, log in, restart or close applications.
One registered instance is selected automatically; multiple candidates require `--instance ID`. Message/media references and query cursors infer their instance; event checkpoints require the original selected instance and explicit conversation. Never guess from the last active window.
Account identifiers are strings. Read `meta.scope` on JSON responses or `ready.scope` on event streams to identify the account actually used. On `ACCOUNT_CHANGED`, inspect `status` and reassess the target; old references and request keys do not migrate.
Ordinary commands return compact JSON; event streams return NDJSON. For paginated JSON, exit 7 means an interrupted partial result: inspect `pageInfo` and preserve its continuation. A full page can succeed with `complete=false`. For streams, inspect the final summary and gap records; there is no pageInfo.
Do not translate a backend error or exhausted budget into an empty account, missing message or complete archive. Do not switch to raw API calls to bypass a CLI failure.
