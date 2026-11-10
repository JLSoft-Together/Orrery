pub mod aliases;
pub mod collate;
pub mod daystart;
pub mod datadir;
pub mod favorites;
pub mod forecast;
pub mod history;
pub mod json_file;
pub mod jsstr;
pub mod localtime;
pub mod notify_file;
pub mod paths;
pub mod response;
pub mod snooze;
pub mod timeline;

pub type Clock = Box<dyn Fn() -> i64 + Send>;

pub fn system_clock() -> Clock {
    Box::new(crate::time::now_ms)
}
