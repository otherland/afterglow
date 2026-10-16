# Afterglow

**Your Copilot credits, at a glance.** A small, unofficial menu-bar app built
with Rust and Tauri 2. No web UI, backend, telemetry, OAuth implementation, or
token storage. Uses your existing GitHub CLI authentication.

macOS is the first supported/tested platform. The core and tray code also target
Windows, but Windows releases are not yet verified.

## Run on macOS

The [preview release](https://github.com/otherland/afterglow/releases) includes
an **unsigned Apple Silicon DMG** (about 3 MB). Intel Mac users can build from
source. The installed Apple Silicon app is about 9 MB. This is an early testing
build, not a signed/notarized public distribution.

Install [GitHub CLI](https://cli.github.com/) and authenticate for your account's
host:

```sh
gh auth login --hostname procode.ghe.com
gh auth status --hostname procode.ghe.com
```

For development, install Node.js, Rust, and Xcode Command Line Tools, then:

```sh
npm ci
npm run dev
```

The ring and remaining percentage appear in the menu bar, not the Dock.
Click to see remaining credits, allowance, used credits, percentage, reset date,
and the API's reporting timestamp. Refresh is automatic every ten minutes.
Below 20% is marked Low; below 10% is Critical. No desktop notifications.

## Settings

Choose **Open settings file…** from the tray menu. The only setting is:

```json
{
  "hostname": "procode.ghe.com"
}
```

Save and choose **Refresh now**. Authentication must already exist for that host;
Afterglow never changes your active GitHub account. On macOS the file is
`~/Library/Application Support/com.otherland.afterglow/config.json`.
Settings are local and never committed or uploaded.

Finder-launched apps check `/opt/homebrew/bin/gh` and `/usr/local/bin/gh` before
trying `gh` on PATH. Other installations need `gh` on the GUI process's PATH.

## Build and checks

```sh
npm test
npm run check
npm run build
```

macOS artifacts are in `src-tauri/target/release/bundle/`. Local builds are
**unsigned and not notarized**. Do not disable Gatekeeper system-wide; run your
own source build for initial testing. Signed distribution is future work.
On Windows, install Tauri's prerequisites (Microsoft C++ Build Tools and
WebView2) and use `npm run build -- --bundles nsis`; this path is not yet tested.
No administrator rights are needed by the running app; prerequisites may require
them during installation.

## How it works and limitations

Every refresh executes `gh` directly, without a shell:

```sh
gh api --hostname procode.ghe.com /copilot_internal/user \
  --jq '{plan: .copilot_plan, resets: .quota_reset_date, credits: .quota_snapshots.premium_interactions}'
```

The app displays `quota_remaining`, `entitlement`, `credits_used`,
`percent_remaining`, and `quota_reset_date` from the response, without
recalculating them or substituting zero for missing fields. Fractional credits
are preserved. Monthly balance is not CLI session usage.

**This is an internal GitHub endpoint, not a supported public quota API.**
Availability, authentication requirements, and response shape may change.
Compatibility with arbitrary GitHub accounts is not promised. Unlimited quotas
are explicitly reported as unavailable rather than shown as zero.
The API may report delayed data; its timestamp is displayed separately from
the app's check time. On failure, balances are cleared, not left looking fresh.

Requests time out after 30 seconds. Errors appear in the menu; run
`gh auth status --hostname YOUR_HOST` in Terminal to diagnose authentication.
Afterglow does not display or log raw subprocess error output or tokens.
Only credit fields selected by `gh --jq` enter the app.

## Audit surface

Application logic lives in three Rust files under `src-tauri/src/`.
No frontend JavaScript is shipped and no webview windows are opened.
Dependencies still form part of the audit surface; a small source file count is
not a security guarantee. Both npm and Cargo lockfiles are committed.
Builds are not currently claimed to be reproducible.

GitHub Actions runs checks and unsigned packaging on macOS and Windows. A
successful build does not substitute for testing the actual tray and credentials
on each operating system.

Not affiliated with or endorsed by GitHub.

## License

MIT
