use crate::json::{ignore, value, Arr, FromObj, JNum, JStr, JTrue, Obj};
use crate::time::{local_day_hour, parse_iso};
use serde::de::MapAccess;

#[derive(Default)]
pub struct Line {
    pub kind: Option<String>,
    pub timestamp: Option<String>,
    pub cwd: Option<String>,
    pub message: Option<Message>,
}

#[derive(Default)]
pub struct Message {
    pub model: Option<String>,
    pub id: Option<String>,
    pub usage: Option<Usage>,
    pub content: Option<Vec<Option<Block>>>,
}

#[derive(Default)]
pub struct Usage {
    pub input: f64,
    pub output: f64,
    pub creation: f64,
    pub read: f64,
    pub split: Option<Split>,
    pub fast: bool,
}

#[derive(Default)]
pub struct Split {
    pub e5m: f64,
    pub e1h: f64,
}

#[derive(Default)]
pub struct Block {
    pub kind: Option<String>,
    pub name: Option<String>,
    pub id: Option<String>,
    pub input: Option<Input>,
    pub tool_use_id: Option<String>,
    pub is_error: bool,
}

#[derive(Default)]
pub struct Input {
    pub skill: Option<String>,
    pub subagent_type: Option<String>,
}

fn s<'de, A: MapAccess<'de>>(map: &mut A) -> Result<Option<String>, A::Error> {
    value::<A, JStr>(map).map(|v| v.0)
}

fn n<'de, A: MapAccess<'de>>(map: &mut A) -> Result<f64, A::Error> {
    value::<A, JNum>(map).map(|v| v.0)
}

fn o<'de, A: MapAccess<'de>, T: FromObj>(map: &mut A) -> Result<Option<T>, A::Error> {
    value::<A, Obj<T>>(map).map(|v| v.0)
}

impl FromObj for Line {
    fn field<'de, A: MapAccess<'de>>(&mut self, key: &str, map: &mut A) -> Result<(), A::Error> {
        match key {
            "type" => self.kind = s(map)?,
            "timestamp" => self.timestamp = s(map)?,
            "cwd" => self.cwd = s(map)?,
            "message" => self.message = o(map)?,
            _ => ignore(map)?,
        }
        Ok(())
    }
}

impl FromObj for Message {
    fn field<'de, A: MapAccess<'de>>(&mut self, key: &str, map: &mut A) -> Result<(), A::Error> {
        match key {
            "model" => self.model = s(map)?,
            "id" => self.id = s(map)?,
            "usage" => self.usage = o(map)?,
            "content" => self.content = value::<A, Arr<Obj<Block>>>(map)?.0.map(|v| v.into_iter().map(|b| b.0).collect()),
            _ => ignore(map)?,
        }
        Ok(())
    }
}

impl FromObj for Usage {
    fn field<'de, A: MapAccess<'de>>(&mut self, key: &str, map: &mut A) -> Result<(), A::Error> {
        match key {
            "input_tokens" => self.input = n(map)?,
            "output_tokens" => self.output = n(map)?,
            "cache_creation_input_tokens" => self.creation = n(map)?,
            "cache_read_input_tokens" => self.read = n(map)?,
            "cache_creation" => self.split = o(map)?,
            "speed" => self.fast = s(map)?.as_deref() == Some("fast"),
            _ => ignore(map)?,
        }
        Ok(())
    }
}

impl FromObj for Split {
    fn field<'de, A: MapAccess<'de>>(&mut self, key: &str, map: &mut A) -> Result<(), A::Error> {
        match key {
            "ephemeral_5m_input_tokens" => self.e5m = n(map)?,
            "ephemeral_1h_input_tokens" => self.e1h = n(map)?,
            _ => ignore(map)?,
        }
        Ok(())
    }
}

impl FromObj for Block {
    fn field<'de, A: MapAccess<'de>>(&mut self, key: &str, map: &mut A) -> Result<(), A::Error> {
        match key {
            "type" => self.kind = s(map)?,
            "name" => self.name = s(map)?,
            "id" => self.id = s(map)?,
            "input" => self.input = o(map)?,
            "tool_use_id" => self.tool_use_id = s(map)?,
            "is_error" => self.is_error = value::<A, JTrue>(map)?.0,
            _ => ignore(map)?,
        }
        Ok(())
    }
}

impl FromObj for Input {
    fn field<'de, A: MapAccess<'de>>(&mut self, key: &str, map: &mut A) -> Result<(), A::Error> {
        match key {
            "skill" => self.skill = s(map)?,
            "subagent_type" => self.subagent_type = s(map)?,
            _ => ignore(map)?,
        }
        Ok(())
    }
}

pub fn parse_line(line: &[u8]) -> Option<Line> {
    match serde_json::from_slice::<Obj<Line>>(line) {
        Ok(v) => v.0,
        Err(_) if std::str::from_utf8(line).is_err() => serde_json::from_str::<Obj<Line>>(&String::from_utf8_lossy(line)).ok().and_then(|v| v.0),
        Err(_) => None,
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Totals {
    pub messages: f64,
    pub input: f64,
    pub output: f64,
    pub cache_write_5m: f64,
    pub cache_write_1h: f64,
    pub cache_read: f64,
}

impl Totals {
    pub fn add(&mut self, src: &Totals, sign: f64) {
        self.messages += sign * src.messages;
        self.input += sign * src.input;
        self.output += sign * src.output;
        self.cache_write_5m += sign * src.cache_write_5m;
        self.cache_write_1h += sign * src.cache_write_1h;
        self.cache_read += sign * src.cache_read;
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Record {
    pub key: String,
    pub day: String,
    pub project: String,
    pub message_id: String,
    pub hour: u32,
    pub totals: Totals,
}

pub fn bucket_key(day: &str, model: &str, project: &str, fast: bool) -> String {
    format!("{day}|{model}|{project}|{}", if fast { 'f' } else { 's' })
}

pub fn usage_record_of(rec: &Line, fallback_project: &str) -> Option<Record> {
    if rec.kind.as_deref() != Some("assistant") {
        return None;
    }
    let msg = rec.message.as_ref()?;
    let model = msg.model.as_deref().filter(|m| !m.is_empty() && *m != "<synthetic>")?;
    let id = msg.id.as_deref().filter(|i| !i.is_empty())?;
    let u = msg.usage.as_ref()?;
    let (day, hour) = local_day_hour(parse_iso(rec.timestamp.as_deref()?)?);
    let write_1h = u.split.as_ref().map_or(0.0, |s| s.e1h);
    let write_5m = match &u.split {
        Some(s) if s.e5m != 0.0 => s.e5m,
        Some(_) => (u.creation - write_1h).max(0.0),
        None => u.creation,
    };
    let project = rec.cwd.as_deref().filter(|c| !c.is_empty()).unwrap_or(fallback_project).to_owned();
    Some(Record {
        key: bucket_key(&day, model, &project, u.fast),
        day,
        project,
        message_id: id.to_owned(),
        hour,
        totals: Totals { messages: 1.0, input: u.input, output: u.output, cache_write_5m: write_5m, cache_write_1h: write_1h, cache_read: u.read },
    })
}

const NAME_MAX: usize = 100;

fn js_space(c: char) -> bool {
    matches!(c, '\u{9}'..='\u{d}' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}')
}

pub fn clean_name(v: Option<&str>) -> Option<String> {
    let raw: String = v?.chars().filter(|&c| c != '|' && c > '\u{1f}').collect();
    let trimmed = raw.trim_matches(js_space);
    let mut units = 0;
    let mut out = String::new();
    for c in trimmed.chars() {
        units += c.len_utf16();
        if units > NAME_MAX {
            break;
        }
        out.push(c);
    }
    (!out.is_empty()).then_some(out)
}

pub fn mcp_server_of(tool: &str) -> Option<&str> {
    let rest = tool.strip_prefix("mcp__")?;
    let end = rest.find("__")?;
    (end > 0).then(|| &rest[..end])
}

const AGENT_TOOLS: [&str; 2] = ["Agent", "Task"];

pub fn tool_calls_of(block: &Block) -> Vec<(char, String)> {
    if block.kind.as_deref() != Some("tool_use") {
        return Vec::new();
    }
    let Some(name) = clean_name(block.name.as_deref()) else { return Vec::new() };
    if let Some(server) = mcp_server_of(&name) {
        return vec![('m', server.to_owned())];
    }
    let input = block.input.as_ref();
    let extra = if name == "Skill" {
        clean_name(input.and_then(|i| i.skill.as_deref())).map(|s| ('s', s))
    } else if AGENT_TOOLS.contains(&name.as_str()) {
        Some(('a', clean_name(input.and_then(|i| i.subagent_type.as_deref())).unwrap_or_else(|| "general-purpose".into())))
    } else {
        None
    };
    let mut out = vec![('t', name)];
    out.extend(extra);
    out
}

pub fn tool_key(day: &str, kind: char, name: &str, project: &str) -> String {
    format!("{day}|{kind}|{name}|{project}")
}
