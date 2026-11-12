use std::time::{Duration, Instant};

use serde_json::json;

use super::push::{test_text, title_body};
use super::push_content::*;

fn mask(s: &str) -> String {
    core_desk::claude::mask::mask_hook_text(s, "C:\\Users\\me", "win32")
}

fn scene() -> Scene {
    Scene { agent: "claude".into(), session: "✳ Fix login bug".into(), tab: "Backend".into(), project: "orrery".into() }
}

fn fact(reason: &'static str) -> Fact {
    Fact { reason, name: "api".into(), cwd: "D:\\Code\\orrery".into(), tool: "Bash".into(), question: "Deploy to prod with API_KEY=sk-live-123456?".into(), worked: Some(754_000) }
}

#[test]
fn the_title_is_agent_session_project_without_spinner_glyphs_or_repeats() {
    assert_eq!(title_of(&scene(), None, &mask), "Claude · Fix login bug · orrery");
    let s = Scene { session: "orrery".into(), ..scene() };
    assert_eq!(title_of(&s, None, &mask), "Claude · orrery");
    let s = Scene { session: String::new(), ..scene() };
    assert_eq!(title_of(&s, None, &mask), "Claude · Backend · orrery");
    let s = Scene { agent: "codex".into(), session: String::new(), tab: String::new(), project: String::new() };
    assert_eq!(title_of(&s, Some(&fact("done")), &mask), "Codex · api · orrery");
    assert_eq!(title_of(&Scene::default(), None, &mask), "Agent");
    assert_eq!(agent_label("deepseek"), "DeepSeek");
    assert_eq!(agent_label("shell"), "Shell");
}

#[test]
fn bodies_follow_the_kind_in_both_locales() {
    let q = fact("question");
    assert_eq!(body_of("question", true, Some(&q), None, &mask), "Deploy to prod with API_KEY=••••");
    assert_eq!(body_of("question", false, None, None, &mask), "Waiting for your answer");
    assert_eq!(body_of("question", true, None, None, &mask), "Đang chờ bạn trả lời");
    assert_eq!(body_of("permission", true, Some(&q), None, &mask), "Cần duyệt: Bash");
    assert_eq!(body_of("permission", false, Some(&q), None, &mask), "Needs approval: Bash");
    assert_eq!(body_of("permission", false, None, None, &mask), "Needs your approval");
    assert_eq!(body_of("done", true, None, Some(754_000), &mask), "Xong sau 13m");
    assert_eq!(body_of("done", false, None, Some(20_000), &mask), "Done after <1m");
    assert_eq!(body_of("done", false, None, None, &mask), "Finished its turn");
    assert_eq!(body_of("error", true, None, None, &mask), "Dừng vì lỗi");
    assert_eq!(body_of("error", false, None, None, &mask), "Stopped with an error");
    assert_eq!(body_of("attention", true, Some(&q), None, &mask), "Đang chờ bạn");
    assert_eq!(body_of("attention", false, None, None, &mask), "Waiting for you");
}

#[test]
fn durations_read_like_the_example() {
    assert_eq!(duration(0), "<1m");
    assert_eq!(duration(29_999), "<1m");
    assert_eq!(duration(30_000), "1m");
    assert_eq!(duration(12 * 60_000), "12m");
    assert_eq!(duration(59 * 60_000), "59m");
    assert_eq!(duration(65 * 60_000), "1h 05m");
    assert_eq!(duration(26 * 3_600_000), "26h 00m");
}

#[test]
fn secrets_paths_controls_and_bidi_never_reach_the_text() {
    let raw = "Bearer abcdefghijkl\u{202E}gpj.exe\u{0007} at C:\\Users\\me\\x.env token=zzz\nnext\u{200B}line https://u:p4ss@h.io/a?key=k123";
    let out = clean(raw, &mask, TEXT_MAX);
    assert!(!out.contains("abcdefghijkl") && !out.contains("zzz") && !out.contains("p4ss") && !out.contains("k123"), "{out}");
    assert!(!out.contains("C:\\Users\\me") && out.contains("~\\x.env"), "{out}");
    assert!(!out.chars().any(|c| c.is_control() || matches!(c, '\u{202E}' | '\u{200B}')), "{out}");
    assert!(out.contains("•••• nextline"), "{out}");
    let t = title_of(&Scene { session: "export GITHUB_TOKEN=ghp_secret".into(), ..scene() }, None, &mask);
    assert!(!t.contains("ghp_secret"), "{t}");
}

#[test]
fn text_is_cut_to_120_code_points_without_splitting_a_mark_or_an_emoji() {
    let long = "a".repeat(300);
    let out = cut(&long, TEXT_MAX);
    assert_eq!(out.chars().count(), TEXT_MAX);
    assert!(out.ends_with('…'));
    assert_eq!(cut("ngắn", TEXT_MAX), "ngắn");
    let marks = format!("{}e\u{0301}\u{0323}tail", "b".repeat(118));
    let out = cut(&marks, TEXT_MAX);
    assert_eq!(out, format!("{}…", "b".repeat(118)));
    let emoji = format!("{}😀😀😀", "c".repeat(118));
    let out = cut(&emoji, TEXT_MAX);
    assert!(out.chars().count() <= TEXT_MAX && out.ends_with('…'));
    assert_eq!(String::from_utf16(&out.encode_utf16().collect::<Vec<_>>()).unwrap(), out);
    let skin = format!("{}👍\u{1F3FD}xyz", "d".repeat(118));
    assert_eq!(cut(&skin, TEXT_MAX), format!("{}…", "d".repeat(118)));
    let q = Fact { question: "Hỏi ".repeat(80), ..fact("question") };
    assert!(body_of("question", true, Some(&q), None, &mask).chars().count() <= TEXT_MAX);
}

#[test]
fn content_off_security_and_test_keep_the_fixed_text() {
    let f = fact("question");
    let sc = scene();
    let ask = |kind: &'static str, content: bool| notice(&Ask { kind, locale: "vi", content, scene: &sc, fact: Some(&f), worked: Some(60_000) }, &mask);
    let fixed = |(t, b): (&str, &str)| (t.to_owned(), b.to_owned());
    assert_eq!(ask("question", false), fixed(title_body("question", "vi")));
    assert_eq!(ask("done", false), fixed(title_body("done", "vi")));
    assert_eq!(ask("security", true), fixed(title_body("security", "vi")));
    assert_eq!(ask("test", true), fixed(test_text("vi")));
    assert_eq!(ask("question", true), ("Claude · Fix login bug · orrery".to_owned(), "Deploy to prod with API_KEY=••••".to_owned()));
    assert_eq!(ask("done", true).1, "Xong sau 1m");
}

#[test]
fn episodes_feed_facts_that_refine_attention_and_expire() {
    let mut facts = Facts::default();
    let now = Instant::now();
    let ep = json!({ "type": "telegram", "episode": "tw:t1|0", "reason": "permission", "terminalId": "t1", "name": "api", "cwd": "/p/orrery", "tool": "Bash", "since": 1 });
    assert!(facts.note(&ep, now));
    let f = facts.get("t1", now).unwrap().clone();
    assert_eq!((f.reason, f.tool.as_str(), f.name.as_str()), ("permission", "Bash", "api"));
    assert_eq!(refine("attention", Some(&f)), "permission");
    assert_eq!(refine("done", Some(&f)), "done");
    assert_eq!(refine("attention", Some(&fact("waiting"))), "attention");
    assert_eq!(refine("attention", None), "attention");
    assert!(facts.get("t1", now + FACT_TTL + Duration::from_secs(1)).is_none());
    let ask = json!({ "type": "telegram", "episode": "tw:t2|0", "reason": "question", "terminalId": "t2", "name": "n", "cwd": "/p", "ask": { "question": "Which DB?", "options": ["a"], "multi": false }, "tool": "bad tool" });
    assert!(facts.note(&ask, now));
    let q = facts.get("t2", now).unwrap();
    assert_eq!((q.question.as_str(), q.tool.as_str()), ("Which DB?", ""));
    let done = json!({ "type": "telegram", "episode": "td:t3|0", "reason": "done", "terminalId": "t3", "name": "n", "cwd": "/p", "worked": 125000.5 });
    assert!(facts.note(&done, now));
    assert_eq!(facts.get("t3", now).unwrap().worked, Some(125_000));
    assert!(!facts.note(&json!({ "type": "phone", "reason": "done", "terminalId": "t4" }), now));
    assert!(!facts.note(&json!({ "type": "telegram", "reason": "boom", "terminalId": "t4" }), now));
    assert!(!facts.note(&json!({ "type": "telegram", "reason": "done", "terminalId": "" }), now));
    for i in 0..400 {
        facts.note(&json!({ "type": "telegram", "reason": "waiting", "terminalId": format!("x{i}") }), now + Duration::from_millis(i));
    }
    assert!(facts.get("x399", now + Duration::from_secs(1)).is_some());
    assert!(facts.get("t1", now).is_none());
}

#[test]
fn the_scene_comes_from_the_workspace_summary() {
    let summary = json!({ "rev": 1, "tabs": [{ "key": "t", "name": "Backend", "project": "D:\\Code\\orrery", "panes": [{ "key": "p1", "agent": "codex", "title": "Refactor push", "cwd": "D:\\Code\\orrery\\apps" }, { "key": "p2", "preset": "shell", "title": "pwsh" }] }] });
    assert_eq!(scene_of(&summary, "p1"), Scene { agent: "codex".into(), session: "Refactor push".into(), tab: "Backend".into(), project: "orrery".into() });
    assert_eq!(scene_of(&summary, "p2").agent, "shell");
    assert_eq!(scene_of(&summary, "nope"), Scene::default());
    let no_project = json!({ "tabs": [{ "key": "t", "panes": [{ "key": "p", "cwd": "/home/u/web/" }] }] });
    assert_eq!(scene_of(&no_project, "p").project, "web");
}

#[test]
fn secrets_split_by_hidden_or_control_characters_are_joined_before_masking() {
    for raw in ["API_\u{200B}KEY=sk-live-123456", "API_\u{00AD}KEY=sk-live-123456", "API_\u{0007}KEY=sk-live-123456", "API_\u{2066}KEY=sk-live-123456\u{2069}"] {
        let out = clean(raw, &mask, TEXT_MAX);
        assert!(!out.contains("sk-live-123456"), "{raw:?} -> {out}");
        assert!(out.starts_with("API_KEY="), "{out}");
    }
    let ant = clean("key sk-ant-\u{200B}api03-AbCdEfGhIjKlMnOpQrStUvWxYz0123456789", &mask, TEXT_MAX);
    assert!(!ant.contains("AbCdEfGhIjKlMnOp"), "{ant}");
    let bearer = clean("Bearer abc\u{200C}defghijklmnop", &mask, TEXT_MAX);
    assert!(!bearer.contains("defghijklmnop"), "{bearer}");
    assert_eq!(clean("a\tb\n\nc", &mask, TEXT_MAX), "a b c");
}
