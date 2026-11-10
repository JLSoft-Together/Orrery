use super::{Doc, Link, MtError};
use std::collections::VecDeque;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::Mutex;

pub const PART_SIZE: u64 = 512 * 1024;
pub const MAX_PARTS: u64 = 4000;
pub const WORKERS: usize = 4;
pub const PART_TRIES: u32 = 5;
pub const FLOOD_MAX_S: u64 = 60;
pub const MISSING_TRIES: u32 = 3;
pub const SEND_TRIES: u32 = 3;

pub fn total_parts(size: u64) -> Option<i32> {
    if size == 0 {
        return None;
    }
    let n = size.div_ceil(PART_SIZE);
    (n <= MAX_PARTS).then_some(n as i32)
}

pub fn part_range(size: u64, part: i32) -> (u64, usize) {
    let start = part as u64 * PART_SIZE;
    (start, size.saturating_sub(start).min(PART_SIZE) as usize)
}

pub fn backoff_ms(tries: u32) -> u64 {
    (1000u64 << tries.saturating_sub(1).min(3)).min(8000)
}

pub struct Parts {
    pub total: i32,
    done: Vec<bool>,
}

impl Parts {
    pub fn new(total: i32) -> Self {
        Parts { total, done: vec![false; total.max(0) as usize] }
    }

    pub fn missing(&self) -> Vec<i32> {
        self.done.iter().enumerate().filter(|(_, d)| !**d).map(|(i, _)| i as i32).collect()
    }

    pub fn done_count(&self) -> usize {
        self.done.iter().filter(|d| **d).count()
    }

    fn mark(&mut self, part: i32, done: bool) {
        if let Some(d) = self.done.get_mut(part as usize) {
            *d = done;
        }
    }
}

pub struct Job<'a> {
    pub link: &'a dyn Link,
    pub file: &'a Path,
    pub size: u64,
    pub file_id: i64,
    pub sleep: &'a (dyn Fn(u64) + Sync),
    pub progress: &'a (dyn Fn(f64) + Sync),
}

fn read_part(f: &mut std::fs::File, size: u64, part: i32) -> Result<Vec<u8>, MtError> {
    let (start, len) = part_range(size, part);
    let mut buf = vec![0u8; len];
    f.seek(SeekFrom::Start(start)).map_err(|_| MtError::Other("read"))?;
    f.read_exact(&mut buf).map_err(|_| MtError::Other("read"))?;
    Ok(buf)
}

fn wait_for(e: &MtError, tries: u32, max: u32) -> Option<u64> {
    if tries >= max {
        return None;
    }
    if let Some(s) = e.flood_wait() {
        return (s <= FLOOD_MAX_S).then_some(s * 1000);
    }
    e.transient().then(|| backoff_ms(tries))
}

fn send_part(job: &Job, part: i32, total: i32, bytes: Vec<u8>) -> Result<(), MtError> {
    let mut tries = 0;
    loop {
        match job.link.save_part(job.file_id, part, total, bytes.clone()) {
            Ok(()) => return Ok(()),
            Err(e) => {
                tries += 1;
                match wait_for(&e, tries, PART_TRIES) {
                    Some(ms) => (job.sleep)(ms),
                    None => return Err(e),
                }
            }
        }
    }
}

pub fn upload_parts(job: &Job, parts: &Mutex<Parts>, only: Vec<i32>) -> Result<(), MtError> {
    let total = parts.lock().unwrap_or_else(|e| e.into_inner()).total;
    let queue = Mutex::new(only.into_iter().collect::<VecDeque<i32>>());
    let failed: Mutex<Option<MtError>> = Mutex::new(None);
    let workers = WORKERS.min(queue.lock().unwrap_or_else(|e| e.into_inner()).len()).max(1);
    std::thread::scope(|s| {
        for _ in 0..workers {
            s.spawn(|| {
                let mut file = match std::fs::File::open(job.file) {
                    Ok(f) => f,
                    Err(_) => {
                        failed.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert(MtError::Other("read"));
                        return;
                    }
                };
                loop {
                    if failed.lock().unwrap_or_else(|e| e.into_inner()).is_some() {
                        return;
                    }
                    let Some(part) = queue.lock().unwrap_or_else(|e| e.into_inner()).pop_front() else { return };
                    let sent = read_part(&mut file, job.size, part).and_then(|b| send_part(job, part, total, b));
                    match sent {
                        Ok(()) => {
                            let fraction = {
                                let mut p = parts.lock().unwrap_or_else(|e| e.into_inner());
                                p.mark(part, true);
                                p.done_count() as f64 / total.max(1) as f64
                            };
                            (job.progress)(fraction);
                        }
                        Err(e) => {
                            failed.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert(e);
                            return;
                        }
                    }
                }
            });
        }
    });
    match failed.into_inner().unwrap_or_else(|e| e.into_inner()) {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

pub fn run(job: &Job, parts: &Mutex<Parts>, doc: &Doc) -> Result<(), MtError> {
    let missing = parts.lock().unwrap_or_else(|e| e.into_inner()).missing();
    if !missing.is_empty() {
        upload_parts(job, parts, missing)?;
    }
    let (mut gaps, mut tries) = (0, 0);
    loop {
        match job.link.send_document(doc) {
            Ok(()) => return Ok(()),
            Err(e) if e.is("RANDOM_ID_DUPLICATE") => return Ok(()),
            Err(e) if e.is("FILE_PART_MISSING") && gaps < MISSING_TRIES => {
                gaps += 1;
                let part = e.value().map(|v| v as i32).filter(|p| *p < doc.parts).ok_or(e)?;
                parts.lock().unwrap_or_else(|e| e.into_inner()).mark(part, false);
                upload_parts(job, parts, vec![part])?;
            }
            Err(e) => {
                tries += 1;
                match wait_for(&e, tries, SEND_TRIES) {
                    Some(ms) => (job.sleep)(ms),
                    None => return Err(e),
                }
            }
        }
    }
}
