# Install and maintain QQ CLI and NapCat

## Windows first setup

Use Windows x64 with official QQ NT installed normally. QQ discovery uses its installation registration; if `doctor` cannot find it, ask the user to install/repair official QQ. Do not guess executable paths or edit private registration files.

If `qq-cli --version` is unavailable, first check `$env:LOCALAPPDATA\Programs\qq-cli\qq-cli.exe` (the agent may have an older PATH). Otherwise run in PowerShell:

```powershell
& ([scriptblock]::Create((Invoke-WebRequest -UseBasicParsing https://raw.githubusercontent.com/noctisynth/qq-cli/main/scripts/install.ps1).Content))
```

The installer downloads the latest stable release, verifies SHA256, installs the executable and licenses, and adds its directory to user/current-process PATH. It needs no administrator, Git, Rust or token. Existing agent processes can call the returned absolute `executable` path directly. Re-run the installer to update QQ CLI; `napcat update` updates only NapCat. If download fails, report the network/release error instead of treating the CLI as installed.

The CLI embeds all task instructions. `skill export --output DIR` writes `SKILL.md` directly inside DIR, together with `references/`; use the host's skill directory ending in `qq-cli` if persistent skill installation was requested. Create the parent directory first, but leave DIR itself absent. An existing destination is preserved: export to a fresh directory and compare before updating it. Do not overwrite unrelated skills. No host-specific setup is required to run `skill get TASK` directly.

Default setup needs no config file, profile, predeclared account, endpoint URL or API token. The CLI manages its state and backend connection.

Within the user's setup authorization:

1. Run `napcat instances` to reuse an intended instance or select one with `--instance ID`. Do not create another account session just because PATH or an API call failed.
2. For a new installation, use `napcat install --version v4.18.38`, then `doctor --version v4.18.38`. The catalog also includes v4.18.37 and the macOS development baseline v4.18.41. Installation is explicit; start/update do not download.
3. For a new instance use `napcat start --version v4.18.38 --new --request-id KEY`. Generate one operation key and retain it. For an existing intended instance use `--instance ID napcat start`.
4. A combination without prior startup evidence requires an explicitly authorized compatibility trial: add `--verify-compatibility` to the new-instance command. Explain the diagnostic and obtain that authorization if absent. Known unsupported/conflicting/modified inputs still fail. Do not substitute Shell or LiteLoader.
5. On `AUTH_REQUIRED`, ask the user to complete login in the newly opened official QQ window, then run `status` for that instance. Identify the actual account from the response, not an assumed account number. Stop setup once online identity is confirmed; do not send a test message without authorization.

Do not take over an already running ordinary QQ process. Preserve other QQ windows. Instances share official QQ data; consider saved auto-login before creating another instance so it does not displace an existing same-account session.

## Platform scope

For Apple Silicon macOS development, use the [macOS runtime guide](runtime-macos.md). The setup and maintenance instructions here target Windows.

## Startup and recovery

`napcat installations` lists artifacts; `napcat instances` lists instances. Multiple instances may share an installation. One candidate is selected automatically; multiple candidates require `--instance ID`.

`preparing`/`prepared` requires following the returned next action with bounded `napcat start` on the same instance. Starting a stopped instance retains its ID/version with a fresh run and unconfirmed account. Never replace a pending operation with a new `--new` key.

`request get KEY` inspects creation or writes offline. `dispatched`/unknown startup outcomes require `status`/`doctor`; do not launch again. Keep startup keys separate from message keys. `loginObservation` is progress information; wait for verified identity before business operations. Ask the user for login/second-factor challenges, never derive identity from QR data or logs.

Use `doctor` for a concrete environment, compatibility or connection failure. Preserve failed installation/runtime materials; do not delete unexpected files, replace signing keys, extract endpoint credentials or call raw APIs to bypass a failure. Local compatibility records apply to the exact inputs checked by the CLI.

## Maintenance

`napcat stop`, `restart`, `update` and `rollback` affect the shared QQ session and require the user's maintenance authorization. Do not use them as automatic recovery for a read/send failure or terminate another process to resolve conflicts.

- `napcat restart` prechecks the selected run, then stops it and prepares its successor under the same instance ID/version. A pending preparation resumes. After uncertain dispatch inspect status, not another restart. A completed successor needs new authorization before another restart.
- `napcat install --version VERSION`, then `napcat update --version VERSION --timeout 120` changes the selected instance's NapCat version. A missing installation leaves the original run intact and returns install guidance. Same-target maintenance is reused. `--verify-compatibility` is available for an explicitly authorized new target combination trial.
- `napcat rollback --timeout 120` uses the recorded previous installation. `ROLLBACK_UNAVAILABLE` means no recorded rollback target; do not guess one by version ordering. Resume an interrupted rollback's original plan, not a different version. Verify the actual account after any new run.
- `VERSION_MAINTENANCE_PENDING` means follow the original update/rollback next action, not start the old source. Preserve missing/conflicting recovery materials and report the diagnostic.
- `napcat stop` forcefully closes the selected QQ process group and window. Ask the user to save work as appropriate; it is not a logout or draft-save operation. `STOP_OUTCOME_UNKNOWN` requires diagnosis of the same instance, not another PID. A successful repeat may return `reused: true`.
- `napcat remove INSTANCE_ID` unregisters an already stopped instance while retaining request/runtime records and QQ data. Pass the intended ID once; a conflicting global `--instance` is invalid. Missing IDs fail, while repeat completion is reused.
- `napcat uninstall INSTALLATION_ID` removes an unreferenced installation. Active instance, rollback and pending-transition references must first be resolved through their normal lifecycle. Stop does not uninstall, and no maintenance command deletes QQ private data.
