use std::collections::BTreeSet;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug)]
pub struct Note {
    pub file: String,
    pub text: String,
    pub pinned: bool,
    modified: SystemTime,
}

impl Note {
    pub fn title(&self) -> &str {
        self.text
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .unwrap_or("Untitled note")
    }
}

pub struct Notes {
    dir: PathBuf,
    entries: Vec<Note>,
    deleted: Option<Note>,
    needs_directory_sync: bool,
    #[cfg(test)]
    directory_sync: fn(&Path) -> io::Result<()>,
}

fn is_note(name: &str) -> bool {
    name.ends_with(".txt") && !name.starts_with('.') && !name.contains(['\r', '\n', '/', '\\'])
}

fn replace(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let file = path.file_name().unwrap().to_string_lossy();
    let tmp = path.with_file_name(format!(".{file}.tmp"));
    let mut output = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&tmp)?;
    output.write_all(bytes)?;
    output.sync_all()?;
    drop(output);
    fs::rename(tmp, path)
}

fn create_file(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut output = OpenOptions::new().write(true).create_new(true).open(path)?;
    let result = output.write_all(bytes).and_then(|()| output.sync_all());
    drop(output);
    if result.is_err() {
        let _ = fs::remove_file(path);
    }
    result
}

fn sync_directory(directory: &Path) -> io::Result<()> {
    File::open(directory)?.sync_all()
}

impl Notes {
    pub fn open(dir: PathBuf) -> io::Result<Self> {
        fs::create_dir_all(&dir)?;
        let pins = fs::read_to_string(dir.join(".pins")).unwrap_or_default();
        let pins: BTreeSet<_> = pins.lines().collect();
        let mut entries = Vec::new();
        for file in fs::read_dir(&dir)? {
            let Ok(file) = file else { continue };
            let Ok(name) = file.file_name().into_string() else {
                continue;
            };
            if !is_note(&name) || !file.file_type().is_ok_and(|kind| kind.is_file()) {
                continue;
            }
            let Ok(text) = fs::read_to_string(file.path()) else {
                continue;
            };
            let modified = file
                .metadata()
                .and_then(|file| file.modified())
                .unwrap_or(UNIX_EPOCH);
            let pinned = pins.contains(name.as_str());
            entries.push(Note {
                file: name,
                text,
                pinned,
                modified,
            });
        }
        let mut notes = Self {
            dir,
            entries,
            deleted: None,
            needs_directory_sync: false,
            #[cfg(test)]
            directory_sync: sync_directory,
        };
        notes.sort();
        Ok(notes)
    }

    pub fn filtered(&self, query: &str) -> impl Iterator<Item = &Note> {
        let query = query.trim().to_lowercase();
        self.entries
            .iter()
            .filter(move |note| query.is_empty() || note.text.to_lowercase().contains(&query))
    }

    pub fn note(&self, file: &str) -> Option<&Note> {
        self.entries.iter().find(|note| note.file == file)
    }

    pub fn refreshed(&self) -> io::Result<Self> {
        let mut fresh = Self::open(self.dir.clone())?;
        fresh.deleted = self
            .deleted
            .clone()
            .filter(|note| fresh.note(&note.file).is_none());
        fresh.needs_directory_sync = self.needs_directory_sync;
        #[cfg(test)]
        {
            fresh.directory_sync = self.directory_sync;
        }
        Ok(fresh)
    }

    pub fn has_pending_directory_sync(&self) -> bool {
        self.needs_directory_sync
    }

    /// Retry a directory flush after a visible change whose durability could not be confirmed.
    pub fn sync_pending_directory(&mut self) -> io::Result<()> {
        if self.needs_directory_sync {
            #[cfg(not(test))]
            sync_directory(&self.dir)?;
            #[cfg(test)]
            (self.directory_sync)(&self.dir)?;
            self.needs_directory_sync = false;
        }
        Ok(())
    }

    fn directory_changed(&mut self) {
        self.needs_directory_sync = true;
        // The change is already visible. Keep the note state and retry a failed flush separately.
        let _ = self.sync_pending_directory();
    }

    fn sort(&mut self) {
        self.entries.sort_by(|a, b| {
            b.pinned
                .cmp(&a.pinned)
                .then_with(|| b.modified.cmp(&a.modified))
                .then_with(|| a.file.cmp(&b.file))
        });
    }

    pub fn create(&mut self) -> io::Result<String> {
        self.create_text(String::new())
    }

    pub fn create_text(&mut self, text: String) -> io::Result<String> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let file = format!("note-{timestamp}.txt");
        let path = self.dir.join(&file);
        create_file(&path, text.as_bytes())?;
        self.entries.push(Note {
            file: file.clone(),
            text,
            pinned: false,
            modified: SystemTime::now(),
        });
        self.sort();
        self.directory_changed();
        Ok(file)
    }

    pub fn edit(&mut self, file: &str, text: String) -> io::Result<()> {
        let note = self
            .entries
            .iter_mut()
            .find(|note| note.file == file)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "note is gone"))?;
        if note.text != text {
            let path = self.dir.join(file);
            if fs::read(&path)? != note.text.as_bytes() {
                return Err(io::Error::new(
                    io::ErrorKind::WouldBlock,
                    "This note changed outside the app. The draft is still open.",
                ));
            }
            replace(&path, text.as_bytes())?;
            note.text = text;
            note.modified = SystemTime::now();
            self.sort();
            self.directory_changed();
        }
        Ok(())
    }

    pub fn toggle_pin(&mut self, file: &str) -> io::Result<()> {
        let selected = self
            .note(file)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "note is gone"))?;
        let pinned = !selected.pinned;
        let pins = self
            .entries
            .iter()
            .chain(self.deleted.iter())
            .filter(|note| {
                if note.file == file {
                    pinned
                } else {
                    note.pinned
                }
            })
            .map(|note| note.file.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        replace(&self.dir.join(".pins"), pins.as_bytes())?;
        if let Some(note) = self.entries.iter_mut().find(|note| note.file == file) {
            note.pinned = pinned;
        }
        self.sort();
        self.directory_changed();
        Ok(())
    }

    pub fn delete(&mut self, file: &str) -> io::Result<()> {
        let index = self
            .entries
            .iter()
            .position(|note| note.file == file)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "note is gone"))?;
        fs::remove_file(self.dir.join(file))?;
        self.deleted = Some(self.entries.remove(index));
        self.directory_changed();
        Ok(())
    }

    pub fn can_undo(&self) -> bool {
        self.deleted.is_some()
    }

    pub fn undo(&mut self) -> io::Result<()> {
        let Some(note) = &self.deleted else {
            return Ok(());
        };
        let path = self.dir.join(&note.file);
        create_file(&path, note.text.as_bytes())?;
        self.entries.push(self.deleted.take().unwrap());
        self.sort();
        self.directory_changed();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_edits_and_pin_persist_and_undo_restores_the_file() {
        let dir = std::env::temp_dir().join(format!("primitive-notes-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let mut notes = Notes::open(dir.clone()).unwrap();
        let file = notes.create().unwrap();
        notes
            .edit(&file, "Hẹn gặp\nStudy Vietnamese".into())
            .unwrap();
        notes.toggle_pin(&file).unwrap();
        let mut reloaded = Notes::open(dir.clone()).unwrap();
        let note = reloaded.filtered("VIETNAMESE").next().unwrap();
        assert!(note.pinned);
        assert_eq!(note.title(), "Hẹn gặp");
        reloaded.delete(&file).unwrap();
        assert!(!dir.join(&file).exists());
        let other = reloaded.create_text("Another note".into()).unwrap();
        reloaded.toggle_pin(&other).unwrap();
        reloaded.undo().unwrap();
        assert_eq!(
            fs::read_to_string(dir.join(&file)).unwrap(),
            "Hẹn gặp\nStudy Vietnamese"
        );
        let reloaded = Notes::open(dir.clone()).unwrap();
        assert!(reloaded.note(&file).unwrap().pinned);
        assert!(reloaded.note(&other).unwrap().pinned);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn directory_sync_failure_preserves_visible_changes_and_can_be_retried() {
        fn fail_directory_sync(_: &Path) -> io::Result<()> {
            Err(io::Error::other("simulated directory sync failure"))
        }

        fn check_and_rearm(notes: &mut Notes) {
            assert!(notes.has_pending_directory_sync());
            assert!(notes.sync_pending_directory().is_err());
            notes.directory_sync = sync_directory;
            notes.sync_pending_directory().unwrap();
            assert!(!notes.has_pending_directory_sync());
            notes.directory_sync = fail_directory_sync;
        }

        let dir = std::env::temp_dir().join(format!("primitive-notes-sync-{}", std::process::id()));
        let mut notes = Notes::open(dir.clone()).unwrap();
        notes.directory_sync = fail_directory_sync;

        let file = notes.create_text("First".into()).unwrap();
        assert_eq!(notes.note(&file).unwrap().text, "First");
        assert_eq!(fs::read_to_string(dir.join(&file)).unwrap(), "First");
        check_and_rearm(&mut notes);

        notes.edit(&file, "Edited".into()).unwrap();
        assert_eq!(notes.note(&file).unwrap().text, "Edited");
        assert_eq!(fs::read_to_string(dir.join(&file)).unwrap(), "Edited");
        check_and_rearm(&mut notes);

        notes.toggle_pin(&file).unwrap();
        assert!(notes.note(&file).unwrap().pinned);
        assert_eq!(fs::read_to_string(dir.join(".pins")).unwrap(), file);
        check_and_rearm(&mut notes);

        notes.delete(&file).unwrap();
        assert!(notes.note(&file).is_none());
        assert!(notes.can_undo());
        assert!(!dir.join(&file).exists());
        notes = notes.refreshed().unwrap();
        check_and_rearm(&mut notes);

        notes.undo().unwrap();
        assert!(notes.note(&file).unwrap().pinned);
        assert!(!notes.can_undo());
        assert_eq!(fs::read_to_string(dir.join(&file)).unwrap(), "Edited");
        check_and_rearm(&mut notes);
        fs::remove_dir_all(dir).unwrap();
    }
}
