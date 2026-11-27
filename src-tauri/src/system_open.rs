//! Files the operating system hands to the app when the user double-clicks a
//! `.varde` file (CSD-002 REQ-34 to REQ-36).
//!
//! A system-handed path is untrusted input: it is checked here, held in Rust
//! and never sent to the frontend (CON-8).

use std::path::{Path, PathBuf};

/// Whether the app may open `path` when the system hands it over: absolute,
/// ending in `.varde` and a regular file (CSD-002 REQ-35).
#[must_use]
pub fn is_valid(path: &Path) -> bool {
    path.is_absolute()
        && path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("varde"))
        && path.metadata().is_ok_and(|meta| meta.is_file())
}

/// The first valid path of `paths`; the others are ignored (NG-3).
#[must_use]
pub fn first_valid(paths: impl IntoIterator<Item = PathBuf>) -> Option<PathBuf> {
    paths.into_iter().find(|path| is_valid(path))
}

/// The file in the command line of a launch (Windows): every argument is a
/// candidate, so the program's own name and any flag fall away as not valid.
#[must_use]
pub fn from_args(args: impl IntoIterator<Item = String>) -> Option<PathBuf> {
    first_valid(args.into_iter().map(PathBuf::from))
}

/// The files of the URLs of a macOS open-file event.
#[must_use]
pub fn from_urls(urls: &[tauri::Url]) -> Vec<PathBuf> {
    urls.iter()
        .filter_map(|url| url.to_file_path().ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    fn file(dir: &TempDir, name: &str) -> PathBuf {
        let path = dir.path().join(name);
        fs::write(&path, "{}").unwrap();
        path
    }

    #[test]
    fn test_csd002_ac_35_4_a_path_that_is_not_valid_is_ignored() {
        let dir = TempDir::new().unwrap();
        let folder = dir.path().join("example-folder.varde");
        fs::create_dir(&folder).unwrap();
        for (name, path) in [
            ("relative", PathBuf::from("example-a.varde")),
            ("wrong extension", file(&dir, "example-a.txt")),
            ("no extension", file(&dir, "example-a")),
            ("a folder", folder),
            ("missing", dir.path().join("example-missing.varde")),
        ] {
            assert!(!is_valid(&path), "{name}");
            assert_eq!(first_valid([path]), None, "{name}");
        }
        assert!(is_valid(&file(&dir, "example-a.varde")));
        assert!(is_valid(&file(&dir, "example-b.VARDE")));
    }

    #[test]
    fn test_csd002_ac_35_5_several_files_open_the_first_one() {
        let dir = TempDir::new().unwrap();
        let a = file(&dir, "example-a.varde");
        let b = file(&dir, "example-b.varde");
        assert_eq!(first_valid([b.clone(), a.clone()]), Some(b));
        // A path that is not valid does not hide a valid one behind it.
        assert_eq!(first_valid([dir.path().join("x.txt"), a.clone()]), Some(a));
    }

    #[test]
    fn test_csd002_ac_34_2_a_launch_argument_is_the_file() {
        let dir = TempDir::new().unwrap();
        let a = file(&dir, "example-a.varde");
        let args = [
            "C:\\Program Files\\Varde\\varde.exe".to_owned(),
            "--flag".to_owned(),
            a.to_str().unwrap().to_owned(),
        ];
        assert_eq!(from_args(args), Some(a));
        assert_eq!(from_args(["varde".to_owned()]), None);
    }

    #[cfg(unix)]
    #[test]
    fn test_csd002_ac_35_1_a_file_url_is_the_file() {
        let dir = TempDir::new().unwrap();
        let a = file(&dir, "example-a.varde");
        let url = tauri::Url::from_file_path(&a).unwrap();
        let other = tauri::Url::parse("https://example.com/example-b.varde").unwrap();
        assert_eq!(from_urls(&[other, url]), vec![a]);
    }
}
