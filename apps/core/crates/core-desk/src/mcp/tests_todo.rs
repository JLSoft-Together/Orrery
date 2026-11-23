use super::defs::tool_defs;
use super::tests_tools::{call, is_error, text, Fake};
use crate::preview::checklist;
use serde_json::{json, Value};

fn host(tag: &str) -> (Fake, std::path::PathBuf) {
    let data = std::env::temp_dir().join(format!("orrery-todo-{tag}-{}-{}", std::process::id(), crate::util::now_ms() as u64));
    std::fs::create_dir_all(&data).unwrap();
    let wt = data.join("wt");
    std::fs::create_dir_all(&wt).unwrap();
    let mut h = Fake::new(Box::new(|_| json!({ "ok": false })));
    h.root = wt.to_string_lossy().into_owned();
    h.data = Some(data.clone());
    (h, data)
}

#[test]
fn the_three_tools_are_listed_without_the_deepseek_gate() {
    for deepseek in [false, true] {
        let names: Vec<String> = tool_defs(deepseek).iter().map(|t| t["name"].as_str().unwrap().to_string()).collect();
        for n in ["todo_add", "todo_done", "todo_list"] {
            assert!(names.iter().any(|x| x == n), "{n}");
        }
    }
}

#[test]
fn an_agent_adds_lists_and_finishes_items_of_its_own_worktree() {
    let (h, data) = host("flow");
    let added = call(&h, "todo_add", json!({ "text": "cover the empty case" }));
    assert!(!is_error(&added));
    assert_eq!(added["structuredContent"]["open"], 1);
    let id = added["structuredContent"]["item"]["id"].as_str().unwrap().to_string();
    assert_eq!(added["structuredContent"]["item"]["by"], "agent");
    let listed = call(&h, "todo_list", json!({}));
    assert!(text(&listed).contains(&format!("[ ] {id} (agent) cover the empty case")), "{}", text(&listed));
    assert!(text(&listed).contains("No check script chosen."));
    let done = call(&h, "todo_done", json!({ "id": id }));
    assert!(!is_error(&done));
    assert_eq!(done["structuredContent"]["open"], 0);
    assert!(text(&call(&h, "todo_list", json!({}))).contains(&format!("[x] {id}")));
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn the_list_is_per_worktree_and_the_owner_items_show_up_for_the_agent() {
    let (h, data) = host("scope");
    checklist::add(&data, &checklist::toplevel(&h.root), "owner", "owner wrote this", 1.0).unwrap();
    assert!(text(&call(&h, "todo_list", json!({}))).contains("(owner) owner wrote this"));
    let (mut other, _) = host("scope2");
    other.data = Some(data.clone());
    assert!(text(&call(&other, "todo_list", json!({}))).contains("empty"));
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn bad_input_and_a_missing_store_are_refused_clearly() {
    let (h, data) = host("bad");
    for args in [json!({}), json!({ "text": "" }), json!({ "text": "x".repeat(201) }), json!({ "text": 5 })] {
        let r = call(&h, "todo_add", args);
        assert!(is_error(&r));
        assert_eq!(r["structuredContent"]["error"], "text");
    }
    let r = call(&h, "todo_done", json!({ "id": "t99" }));
    assert!(is_error(&r));
    assert_eq!(r["structuredContent"]["error"], "unknown");
    assert_eq!(call(&h, "todo_done", json!({}))["structuredContent"]["error"], "id");
    let mut none = Fake::new(Box::new(|_| Value::Null));
    none.data = None;
    assert_eq!(call(&none, "todo_list", json!({}))["structuredContent"]["error"], "unavailable");
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn changes_are_rate_limited_through_the_host() {
    let (mut h, data) = host("rate");
    h.allow = false;
    let r = call(&h, "todo_add", json!({ "text": "x" }));
    assert_eq!(r["structuredContent"]["error"], "rate");
    assert_eq!(h.takes.lock().unwrap()[0], ("todo".to_string(), 20, 60_000));
    assert_eq!(call(&h, "todo_done", json!({ "id": "t1" }))["structuredContent"]["error"], "rate");
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn no_tool_lets_an_agent_remove_an_owner_item() {
    let names: Vec<String> = tool_defs(true).iter().map(|t| t["name"].as_str().unwrap().to_string()).collect();
    assert!(!names.iter().any(|n| n.starts_with("todo_") && (n.contains("remove") || n.contains("delete"))));
    let (h, data) = host("perm");
    let root = checklist::toplevel(&h.root);
    let mine = checklist::add(&data, &root, "owner", "keep me", 1.0).unwrap();
    assert_eq!(checklist::remove(&data, &root, mine["id"].as_str().unwrap(), "agent"), Err("owner"));
    let _ = std::fs::remove_dir_all(&data);
}
