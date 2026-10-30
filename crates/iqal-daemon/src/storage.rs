//! Durable storage of the configuration.
//!
//! Writes go to a temporary file which is synced and then renamed over the
//! real file, so a crash or power loss leaves either the old or the new
//! config, never a torn one. A backup copy is kept for the case where the
//! main file is damaged anyway, and a hidden second copy in another
//! protected place for the case where someone deletes the settings: the
//! service writes them back while it runs, and starts from the hidden copy
//! after a restart.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use iqal_core::Config;

const CONFIG: &str = "config.json";
const BACKUP: &str = "config.json.bak";

pub struct Store {
    dir: PathBuf,
    /// The hidden second copy (a file elsewhere), if any.
    keep: Option<PathBuf>,
    /// Whether a block is locked, so the settings are sealed: Linux marks
    /// the files "cannot be changed or deleted" (lifted for its own saves),
    /// Windows protects their folders (see `protect_folder`).
    sealed: AtomicBool,
}

impl Store {
    pub fn open(dir: impl Into<PathBuf>) -> io::Result<Self> {
        let dir = dir.into();
        create_private_dir(&dir)?;
        Ok(Self { dir, keep: None, sealed: AtomicBool::new(false) })
    }

    /// Also keeps a hidden second copy of the settings in this file.
    pub fn with_keep(mut self, keep: PathBuf) -> Self {
        // Only a folder made here is made private: an existing one (such as
        // /etc, which everyone must be able to read) keeps its permissions.
        if let Some(parent) = keep.parent()
            && !parent.exists()
            && let Err(e) = create_private_dir(parent)
        {
            eprintln!("iqal: {}: {e}", parent.display());
        }
        self.keep = Some(keep);
        self
    }

    /// The files that hold the settings.
    fn files(&self) -> Vec<PathBuf> {
        let mut files = vec![self.dir.join(CONFIG), self.dir.join(BACKUP)];
        files.extend(self.keep.clone());
        files
    }

    /// Seals the settings files while a block is locked (Linux), or opens
    /// them again.
    pub fn seal(&self, on: bool) {
        self.sealed.store(on, Ordering::Release);
        for f in self.files() {
            seal_file(&f, on);
        }
    }

    /// Writes the settings back if any of their files is missing (someone
    /// deleted them). Cheap: only looks whether the files are there.
    pub fn repair(&self, config: &Config) {
        if self.files().iter().all(|f| f.exists()) {
            return;
        }
        eprintln!("iqal: settings files were deleted; writing them back");
        if let Err(e) = self.save(config) {
            eprintln!("iqal: writing the settings back: {e}");
        }
    }

    /// Loads the config: the main file, else the backup, else the hidden
    /// copy (and then writes the others back). None of them there: a fresh
    /// install. All damaged: they are kept aside for inspection and an
    /// empty config is used.
    pub fn load(&self) -> io::Result<Config> {
        let mut damaged = false;
        for (i, path) in self.files().iter().enumerate() {
            match read_config(path) {
                Ok(Some(c)) => {
                    if i > 0 {
                        eprintln!("iqal: restored the settings from {}", path.display());
                        self.save(&c)?;
                    }
                    return Ok(c);
                }
                Ok(None) => {}
                Err(e) => {
                    eprintln!("iqal: {} is damaged ({e})", path.display());
                    damaged = true;
                }
            }
        }
        if damaged {
            let stamp =
                std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
            for f in [CONFIG, BACKUP] {
                let _ = fs::rename(self.dir.join(f), self.dir.join(format!("{f}.damaged-{stamp}")));
            }
            eprintln!("iqal: starting with an empty configuration");
        }
        Ok(Config::default())
    }

    /// Small auxiliary state files next to the config.
    pub fn read_aux(&self, name: &str) -> Option<String> {
        fs::read_to_string(self.dir.join(name)).ok()
    }

    pub fn write_aux(&self, name: &str, text: &str) -> io::Result<()> {
        write_atomic(&self.dir, name, text.as_bytes())
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn save(&self, config: &Config) -> io::Result<()> {
        let json = config.to_json();
        let sealed = self.sealed.load(Ordering::Acquire);
        // Folders deleted by someone: made again, as private as before.
        let folders = std::iter::once(self.dir.as_path()).chain(self.keep.as_deref().and_then(Path::parent));
        for dir in folders {
            if !dir.is_dir() {
                create_private_dir(dir)?;
                protect_folder(dir, sealed);
            }
        }
        if sealed {
            self.files().iter().for_each(|f| seal_file(f, false));
        }
        let result = (|| {
            write_atomic(&self.dir, CONFIG, json.as_bytes())?;
            write_atomic(&self.dir, BACKUP, json.as_bytes())?;
            if let Some(keep) = &self.keep
                && let (Some(dir), Some(name)) = (keep.parent(), keep.file_name())
            {
                write_atomic(dir, &name.to_string_lossy(), json.as_bytes())?;
            }
            Ok(())
        })();
        if sealed {
            self.files().iter().for_each(|f| seal_file(f, true));
        }
        result
    }
}

/// Linux: sets or clears the "cannot be changed or deleted" mark, which
/// even root must clear first. A missing file is fine.
#[cfg(target_os = "linux")]
fn seal_file(path: &Path, on: bool) {
    if let Err(e) = crate::linux::system::set_immutable(path, on)
        && e.kind() != io::ErrorKind::NotFound
    {
        eprintln!("iqal: {}: {e}", path.display());
    }
}

/// Windows protects the settings folder with its permissions instead.
#[cfg(not(target_os = "linux"))]
fn seal_file(_path: &Path, _on: bool) {}

/// Windows: the permissions of a settings folder made again (Linux makes
/// it private when creating it). Only for the real service: a development
/// folder stays the developer's own.
#[cfg(windows)]
fn protect_folder(dir: &Path, sealed: bool) {
    if crate::windows::system::is_elevated() {
        crate::windows::system::protect_settings_dir(dir, sealed);
    }
}

#[cfg(not(windows))]
fn protect_folder(_dir: &Path, _sealed: bool) {}

fn read_config(path: &Path) -> io::Result<Option<Config>> {
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };
    Config::from_json(&text).map(Some).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))
}

fn write_atomic(dir: &Path, name: &str, data: &[u8]) -> io::Result<()> {
    let tmp = dir.join(format!(".{name}.tmp"));
    {
        let mut f = private_file(&tmp)?;
        f.write_all(data)?;
        f.sync_all()?;
    }
    fs::rename(&tmp, dir.join(name))?;
    sync_dir(dir)
}

/// Makes a rename durable.
#[cfg(unix)]
fn sync_dir(dir: &Path) -> io::Result<()> {
    File::open(dir)?.sync_all()
}

/// NTFS journals renames itself (and folders cannot be opened as files).
#[cfg(not(unix))]
fn sync_dir(_dir: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(unix)]
fn private_file(path: &Path) -> io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;
    OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(path)
}

#[cfg(not(unix))]
fn private_file(path: &Path) -> io::Result<File> {
    OpenOptions::new().write(true).create(true).truncate(true).open(path)
}

#[cfg(unix)]
fn create_private_dir(dir: &Path) -> io::Result<()> {
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    fs::DirBuilder::new().recursive(true).mode(0o700).create(dir)?;
    fs::set_permissions(dir, fs::Permissions::from_mode(0o700))
}

#[cfg(not(unix))]
fn create_private_dir(dir: &Path) -> io::Result<()> {
    fs::create_dir_all(dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use iqal_core::BlockSet;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("iqal-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn save_and_load() {
        let dir = temp_dir("save");
        let store = Store::open(&dir).unwrap();
        assert_eq!(store.load().unwrap(), Config::default());
        let mut config = Config::default();
        config.block_sets.push(BlockSet::new("a"));
        store.save(&config).unwrap();
        assert_eq!(store.load().unwrap(), config);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn falls_back_to_backup() {
        let dir = temp_dir("backup");
        let store = Store::open(&dir).unwrap();
        let mut config = Config::default();
        config.block_sets.push(BlockSet::new("a"));
        store.save(&config).unwrap();
        fs::write(dir.join(CONFIG), "{ damaged").unwrap();
        assert_eq!(store.load().unwrap(), config);
        // The main file was repaired.
        assert_eq!(read_config(&dir.join(CONFIG)).unwrap().unwrap(), config);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn hidden_copy_brings_deleted_settings_back() {
        let dir = temp_dir("keep");
        let keep = temp_dir("keep-hidden").join("keep.json");
        let store = Store::open(&dir).unwrap().with_keep(keep.clone());
        let mut config = Config::default();
        config.block_sets.push(BlockSet::new("a"));
        store.save(&config).unwrap();
        assert!(keep.exists());
        // Deleted while the service runs: written back.
        fs::remove_file(dir.join(CONFIG)).unwrap();
        store.repair(&config);
        assert_eq!(read_config(&dir.join(CONFIG)).unwrap().unwrap(), config);
        // The whole folder deleted: made again.
        fs::remove_dir_all(&dir).unwrap();
        store.repair(&config);
        assert_eq!(read_config(&dir.join(CONFIG)).unwrap().unwrap(), config);
        // Deleted, then a restart: loaded from the hidden copy.
        fs::remove_dir_all(&dir).unwrap();
        let store = Store::open(&dir).unwrap().with_keep(keep.clone());
        assert_eq!(store.load().unwrap(), config);
        assert!(dir.join(CONFIG).exists() && dir.join(BACKUP).exists());
        fs::remove_dir_all(dir).unwrap();
        fs::remove_dir_all(keep.parent().unwrap()).unwrap();
    }

    #[test]
    fn keeps_damaged_files_aside() {
        let dir = temp_dir("damaged");
        let store = Store::open(&dir).unwrap();
        fs::write(dir.join(CONFIG), "x").unwrap();
        fs::write(dir.join(BACKUP), "y").unwrap();
        assert_eq!(store.load().unwrap(), Config::default());
        let kept = fs::read_dir(&dir).unwrap().filter(|e| {
            e.as_ref().unwrap().file_name().to_string_lossy().contains(".damaged-")
        });
        assert_eq!(kept.count(), 2);
        fs::remove_dir_all(dir).unwrap();
    }
}
