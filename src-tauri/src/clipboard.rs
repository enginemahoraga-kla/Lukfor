use serde::Serialize;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const MAX_CLIPS: usize = 50;
const MAX_CLIP_LEN: usize = 10_000;

#[derive(Clone, Serialize)]
pub struct Clip {
    pub text: String,
    pub at: u64,
}

pub type SharedClips = Arc<Mutex<VecDeque<Clip>>>;

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn remember(clips: &SharedClips, text: &str) {
    let trimmed = text.trim();
    if trimmed.is_empty() || trimmed.len() > MAX_CLIP_LEN {
        return;
    }
    let mut q = clips.lock().unwrap();
    if let Some(pos) = q.iter().position(|c| c.text == trimmed) {
        q.remove(pos);
    }
    q.push_front(Clip {
        text: trimmed.to_string(),
        at: now(),
    });
    q.truncate(MAX_CLIPS);
}

/// Poll the OS clipboard for new text every 800 ms.
pub fn spawn_watcher(clips: SharedClips) {
    std::thread::spawn(move || {
        let mut last: Option<String> = None;
        loop {
            if let Ok(mut cb) = arboard::Clipboard::new() {
                if let Ok(text) = cb.get_text() {
                    if last.as_deref() != Some(text.as_str()) {
                        remember(&clips, &text);
                        last = Some(text);
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(800));
        }
    });
}
