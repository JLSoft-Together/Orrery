use std::collections::HashMap;
use std::sync::Mutex;

use tauri::{AppHandle, Manager};
use tauri_plugin_notification::{Action, ActionType, NotificationExt};

use crate::menu_toast::Content;
use crate::state::lock;
use crate::toast::MAX_ANSWERS;

#[derive(Default)]
pub struct Notes(Mutex<HashMap<i32, Content>>);

pub fn note_id(tag: &str) -> i32 {
    let mut h: u32 = 0x811c_9dc5;
    for b in tag.bytes() {
        h ^= u32::from(b);
        h = h.wrapping_mul(0x0100_0193);
    }
    (h & 0x7fff_ffff) as i32
}

pub fn type_id(id: i32) -> String {
    format!("menu-{id}")
}

pub fn actions_of(c: &Content) -> Vec<(String, String)> {
    c.answers.iter().take(MAX_ANSWERS).enumerate().map(|(i, a)| (format!("a{i}"), a.label.clone())).collect()
}

pub fn url_for(c: &Content, action_id: &str) -> Option<String> {
    if action_id == "tap" || action_id == "default" {
        return Some(c.open.url.clone());
    }
    let i: usize = action_id.strip_prefix('a')?.parse().ok()?;
    c.answers.get(i).filter(|_| i < MAX_ANSWERS).map(|a| a.url.clone())
}

pub fn install(app: &AppHandle) {
    app.manage(Notes::default());
    let handle = app.clone();
    let _ = app.notification().on_action(move |performed| {
        let Some(id) = performed.notification().map(|n| n.id()) else { return };
        let Some(notes) = handle.try_state::<Notes>() else { return };
        let url = lock(&notes.0).get(&id).and_then(|c| url_for(c, performed.action_id()));
        if let Some(url) = url {
            crate::links::handle(&handle, &url);
        }
    });
}

pub fn show(app: &AppHandle, c: Content) {
    let Some(notes) = app.try_state::<Notes>() else { return };
    let id = note_id(&c.tag);
    let kind = type_id(id);
    let actions = actions_of(&c).into_iter().map(|(aid, title)| Action::builder(aid, title).build()).collect();
    let n = app.notification();
    let _ = n.register_action_types(vec![ActionType::builder(&kind).actions(actions).build()]);
    let shown = n.builder().id(id).title(&c.title).body(&c.body).action_type_id(&kind).show();
    if shown.is_ok() {
        lock(&notes.0).insert(id, c);
    }
}

pub fn hide(app: &AppHandle, tag: &str) {
    if let Some(notes) = app.try_state::<Notes>() {
        lock(&notes.0).remove(&note_id(tag));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::toast::Answer;

    fn content() -> Content {
        let a = |l: &str, u: &str| Answer { label: l.into(), url: u.into() };
        Content { tag: "t".into(), title: "T".into(), body: "B".into(), persistent: true, open: a("Open", "agent-monitor://menu/open"), dismiss: "x".into(), answers: vec![a("Yes", "agent-monitor://menu/1"), a("No", "agent-monitor://menu/2"), a("Later", "agent-monitor://menu/3"), a("Never", "agent-monitor://menu/4")] }
    }

    #[test]
    fn a_click_opens_and_each_button_answers_its_option() {
        let c = content();
        assert_eq!(actions_of(&c), vec![("a0".into(), "Yes".into()), ("a1".into(), "No".into()), ("a2".into(), "Later".into())]);
        assert_eq!(url_for(&c, "tap").as_deref(), Some("agent-monitor://menu/open"));
        assert_eq!(url_for(&c, "default").as_deref(), Some("agent-monitor://menu/open"));
        assert_eq!(url_for(&c, "a1").as_deref(), Some("agent-monitor://menu/2"));
        assert_eq!(url_for(&c, "a3"), None);
        assert_eq!(url_for(&c, "b1"), None);
        assert_eq!(url_for(&c, "dismiss"), None);
    }

    #[test]
    fn ids_are_stable_positive_and_per_tag() {
        assert_eq!(note_id("menu:a"), note_id("menu:a"));
        assert_ne!(note_id("menu:a"), note_id("menu:b"));
        assert!(note_id("x") >= 0);
        assert_eq!(type_id(5), "menu-5");
    }
}
