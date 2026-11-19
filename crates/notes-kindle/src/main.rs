use std::path::PathBuf;

use notes_core::Notes;
use notes_render::View;

fn main() -> Result<(), String> {
    let dir = PathBuf::from("/mnt/us/notes");
    let notes = Notes::open(dir.clone()).map_err(|error| error.to_string())?;
    primitive_app::run(View::new(notes), &dir)
}
