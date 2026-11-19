# Primitive Notes

Write, find, and pin notes on a Kindle. Notes are plain UTF-8 text files in `/mnt/us/notes/`. The first nonempty
line is the title. A note is saved after each edit, and deleting a note offers Undo from the list.

Run `cargo test --offline --workspace` and `cargo run --offline -p notes-desktop` with the shared Tools and
Launcher repositories next to this one. The desktop target renders its own sample notes. The ARMHF package
uses the fonts installed by Primitive Launcher.

Source is PolyForm Noncommercial 1.0.0. Documentation is CC BY-NC-SA 4.0.
