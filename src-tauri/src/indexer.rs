use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use serde::Serialize;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use walkdir::WalkDir;

const MAX_ENTRIES: usize = 250_000;
const MAX_DEPTH: usize = 8;
const SKIP_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    "target",
    "__pycache__",
    ".venv",
    "venv",
    ".cache",
    "AppData",
    "$RECYCLE.BIN",
    "System Volume Information",
];

#[derive(Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    App,
    File,
    Folder,
}

pub struct Entry {
    pub name: String,
    pub name_lower: String,
    pub path: String,
    pub kind: Kind,
}

#[derive(Serialize)]
pub struct SearchResult {
    pub name: String,
    pub path: String,
    pub kind: Kind,
    pub score: i64,
}

pub type SharedIndex = Arc<RwLock<Vec<Entry>>>;

fn push_entry(out: &mut Vec<Entry>, name: &str, path: &Path, kind: Kind) {
    out.push(Entry {
        name: name.to_string(),
        name_lower: name.to_lowercase(),
        path: path.to_string_lossy().to_string(),
        kind,
    });
}

/// Start Menu shortcuts (per-user + all-users) become "app" entries.
fn index_apps(out: &mut Vec<Entry>) {
    let mut roots: Vec<std::path::PathBuf> = Vec::new();
    if let Ok(appdata) = std::env::var("APPDATA") {
        roots.push(Path::new(&appdata).join("Microsoft/Windows/Start Menu/Programs"));
    }
    if let Ok(progdata) = std::env::var("ProgramData") {
        roots.push(Path::new(&progdata).join("Microsoft/Windows/Start Menu/Programs"));
    }
    for root in roots {
        for e in WalkDir::new(&root)
            .max_depth(4)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let p = e.path();
            let ext = p
                .extension()
                .map(|x| x.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            if e.file_type().is_file() && (ext == "lnk" || ext == "url") {
                if let Some(stem) = p.file_stem() {
                    let name = stem.to_string_lossy();
                    // Skip uninstallers and website links that clutter results
                    let lower = name.to_lowercase();
                    if lower.contains("uninstall") {
                        continue;
                    }
                    push_entry(out, &name, p, Kind::App);
                }
            }
        }
    }
}

/// Store/UWP apps (WhatsApp, Photos, Calculator, …) have no Start Menu
/// .lnk file — they live in the shell "AppsFolder" namespace. `Get-StartApps`
/// is the authoritative list Windows itself shows; we take the packaged ones
/// (their AUMID contains '!') and later launch them via `shell:AppsFolder\<AUMID>`.
#[cfg(windows)]
fn index_uwp_apps(out: &mut Vec<Entry>) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let output = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Get-StartApps | ConvertTo-Json -Compress",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output();
    let Ok(output) = output else {
        crate::log_line("uwp: powershell Get-StartApps failed to run");
        return;
    };
    if !output.status.success() {
        return;
    }

    #[derive(serde::Deserialize)]
    struct StartApp {
        #[serde(rename = "Name")]
        name: String,
        #[serde(rename = "AppID")]
        app_id: String,
    }
    let apps: Vec<StartApp> = match serde_json::from_slice(&output.stdout) {
        Ok(a) => a,
        Err(e) => {
            crate::log_line(&format!("uwp: could not parse Get-StartApps json: {e}"));
            return;
        }
    };

    // Don't re-add anything already covered by a Start Menu shortcut.
    let existing: std::collections::HashSet<String> =
        out.iter().map(|e| e.name_lower.clone()).collect();
    let mut added = 0usize;
    for app in apps {
        // Packaged apps have AUMIDs of the form "PackageFamily!AppId"; the
        // plain GUID / "*.EXE.15" entries are control-panel shims, not apps.
        if !app.app_id.contains('!') {
            continue;
        }
        let name = app.name.trim();
        if name.is_empty() {
            continue;
        }
        let name_lower = name.to_lowercase();
        if !existing.contains(&name_lower) {
            out.push(Entry {
                name: name.to_string(),
                name_lower,
                path: format!("shell:AppsFolder\\{}", app.app_id),
                kind: Kind::App,
            });
            added += 1;
        }
    }
    crate::log_line(&format!("uwp: added {added} Store/packaged apps"));
}

#[cfg(not(windows))]
fn index_uwp_apps(_out: &mut Vec<Entry>) {}

/// User folders become file/folder entries, bounded by depth and count.
fn index_files(index: &SharedIndex, total_apps: usize) {
    let mut roots: Vec<std::path::PathBuf> = Vec::new();
    for d in [
        dirs::desktop_dir(),
        dirs::document_dir(),
        dirs::download_dir(),
        dirs::picture_dir(),
        dirs::video_dir(),
        dirs::audio_dir(),
    ]
    .into_iter()
    .flatten()
    {
        roots.push(d);
    }
    // Top-level folders of the home dir not already covered / skipped.
    if let Some(home) = dirs::home_dir() {
        if let Ok(rd) = std::fs::read_dir(&home) {
            for e in rd.filter_map(|e| e.ok()) {
                let p = e.path();
                let name = e.file_name().to_string_lossy().to_string();
                if p.is_dir()
                    && !SKIP_DIRS.iter().any(|s| s.eq_ignore_ascii_case(&name))
                    && !name.starts_with('.')
                    && !roots.contains(&p)
                {
                    roots.push(p);
                }
            }
        }
    }

    let mut count = total_apps;
    let mut batch: Vec<Entry> = Vec::with_capacity(4096);
    for root in roots {
        if count >= MAX_ENTRIES {
            break;
        }
        let walker = WalkDir::new(&root)
            .max_depth(MAX_DEPTH)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                !(e.file_type().is_dir()
                    && (name.starts_with('.')
                        || SKIP_DIRS.iter().any(|s| s.eq_ignore_ascii_case(&name))))
            });
        for e in walker.filter_map(|e| e.ok()).skip(1) {
            if count >= MAX_ENTRIES {
                break;
            }
            let p = e.path();
            let Some(fname) = p.file_name() else { continue };
            let mut name = fname.to_string_lossy().to_string();
            let kind = if e.file_type().is_dir() {
                Kind::Folder
            } else {
                let ext = p
                    .extension()
                    .map(|x| x.to_string_lossy().to_lowercase())
                    .unwrap_or_default();
                // Launchables outside the Start Menu (portable apps, .exe in
                // Downloads, desktop shortcuts) count as apps too.
                if (ext == "exe" || ext == "lnk")
                    && !name.to_lowercase().contains("unins")
                {
                    if let Some(stem) = p.file_stem() {
                        name = stem.to_string_lossy().to_string();
                    }
                    Kind::App
                } else {
                    Kind::File
                }
            };
            batch.push(Entry {
                name_lower: name.to_lowercase(),
                name,
                path: p.to_string_lossy().to_string(),
                kind,
            });
            count += 1;
            if batch.len() >= 4096 {
                index.write().unwrap().append(&mut batch);
            }
        }
    }
    if !batch.is_empty() {
        index.write().unwrap().append(&mut batch);
    }
}

/// Build (or rebuild) the index: apps first (instant results), then the
/// larger file walk. Runs on the calling thread.
fn build_index(index: &SharedIndex, indexing: &AtomicBool) {
    indexing.store(true, Ordering::SeqCst);
    let mut apps = Vec::new();
    index_apps(&mut apps);
    index_uwp_apps(&mut apps);
    let n_apps = apps.len();
    {
        let mut w = index.write().unwrap();
        w.clear();
        w.append(&mut apps);
    }
    let t0 = std::time::Instant::now();
    index_files(index, n_apps);
    indexing.store(false, Ordering::SeqCst);
    crate::log_line(&format!(
        "index ready: {} apps, {} total entries in {:.1}s",
        n_apps,
        index.read().unwrap().len(),
        t0.elapsed().as_secs_f32()
    ));
}

/// Build the index on a background thread at startup.
pub fn spawn_indexer(index: SharedIndex, indexing: Arc<AtomicBool>) {
    std::thread::spawn(move || build_index(&index, &indexing));
}

fn next_run_after(now: chrono::NaiveDateTime) -> chrono::NaiveDateTime {
    let three_am = now.date().and_hms_opt(3, 0, 0).unwrap();
    if now < three_am {
        three_am
    } else {
        three_am + chrono::Duration::days(1)
    }
}

/// Rebuild the index every day at 03:00 local time while the app runs.
/// Polling every minute (instead of one long sleep) also covers sleep or
/// hibernate across 03:00 — the rebuild then fires right after wake-up.
pub fn spawn_scheduler(index: SharedIndex, indexing: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let mut next_run = next_run_after(chrono::Local::now().naive_local());
        loop {
            std::thread::sleep(std::time::Duration::from_secs(60));
            let now = chrono::Local::now().naive_local();
            if now >= next_run {
                if !indexing.load(Ordering::SeqCst) {
                    crate::log_line("scheduled 03:00 reindex starting");
                    build_index(&index, &indexing);
                }
                next_run = next_run_after(now);
            }
        }
    });
}

pub fn search(index: &SharedIndex, query: &str, limit: usize) -> Vec<SearchResult> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return Vec::new();
    }
    let matcher = SkimMatcherV2::default();
    let idx = index.read().unwrap();

    let mut hits: Vec<(i64, &Entry)> = Vec::new();
    for e in idx.iter() {
        // Cheap pre-filter for single-char queries: prefix only.
        if q.len() == 1 && !e.name_lower.starts_with(&q) {
            continue;
        }
        if let Some(mut score) = matcher.fuzzy_match(&e.name_lower, &q) {
            if e.kind == Kind::App {
                score += 120;
            } else if e.kind == Kind::Folder {
                score += 15;
            }
            if e.name_lower.starts_with(&q) {
                score += 100;
            } else if e.name_lower.contains(&q) {
                score += 40;
            }
            // Shorter names that match are usually what the user wants.
            score -= (e.name.len() as i64).min(60) / 4;
            hits.push((score, e));
        }
    }
    hits.sort_unstable_by(|a, b| b.0.cmp(&a.0));
    hits.truncate(limit);
    hits.into_iter()
        .map(|(score, e)| SearchResult {
            name: e.name.clone(),
            path: e.path.clone(),
            kind: e.kind,
            score,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic_index(n: usize) -> SharedIndex {
        let mut v = Vec::with_capacity(n);
        for i in 0..n {
            let name = format!("document-report-{i:06}.txt");
            v.push(Entry {
                name_lower: name.to_lowercase(),
                name,
                path: format!("C:\\Users\\test\\Documents\\d{}\\f{i}", i % 100),
                kind: if i % 50 == 0 { Kind::Folder } else { Kind::File },
            });
        }
        v.push(Entry {
            name: "Google Chrome".into(),
            name_lower: "google chrome".into(),
            path: "C:\\fake\\chrome.lnk".into(),
            kind: Kind::App,
        });
        Arc::new(RwLock::new(v))
    }

    #[test]
    fn search_ranks_apps_first() {
        let idx = synthetic_index(10_000);
        let res = search(&idx, "chrome", 20);
        assert!(!res.is_empty());
        assert_eq!(res[0].name, "Google Chrome");
    }

    #[test]
    fn search_is_fast_on_large_index() {
        let idx = synthetic_index(250_000);
        // warm-up
        search(&idx, "report 1234", 20);
        let t0 = std::time::Instant::now();
        for q in ["doc", "report-0999", "chrome", "zzz-no-match"] {
            search(&idx, q, 20);
        }
        let per_query = t0.elapsed() / 4;
        // Budget: a keystroke must feel instant even on 250k entries.
        assert!(
            per_query.as_millis() < 250,
            "search too slow: {per_query:?} per query"
        );
    }

    #[test]
    fn empty_query_returns_nothing() {
        let idx = synthetic_index(10);
        assert!(search(&idx, "  ", 20).is_empty());
    }
}
