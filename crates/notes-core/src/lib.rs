use std::collections::BTreeSet;
use std::fs::{self, OpenOptions};
use std::io;
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
}

fn is_note(name: &str) -> bool {
    name.ends_with(".txt") && !name.starts_with('.') && !name.contains(['\r', '\n', '/', '\\'])
}

fn replace(path: &Path, bytes: &[u8]) -> io::Result<()> {
    use std::io::Write;
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

    fn sort(&mut self) {
        self.entries.sort_by(|a, b| {
            b.pinned
                .cmp(&a.pinned)
                .then_with(|| b.modified.cmp(&a.modified))
                .then_with(|| a.file.cmp(&b.file))
        });
    }

    pub fn create(&mut self) -> io::Result<String> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let file = format!("note-{timestamp}.txt");
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(self.dir.join(&file))?
            .sync_all()?;
        self.entries.push(Note {
            file: file.clone(),
            text: String::new(),
            pinned: false,
            modified: SystemTime::now(),
        });
        self.sort();
        Ok(file)
    }

    pub fn edit(&mut self, file: &str, text: String) -> io::Result<()> {
        let note = self
            .entries
            .iter_mut()
            .find(|note| note.file == file)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "note is gone"))?;
        if note.text != text {
            replace(&self.dir.join(file), text.as_bytes())?;
            note.text = text;
            note.modified = SystemTime::now();
            self.sort();
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
        Ok(())
    }

    pub fn can_undo(&self) -> bool {
        self.deleted.is_some()
    }

    pub fn undo(&mut self) -> io::Result<()> {
        use std::io::Write;
        let Some(note) = &self.deleted else {
            return Ok(());
        };
        let path = self.dir.join(&note.file);
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        if let Err(error) = output
            .write_all(note.text.as_bytes())
            .and_then(|()| output.sync_all())
        {
            drop(output);
            let _ = fs::remove_file(path);
            return Err(error);
        }
        self.entries.push(self.deleted.take().unwrap());
        self.sort();
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
        reloaded.undo().unwrap();
        assert_eq!(
            fs::read_to_string(dir.join(file)).unwrap(),
            "Hẹn gặp\nStudy Vietnamese"
        );
        fs::remove_dir_all(dir).unwrap();
    }
}
