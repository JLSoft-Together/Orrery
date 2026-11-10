use super::html::{Entity, Kind};
use super::{display_name, Doc, ForumChat, Peer, Topic, TopicPage};
use grammers_tl_types as tl;
use std::collections::HashMap;

pub fn mime_of(name: &str) -> &'static str {
    let ext = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "apk" => "application/vnd.android.package-archive",
        "zip" => "application/zip",
        "pdf" => "application/pdf",
        "mp4" => "video/mp4",
        "mov" => "video/quicktime",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "txt" | "log" => "text/plain",
        "json" => "application/json",
        "gz" | "tgz" => "application/gzip",
        _ => "application/octet-stream",
    }
}

fn entity(e: &Entity) -> tl::enums::MessageEntity {
    let (offset, length) = (e.offset, e.length);
    match e.kind {
        Kind::Bold => tl::types::MessageEntityBold { offset, length }.into(),
        Kind::Italic => tl::types::MessageEntityItalic { offset, length }.into(),
        Kind::Underline => tl::types::MessageEntityUnderline { offset, length }.into(),
        Kind::Strike => tl::types::MessageEntityStrike { offset, length }.into(),
        Kind::Code => tl::types::MessageEntityCode { offset, length }.into(),
        Kind::Pre => tl::types::MessageEntityPre { offset, length, language: String::new() }.into(),
        Kind::Url => tl::types::MessageEntityTextUrl { offset, length, url: e.url.clone().unwrap_or_default() }.into(),
        Kind::Quote => tl::types::MessageEntityBlockquote { collapsed: e.collapsed, offset, length }.into(),
        Kind::Spoiler => tl::types::MessageEntitySpoiler { offset, length }.into(),
    }
}

fn input_peer(p: Peer) -> tl::enums::InputPeer {
    match p {
        Peer::Channel { id, hash } => tl::types::InputPeerChannel { channel_id: id, access_hash: hash }.into(),
        Peer::Chat(id) => tl::types::InputPeerChat { chat_id: id }.into(),
    }
}

pub fn doc_request(doc: &Doc) -> tl::functions::messages::SendMedia {
    let media = tl::types::InputMediaUploadedDocument {
        nosound_video: false,
        force_file: true,
        spoiler: false,
        file: tl::types::InputFileBig { id: doc.file_id, parts: doc.parts, name: doc.name.clone() }.into(),
        thumb: None,
        mime_type: mime_of(&doc.name).to_string(),
        attributes: vec![tl::types::DocumentAttributeFilename { file_name: doc.name.clone() }.into()],
        stickers: None,
        video_cover: None,
        video_timestamp: None,
        ttl_seconds: None,
    };
    let reply_to = doc.thread.map(|t| {
        tl::types::InputReplyToMessage { reply_to_msg_id: t, top_msg_id: Some(t), reply_to_peer_id: None, quote_text: None, quote_entities: None, quote_offset: None, monoforum_peer_id: None, todo_item_id: None, poll_option: None, quote_rich: None }.into()
    });
    tl::functions::messages::SendMedia {
        silent: false,
        background: false,
        clear_draft: false,
        noforwards: false,
        update_stickersets_order: false,
        invert_media: false,
        allow_paid_floodskip: false,
        peer: input_peer(doc.peer),
        reply_to,
        media: media.into(),
        message: doc.caption.clone(),
        random_id: doc.random_id,
        reply_markup: None,
        entities: (!doc.entities.is_empty()).then(|| doc.entities.iter().map(entity).collect()),
        schedule_date: None,
        schedule_repeat_period: None,
        send_as: None,
        quick_reply_shortcut: None,
        effect: None,
        allow_paid_stars: None,
        suggested_post: None,
    }
}

pub fn entity_user(u: &tl::enums::User) -> String {
    match u {
        tl::enums::User::User(u) => display_name(u.first_name.as_deref(), u.last_name.as_deref(), u.username.as_deref()),
        tl::enums::User::Empty(_) => display_name(None, None, None),
    }
}

pub struct DialogPage {
    pub chats: Vec<ForumChat>,
    pub count: usize,
    pub next: Option<(i32, i32, tl::enums::InputPeer)>,
}

fn message_date(m: &tl::enums::Message) -> Option<(i32, i32)> {
    match m {
        tl::enums::Message::Message(x) => Some((x.id, x.date)),
        tl::enums::Message::Service(x) => Some((x.id, x.date)),
        tl::enums::Message::Empty(_) => None,
    }
}

pub fn forum_chats_page(d: tl::enums::messages::Dialogs) -> DialogPage {
    let (dialogs, messages, chats, users) = match d {
        tl::enums::messages::Dialogs::Dialogs(x) => (x.dialogs, x.messages, x.chats, x.users),
        tl::enums::messages::Dialogs::Slice(x) => (x.dialogs, x.messages, x.chats, x.users),
        tl::enums::messages::Dialogs::NotModified(_) => return DialogPage { chats: Vec::new(), count: 0, next: None },
    };
    let channels: HashMap<i64, tl::types::Channel> = chats
        .into_iter()
        .filter_map(|c| match c {
            tl::enums::Chat::Channel(ch) => Some((ch.id, ch)),
            _ => None,
        })
        .collect();
    let user_hash: HashMap<i64, i64> = users
        .iter()
        .filter_map(|u| match u {
            tl::enums::User::User(u) => Some((u.id, u.access_hash.unwrap_or(0))),
            _ => None,
        })
        .collect();
    let dates: Vec<(i32, i32)> = messages.iter().filter_map(message_date).collect();
    let mut out = Vec::new();
    let mut next = None;
    let count = dialogs.len();
    for d in &dialogs {
        let tl::enums::Dialog::Dialog(d) = d else { continue };
        let peer = match &d.peer {
            tl::enums::Peer::Channel(p) => {
                if let Some(ch) = channels.get(&p.channel_id) {
                    if ch.forum && ch.megagroup {
                        out.push(ForumChat { id: ch.id, hash: ch.access_hash.unwrap_or(0), title: ch.title.clone() });
                    }
                }
                let hash = channels.get(&p.channel_id).and_then(|c| c.access_hash).unwrap_or(0);
                tl::types::InputPeerChannel { channel_id: p.channel_id, access_hash: hash }.into()
            }
            tl::enums::Peer::Chat(p) => tl::types::InputPeerChat { chat_id: p.chat_id }.into(),
            tl::enums::Peer::User(p) => tl::types::InputPeerUser { user_id: p.user_id, access_hash: user_hash.get(&p.user_id).copied().unwrap_or(0) }.into(),
        };
        let date = dates.iter().find(|(id, _)| *id == d.top_message).map(|(_, at)| *at).unwrap_or(0);
        next = Some((date, d.top_message, peer));
    }
    DialogPage { chats: out, count, next }
}

pub fn topic_page(t: tl::enums::messages::ForumTopics) -> TopicPage {
    let tl::enums::messages::ForumTopics::Topics(t) = t;
    let raw_len = t.topics.len();
    let topics = t
        .topics
        .into_iter()
        .filter_map(|x| match x {
            tl::enums::ForumTopic::Topic(x) => Some(Topic { id: x.id, title: x.title, top_message: x.top_message }),
            tl::enums::ForumTopic::Deleted(_) => None,
        })
        .collect();
    TopicPage { topics, raw_len, dates: t.messages.iter().filter_map(message_date).collect() }
}
