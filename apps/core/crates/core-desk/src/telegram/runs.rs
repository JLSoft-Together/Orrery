use super::base::{Entry, Tg};
use super::html::{cell_t, markup, Row};
use super::message::TgMessage;
use super::text_more::{more, more1};
use serde_json::{json, Value};
use std::sync::Arc;

impl Tg {
    pub fn run_cmd(self: &Arc<Self>, msg: &TgMessage, project: &str, config: &str, vi: bool) {
        let configs: Vec<Value> = match self.host("telegram.runConfigs", json!({"project": project})) {
            Some(Value::Array(l)) => l.into_iter().filter(|c| c["id"].is_string() && c["name"].is_string()).collect(),
            _ => Vec::new(),
        };
        if configs.is_empty() {
            self.reply(msg, more(vi, "runNone"), None);
            return;
        }
        let want = super::jsx::trim(config).to_lowercase();
        let found: Vec<Value> = if want.is_empty() {
            configs
        } else {
            configs.into_iter().filter(|c| {
                let n = c["name"].as_str().unwrap_or("").to_lowercase();
                n == want || n.contains(&want)
            }).collect()
        };
        if !want.is_empty() && found.is_empty() {
            self.reply(msg, &more1(vi, "runNoMatch", config), None);
            return;
        }
        if found.len() > 1 {
            let rows: Vec<Row> = found
                .iter()
                .take(8)
                .map(|c| {
                    let name = c["name"].as_str().unwrap_or("").to_string();
                    let key = self.add_action(Entry { kind: "rn".into(), project: project.into(), cwd: project.into(), name: name.clone(), chat_id: msg.chat_id.clone(), thread_id: msg.thread_id, extra: c["id"].as_str().map(str::to_string), ..Entry::default() });
                    vec![cell_t("rn", &key, &name)]
                })
                .collect();
            self.reply(msg, more(vi, "runPick"), Some(markup(vi, &rows)));
            return;
        }
        let c = &found[0];
        self.start_run(project, c["id"].as_str().unwrap_or(""), c["name"].as_str().unwrap_or(""), &msg.chat_id, msg.thread_id);
    }
}
