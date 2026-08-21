//! Privacy-safe Human hook identity resolution.
//!
//! Commands are accepted only as ephemeral input at discovery time. This
//! module returns a bounded display candidate or an event fallback; it never
//! retains a command, argument list, or parent path.

use crate::domain::HookEvent;

const MAX_DISPLAY_NAME_LEN: usize = 96;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DisplayIdentitySource {
    UserAnnotation,
    ExplicitMetadata,
    ScriptFilename,
    CommandBasename,
    EventFallback,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DisplayName {
    Literal(String),
    EventFallback(HookEvent),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedDisplayIdentity {
    pub name: DisplayName,
    pub source: DisplayIdentitySource,
    pub source_label: &'static str,
}

/// Resolves the first admitted Human candidate without changing stable
/// attribution. All literal inputs are reduced to bounded, terminal-safe text.
pub fn resolve_display_identity(
    user_annotation: Option<&str>,
    explicit_metadata: Option<&str>,
    script_filename: Option<&str>,
    command_basename: Option<&str>,
    event: HookEvent,
    source_label: &'static str,
) -> ResolvedDisplayIdentity {
    for (value, source) in [
        (user_annotation, DisplayIdentitySource::UserAnnotation),
        (explicit_metadata, DisplayIdentitySource::ExplicitMetadata),
        (script_filename, DisplayIdentitySource::ScriptFilename),
        (command_basename, DisplayIdentitySource::CommandBasename),
    ] {
        if let Some(value) = value.and_then(sanitize_display_name) {
            return ResolvedDisplayIdentity {
                name: DisplayName::Literal(value),
                source,
                source_label,
            };
        }
    }
    ResolvedDisplayIdentity {
        name: DisplayName::EventFallback(event),
        source: DisplayIdentitySource::EventFallback,
        source_label,
    }
}

/// Derives one safe display name from a command without storing that command.
/// The tokenizer supports quoted Windows and Unix path arguments and only
/// recognizes script arguments for a short, audited wrapper list.
pub fn display_name_from_command(command: &str) -> Option<String> {
    let tokens = command_tokens(command);
    let executable = tokens.first()?;
    let executable_name = safe_basename(executable)?;
    let lower = executable_name.to_ascii_lowercase();
    if is_shell_wrapper(&lower) {
        let script = script_argument(&tokens).or_else(|| direct_script_argument(&tokens))?;
        return humanize_basename(&script);
    }
    if has_script_extension(&executable_name) {
        return humanize_basename(&executable_name);
    }
    if is_weak_shell(&lower) {
        return None;
    }
    humanize_basename(&executable_name)
}

/// Validates literal user annotation text before it is stored in HookStat
/// state. It rejects controls, path-shaped values, and likely secret shapes.
pub fn sanitize_display_name(value: &str) -> Option<String> {
    let compact = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.is_empty()
        || compact.chars().count() > MAX_DISPLAY_NAME_LEN
        || compact.chars().any(char::is_control)
        || compact.contains(['/', '\\'])
        || looks_secret_like(&compact)
        || looks_like_command(&compact)
    {
        return None;
    }
    Some(compact)
}

pub fn generated_label(value: &str) -> bool {
    value.trim().is_empty() || value.starts_with("Codex /") || value.starts_with("hk_")
}

fn command_tokens(command: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    for character in command.chars() {
        match (quote, character) {
            (Some(active), value) if value == active => quote = None,
            (None, '\'' | '"') => quote = Some(character),
            (None, value) if value.is_whitespace() => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(character),
        }
    }
    if quote.is_some() {
        return Vec::new();
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn script_argument(tokens: &[String]) -> Option<String> {
    let index = tokens.iter().position(|value| {
        value.eq_ignore_ascii_case("-file")
            || value.eq_ignore_ascii_case("-f")
            || value.eq_ignore_ascii_case("--file")
    })?;
    tokens.get(index + 1).and_then(|value| safe_basename(value))
}

fn direct_script_argument(tokens: &[String]) -> Option<String> {
    tokens
        .iter()
        .skip(1)
        .find(|value| !value.starts_with('-'))
        .and_then(|value| safe_basename(value))
}

fn safe_basename(value: &str) -> Option<String> {
    let basename = value.rsplit(['/', '\\']).next()?.trim();
    if basename.is_empty() || basename.chars().any(char::is_control) || looks_secret_like(basename)
    {
        return None;
    }
    Some(basename.to_owned())
}

fn humanize_basename(value: &str) -> Option<String> {
    let without_extension = value
        .rsplit_once('.')
        .filter(|(_, extension)| is_known_extension(extension))
        .map_or(value, |(stem, _)| stem);
    let words = without_extension
        .split(['-', '_', '.'])
        .filter(|word| !word.is_empty())
        .map(humanize_word)
        .collect::<Vec<_>>();
    sanitize_display_name(&words.join(" "))
}

fn humanize_word(value: &str) -> String {
    match value.to_ascii_lowercase().as_str() {
        "tabbeacon" => "TabBeacon".into(),
        "hapi" => "HAPI".into(),
        "ntfy" => "ntfy".into(),
        _ => {
            let mut characters = value.chars();
            match characters.next() {
                Some(first) => first.to_uppercase().collect::<String>() + characters.as_str(),
                None => String::new(),
            }
        }
    }
}

fn has_script_extension(value: &str) -> bool {
    value
        .rsplit_once('.')
        .is_some_and(|(_, extension)| is_known_extension(extension))
}

fn is_known_extension(value: &str) -> bool {
    matches!(
        value.to_ascii_lowercase().as_str(),
        "ps1" | "sh" | "bash" | "py" | "js" | "mjs" | "cjs" | "cmd" | "bat"
    )
}

fn is_shell_wrapper(value: &str) -> bool {
    matches!(
        value,
        "pwsh"
            | "pwsh.exe"
            | "powershell"
            | "powershell.exe"
            | "python"
            | "python.exe"
            | "node"
            | "node.exe"
            | "bash"
            | "sh"
    )
}

fn is_weak_shell(value: &str) -> bool {
    is_shell_wrapper(value) || matches!(value, "cmd" | "cmd.exe")
}

fn looks_secret_like(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.contains("token")
        || lower.contains("secret")
        || lower.contains("password")
        || lower.starts_with("sk-")
        || value
            .chars()
            .filter(|character| character.is_ascii_alphanumeric())
            .count()
            > 64
}

fn looks_like_command(value: &str) -> bool {
    let mut words = value.split_whitespace();
    let Some(first) = words.next() else {
        return false;
    };
    words.next().is_some()
        && matches!(
            first.to_ascii_lowercase().as_str(),
            "pwsh"
                | "pwsh.exe"
                | "powershell"
                | "powershell.exe"
                | "python"
                | "python.exe"
                | "node"
                | "node.exe"
                | "bash"
                | "sh"
                | "cmd"
                | "cmd.exe"
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_resolution_uses_a_sanitized_script_basename() {
        assert_eq!(
            display_name_from_command(
                "pwsh -File 'C:\\Program Files\\TabBeacon\\tabbeacon-stop.ps1'"
            ),
            Some("TabBeacon Stop".into())
        );
        assert_eq!(
            display_name_from_command("python /opt/钩子/ntfy-notification.py"),
            Some("ntfy Notification".into())
        );
        assert_eq!(
            display_name_from_command("bash /opt/hooks/hapi-session.sh"),
            Some("HAPI Session".into())
        );
    }

    #[test]
    fn command_resolution_never_returns_paths_or_secret_shaped_text() {
        assert_eq!(
            display_name_from_command("pwsh -File C:\\private\\sk-secret.ps1"),
            None
        );
        assert_eq!(display_name_from_command("cmd /c private.cmd"), None);
    }

    #[test]
    fn priority_and_event_fallback_are_deterministic() {
        let value = resolve_display_identity(
            Some("Owner Alias"),
            Some("Metadata"),
            Some("script"),
            Some("command"),
            HookEvent::Stop,
            "user_hooks",
        );
        assert_eq!(value.name, DisplayName::Literal("Owner Alias".into()));
        assert_eq!(value.source, DisplayIdentitySource::UserAnnotation);
        let fallback =
            resolve_display_identity(None, None, None, None, HookEvent::Stop, "user_hooks");
        assert_eq!(fallback.name, DisplayName::EventFallback(HookEvent::Stop));
    }

    #[test]
    fn user_annotations_are_bounded_literal_text() {
        assert_eq!(
            sanitize_display_name(" HAPI   Session Hook "),
            Some("HAPI Session Hook".into())
        );
        assert_eq!(sanitize_display_name("C:\\private\\hook"), None);
        assert_eq!(sanitize_display_name("sk-private-token"), None);
        assert_eq!(sanitize_display_name("pwsh -NoProfile -Command task"), None);
    }
}
