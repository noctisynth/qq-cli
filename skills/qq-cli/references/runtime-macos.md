# Apple Silicon QQ CLI setup and maintenance

Use native arm64 macOS with official QQ 7.0.2-53644 and NapCat Framework v4.18.41. This is a source-built development platform; real session, Keychain upgrade and distribution acceptance remains incomplete. Intel/Rosetta is outside scope. Preserve ordinary QQ installations and unrelated windows.

## Get the CLI

Use an existing intended qq-cli binary. If absent, obtain the project source and build it with rustup and Xcode Command Line Tools:

```sh
cargo build --release --locked --target aarch64-apple-darwin
./target/aarch64-apple-darwin/release/qq-cli --version
```

Use that executable path for commands below until installed on PATH. The Windows PowerShell installer does not install macOS builds. No account number, endpoint URL, API token or config file is needed for default setup.

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

Only the fixed macOS Framework release is currently supported for startup. macOS `napcat update`/`rollback` are not yet connected; do not apply Windows version-maintenance instructions or guess rollback targets.

## Failures and permissions

Use `request get KEY` for the original creation/start outcome; retain returned internal startup keys too. `preparing`/`prepared` can resume the same intended run. Starting/unknown outcomes are not permission to dispatch it again or choose another process.

Doctor is read-only. Use it for a concrete failure, not before every business call. Respect system permission prompts and return Keychain failures to the user; do not weaken access controls, replace signing keys, extract credentials or edit private records to bypass errors. Never terminate, inject into or take over an ordinary QQ process.

Reads, contacts, history and sends still require their own task scope. Respect the intended account and other active sessions when a new login could displace one.
