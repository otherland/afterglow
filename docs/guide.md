# Afterglow guide

[Back to Afterglow](../README.md)

## Installation help

### macOS

The preview DMG is for Apple Silicon Macs running macOS 11 or later.
Intel Mac users can build from source.

Open the DMG, drag Afterglow into Applications, and launch it. Look in the
menu bar, not the Dock.

The app is unsigned and not notarized. If macOS blocks it and you trust the
source and download, use the per-app **Open Anyway** option in
**System Settings → Privacy & Security**, if offered. Don't disable Gatekeeper
system-wide or bypass your organization's policy.

### Windows

The preview installer is for Windows 10/11, x64. Automated builds, tests, and
packaging pass, but tray behavior and credential access need real-machine testing.

Install [GitHub CLI](https://cli.github.com/), or use PowerShell:

```powershell
winget install --id GitHub.cli --exact
```

Open a new terminal afterward so `gh` is on PATH, then authenticate:

```powershell
gh auth login --hostname YOUR_HOST
```

Run the Afterglow installer and launch from Start. The installer may offer to
install Microsoft's WebView2 runtime if missing. Right-click the ring in the
system tray; it may be under **Show hidden icons**.

The installer is unsigned. SmartScreen may warn or block it. Only proceed if
you trust the download and your organization's policy allows it. Don't disable
system protections; ask your administrator if blocked.

## Settings and removal

Choose **Open settings file…** from the ring's menu:

```json
{
  "hostname": "procode.ghe.com"
}
```

Only the hostname is configurable. Save and choose **Refresh now**.
You must already be authenticated through `gh` for that host. Afterglow never
changes your active GitHub account. A different hostname doesn't guarantee
that its internal quota endpoint works.

On macOS the file is:
`~/Library/Application Support/com.otherland.afterglow/config.json`.

To uninstall, quit from the ring's menu, then remove the app from Applications
on macOS or use **Settings → Apps → Installed apps** on Windows.
Your GitHub CLI login is untouched. On macOS you can also remove the settings
folder above.

## Troubleshooting

**Balance unavailable:** Run `gh auth status --hostname YOUR_HOST` in a terminal.
Check the configured hostname, login, network, and endpoint availability.
Requests time out after 30 seconds. Missing fields and failed requests are
reported as unavailable, never zero or an old balance pretending to be current.
Unlimited quotas don't have a finite balance to display.

**Percentage hasn't changed:** The exact balance can change while the rounded
percentage stays the same. Check **Credits remaining**, the API timestamp, and
the last check time. Upstream data may lag. Automatic refresh is every ten minutes.

**GitHub CLI isn't found on macOS:** Finder-launched apps don't inherit your
Terminal's PATH. Afterglow checks `/opt/homebrew/bin/gh` and `/usr/local/bin/gh`
first. Other installations must be on the GUI process's PATH.

**GitHub CLI isn't found on Windows:** Confirm `gh --version` works in a newly
opened terminal. Restart Afterglow after changing PATH; signing out and back in
may be needed for desktop applications to inherit it.

## Build from source

Install Node.js and Rust, plus Xcode Command Line Tools on macOS or Microsoft
C++ Build Tools and WebView2 on Windows.

```sh
git clone https://github.com/otherland/afterglow.git
cd afterglow
npm ci
npm run dev
```

Run checks and package on macOS:

```sh
npm test
npm run check
npm run build
```

For Windows packaging:

```sh
npm test
npm run check
npm run build -- --bundles nsis
```

Packages land in `src-tauri/target/release/bundle/`. Intel Mac users can build
for their own machine. Source builds are unsigned; Mac builds aren't notarized.
GitHub Actions includes both platforms, but successful builds don't substitute
for testing the tray and credentials on each OS.

## How it works

Afterglow is a tray-only Rust + Tauri 2 app. Application logic and tests live
in [three Rust files](../src-tauri/src). No frontend JavaScript or webview
window is shipped.

Each refresh invokes `gh` directly with separate arguments, without a shell:

```sh
gh api --hostname YOUR_HOST /copilot_internal/user \
  --jq '{plan: .copilot_plan, resets: .quota_reset_date, credits: .quota_snapshots.premium_interactions}'
```

The app displays GitHub's `quota_remaining`, `entitlement`, `credits_used`,
`percent_remaining`, and `quota_reset_date`. Fractional credits are preserved;
figures aren't recalculated or replaced with zero when missing.

**This endpoint is internal, not a supported public quota API.** Its response,
authentication requirements, and availability can change. Compatibility with
every Copilot plan isn't promised.

Authentication stays with GitHub CLI. Only selected quota data enters the app.
Raw subprocess errors and tokens aren't displayed or logged. Settings stay
on your computer; there's no backend or telemetry.

Small source size isn't a security guarantee or an independent audit.
Dependencies remain part of the audit surface. npm and Cargo lockfiles are
committed; builds aren't currently claimed to be reproducible.
