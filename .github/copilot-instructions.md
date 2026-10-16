# Afterglow

Keep this app small, auditable, and tray-only. It uses Tauri 2 and Rust, with no
frontend webview, server, telemetry, or token storage.

Invoke GitHub CLI directly with argument arrays, never shell interpolation.
Reuse existing `gh auth` credentials and specify the configured hostname on
every request. Never change the active account or log tokens or raw API errors.
The quota endpoint is internal and unstable; missing fields are errors, never
zero. Preserve fractional balances and report all five monthly quota fields.

macOS is the first supported platform. Preserve Windows compatibility without
claiming it has been tested. Do not add dependencies or abstractions unnecessarily.
Run `npm test`, `npm run check`, and the relevant platform build after changes.
Do not publish signing credentials, personal settings, or account responses.
