use crate::disk_text::{format_bytes, Box3};

pub struct MoveAsk {
    pub kind: &'static str,
    pub source: String,
    pub target: String,
    pub bytes: u64,
}

pub fn move_box(a: &MoveAsk, locale: &str) -> Box3 {
    let vi = locale.to_lowercase().starts_with("vi");
    let size = format_bytes(a.bytes);
    let (s, t) = (&a.source, &a.target);
    let (message, detail, ok) = match (a.kind, vi) {
        ("move", false) => (format!("Move {size} to another drive?"), format!("From: {s}\nTo: {t}\n\nOrrery copies, checks the copy, leaves a link at the old place, then sends the old copy to the Recycle Bin. Close the tools that use this folder first."), "Move"),
        ("move", true) => (format!("Chuyển {size} sang ổ khác?"), format!("Từ: {s}\nSang: {t}\n\nOrrery copy, kiểm bản copy, để lại link ở chỗ cũ rồi đưa bản cũ vào Thùng rác. Đóng các tool đang dùng thư mục này trước."), "Chuyển"),
        ("back", false) => (format!("Move {size} back?"), format!("From: {t}\nTo: {s}\n\nOrrery copies the folder back, removes the link, then deletes the copy on the other drive."), "Move back"),
        ("back", true) => (format!("Chuyển {size} về chỗ cũ?"), format!("Từ: {t}\nVề: {s}\n\nOrrery copy thư mục về, gỡ link rồi xoá bản ở ổ kia."), "Chuyển về"),
        ("permanent", false) => ("The old copy cannot go to the Recycle Bin".to_owned(), format!("{t}\n\nThe folder already lives on the other drive behind the link. Delete the old copy for good?"), "Delete for good"),
        ("permanent", true) => ("Bản cũ không vào được Thùng rác".to_owned(), format!("{t}\n\nThư mục đã nằm ở ổ kia sau link. Xoá hẳn bản cũ?"), "Xoá hẳn"),
        ("restore", false) => ("Restore the interrupted move?".to_owned(), format!("{s}\n\nOrrery puts the folder back where it was and deletes the unfinished copy at {t}."), "Restore"),
        ("restore", true) => ("Khôi phục lần chuyển bị dừng?".to_owned(), format!("{s}\n\nOrrery đưa thư mục về chỗ cũ và xoá bản copy dở ở {t}."), "Khôi phục"),
        (_, false) => ("Finish the move?".to_owned(), format!("{s}\n\nThe folder works from {t}. Orrery removes what is left of the old copy."), "Finish"),
        (_, true) => ("Làm nốt lần chuyển?".to_owned(), format!("{s}\n\nThư mục đang chạy từ {t}. Orrery xoá phần còn lại của bản cũ."), "Làm nốt"),
    };
    Box3 { message, detail, ok: ok.to_owned(), cancel: if vi { "Huỷ" } else { "Cancel" }.to_owned() }
}
