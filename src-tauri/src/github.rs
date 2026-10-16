use std::{path::PathBuf, time::Duration};

use serde::{Deserialize, Serialize};
use tokio::process::Command;

use crate::credits::{self, Credits};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub hostname: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            hostname: "procode.ghe.com".into(),
        }
    }
}

pub fn validate_host(host: &str) -> Result<(), String> {
    if host.len() > 253
        || !host.contains('.')
        || !host.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'-')
        })
    {
        return Err(
            "Invalid hostname in config.json; use a hostname without https:// or a path.".into(),
        );
    }
    Ok(())
}

pub fn executable() -> PathBuf {
    // Finder-launched macOS applications do not inherit the terminal's PATH.
    #[cfg(target_os = "macos")]
    for path in ["/opt/homebrew/bin/gh", "/usr/local/bin/gh"] {
        if std::path::Path::new(path).is_file() {
            return path.into();
        }
    }
    if cfg!(windows) {
        "gh.exe".into()
    } else {
        "gh".into()
    }
}

fn failure(stderr: &[u8], hostname: &str) -> String {
    // Classify errors without exposing raw subprocess output or credentials.
    let message = String::from_utf8_lossy(stderr).to_ascii_lowercase();
    let reason = if message.contains("rate limit") || message.contains("http 429") {
        "GitHub rate limit reached; retry later"
    } else if message.contains("http 401")
        || message.contains("gh auth login")
        || message.contains("not logged")
    {
        "GitHub authentication required or expired"
    } else if message.contains("http 403") {
        "GitHub denied access to this quota endpoint"
    } else if message.contains("http 404") {
        "Internal quota endpoint unavailable for this host or account"
    } else {
        "GitHub request failed; check network and endpoint access"
    };
    format!("{reason}. Diagnose: gh auth status --hostname {hostname}")
}

pub async fn fetch(config: &Config) -> Result<Credits, String> {
    validate_host(&config.hostname)?;
    let mut command = Command::new(executable());
    command.args([
        "api", "--hostname", &config.hostname, "/copilot_internal/user",
        "--jq",
        "{plan: .copilot_plan, resets: .quota_reset_date, credits: .quota_snapshots.premium_interactions}",
    ]);
    command
        .kill_on_drop(true)
        .env("GH_PROMPT_DISABLED", "1")
        .env("GH_PAGER", "cat");
    #[cfg(windows)]
    command.creation_flags(0x08000000);
    let output = tokio::time::timeout(Duration::from_secs(30), command.output())
        .await
        .map_err(|_| "GitHub CLI request timed out after 30 seconds. Try Refresh.".to_string())?
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                "GitHub CLI not found. Install gh from https://cli.github.com, then restart Afterglow.".into()
            } else {
                format!("Unable to run GitHub CLI: {e}")
            }
        })?;
    if !output.status.success() {
        return Err(failure(&output.stderr, &config.hostname));
    }
    credits::parse(&output.stdout)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_hosts_not_urls_or_arguments() {
        for host in ["github.com", "procode.ghe.com", "my-company.ghe.com"] {
            assert!(validate_host(host).is_ok());
        }
        for host in [
            "",
            "--help",
            "https://github.com",
            "github.com/path",
            "a..com",
            "-a.com",
            "a.com\n",
            "a.com:443",
        ] {
            assert!(validate_host(host).is_err(), "{host}");
        }
    }

    #[test]
    fn config_is_explicit_and_rejects_typos() {
        assert!(serde_json::from_str::<Config>(r#"{"host":"github.com"}"#).is_err());
        assert_eq!(Config::default().hostname, "procode.ghe.com");
    }

    #[test]
    fn errors_are_actionable_without_leaking_subprocess_output() {
        for (stderr, expected) in [
            ("HTTP 401: secret-example", "authentication"),
            ("please run: gh auth login secret-example", "authentication"),
            ("HTTP 403: secret-example", "denied access"),
            ("HTTP 404: secret-example", "unavailable"),
            ("HTTP 429: secret-example", "rate limit"),
            ("API rate limit exceeded secret-example", "rate limit"),
            ("connection failed secret-example", "network"),
        ] {
            let error = failure(stderr.as_bytes(), "example.ghe.com");
            assert!(error.contains(expected));
            assert!(error.contains("gh auth status --hostname example.ghe.com"));
            assert!(!error.contains("secret-example"));
        }
    }
}
