# Change your QQ profile or presence

Use `account set --nickname NAME --request-id KEY` for a nickname.
Use `account set --signature TEXT --request-id KEY`, or `--signature=` to clear it.
Nickname and signature can be set together in one operation. `--sex unknown|male|female` requires a nickname because the backend profile action requires one.

Use `account avatar FILE --request-id KEY` with an explicit local image, at most 8 MiB.
The CLI snapshots the bytes for the request; it does not ask the backend to open your original path, download a URL, resize or play the image.

Use `account presence --status CODE --request-id KEY` for QQ presence (for example, 10 online, 40 invisible, 70 do-not-disturb).
Optional `--ext-status` and `--battery-status` default to zero.
For custom wording use `account status-text TEXT --face-id ID --request-id KEY`; choose an icon ID supported by QQ. `--face-type` defaults to one.

`--dry-run` is offline and needs no request key. It reports the intended fields and file size without reflecting profile text or image bytes.
Actual writes use the selected verified account. Keep the request key for recovery with `request get KEY`.
The key is shared with the equivalent explicit API action: reusing the same action and payload does not perform a second update.
On an unknown outcome, inspect the existing request rather than inventing a new key or repeating the change.
An accepted receipt does not prove the QQ window has refreshed; do not repeat an acknowledged update to reconcile a cached reading.

For backend action details, inspect `api describe ACTION`.
`compatibilityOnly:true` marks fixed upstream compatibility stubs, including online-device and device-model routes; an empty or constant response is not proof of those business capabilities.
