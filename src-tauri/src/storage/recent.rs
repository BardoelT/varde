use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::file::write_atomic_with;

const FILE_NAME: &str = "recent-projects.json";

/// The single remembered path of versions before 3.5, read once (CSD-002 AC-31.6).
const LEGACY_FILE_NAME: &str = "last-project.json";

/// The settings file is tiny; anything bigger is not ours.
const MAX_BYTES: u64 = 64 * 1024;

/// How many paths the recent list keeps (CSD-002 REQ-31).
pub const MAX_RECENT: usize = 10;

#[derive(Serialize, Deserialize)]
struct Stored {
    paths: Vec<String>,
}

#[derive(Deserialize)]
struct Legacy {
    path: String,
}

/// The recent list: the paths of the last project files that were opened or
/// saved, newest first, kept in the app's own config folder so it can be read
/// before any project is open (CSD-002 REQ-31, CON-7). It holds paths only,
/// never project data. Its first entry is the project reopened at start.
#[derive(Debug, Clone)]
pub struct RecentProjects {
    file: PathBuf,
    legacy_file: PathBuf,
}

impl RecentProjects {
    /// The list file inside `config_dir`.
    #[must_use]
    pub fn in_dir(config_dir: &Path) -> Self {
        Self {
            file: config_dir.join(FILE_NAME),
            legacy_file: config_dir.join(LEGACY_FILE_NAME),
        }
    }

    /// The paths, newest first. An unreadable, oversized or malformed file
    /// counts as an empty list, and relative paths are left out. Without a list
    /// file, the single path of an older version becomes the only entry.
    #[must_use]
    pub fn load(&self) -> Vec<PathBuf> {
        if !self.file.exists() {
            return self.migrate_legacy();
        }
        read_limited(&self.file)
            .and_then(|text| serde_json::from_str::<Stored>(&text).ok())
            .map(|stored| {
                let mut paths: Vec<PathBuf> = Vec::new();
                for path in stored.paths.into_iter().map(PathBuf::from) {
                    if path.is_absolute() && !paths.contains(&path) {
                        paths.push(path);
                    }
                }
                paths.truncate(MAX_RECENT);
                paths
            })
            .unwrap_or_default()
    }

    /// The newest path, the one reopened at start (CSD-002 REQ-8).
    #[must_use]
    pub fn latest(&self) -> Option<PathBuf> {
        self.load().into_iter().next()
    }

    fn migrate_legacy(&self) -> Vec<PathBuf> {
        let Some(path) = read_limited(&self.legacy_file)
            .and_then(|text| serde_json::from_str::<Legacy>(&text).ok())
            .map(|legacy| PathBuf::from(legacy.path))
            .filter(|path| path.is_absolute())
        else {
            return Vec::new();
        };
        let paths = vec![path];
        if self.store(&paths).is_ok() {
            let _ = fs::remove_file(&self.legacy_file);
        }
        paths
    }

    /// Puts `path` at the top, removes an earlier entry of the same path and
    /// keeps the newest [`MAX_RECENT`].
    ///
    /// # Errors
    /// Fails if the path is not valid UTF-8 or the list cannot be written.
    pub fn record(&self, path: &Path) -> io::Result<()> {
        let mut paths = self.load();
        paths.retain(|entry| entry != path);
        paths.insert(0, path.to_path_buf());
        paths.truncate(MAX_RECENT);
        self.store(&paths)
    }

    /// Removes `path` from the list. Best effort, like the rest of the list.
    pub fn remove(&self, path: &Path) {
        let mut paths = self.load();
        let before = paths.len();
        paths.retain(|entry| entry != path);
        if paths.len() != before {
            let _ = self.store(&paths);
        }
    }

    /// Empties the list. A missing file is fine; an older single path is gone too.
    pub fn clear(&self) {
        let _ = fs::remove_file(&self.file);
        let _ = fs::remove_file(&self.legacy_file);
    }

    fn store(&self, paths: &[PathBuf]) -> io::Result<()> {
        let mut texts = Vec::with_capacity(paths.len());
        for path in paths {
            let text = path
                .to_str()
                .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?;
            texts.push(text.to_owned());
        }
        let json =
            serde_json::to_string_pretty(&Stored { paths: texts }).map_err(io::Error::other)?;
        if let Some(parent) = self.file.parent() {
            fs::create_dir_all(parent)?;
        }
        write_atomic_with(&self.file, |file| {
            use std::io::Write;
            file.write_all(json.as_bytes())
        })
    }
}

fn read_limited(path: &Path) -> Option<String> {
    let file = fs::File::open(path).ok()?;
    let mut text = String::new();
    file.take(MAX_BYTES).read_to_string(&mut text).ok()?;
    Some(text)
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    fn list_in(dir: &TempDir) -> RecentProjects {
        RecentProjects::in_dir(&dir.path().join("config"))
    }

    fn path(dir: &TempDir, name: &str) -> PathBuf {
        dir.path().join(name)
    }

    #[test]
    fn test_csd002_ac_31_1_a_path_goes_on_top() {
        let dir = TempDir::new().unwrap();
        let list = list_in(&dir);
        assert_eq!(list.load(), Vec::<PathBuf>::new());
        assert_eq!(list.latest(), None);

        list.record(&path(&dir, "example-a.varde")).unwrap();
        list.record(&path(&dir, "example-b.varde")).unwrap();
        assert_eq!(
            list.load(),
            vec![path(&dir, "example-b.varde"), path(&dir, "example-a.varde")]
        );
        assert_eq!(list.latest(), Some(path(&dir, "example-b.varde")));
    }

    #[test]
    fn test_csd002_ac_31_2_a_path_is_listed_once() {
        let dir = TempDir::new().unwrap();
        let list = list_in(&dir);
        list.record(&path(&dir, "example-a.varde")).unwrap();
        list.record(&path(&dir, "example-b.varde")).unwrap();
        list.record(&path(&dir, "example-a.varde")).unwrap();
        assert_eq!(
            list.load(),
            vec![path(&dir, "example-a.varde"), path(&dir, "example-b.varde")]
        );
    }

    #[test]
    fn test_csd002_ac_31_3_the_list_keeps_10_entries() {
        let dir = TempDir::new().unwrap();
        let list = list_in(&dir);
        for n in 0..11 {
            list.record(&path(&dir, &format!("example-{n}.varde")))
                .unwrap();
        }
        let paths = list.load();
        assert_eq!(paths.len(), MAX_RECENT);
        assert_eq!(paths[0], path(&dir, "example-10.varde"));
        assert!(!paths.contains(&path(&dir, "example-0.varde")));
    }

    #[test]
    fn test_csd002_ac_31_6_the_old_single_path_becomes_the_list() {
        let dir = TempDir::new().unwrap();
        let config = dir.path().join("config");
        fs::create_dir_all(&config).unwrap();
        let old = path(&dir, "example-a.varde");
        let text = serde_json::json!({ "path": old.to_str().unwrap() }).to_string();
        fs::write(config.join(LEGACY_FILE_NAME), text).unwrap();

        let list = list_in(&dir);
        assert_eq!(list.load(), vec![old.clone()]);
        // Migrated once: the list file exists and the old file is gone.
        assert!(config.join(FILE_NAME).exists());
        assert!(!config.join(LEGACY_FILE_NAME).exists());
        assert_eq!(list.load(), vec![old]);
    }

    #[test]
    fn test_csd002_ac_31_6_an_existing_list_wins_over_the_old_file() {
        let dir = TempDir::new().unwrap();
        let config = dir.path().join("config");
        let list = list_in(&dir);
        list.record(&path(&dir, "example-b.varde")).unwrap();
        let text = serde_json::json!({ "path": path(&dir, "example-a.varde").to_str().unwrap() })
            .to_string();
        fs::write(config.join(LEGACY_FILE_NAME), text).unwrap();
        assert_eq!(list.load(), vec![path(&dir, "example-b.varde")]);
    }

    #[test]
    fn test_csd002_ac_31_7_an_unusable_file_is_an_empty_list() {
        let dir = TempDir::new().unwrap();
        let list = RecentProjects::in_dir(dir.path());
        let file = dir.path().join(FILE_NAME);
        for content in [
            "not json",
            "{}",
            r#"{"paths": [5]}"#,
            r#"{"paths": "example-a.varde"}"#,
            "",
        ] {
            fs::write(&file, content).unwrap();
            assert_eq!(list.load(), Vec::<PathBuf>::new(), "{content:?}");
        }
        fs::write(&file, "x".repeat(70_000)).unwrap();
        assert_eq!(list.load(), Vec::<PathBuf>::new());
    }

    #[test]
    fn test_csd002_ac_31_7_relative_paths_are_not_used() {
        let dir = TempDir::new().unwrap();
        let list = RecentProjects::in_dir(dir.path());
        let absolute = path(&dir, "example-a.varde");
        let text = serde_json::json!({
            "paths": ["relative/example-b.varde", absolute.to_str().unwrap(), absolute.to_str().unwrap()]
        })
        .to_string();
        fs::write(dir.path().join(FILE_NAME), text).unwrap();
        assert_eq!(list.load(), vec![absolute]);
    }

    #[cfg(unix)]
    #[test]
    fn test_csd002_ac_31_8_the_file_holds_paths_only_and_is_private() {
        use std::os::unix::fs::PermissionsExt;
        let dir = TempDir::new().unwrap();
        let list = RecentProjects::in_dir(dir.path());
        list.record(&path(&dir, "example-a.varde")).unwrap();

        let file = dir.path().join(FILE_NAME);
        let value: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
        assert_eq!(value.as_object().unwrap().len(), 1);
        assert!(value["paths"][0].is_string());
        let mode = fs::metadata(&file).unwrap().permissions().mode();
        assert_eq!(mode & 0o077, 0, "mode {mode:o}");
    }

    #[test]
    fn remove_and_clear_forget_paths() {
        let dir = TempDir::new().unwrap();
        let list = list_in(&dir);
        list.record(&path(&dir, "example-a.varde")).unwrap();
        list.record(&path(&dir, "example-b.varde")).unwrap();
        list.remove(&path(&dir, "example-b.varde"));
        assert_eq!(list.load(), vec![path(&dir, "example-a.varde")]);
        list.remove(&path(&dir, "example-z.varde")); // not listed: fine

        list.clear();
        assert_eq!(list.load(), Vec::<PathBuf>::new());
        list.clear(); // clearing twice is fine
    }
}
