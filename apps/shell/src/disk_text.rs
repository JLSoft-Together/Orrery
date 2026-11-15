pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut v = bytes as f64;
    let mut u = 0;
    while v >= 1024.0 && u < UNITS.len() - 1 {
        v /= 1024.0;
        u += 1;
    }
    if u == 0 {
        return format!("{bytes} B");
    }
    if v >= 100.0 {
        format!("{} {}", v.round() as u64, UNITS[u])
    } else {
        format!("{v:.1} {}", UNITS[u])
    }
}

pub struct Ask<'a> {
    pub count: usize,
    pub bytes: u64,
    pub first: &'a [String],
    pub permanent: bool,
}

pub struct Box3 {
    pub message: String,
    pub detail: String,
    pub ok: String,
    pub cancel: String,
}

fn vi(locale: &str) -> bool {
    !locale.starts_with("en")
}

fn items_en(n: usize) -> String {
    if n == 1 {
        "1 item".into()
    } else {
        format!("{n} items")
    }
}

fn list(first: &[String], more: usize, vi: bool) -> String {
    let tail = if more == 0 {
        String::new()
    } else if vi {
        format!("\n…và {more} mục nữa")
    } else {
        format!("\n…and {more} more")
    };
    format!("{}{tail}", first.join("\n"))
}

pub fn delete_box(ask: &Ask, second: bool, locale: &str, mac: bool) -> Box3 {
    let v = vi(locale);
    let size = format_bytes(ask.bytes);
    let cancel = if v { "Huỷ" } else { "Cancel" }.to_owned();
    let delete_ok = if v { "Xoá hẳn" } else { "Delete permanently" }.to_owned();
    if second {
        let (message, detail) = if v {
            ("Không khôi phục được".to_owned(), format!("{} mục ({size}) sẽ bị xoá vĩnh viễn, không vào Thùng rác.", ask.count))
        } else {
            let bin = if mac { "They do not go to the Trash." } else { "They do not go to the Recycle Bin." };
            ("This cannot be undone".to_owned(), format!("{} ({size}) will be deleted for good. {bin}", items_en(ask.count)))
        };
        return Box3 { message, detail, ok: delete_ok, cancel };
    }
    let message = match (v, ask.permanent) {
        (true, true) => format!("Xoá hẳn {} mục?", ask.count),
        (true, false) => format!("Chuyển {} mục vào Thùng rác?", ask.count),
        (false, true) => format!("Delete {} permanently?", items_en(ask.count)),
        (false, false) => format!("Move {} to the {}?", items_en(ask.count), if mac { "Trash" } else { "Recycle Bin" }),
    };
    let more = ask.count.saturating_sub(ask.first.len());
    let detail = if v { format!("Tổng: {size}\n\n{}", list(ask.first, more, true)) } else { format!("Total: {size}\n\n{}", list(ask.first, more, false)) };
    let ok = match (ask.permanent, v) {
        (true, _) => delete_ok,
        (false, true) => "Chuyển vào Thùng rác".to_owned(),
        (false, false) => if mac { "Move to Trash" } else { "Move to Recycle Bin" }.to_owned(),
    };
    Box3 { message, detail, ok, cancel }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_read_like_the_electron_dialog() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(3_000_000), "2.9 MB");
        assert_eq!(format_bytes(200 * 1024 * 1024), "200 MB");
    }

    #[test]
    fn the_trash_box_lists_the_first_paths_and_the_rest_count() {
        let first = vec!["C:\\a".to_owned(), "C:\\b".to_owned()];
        let b = delete_box(&Ask { count: 5, bytes: 2048, first: &first, permanent: false }, false, "en", false);
        assert_eq!(b.message, "Move 5 items to the Recycle Bin?");
        assert_eq!(b.detail, "Total: 2.0 KB\n\nC:\\a\nC:\\b\n…and 3 more");
        assert_eq!(b.ok, "Move to Recycle Bin");
        let v = delete_box(&Ask { count: 1, bytes: 10, first: &first[..1], permanent: true }, true, "vi", false);
        assert_eq!(v.message, "Không khôi phục được");
        assert_eq!(v.ok, "Xoá hẳn");
    }
}
