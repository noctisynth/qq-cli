# Discover a recipient

Use `contact list --query TEXT` to find friends, `group list --query TEXT` for joined groups, and `conversation list --query TEXT` for recent chats. These scopes differ; recent chats are not a full contact directory.
Use `contact get USER_ID`, `group get GROUP_ID` or `group members GROUP_ID` for details. Conversation targets are `private:USER_ID` and `group:GROUP_ID`.
Member roles are backend snapshots and may lag changes. Do not use them alone as proof of current permissions or a failed role change, or repeat an acknowledged change to reconcile a stale role.
Choose from returned identifiers. If names match multiple recipients, resolve the ambiguity from available context or ask for the intended target; do not silently select the first.
Follow `pageInfo.nextCursor` with `--cursor TOKEN`; original filters are restored, and conflicting arguments fail. Adjust `--limit` only when the task benefits from more results.
For instance ambiguity, use `napcat instances` and choose the requested instance; `status --instance ID` verifies its current identity. Do not infer the account from an instance name.
