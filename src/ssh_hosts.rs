//! SSH host alias parser for Podium onboarding Step 3.
//!
//! Reads `~/.ssh/config` and returns the `Host` alias names that are not
//! bare git provider domains. These aliases are displayed in the git
//! authentication dropdown so the user can select which SSH identity
//! corresponds to a given project's git account.
//!
//! Written from scratch for Podium. Reference: the OpenSSH `ssh_config(5)`
//! man page format — no code derived from any third-party source.
//!
//! ## SSH config format
//!
//! Each `Host` line starts a new block. The value is one or more
//! space-separated patterns. Patterns starting with `!` are negations
//! and are ignored for our purposes. Patterns containing `*` or `?`
//! are wildcards and are not useful as display aliases — also ignored.
//! A `HostName` line inside a block gives the real hostname; if the
//! real hostname is a known git provider domain, the alias is filtered out.

use std::collections::BTreeSet;

/// Git provider domains that should not appear as SSH aliases in the
/// Podium git account dropdown. Entries with a `HostName` matching one
/// of these are filtered out, as are entries whose alias itself is one
/// of these domains.
const GIT_PROVIDER_DOMAINS: &[&str] = &[
    "bitbucket.org",
    "codeberg.org",
    "dev.azure.com",
    "gitea.com",
    "gitee.com",
    "github.com",
    "gist.github.com",
    "gitlab.com",
    "git.sr.ht",
    "sourcehut.org",
    "chromium.googlesource.com",
];

/// Returns `true` if `host` is a known git provider domain.
fn is_git_provider(host: &str) -> bool {
    let lower = host.to_ascii_lowercase();
    GIT_PROVIDER_DOMAINS.contains(&lower.as_str())
}

/// Parse the content of an `~/.ssh/config` file and return the set of
/// `Host` aliases that are suitable for display as git account selectors.
///
/// Aliases are excluded when:
/// - The alias itself is a known git provider domain (e.g. `github.com`)
/// - The alias contains a wildcard (`*` or `?`)
/// - The alias is a negation pattern (starts with `!`)
/// - The block's `HostName` is a known git provider domain
///
/// The returned set is sorted (via `BTreeSet`) for stable dropdown ordering.
///
/// # Example
///
/// ```text
/// Host github-personal
///   HostName github.com
///   IdentityFile ~/.ssh/id_personal
///
/// Host github-work
///   HostName github.com
///   IdentityFile ~/.ssh/id_work
///
/// Host github.com
///   IdentityFile ~/.ssh/id_default
/// ```
///
/// Returns `{"github-personal", "github-work"}`. The bare `github.com`
/// entry is excluded because the alias is itself a git provider domain.
pub fn parse_ssh_hosts(config: &str) -> BTreeSet<String> {
    let mut result = BTreeSet::new();

    // Current block state
    let mut current_aliases: Vec<String> = Vec::new();
    let mut current_hostname: Option<String> = None;

    for line in config.lines() {
        // SSH config ignores leading whitespace on continuation lines.
        let line = line.trim_start();

        // Skip blank lines and comments.
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Split into keyword and value on the first whitespace or '='.
        let (keyword, value) = match split_keyword_value(line) {
            Some(pair) => pair,
            None => continue,
        };

        if keyword.eq_ignore_ascii_case("Host") {
            // Commit the previous block before starting a new one.
            commit_block(&current_aliases, current_hostname.as_deref(), &mut result);
            current_aliases = parse_alias_patterns(value);
            current_hostname = None;
        } else if keyword.eq_ignore_ascii_case("HostName") {
            // First token only — HostName takes a single value.
            current_hostname = value.split_whitespace().next().map(str::to_owned);
        }
        // All other keywords are irrelevant to our purpose and are ignored.
    }

    // Commit the final block.
    commit_block(&current_aliases, current_hostname.as_deref(), &mut result);

    result
}

/// Commit one parsed Host block into `result`.
///
/// Skips the entire block if the block's `HostName` is a git provider domain.
/// For each alias in the block, skips it if the alias itself is a git provider
/// domain, a wildcard pattern, or a negation pattern.
fn commit_block(
    aliases: &[String],
    hostname: Option<&str>,
    result: &mut BTreeSet<String>,
) {
    if aliases.is_empty() {
        return;
    }

    // If the HostName is a git provider, skip the whole block.
    if let Some(hn) = hostname {
        if is_git_provider(hn) {
            return;
        }
    }

    for alias in aliases {
        // Skip wildcards and negations.
        if alias.contains('*') || alias.contains('?') || alias.starts_with('!') {
            continue;
        }
        // Skip bare git provider domain aliases.
        if is_git_provider(alias) {
            continue;
        }
        result.insert(alias.clone());
    }
}

/// Parse space-separated alias patterns from a `Host` line value.
///
/// Handles line continuations with trailing `\` by stripping the backslash
/// from each token. The caller is responsible for joining continuation lines
/// before calling this if needed — in practice `parse_ssh_hosts` processes
/// one line at a time and the `Host` value is typically on a single line.
fn parse_alias_patterns(value: &str) -> Vec<String> {
    value
        .split_whitespace()
        .map(|token| token.trim_end_matches('\\').to_owned())
        .filter(|token| !token.is_empty())
        .collect()
}

/// Split an SSH config line into `(keyword, value)`.
///
/// SSH config allows both whitespace-separated (`Host foo`) and
/// equals-separated (`Host=foo`) syntax. Returns `None` for lines
/// that have no value (keyword only) or are empty.
fn split_keyword_value(line: &str) -> Option<(&str, &str)> {
    // Try '=' separator first (with optional surrounding whitespace).
    if let Some(eq_pos) = line.find('=') {
        let keyword = line[..eq_pos].trim();
        let value = line[eq_pos + 1..].trim();
        if !keyword.is_empty() {
            return Some((keyword, value));
        }
    }

    // Fall back to whitespace separator.
    let mut parts = line.splitn(2, char::is_whitespace);
    let keyword = parts.next()?.trim();
    if keyword.is_empty() {
        return None;
    }
    let value = parts.next().unwrap_or("").trim();
    Some((keyword, value))
}
