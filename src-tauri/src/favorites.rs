//! Entries the user pinned. They live in their own small file rather than in
//! the index: the index is rebuilt from disk all the time, pins are the user's.

use crate::indexer::Kind;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Favorite {
    pub name: String,
    pub path: String,
    pub kind: Kind,
}

pub struct Favorites {
    items: Vec<Favorite>,
    file: PathBuf,
}

impl Favorites {
    /// `%LOCALAPPDATA%\Lukfor\favorites.json`, next to the log.
    pub fn load_default() -> Self {
        let dir = dirs::data_local_dir().unwrap_or_else(std::env::temp_dir);
        Self::load(dir.join("Lukfor").join("favorites.json"))
    }

    pub fn load(file: PathBuf) -> Self {
        let items = match std::fs::read(&file) {
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|e| {
                // Set the unreadable file aside: starting empty and saving
                // over it on the next pin would silently lose every pin.
                let backup = file.with_extension("json.corrupt");
                let moved = std::fs::rename(&file, &backup).is_ok();
                crate::log_line(&format!(
                    "favorites: unreadable ({e}); {}",
                    if moved {
                        format!("kept as {}", backup.display())
                    } else {
                        "could not move it aside".to_string()
                    }
                ));
                Vec::new()
            }),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Vec::new(),
            Err(e) => {
                crate::log_line(&format!("favorites: could not read ({e})"));
                Vec::new()
            }
        };
        Self { items, file }
    }

    pub fn items(&self) -> &[Favorite] {
        &self.items
    }

    pub fn paths(&self) -> HashSet<String> {
        self.items.iter().map(|f| f.path.clone()).collect()
    }

    pub fn contains(&self, path: &str) -> bool {
        self.items.iter().any(|f| f.path == path)
    }

    /// Pinned, and pinned as an app. Pins were checked against the index when
    /// they were made, so this vouches for a path while the index is still
    /// being built (the favorites list asks for icons right at startup).
    pub fn is_pinned_app(&self, path: &str) -> bool {
        self.items.iter().any(|f| f.path == path && f.kind == Kind::App)
    }

    /// New pins go to the end, so the ones already there keep their place —
    /// and the number of arrow presses it takes to reach them.
    pub fn pin(&mut self, fav: Favorite) -> io::Result<()> {
        if self.contains(&fav.path) {
            return Ok(());
        }
        let next = self.items.iter().cloned().chain([fav]).collect();
        self.commit(next)
    }

    pub fn unpin(&mut self, path: &str) -> io::Result<()> {
        let next = self
            .items
            .iter()
            .filter(|f| f.path != path)
            .cloned()
            .collect();
        self.commit(next)
    }

    /// Point the pin at `from` to `to` instead, in the same position. Used when
    /// the copy a pin pointed at was dropped as a duplicate but another copy
    /// of the same app is still indexed.
    pub fn repoint(&mut self, from: &str, to: &str) -> io::Result<()> {
        if self.contains(to) {
            return self.unpin(from);
        }
        let next = self
            .items
            .iter()
            .map(|f| match f.path == from {
                true => Favorite { path: to.to_string(), ..f.clone() },
                false => f.clone(),
            })
            .collect();
        self.commit(next)
    }

    /// Save first, adopt second: if the write fails, what's in memory still
    /// matches what's on disk.
    fn commit(&mut self, next: Vec<Favorite>) -> io::Result<()> {
        save(&self.file, &next)?;
        self.items = next;
        Ok(())
    }
}

fn save(file: &Path, items: &[Favorite]) -> io::Result<()> {
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let json = serde_json::to_vec_pretty(items).map_err(io::Error::other)?;
    // Write beside it, then swap in, so a crash mid-write can't leave half a file.
    let tmp = file.with_extension("json.tmp");
    std::fs::write(&tmp, json)?;
    std::fs::rename(&tmp, file)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh, empty directory per test so runs can't see each other's pins.
    fn scratch(test: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("lukfor-fav-{test}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("favorites.json")
    }

    fn fav(path: &str) -> Favorite {
        Favorite {
            name: path.rsplit('\\').next().unwrap().to_string(),
            path: path.to_string(),
            kind: Kind::App,
        }
    }

    #[test]
    fn a_missing_file_means_no_favorites_yet() {
        let favs = Favorites::load(scratch("missing"));
        assert!(favs.items().is_empty());
    }

    #[test]
    fn pins_survive_a_restart_and_unpins_do_too() {
        let file = scratch("roundtrip");

        Favorites::load(file.clone()).pin(fav("C:\\a\\Steam")).unwrap();
        assert!(Favorites::load(file.clone()).contains("C:\\a\\Steam"));

        Favorites::load(file.clone()).unpin("C:\\a\\Steam").unwrap();
        assert!(!Favorites::load(file).contains("C:\\a\\Steam"));
    }

    #[test]
    fn pins_keep_the_order_they_were_added_in() {
        let file = scratch("order");
        let mut favs = Favorites::load(file.clone());
        for p in ["C:\\a\\one", "C:\\a\\two", "C:\\a\\three"] {
            favs.pin(fav(p)).unwrap();
        }

        let reloaded: Vec<String> = Favorites::load(file)
            .items()
            .iter()
            .map(|f| f.path.clone())
            .collect();

        assert_eq!(reloaded, ["C:\\a\\one", "C:\\a\\two", "C:\\a\\three"]);
    }

    #[test]
    fn pinning_the_same_entry_twice_keeps_one() {
        let mut favs = Favorites::load(scratch("twice"));
        favs.pin(fav("C:\\a\\Steam")).unwrap();
        favs.pin(fav("C:\\a\\Steam")).unwrap();
        assert_eq!(favs.items().len(), 1);
    }

    #[test]
    fn only_app_pins_count_as_pinned_apps() {
        let mut favs = Favorites::load(scratch("pinned-app"));
        favs.pin(fav("C:\\a\\Steam")).unwrap();
        favs.pin(Favorite {
            name: "notes".into(),
            path: "C:\\a\\notes".into(),
            kind: Kind::Folder,
        })
        .unwrap();

        assert!(favs.is_pinned_app("C:\\a\\Steam"));
        assert!(!favs.is_pinned_app("C:\\a\\notes"), "a folder pin is not an app");
        assert!(!favs.is_pinned_app("C:\\a\\never-pinned"));
    }

    #[test]
    fn a_pin_can_be_moved_to_another_copy_and_keeps_its_place() {
        let file = scratch("repoint");
        let mut favs = Favorites::load(file.clone());
        for p in ["C:\\a\\one", "C:\\backup\\Steam", "C:\\a\\three"] {
            favs.pin(fav(p)).unwrap();
        }

        favs.repoint("C:\\backup\\Steam", "C:\\a\\Steam").unwrap();

        let paths: Vec<String> = Favorites::load(file)
            .items()
            .iter()
            .map(|f| f.path.clone())
            .collect();
        assert_eq!(paths, ["C:\\a\\one", "C:\\a\\Steam", "C:\\a\\three"]);
    }

    #[test]
    fn an_unreadable_file_is_set_aside_not_overwritten() {
        let file = scratch("corrupt");
        std::fs::write(&file, b"{ not json").unwrap();

        let mut favs = Favorites::load(file.clone());
        assert!(favs.items().is_empty());
        favs.pin(fav("C:\\a\\Steam")).unwrap();

        let kept = std::fs::read(file.with_extension("json.corrupt")).unwrap();
        assert_eq!(kept, b"{ not json", "the original bytes must survive");
    }
}
