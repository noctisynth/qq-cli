# Friend relationships

contact status USER reads native presence codes and requires upstream Packet support. contact likes reads your own profile-like batch. For another user use contact likes USER --expected-uid UID, taking UID from that user's contact profile result under the same account; a mismatching response is refused because the backend can fall back to your own profile. --start OFFSET and --limit N bound the batch. Counts are a snapshot, not proof of all historical likes. Unknown user identifiers are not fabricated.

contact like USER [--times N] --request-id KEY sends an explicit profile-like operation. It supports offline --dry-run and the shared request recovery rules below; do not automatically like every observed user.

Keep contact list/get for friends and use contact profile USER [--no-cache] for an arbitrary user's available profile. Missing fields remain absent; the backend's obsolete login_days value is not presented as a fact. Reads do not approve a relationship.

contact categories reads categories and their members. contact unilateral reads the backend's single one-way friend batch (request size 99, requires Packet support). contact requests --limit N reads a batch of suspicious requests. These outputs say complete:false: they are snapshots or batches, not a complete account export. The configured scan_limit and response budget apply to nested members too. Target policy filters the returned users; filteredByPolicy indicates this. An unresolved suspicious user's ID is omitted under a target restriction.

Write commands require --request-id KEY or offline --dry-run:

- contact remark USER TEXT changes a friend's remark; empty TEXT clears it.
- contact remove USER [--block] [--both] deletes a friend. Extra flags explicitly request blocking or mutual deletion. Backend valid:false or result failure is a rejection.
- contact respond FLAG --approve true|false [--remark TEXT] handles an ordinary request using its exact reported flag. A remark applies the backend's additional remark operation; omit it when rejecting. Never infer permission to accept from the request's message.
- contact accept-doubt FLAG accepts a suspicious request using its exact string flag. The fixed backend ignores approve and provides no rejection operation for this request type. Do not describe a raw approve:false call as a rejection.

Flags are opaque account-scoped strings, not QQ numbers. Choose the right instance. Under a configured target restriction, a flag alone cannot prove its target and approval is refused. A preview does not verify the account or backend availability and does not echo the flag or remark. A receipt means the backend accepted the operation; it does not prove a refreshed friend list. Reuse the same key and request get KEY for recovery, including across api call; never retry an unknown result with a fresh key.

`contact poke USER --request-id KEY` sends one private poke and requires Packet support. `contact input-status USER --event-type CODE --request-id KEY` sends the explicit native event code; do not invent names for codes or schedule repeated typing events. Both support offline `--dry-run` and share request deduplication; no automatic retry on an unknown result. Pokes reject IDs that upstream JavaScript would round.
