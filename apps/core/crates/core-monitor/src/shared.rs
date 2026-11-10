use core_store::model::{AgentStatus, TerminalStatus};

pub const CONTEXT_WINDOW_DEFAULT: f64 = 200_000.0;
pub const CONTEXT_WINDOW_1M: f64 = 1_000_000.0;
pub const AUTOCOMPACT_BUFFER_PCT: f64 = 16.5;

pub fn autocompact_buffer_pct(auto_compact_window: Option<f64>) -> f64 {
    match auto_compact_window {
        Some(w) if w > 0.0 => (w / CONTEXT_WINDOW_1M * 100.0).min(100.0),
        _ => AUTOCOMPACT_BUFFER_PCT,
    }
}

pub fn usable_pct(raw: f64, buffer: f64) -> f64 {
    if buffer >= 100.0 {
        return 100.0;
    }
    (raw / (100.0 - buffer) * 100.0).clamp(0.0, 100.0)
}

const FAMILIES: [&str; 5] = ["opus", "sonnet", "haiku", "fable", "mythos"];

pub fn model_family(model: Option<&str>) -> Option<&'static str> {
    let m = model?.to_lowercase();
    if m.is_empty() {
        return None;
    }
    FAMILIES.into_iter().find(|f| m.contains(f))
}

fn valid_window(n: Option<f64>) -> Option<f64> {
    n.filter(|n| n.is_finite() && (1_000.0..=100_000_000.0).contains(n))
}

#[derive(Default, Clone, Copy)]
pub struct WindowHints<'a> {
    pub settings_model: Option<&'a str>,
    pub peak: Option<f64>,
    pub reported: Option<f64>,
    pub catalog: Option<f64>,
}

pub fn context_window_of(model: Option<&str>, hints: WindowHints) -> Option<f64> {
    if let Some(r) = valid_window(hints.reported) {
        return Some(r);
    }
    let model = model.filter(|m| m.starts_with("claude-"))?;
    let guess = guess_window(model, &hints);
    Some(match valid_window(hints.catalog) {
        Some(c) => c.max(guess),
        None => guess,
    })
}

fn guess_window(model: &str, hints: &WindowHints) -> f64 {
    if model.contains("[1m]") {
        return CONTEXT_WINDOW_1M;
    }
    if let Some(s) = hints.settings_model.filter(|s| s.contains("[1m]")) {
        if model_family(Some(s)) == model_family(Some(model)) || model.starts_with(&s.replace("[1m]", "")) {
            return CONTEXT_WINDOW_1M;
        }
    }
    if hints.peak.unwrap_or(0.0) > CONTEXT_WINDOW_DEFAULT {
        return CONTEXT_WINDOW_1M;
    }
    CONTEXT_WINDOW_DEFAULT
}

pub fn map_registry_status(raw: Option<&str>) -> (TerminalStatus, bool) {
    match raw {
        Some("busy") => (TerminalStatus::Working, false),
        Some("waiting") => (TerminalStatus::Waiting, false),
        Some("shell") => (TerminalStatus::Idle, true),
        Some("idle") => (TerminalStatus::Idle, false),
        _ => (TerminalStatus::Unknown, false),
    }
}

pub fn main_agent_status(status: TerminalStatus) -> AgentStatus {
    match status {
        TerminalStatus::Working => AgentStatus::Working,
        TerminalStatus::Waiting => AgentStatus::Waiting,
        TerminalStatus::Idle => AgentStatus::Idle,
        TerminalStatus::Stale => AgentStatus::Completed,
        TerminalStatus::Error => AgentStatus::Error,
        TerminalStatus::Unknown => AgentStatus::Unknown,
    }
}

pub fn js_round1(v: f64) -> f64 {
    (v * 10.0 + 0.5).floor() / 10.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_percent_like_the_status_line() {
        assert_eq!(usable_pct(50.0, AUTOCOMPACT_BUFFER_PCT).round(), 60.0);
        assert_eq!(usable_pct(90.0, AUTOCOMPACT_BUFFER_PCT), 100.0);
        assert_eq!(usable_pct(10.0, 100.0), 100.0);
        assert_eq!(autocompact_buffer_pct(None), 16.5);
        assert_eq!(autocompact_buffer_pct(Some(0.0)), 16.5);
        assert_eq!(autocompact_buffer_pct(Some(100_000.0)), 10.0);
        assert_eq!(autocompact_buffer_pct(Some(5_000_000.0)), 100.0);
    }

    #[test]
    fn windows_from_reports_catalog_settings_and_peak() {
        let h = WindowHints::default();
        assert_eq!(context_window_of(Some("gpt-5"), h), None);
        assert_eq!(context_window_of(Some("claude-opus-5-5"), h), Some(200_000.0));
        assert_eq!(context_window_of(Some("claude-opus-5-5[1m]"), h), Some(1_000_000.0));
        assert_eq!(context_window_of(Some("claude-opus-5-5"), WindowHints { settings_model: Some("opus[1m]"), ..h }), Some(1_000_000.0));
        assert_eq!(context_window_of(Some("claude-sonnet-5"), WindowHints { settings_model: Some("opus[1m]"), ..h }), Some(200_000.0));
        assert_eq!(context_window_of(Some("claude-opus-5-5"), WindowHints { peak: Some(250_000.0), ..h }), Some(1_000_000.0));
        assert_eq!(context_window_of(Some("claude-opus-5-5"), WindowHints { catalog: Some(500_000.0), ..h }), Some(500_000.0));
        assert_eq!(context_window_of(None, WindowHints { reported: Some(300_000.0), ..h }), Some(300_000.0));
        assert_eq!(js_round1(12.25), 12.3);
        assert_eq!(js_round1(-0.05), 0.0);
    }
}
