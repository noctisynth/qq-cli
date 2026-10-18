# Shared rules

Use the user's authorized account, conversation and operation. This skill adds no permission to read, send, log in, restart or close applications.
One registered instance is selected automatically; multiple candidates require `--instance ID`. References and cursors infer their instance. Never guess from the last active window.
Account identifiers are strings. Read `meta.scope` to identify the account actually used. On `ACCOUNT_CHANGED`, inspect `status` and reassess the target; old references and request keys do not migrate.
Ordinary commands return compact JSON; event streams return NDJSON. Exit 7 means an interrupted partial result: inspect `pageInfo` and preserve its continuation. A full page can be successful with `complete=false`.
Do not translate a backend error or exhausted budget into an empty account, missing message or complete archive. Do not switch to raw API calls to bypass a CLI failure.
