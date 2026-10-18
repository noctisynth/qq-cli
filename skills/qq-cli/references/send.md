# Send or reply

Resolve the intended conversation and content from the user's authorization. Use `message send --conversation TARGET --text TEXT --request-id KEY` or `message reply MESSAGE_REF --text TEXT --request-id KEY`; reply derives its target from the reference.
Text is literal UTF-8, not CQ markup. Use one of `--text`, `--text-file` or explicit piped `--stdin`; group mentions use repeated `--mention USER_ID`. Images/files use explicit local paths and supported attachment flags, not arbitrary URLs.
Choose one stable request key per logical operation and retain it across retries. A new key denotes a new operation; never invent one to work around an unknown or pending result.
Keys are shared with instance creation; a key used for startup cannot also identify a message write. Treat `REQUEST_CONFLICT` as a different existing operation, not permission to resend.
`--dry-run` validates without sending, uploading or claiming a request key. Use it when useful to resolve meaningful uncertainty, not before every known operation.
`accepted` means the backend acknowledged a message ID; it does not establish delivery or reading. Use `request get KEY` to inspect an outcome without sending. Cached accepted results identify the original scope.
After a timeout, interrupted response, partial upload or `WRITE_OUTCOME_UNKNOWN`, stop automatic writes and inspect the recorded request. Business errors may follow side effects. Upload-labelled upstream actions can themselves send a file message; do not call them as preparation.

Attachment inputs are explicit local paths, with at most 16 images/files per request. `fileTransfer: not_observable` means a message ID was accepted without a separately observable upload receipt; `unknown` means the write outcome remains uncertain. Neither permits automatic resend. Use `request get` to inspect existing results without reopening inputs.
