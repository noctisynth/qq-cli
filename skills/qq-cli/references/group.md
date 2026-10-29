# Manage a group

group requests [--source system|legacy-notices|doubt] [--group-id GROUP] [--limit N] reads a request batch without processing it. system passes the limit upstream; legacy-notices reads a fixed ordinary batch of 50 despite the native API's ignored name; doubt reads at most the native requested batch of 10 suspicious join notifications. A group filter does not expand those batches. Results say complete:false and identify the native source; compatibility invite aliases are not duplicated.

Use requestFlag only when present. observedRequestId is an observation: the fixed upstream converts sequence strings to JavaScript numbers and large values can lose precision. requestIdExact:false has no usable flag; obtain the original event flag instead of rounding or guessing. Do not infer approval from a request message.

group respond FLAG --approve true|false [--reason TEXT] [--search-limit N] --request-id KEY explicitly accepts or refuses a join/invitation request on the selected account. It supports offline --dry-run without echoing the flag/reason. Under a configured target restriction the flag cannot prove the group, so approval is refused; an extra ignored group_id parameter would not provide that proof. Use the same recovery rules as other writes. Ordinary numeric request batches and original event flags are distinct evidence.

Read group member GROUP USER for reported member information and role. It defaults to native refresh; --cached allows cached information. The fixed handler also depends on an existing upstream member cache, so a failure does not automatically warm the cache or retry. Roles and timestamps are snapshots.

group detail GROUP reads detailed counts and whole-group mute state; it omits the backend's constant empty remark (use group get for the existing remark path). group extended GROUP projects known owner and extension policy fields, preserving native numeric switch codes. group mention-quota GROUP reads reported group/account @all quotas.

group share GROUP [--full] [--source-id CODE] [--additional TEXT] returns a URL without following it. group honors GROUP [--kind all|talkative|performer|legend|emotion|strong_newbie] and group muted GROUP read bounded snapshots with complete:false. The upstream turns some failures into empty arrays, and strong_newbie is a deprecated constant empty result; an empty list does not establish absence. Returned avatar URLs are data and are not downloaded. The configured group target restriction and scanLimit apply to these queries.

Resolve the intended group and users with discover when IDs are not known. Use the existing explicit instance. Read schema get group.COMMAND for exact flags. Every write needs --request-id KEY or offline --dry-run. Keep a key for the same operation; request get KEY recovers the shared API record. Do not retry unknown outcomes with a new key.

- group rename GROUP NAME changes the public group name; group remark GROUP TEXT changes your own remark.
- group card GROUP USER TEXT and group title GROUP USER TEXT accept empty text to clear. Special titles require upstream Packet support; no duration option exists.
- group admin GROUP USER --enabled true|false changes administrator status.
- group ban GROUP USER --duration SECONDS requires an explicit duration; zero removes a mute. group ban-all GROUP --enabled true|false controls whole-group mute.
- group kick GROUP USER [--reject-add] removes one member; group kick-many GROUP --user-id USER... [--reject-add] submits up to 100 users once. The upstream may omit unresolved users; an accepted receipt does not prove all were removed.
- group leave GROUP exits; group dismiss GROUP explicitly requests dissolution by the owner. Infer neither action from an ordinary read request.
- group avatar GROUP FILE snapshots a local image up to 8 MiB. No arbitrary URL or backend local path is accepted by this task.
- group invite-policy GROUP POLICY accepts disabled, require_approval, no_approval or no_approval_under_100.
- group member-permissions GROUP accepts --upload-album, --temporary-session and --create-group with explicit true or false; supply at least one. Omitted permissions remain unchanged.
- group history-visibility GROUP --visible true|false changes new members' default visibility of recent history.
- group join-policy GROUP --add-type CODE [--question TEXT] [--answer TEXT], group robot-policy GROUP [--member-switch CODE] [--member-examine CODE], and group search-policy GROUP [--no-code-finger-open CODE] [--no-finger-open CODE] expose fixed native numeric options. Do not invent undocumented code meanings. Supply at least one robot/search option.

QQ permissions are enforced by the backend. Cached member roles may lag. A receipt says the backend accepted the operation; role changes, kicks and leaving do not have independent final-effect proof. Do not perform extra sensitive reads just to verify every write. Dry-run validates input only; it does not verify identity, permissions or availability. Payloads, images, questions and answers are not stored in receipts.

`group poke GROUP USER --request-id KEY` explicitly pokes one member in that group and requires Packet support. It uses the group target permission, does not send a private copy and rejects IDs that upstream numeric conversion would round. Use `--dry-run` to preview; acceptance is not proof of visible effect.

`group card-json GROUP` returns the generated group Ark JSON as `cardJson`. It does not send to any recipient. The raw aliases `send_group_ark_share` and `send_ark_share` also only generate content despite their names. Do not report a generated card as a sent message or follow embedded links.

`group sign GROUP --request-id KEY` explicitly signs in once and requires Packet support; it does not create a daily schedule. `group signed GROUP --limit N` reads only the upstream first page and returns `complete:false`. Its `nativeTime` keeps the backend number because units are not established; do not interpret it as seconds or invent a date. Rank can be fractional. IDs that the upstream numeric conversion would round are rejected.

`group notices GROUP --limit N` reads announcement text, opaque image IDs/dimensions and available newcomer/pinned/read metadata. The snapshot has `complete:false`; image IDs are not download URLs. `group notice GROUP --content TEXT [--image FILE] --request-id KEY` publishes explicitly; image-only input is also allowed. Local images are snapshotted at most 8 MiB, never forwarded as local paths. `--pinned`, `--confirm-required true|false` (default true), `--notice-type`, `--show-edit-card` and `--tip-window-type` map to the reviewed upstream settings. Use only settings authorized for the announcement. The upstream uploads an image before publication; failure can leave that resource behind. Do not clean it up or retry with a new key automatically. `group notice-delete GROUP NOTICE_ID --request-id KEY` explicitly requests removal; native acceptance does not independently confirm the deletion.

`group essence-list GROUP --limit N` reads content and exact sequence/random metadata. `nativeMessageId` can be an upstream fabricated cache ID and is not a signed `messageRef`; displayed attachments/replies/forwards grant no download or message authority. Locate the original in bounded history if an actual message reference is needed. To remove an entry whose original is unavailable, use `group essence-remove GROUP --sequence SEQ --random RANDOM --request-id KEY` with the exact returned tuple and intended group. Signed random values use `--random=-9` syntax. Do not guess or substitute another tuple; unknown outcomes are not replayed.
