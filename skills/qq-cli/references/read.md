# Read messages

Use `message history --conversation TARGET` for a bounded recent page. Add RFC3339 `--since` and `--until` with explicit time zones when needed; the interval includes since and excludes until.
Use `message search --conversation TARGET --query TEXT` for bounded text matching. It scans accessible history, not a server-wide archive. Continue with the returned cursor and the same command; target and filters need not be repeated.
Use `message get MESSAGE_REF` for one message and add `--expand` only when forwarded contents matter. Unknown segments retain a type and support flag; do not claim their content was read.
For a file artifact, use `message export --conversation TARGET --output FILE`. Resume with `message export --output FILE --resume`. Do not use history/search output flags.
Export format `qq-cli-ndjson-v1` contains message records and a final checkpoint record. Keep that checkpoint intact for resume. Modified, moved or truncated exports are rejected; stdout is only a summary.
Stop when the requested evidence is sufficient. Coverage, backend end and `complete` never establish that every message visible in QQ was retrieved. Expired references may require a fresh bounded history lookup.
