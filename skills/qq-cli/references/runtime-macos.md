# Apple Silicon QQ CLI setup and maintenance

Use native arm64 macOS with official QQ 7.0.2-53644 and NapCat Framework v4.18.41. This is a source-built development platform; real session, Keychain upgrade and distribution acceptance remains incomplete. Intel/Rosetta is outside scope. Preserve ordinary QQ installations and unrelated windows.

## Get the CLI

Use an existing intended qq-cli binary. If absent, obtain the project source and build it with rustup and Xcode Command Line Tools:

```sh
cargo build --release --locked --target aarch64-apple-darwin
./target/aarch64-apple-darwin/release/qq-cli --version
```

Use that executable path for commands below until installed on PATH. The Windows PowerShell installer does not install macOS builds. No account number, endpoint URL, API token or config file is needed for default setup.

If a checked macOS development ZIP and its SHA256SUMS are available, the repository's `scripts/install-macos.sh --archive ZIP --checksums FILE` installs a user command without Python, Git, Rust or administrator rights. It prints the command path and any needed PATH setting. The default online installer fails explicitly until a stable release has macOS assets; do not substitute a Windows archive. Installation does not start QQ.

Repeated installation retains older executables in separate version directories. If an upgraded CLI cannot access Keychain, retain its original credentials and the previous executable printed by the installer. With the user's explicit approval, run `--instance ID napcat start --authorize-keychain` for an intended instance and let the user approve that exact CLI with **Always Allow** in macOS dialogs. The authorization worker reads the fixed existing signing/runtime items, returns no secrets and checks ordinary access again. Its wait is at most 300 seconds within the command's total timeout. This recovery path awaits real cross-signature acceptance; use the previous CLI while resolving access. Never delete the signing key to make an upgrade appear successful. `scripts/install-macos.sh --uninstall` removes only its managed command link and preserves versions, runtime data and credentials.

## First session

Within the user's current setup authorization:

1. Check `napcat instances`; select an intended instance with `--instance ID` when needed. A failed read is not permission to create another account session.
2. Run `napcat install --version v4.18.41`. Install/start are separate; start uses the registered Framework installation.
3. For the first explicitly authorized compatibility trial, run `napcat start --new --verify-compatibility --request-id KEY`. Retain KEY. The CLI obtains the fixed official QQ image and prepares a dedicated locally signed QQ app with isolated data. It preserves the system QQ installation.
4. On `AUTH_REQUIRED`, let the user complete login in that dedicated QQ window. Run `--instance ID status` and verify the actual account. Do not infer identity from a QR code or send a test message without authorization.
5. Once ready, run `--instance ID napcat start` once to verify and save that exact software combination for later startup/restart. Doctor only reads compatibility evidence; preparation or a previous prototype is insufficient.

Start defaults to 900 seconds; `--timeout` accepts 1 through 1800 seconds. A new OS/software combination requires its own explicit trial. Do not substitute Shell, LiteLoader or an unreviewed version when validation fails.

## Existing sessions

`--instance ID napcat start` reuses a live owned run or prepares a successor of a proven stopped instance, preserving the instance ID. New runs use independent credentials and require actual account confirmation. Login may be requested again. Historical runtime files and write receipts remain available.

With explicit maintenance authorization, `--instance ID napcat restart` checks inputs before closing the original QQ window, then prepares its successor. Its macOS preparation budget is 900 seconds. A retry bound to the original run recovers the recorded successor; it does not stop a later run chosen by guessing a PID. After unknown dispatch use `status`/`doctor`, not another new session.

`--instance ID napcat stop` forcefully closes only the recorded owned tree and dedicated QQ window; it does not log out or save drafts. Preserve unrelated windows. `STOP_OUTCOME_UNKNOWN` may leave original members paused: inspect/recover the same instance and retain its records. Successful repeated completion reports `reused: true`. Older runs missing original stop receipts are refused for automatic stop.

`napcat remove ID` unregisters a proven stopped instance while preserving installations, credentials, QQ data, runtime files and requests. Repeated completion is reused. `napcat uninstall INSTALLATION_ID` removes an unreferenced Framework installation; stopped registrations still hold references, so remove them first when intended.

The default is v4.18.41. A separately pinned v4.18.38 can be installed and explicitly trialed with `napcat start --version v4.18.38 --new --verify-compatibility --request-id KEY`; its runtime compatibility remains unverified and cannot inherit v4.18.41 evidence. Existing instances without an explicit version use their original installation.

For authorized version maintenance, first `napcat install --version VERSION`, then `--instance ID napcat update --version VERSION`. A different installation closes the original owned QQ window and creates its successor, preserving the instance ID, previous materials and a rollback reference. The target needs its exact positive compatibility evidence or explicit `--verify-compatibility`; the trial does not itself mark support. An update to the same installation reuses a live run or resumes its preparation/startup.

`--instance ID napcat rollback` uses the authenticated previous installation, which must remain installed and have positive compatibility evidence for the current system. No history returns `ROLLBACK_UNAVAILABLE`. Update and rollback default to 900 seconds; `--timeout` accepts 1 through 1800. Missing, modified or unsupported targets fail before stopping. Current, pending and rollback installation references block uninstall.

If `VERSION_MAINTENANCE_PENDING` is returned, resume the exact recorded update/rollback. Ordinary start/restart will not silently run the old version. Retain the internal startup request key for recovery. A new rollback after completion is a new switch; inspect an unknown outcome instead of blindly issuing it again.

## Failures and permissions

Use `request get KEY` for the original creation/start outcome; retain returned internal startup keys too. `preparing`/`prepared` can resume the same intended run. Starting/unknown outcomes are not permission to dispatch it again or choose another process.

Doctor is read-only. Use it for a concrete failure, not before every business call. Respect system permission prompts and return Keychain failures to the user; do not weaken access controls, replace signing keys, extract credentials or edit private records to bypass errors. Never terminate, inject into or take over an ordinary QQ process.

Reads, contacts, history and sends still require their own task scope. Respect the intended account and other active sessions when a new login could displace one.
