//! Durable storage of the configuration.
//!
//! Writes go to a temporary file which is synced and then renamed over the
//! real file, so a crash or power loss leaves either the old or the new
//! config, never a torn one. A backup copy is kept for the case where the
//! main file is damaged anyway.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use al_dir_core::Config;

const CONFIG: &str = "config.json";
const BACKUP: &str = "config.json.bak";

pub struct Store {
    dir: PathBuf,
}

impl Store {
    pub fn open(dir: impl Into<PathBuf>) -> io::Result<Self> {
        let dir = dir.into();
        create_private_dir(&dir)?;
        Ok(Self { dir })
    }

    /// Loads the config. A missing file means a fresh install. A damaged
    /// file falls back to the backup; if that is damaged too, both are
    /// kept aside for inspection and an empty config is used.
    pub fn load(&self) -> io::Result<Config> {
        let main = self.dir.join(CONFIG);
        match read_config(&main) {
            Ok(Some(c)) => return Ok(c),
            Ok(None) => return Ok(Config::default()),
            Err(e) => eprintln!("al-dir: {} is damaged ({e}); trying the backup", main.display()),
        }
        let backup = self.dir.join(BACKUP);
        match read_config(&backup) {
            Ok(Some(c)) => {
                eprintln!("al-dir: restored configuration from backup");
                self.save(&c)?;
                Ok(c)
            }
            other => {
                if let Err(e) = other {
                    eprintln!("al-dir: backup is damaged too ({e})");
                }
                let stamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                for f in [CONFIG, BACKUP] {
                    let _ = fs::rename(self.dir.join(f), self.dir.join(format!("{f}.damaged-{stamp}")));
                }
                eprintln!("al-dir: starting with an empty configuration");
                Ok(Config::default())
            }
        }
    }

    pub fn save(&self, config: &Config) -> io::Result<()> {
        let json = config.to_json();
        write_atomic(&self.dir, CONFIG, json.as_bytes())?;
        write_atomic(&self.dir, BACKUP, json.as_bytes())
    }
}

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
    // Make the rename itself durable.
    File::open(dir)?.sync_all()
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
    use al_dir_core::BlockSet;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("al-dir-test-{name}-{}", std::process::id()));
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
