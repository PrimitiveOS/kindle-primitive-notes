use std::{
    fs,
    path::{Path, PathBuf},
};

use notes_core::Notes;
use notes_render::View;
use primitive_app::{font_paths, frame, write_png, App, Change, HEIGHT, WIDTH};
use primitive_ui::{Canvas, Input};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .unwrap_or_else(|| "build/previews".into()),
    );
    fs::create_dir_all(&output)?;
    let data = output.join("sample-data");
    fs::create_dir_all(&data)?;
    fs::write(
        data.join("market.txt"),
        "Market list\nTea, apples and oats.",
    )?;
    let fonts =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../doannt-kindle-launcher/assets/fonts");
    let mut canvas = Canvas::new(WIDTH, HEIGHT, &font_paths(&fonts))?;
    let mut app = View::new(Notes::open(data)?);
    write_png(
        &output.join("notes-list.png"),
        WIDTH,
        HEIGHT,
        &frame(&app, &mut canvas, false),
    )?;
    assert_eq!(app.input(Input::Tap(300.0, 340.0)), Change::Full);
    write_png(
        &output.join("notes-edit.png"),
        WIDTH,
        HEIGHT,
        &frame(&app, &mut canvas, false),
    )?;
    write_png(
        &output.join("notes-edit-dark.png"),
        WIDTH,
        HEIGHT,
        &frame(&app, &mut canvas, true),
    )?;
    let checklist = output.join("checklist-fixture");
    fs::create_dir_all(&checklist)?;
    fs::write(
        checklist.join("review.txt"),
        "Study list\n\n- [x] Read the chapter\n- [ ] Review vocabulary\n- [ ] Write a summary",
    )?;
    let mut app = View::new(Notes::open(checklist)?);
    assert_eq!(app.input(Input::Tap(250.0, 340.0)), Change::Full);
    assert_eq!(app.input(Input::Tap(220.0, 500.0)), Change::Partial);
    write_png(
        &output.join("notes-checklist.png"),
        WIDTH,
        HEIGHT,
        &frame(&app, &mut canvas, false),
    )?;
    let journal = output.join("journal-fixture");
    fs::create_dir_all(&journal)?;
    fs::write(
        journal.join("reading.txt"),
        "Reading journal\n\nBook\nThe Living Forest\n\nPage\n42\n\nQuote\nSmall changes add up.\n\nTakeaway\nLook for a daily example.",
    )?;
    let mut app = View::new(Notes::open(journal)?);
    assert_eq!(app.input(Input::Tap(250.0, 340.0)), Change::Full);
    assert_eq!(app.input(Input::Tap(980.0, 194.0)), Change::Full);
    write_png(
        &output.join("notes-reading-journal-dark.png"),
        WIDTH,
        HEIGHT,
        &frame(&app, &mut canvas, true),
    )?;
    Ok(())
}
