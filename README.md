<div align="center">

# Notes

**A plain-text notebook for Kindle e-readers.**

[Install guide](INSTALL.md) · [Device notes](docs/device-notes.md) · [Changelog](CHANGELOG.md)

</div>

---

Write notes with the on-screen keyboard, search their contents, and pin the ones you use most. Each note is stored as
a UTF-8 text file in `/mnt/us/notes/`, ready to copy over USB. Open **Templates** to start a checklist or reading journal.

## Screenshots

<table>
  <tr>
    <td align="center"><img src="docs/images/notes-list.png" alt="Notes list with search, a new note button, and a sample market list" width="260"><br><sub>Notes list and search</sub></td>
    <td align="center"><img src="docs/images/notes-edit.png" alt="Editing a market list with the Kindle keyboard" width="260"><br><sub>Editing a note</sub></td>
    <td align="center"><img src="docs/images/notes-edit-dark.png" alt="The note editor and keyboard in dark mode" width="260"><br><sub>Dark mode</sub></td>
  </tr>
  <tr>
    <td align="center"><img src="docs/images/notes-checklist.png" alt="Checklist with a checked item and controls to move the selected item" width="260"></td>
    <td align="center"><img src="docs/images/notes-reading-journal-dark.png" alt="A reading journal with book, page, quote, and takeaway in dark mode" width="260"></td>
  </tr>
</table>

<sub>Desktop simulation at 1264 × 1680 with sample notes.</sub>

## Highlights

- The first nonempty line becomes the title. Pinned notes appear above the newest notes.
- Every edit is saved. Delete offers Undo from the notes list.
- With a matching Launcher, Notes refreshes the folder after USB eject and preserves conflicting drafts as separate notes.

## Checklists and reading journals

- Open **Templates** and choose **Checklist** to start a list. Notes also detects rows marked with `- [ ]` or `- [x]`
  in existing text files.
- Tap a checkbox to finish an item. Tap its label to select it, then choose **Move up** or **Move down**. **Edit text**
  opens the normal editor so you can write freely.
- Choose **Reading journal** for a note with Book, Page, Quote and Takeaway fields.

## Quick start

After setting up PrimitiveOS, open **Store** from Primitive Launcher. Choose **Notes** under **Apps**,
tap **Get**, and open it from the Launcher grid. The [install guide](INSTALL.md) also covers local USB testing.

## Status

Version 0.1.1 passed desktop simulation and local ARMHF package checks on the Mac. Version 0.1.0 was checked on an ARMHF Kindle Oasis 3 running firmware
5.18.2, Notes opened from Launcher, opened and dismissed the search keyboard, and returned to Launcher.
USB eject and sleep and wake checks are pending. [Device notes](docs/device-notes.md)

## Develop locally

Keep `doannt-kindle-tools` and `doannt-kindle-launcher` next to this checkout. The desktop command writes sample PNGs
to `build/previews`.

```sh
cargo test --offline --locked --workspace
cargo run --offline --locked -p notes-desktop -- build/previews
```

## Licence

Copyright (C) 2026 Doan Nguyen.

App code is licensed under PolyForm Noncommercial 1.0.0. [Code license](LICENSE)

Original documentation and screenshots are licensed under Creative Commons Attribution-NonCommercial-ShareAlike 4.0
International (CC BY-NC-SA 4.0). [Documentation and image license](LICENSE-DOCS)
