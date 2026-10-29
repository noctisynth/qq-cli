# Manage a group

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
