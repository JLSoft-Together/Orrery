use std::fs::{File, Metadata};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::SystemTime;

pub const MAX_HELD: usize = 2048;

static HELD: AtomicUsize = AtomicUsize::new(0);

pub fn held_count() -> usize {
    HELD.load(Ordering::Relaxed)
}

pub fn keep_default() -> bool {
    cfg!(windows)
}

pub struct Held {
    file: File,
    created: Option<SystemTime>,
    #[cfg(unix)]
    ino: (u64, u64),
}

impl Drop for Held {
    fn drop(&mut self) {
        HELD.fetch_sub(1, Ordering::Relaxed);
    }
}

#[cfg(unix)]
fn ino_of(m: &Metadata) -> (u64, u64) {
    use std::os::unix::fs::MetadataExt;
    (m.dev(), m.ino())
}

impl Held {
    fn new(file: File, m: &Metadata) -> Result<Held, File> {
        if HELD.fetch_add(1, Ordering::Relaxed) >= MAX_HELD {
            HELD.fetch_sub(1, Ordering::Relaxed);
            return Err(file);
        }
        Ok(Held {
            file,
            created: m.created().ok(),
            #[cfg(unix)]
            ino: ino_of(m),
        })
    }

    fn same(&self, path: &Metadata, own: &Metadata) -> bool {
        #[cfg(unix)]
        if ino_of(path) != self.ino {
            return false;
        }
        self.created.is_some() && path.created().ok() == self.created && path.len() == own.len() && path.is_file()
    }

    pub fn file(&self) -> &File {
        &self.file
    }
}

pub enum Opened {
    Held,
    Fresh(File),
}

pub fn open(path: &Path, held: &mut Option<Held>, keep: bool) -> Option<(Opened, u64)> {
    if let Some(h) = held.as_ref() {
        if keep {
            if let (Ok(p), Ok(own)) = (std::fs::metadata(path), h.file.metadata()) {
                if h.same(&p, &own) {
                    return Some((Opened::Held, own.len()));
                }
            }
        }
        *held = None;
    }
    let file = File::open(path).ok()?;
    let m = file.metadata().ok()?;
    let len = m.len();
    if !keep {
        return Some((Opened::Fresh(file), len));
    }
    match Held::new(file, &m) {
        Ok(h) => {
            *held = Some(h);
            Some((Opened::Held, len))
        }
        Err(file) => Some((Opened::Fresh(file), len)),
    }
}
