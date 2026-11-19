use std::cell::RefCell;
use std::io;

use notes_core::{checklist, Notes};
use primitive_app::{with_storage, App, Change};
use primitive_ui::{
    back_button, header, keyboard_key, screen_rect, Canvas, Editor, Input, Layout, Press, Rect,
    TextStyle, BODY, CAPTION,
};

const COLUMN_COUNT: usize = 31;
const LINE_HEIGHT: i32 = 54;
const BODY_TOP: i32 = 250;
const NOTE_SIZE: i32 = 38;
const NOTE_LIMIT: usize = 100_000;
const CHECKLIST_ROWS: usize = 6;

#[derive(Default)]
struct HitLine {
    text: String,
    stops: Vec<i32>,
}

pub struct View {
    pub notes: Notes,
    open: Option<(String, primitive_ui::editor::Multiline)>,
    templates: bool,
    checklist: bool,
    checklist_page: usize,
    selected_item: Option<usize>,
    search: Editor,
    search_active: bool,
    editing: bool,
    confirm_delete: bool,
    page: usize,
    notice: String,
    hit_lines: RefCell<Vec<HitLine>>,
}

impl View {
    pub fn new(notes: Notes) -> Self {
        Self {
            notes,
            open: None,
            templates: false,
            checklist: false,
            checklist_page: 0,
            selected_item: None,
            search: Editor::new(120),
            search_active: false,
            editing: false,
            confirm_delete: false,
            page: 0,
            notice: String::new(),
            hit_lines: RefCell::default(),
        }
    }

    fn rect(x: i32, y: i32, w: i32, h: i32) -> Rect {
        screen_rect(1264, x, y, w, h)
    }

    fn editor_rect(&self) -> Rect {
        Self::rect(40, BODY_TOP, 1184, if self.editing { 648 } else { 1188 })
    }

    fn rows(&self) -> usize {
        if self.search_active {
            4
        } else {
            8
        }
    }

    fn view_note(&mut self, file: String) -> Change {
        if let Some(note) = self.notes.note(&file) {
            self.open = Some((
                file,
                primitive_ui::editor::Multiline::new(&note.text, NOTE_LIMIT),
            ));
            self.search_active = false;
            self.templates = false;
            self.checklist = !checklist::items(&note.text).is_empty();
            self.checklist_page = 0;
            self.selected_item = None;
            self.editing = !self.checklist;
            self.notice.clear();
            self.hit_lines.get_mut().clear();
            Change::Full
        } else {
            Change::None
        }
    }

    fn measure_lines(
        &self,
        canvas: &mut Canvas,
        editor: &primitive_ui::editor::Multiline,
        area: Rect,
    ) {
        let mut hits = self.hit_lines.borrow_mut();
        let visible = (area.h / LINE_HEIGHT) as usize;
        hits.resize_with(visible, HitLine::default);
        let mut lines = editor
            .lines(COLUMN_COUNT)
            .into_iter()
            .skip(editor.first_line);
        for hit in hits.iter_mut() {
            let Some(line) = lines.next() else {
                *hit = HitLine::default();
                continue;
            };
            let text: String = editor.editor.text[line].iter().collect();
            if text == hit.text && !hit.stops.is_empty() {
                continue;
            }
            hit.stops.clear();
            hit.stops.push(0);
            if !text.is_empty() {
                for end in text
                    .char_indices()
                    .skip(1)
                    .map(|(i, _)| i)
                    .chain([text.len()])
                {
                    hit.stops.push(canvas.measure(&text[..end], NOTE_SIZE));
                }
            }
            hit.text = text;
        }
    }

    fn hit_column(&self, row: usize, x: i32) -> Option<usize> {
        self.hit_lines
            .borrow()
            .get(row)?
            .stops
            .iter()
            .enumerate()
            .min_by_key(|(_, offset)| offset.abs_diff(x))
            .map(|(index, _)| index)
    }

    fn report(&mut self, result: io::Result<()>) -> bool {
        match result {
            Ok(()) => true,
            Err(error) => {
                self.notice = format!("Could not save this change. {error}");
                false
            }
        }
    }

    fn notice(&self) -> &str {
        if self.notes.has_pending_directory_sync() {
            "Storage sync is pending. Your changes are still visible."
        } else {
            &self.notice
        }
    }

    fn save_editor(&mut self) -> io::Result<()> {
        if let Some((file, editor)) = &self.open {
            with_storage(|| self.notes.edit(file, editor.editor.string()))?;
        }
        Ok(())
    }

    fn draw_list(&self, canvas: &mut Canvas) {
        header(canvas, "Notes", "Done");
        let query = self.search.string();
        let box_ = Self::rect(40, 170, 824, 100);
        let pressed = canvas.control_pressed(box_, false);
        canvas.panel(box_, 22, 2, if pressed { 220 } else { 255 });
        canvas.text(
            box_.inset(18),
            if query.is_empty() {
                "Search notes"
            } else {
                &query
            },
            BODY,
        );
        canvas.button(Self::rect(886, 170, 338, 100), "New note", true, false);
        let mut found = false;
        for (i, note) in self
            .notes
            .filtered(&query)
            .skip(self.page * self.rows())
            .take(self.rows())
            .enumerate()
        {
            found = true;
            let row = Self::rect(40, 304 + i as i32 * 126, 1184, 112);
            let pressed = canvas.control_pressed(row, false);
            canvas.panel(row, 22, 2, if pressed { 220 } else { 255 });
            let title = if note.pinned {
                format!("Pinned · {}", note.title())
            } else {
                note.title().to_owned()
            };
            canvas.text(row.inset(22), &title, TextStyle::new(40).bold());
        }
        if !found {
            canvas.text(
                Self::rect(40, 330, 1184, 90),
                if query.is_empty() {
                    "Your notes will appear here."
                } else {
                    "No notes match your search."
                },
                CAPTION,
            );
        }
        if self.search_active {
            canvas.keyboard(self.search.layer, "Done", None);
        } else {
            canvas.button(Self::rect(40, 1470, 270, 100), "Previous", false, false);
            canvas.text(
                Self::rect(420, 1496, 424, 62),
                &format!("Page {}", self.page + 1),
                CAPTION.center(),
            );
            canvas.button(Self::rect(954, 1470, 270, 100), "Next", false, false);
            canvas.button(Self::rect(40, 1325, 328, 100), "Templates", false, false);
            if self.notes.can_undo() {
                canvas.button(Self::rect(390, 1325, 484, 100), "Undo delete", false, false);
            }
            canvas.text(Self::rect(40, 1595, 1184, 68), self.notice(), CAPTION);
        }
    }

    fn draw_note(&self, canvas: &mut Canvas, file: &str, editor: &primitive_ui::editor::Multiline) {
        header(canvas, "Note", "Back");
        let pinned = self.notes.note(file).is_some_and(|note| note.pinned);
        canvas.button(
            Self::rect(40, 140, 240, 100),
            if pinned { "Unpin" } else { "Pin" },
            pinned,
            false,
        );
        canvas.button(Self::rect(302, 140, 240, 100), "Delete", false, false);
        if self
            .notes
            .note(file)
            .is_some_and(|note| !checklist::items(&note.text).is_empty())
        {
            canvas.button(Self::rect(552, 140, 220, 100), "Checklist", false, false);
        }
        canvas.button(
            Self::rect(790, 140, 434, 100),
            if self.editing {
                "Hide keyboard"
            } else {
                "Edit"
            },
            false,
            false,
        );
        let area = self.editor_rect();
        editor.draw(canvas, area, COLUMN_COUNT, LINE_HEIGHT, NOTE_SIZE);
        self.measure_lines(canvas, editor, area);
        if self.editing {
            canvas.text(Self::rect(40, 896, 1184, 50), self.notice(), CAPTION);
            canvas.keyboard(editor.editor.layer, "Return", None);
        } else {
            canvas.button(Self::rect(40, 1470, 270, 100), "Previous", false, false);
            canvas.button(Self::rect(954, 1470, 270, 100), "Next", false, false);
            canvas.text(Self::rect(40, 1595, 1184, 68), self.notice(), CAPTION);
        }
        if self.confirm_delete {
            let box_ = Self::rect(200, 570, 864, 440);
            canvas.popup(box_);
            canvas.text(
                Self::rect(250, 625, 764, 85),
                "Delete this note?",
                TextStyle::new(50).bold(),
            );
            canvas.text(
                Self::rect(250, 735, 764, 110),
                "Tap Undo delete from your list to get it back.",
                BODY.lines(2),
            );
            canvas.button(Self::rect(250, 863, 350, 100), "Cancel", false, false);
            canvas.button(Self::rect(664, 863, 350, 100), "Delete", true, false);
        }
    }

    fn draw_templates(&self, canvas: &mut Canvas) {
        header(canvas, "Templates", "Back");
        canvas.button(Self::rect(40, 250, 1184, 120), "Checklist", true, false);
        canvas.text(
            Self::rect(40, 384, 1184, 70),
            "Check items off and change their order.",
            CAPTION,
        );
        canvas.button(
            Self::rect(40, 470, 1184, 120),
            "Reading journal",
            false,
            false,
        );
        canvas.text(
            Self::rect(40, 604, 1184, 70),
            "Keep a quote and what you took from it.",
            CAPTION,
        );
        canvas.text(Self::rect(40, 1595, 1184, 68), self.notice(), CAPTION);
    }

    fn template_tap(&mut self, x: f32, y: f32) -> Change {
        if back_button(1264).contains(x, y) {
            self.templates = false;
            return Change::Full;
        }
        let text = if Self::rect(40, 250, 1184, 120).contains(x, y) {
            checklist::CHECKLIST_TEMPLATE
        } else if Self::rect(40, 470, 1184, 120).contains(x, y) {
            checklist::READING_JOURNAL_TEMPLATE
        } else {
            return Change::None;
        };
        match with_storage(|| self.notes.create_text(text.into())) {
            Ok(file) => {
                let change = self.view_note(file);
                if text == checklist::CHECKLIST_TEMPLATE {
                    self.edit_item_at_end();
                }
                change
            }
            Err(error) => {
                self.report(Err(error));
                Change::Partial
            }
        }
    }

    fn checklist_row(row: usize) -> Rect {
        Self::rect(40, 304 + row as i32 * 151, 1184, 124)
    }

    fn draw_checklist(&self, canvas: &mut Canvas, file: &str) {
        let Some(note) = self.notes.note(file) else {
            return;
        };
        header(canvas, "Checklist", "Back");
        canvas.button(Self::rect(40, 140, 568, 100), "Add item", true, false);
        canvas.button(Self::rect(656, 140, 568, 100), "Edit text", false, false);
        canvas.text(Self::rect(40, 252, 1184, 48), note.title(), CAPTION);
        let items = checklist::items(&note.text);
        for (index, item) in items
            .iter()
            .enumerate()
            .skip(self.checklist_page * CHECKLIST_ROWS)
            .take(CHECKLIST_ROWS)
        {
            let row = Self::checklist_row(index % CHECKLIST_ROWS);
            let selected = self.selected_item == Some(index);
            canvas.panel(row, 22, if selected { 4 } else { 2 }, 255);
            canvas.button(
                Rect::new(row.x + 10, row.y + 10, 104, 104),
                if item.checked { "X" } else { "" },
                item.checked,
                false,
            );
            let label = if item.label.is_empty() {
                "New item"
            } else {
                item.label
            };
            let text = Rect::new(row.x + 138, row.y + 18, row.w - 150, row.h - 36);
            let pressed = canvas.control_pressed(text, false);
            if pressed {
                canvas.panel(text, 16, 2, 220);
            }
            canvas.text(text.inset(10), label, BODY.lines(2));
        }
        if let Some(index) = self.selected_item {
            if index > 0 {
                canvas.button(Self::rect(40, 1240, 568, 100), "Move up", false, false);
            }
            if index + 1 < items.len() {
                canvas.button(Self::rect(656, 1240, 568, 100), "Move down", false, false);
            }
        } else {
            canvas.text(
                Self::rect(40, 1250, 1184, 62),
                "Tap an item to move it.",
                CAPTION,
            );
        }
        if self.checklist_page > 0 {
            canvas.button(Self::rect(40, 1470, 270, 100), "Previous", false, false);
        }
        if (self.checklist_page + 1) * CHECKLIST_ROWS < items.len() {
            canvas.button(Self::rect(954, 1470, 270, 100), "Next", false, false);
        }
        canvas.text(Self::rect(40, 1595, 1184, 68), self.notice(), CAPTION);
    }

    fn checklist_page(&mut self, delta: isize) -> Change {
        let count = self
            .open
            .as_ref()
            .and_then(|(file, _)| self.notes.note(file))
            .map_or(0, |note| checklist::items(&note.text).len());
        let next = self
            .checklist_page
            .saturating_add_signed(delta)
            .min(count.div_ceil(CHECKLIST_ROWS).saturating_sub(1));
        if next == self.checklist_page {
            return Change::None;
        }
        self.checklist_page = next;
        self.selected_item = None;
        Change::Partial
    }

    fn save_checklist(&mut self, text: String) -> bool {
        let Some((file, editor)) = &mut self.open else {
            return false;
        };
        let result = with_storage(|| self.notes.edit(file, text.clone()));
        if result.is_ok() {
            *editor = primitive_ui::editor::Multiline::new(&text, NOTE_LIMIT);
            self.hit_lines.get_mut().clear();
            self.notice.clear();
        }
        self.report(result)
    }

    fn edit_item_at_end(&mut self) {
        if let Some((_, editor)) = &mut self.open {
            self.checklist = false;
            self.editing = true;
            editor.editor.caret = editor.editor.text.len();
            editor.show_caret(COLUMN_COUNT, (648 / LINE_HEIGHT) as usize);
        }
    }

    fn checklist_tap(&mut self, x: f32, y: f32) -> Change {
        if back_button(1264).contains(x, y) {
            self.open = None;
            self.checklist = false;
            self.notice.clear();
            return Change::Full;
        }
        let Some((file, _)) = self.open.as_ref() else {
            return Change::None;
        };
        let Some(note) = self.notes.note(file) else {
            return Change::None;
        };
        if Self::rect(656, 140, 568, 100).contains(x, y) {
            self.checklist = false;
            self.editing = true;
            return Change::Full;
        }
        if Self::rect(40, 140, 568, 100).contains(x, y) {
            let ending = if note.text.contains("\r\n") {
                "\r\n"
            } else {
                "\n"
            };
            let mut text = note.text.clone();
            if !text.ends_with('\n') {
                text.push_str(ending);
            }
            text.push_str("- [ ] ");
            if text.chars().count() >= NOTE_LIMIT {
                self.notice = "This note is too long to add another item.".into();
                return Change::Partial;
            }
            if !self.save_checklist(text) {
                return Change::Partial;
            }
            self.edit_item_at_end();
            return Change::Full;
        }
        if Self::rect(40, 1470, 270, 100).contains(x, y) {
            return self.checklist_page(-1);
        }
        if Self::rect(954, 1470, 270, 100).contains(x, y) {
            return self.checklist_page(1);
        }
        if let Some(index) = self.selected_item {
            let direction = if Self::rect(40, 1240, 568, 100).contains(x, y) {
                -1
            } else if Self::rect(656, 1240, 568, 100).contains(x, y) {
                1
            } else {
                0
            };
            if direction != 0 {
                if let Some((text, index)) = checklist::move_item(&note.text, index, direction) {
                    if self.save_checklist(text) {
                        self.selected_item = Some(index);
                        self.checklist_page = index / CHECKLIST_ROWS;
                    }
                    return Change::Partial;
                }
                return Change::None;
            }
        }
        for row in 0..CHECKLIST_ROWS {
            let bounds = Self::checklist_row(row);
            if !bounds.contains(x, y) {
                continue;
            }
            let index = self.checklist_page * CHECKLIST_ROWS + row;
            if index >= checklist::items(&note.text).len() {
                return Change::None;
            }
            if x < (bounds.x + 130) as f32 {
                if let Some(text) = checklist::toggle(&note.text, index) {
                    self.save_checklist(text);
                }
            } else {
                self.selected_item = Some(index);
            }
            return Change::Partial;
        }
        Change::None
    }

    fn list_page(&mut self, delta: isize) -> Change {
        let pages = self
            .notes
            .filtered(&self.search.string())
            .count()
            .div_ceil(self.rows());
        let next = self
            .page
            .saturating_add_signed(delta)
            .min(pages.saturating_sub(1));
        if next == self.page {
            Change::None
        } else {
            self.page = next;
            Change::Partial
        }
    }

    fn list_tap(&mut self, x: f32, y: f32) -> Change {
        if back_button(1264).contains(x, y) {
            return Change::Exit;
        }
        if Self::rect(886, 170, 338, 100).contains(x, y) {
            let file = with_storage(|| self.notes.create());
            return match file {
                Ok(file) => self.view_note(file),
                Err(error) => {
                    self.report(Err(error));
                    Change::Partial
                }
            };
        }
        if Self::rect(40, 170, 824, 100).contains(x, y) {
            self.search_active = !self.search_active;
            self.page = 0;
            return Change::Full;
        }
        if self.search_active {
            let layout = Layout::new(1264, 1680, 4, 5);
            if let Some(key) = keyboard_key(&layout, &self.search, "Done", x, y) {
                self.page = 0;
                return match self.search.press(key) {
                    Press::Go => {
                        self.search_active = false;
                        Change::Full
                    }
                    Press::Nothing => Change::None,
                    _ => Change::Partial,
                };
            }
        } else {
            if Self::rect(40, 1325, 328, 100).contains(x, y) {
                self.templates = true;
                self.notice.clear();
                return Change::Full;
            }
            for (rect, delta) in [
                (Self::rect(40, 1470, 270, 100), -1),
                (Self::rect(954, 1470, 270, 100), 1),
            ] {
                if rect.contains(x, y) {
                    return self.list_page(delta);
                }
            }
            if self.notes.can_undo() && Self::rect(390, 1325, 484, 100).contains(x, y) {
                let result = with_storage(|| self.notes.undo());
                if self.report(result) {
                    self.notice = "Note restored.".into();
                }
                return Change::Partial;
            }
        }
        let file = self
            .notes
            .filtered(&self.search.string())
            .skip(self.page * self.rows())
            .take(self.rows())
            .enumerate()
            .find(|(i, _)| Self::rect(40, 304 + *i as i32 * 126, 1184, 112).contains(x, y))
            .map(|(_, note)| note.file.clone());
        file.map_or(Change::None, |file| self.view_note(file))
    }

    fn note_tap(&mut self, x: f32, y: f32) -> Change {
        if self.confirm_delete {
            if Self::rect(664, 863, 350, 100).contains(x, y) {
                let result = self.save_editor();
                if !self.report(result) {
                    self.confirm_delete = false;
                    return Change::Full;
                }
                if let Some((file, _)) = &self.open {
                    let result = with_storage(|| self.notes.delete(file));
                    if self.report(result) {
                        self.open = None;
                        self.page = 0;
                        self.notice = "Note deleted.".into();
                    }
                }
            } else if !Self::rect(250, 863, 350, 100).contains(x, y)
                && Self::rect(200, 570, 864, 440).contains(x, y)
            {
                return Change::None;
            }
            self.confirm_delete = false;
            return Change::Full;
        }
        if back_button(1264).contains(x, y) {
            let result = self.save_editor();
            if self.report(result) {
                self.open = None;
                self.editing = false;
                self.notice.clear();
                return Change::Full;
            }
            return Change::Partial;
        }
        let area = self.editor_rect();
        if area.contains(x, y) {
            let row = ((y as i32 - area.y) / LINE_HEIGHT) as usize;
            let Some(column) = self.hit_column(row, x as i32 - area.x) else {
                return Change::None;
            };
            if let Some((_, editor)) = &mut self.open {
                editor.place(row, column, COLUMN_COUNT);
                return Change::Partial;
            }
        }
        if Self::rect(552, 140, 220, 100).contains(x, y)
            && self
                .open
                .as_ref()
                .and_then(|(file, _)| self.notes.note(file))
                .is_some_and(|note| !checklist::items(&note.text).is_empty())
        {
            let result = self.save_editor();
            if !self.report(result) {
                return Change::Partial;
            }
            self.checklist = true;
            self.checklist_page = 0;
            self.selected_item = None;
            return Change::Full;
        }
        let (file, editor) = self.open.as_mut().unwrap();
        if Self::rect(40, 140, 240, 100).contains(x, y) {
            let result = with_storage(|| self.notes.toggle_pin(file));
            self.report(result);
            return Change::Partial;
        }
        if Self::rect(302, 140, 240, 100).contains(x, y) {
            self.confirm_delete = true;
            return Change::Full;
        }
        if Self::rect(790, 140, 434, 100).contains(x, y) {
            self.editing = !self.editing;
            return Change::Full;
        }
        if !self.editing {
            let delta = if Self::rect(40, 1470, 270, 100).contains(x, y) {
                -1
            } else if Self::rect(954, 1470, 270, 100).contains(x, y) {
                1
            } else {
                0
            };
            if delta != 0 {
                editor.page(delta, COLUMN_COUNT, (area.h / LINE_HEIGHT) as usize);
                return Change::Partial;
            }
            return Change::None;
        }
        let layout = Layout::new(1264, 1680, 4, 5);
        let Some(key) = keyboard_key(&layout, &editor.editor, "Return", x, y) else {
            return Change::None;
        };
        let pressed = editor.press(key);
        editor.show_caret(COLUMN_COUNT, (area.h / LINE_HEIGHT) as usize);
        if pressed == Press::Edited {
            let result = self.save_editor();
            if self.report(result) {
                self.notice.clear();
            }
        }
        if pressed == Press::Nothing {
            Change::None
        } else {
            Change::Partial
        }
    }
}

impl App for View {
    fn draw(&self, canvas: &mut Canvas) {
        if self.templates {
            return self.draw_templates(canvas);
        }
        match &self.open {
            Some((file, _)) if self.checklist => self.draw_checklist(canvas, file),
            Some((file, editor)) => self.draw_note(canvas, file, editor),
            None => self.draw_list(canvas),
        }
    }

    fn input(&mut self, input: Input) -> Change {
        match input {
            Input::Tap(x, y) if self.templates => self.template_tap(x, y),
            Input::PageBack if self.templates => {
                self.templates = false;
                Change::Full
            }
            Input::Tap(x, y) if self.checklist && self.open.is_some() => self.checklist_tap(x, y),
            Input::PageBack if self.checklist => self.checklist_page(-1),
            Input::PageForward if self.checklist => self.checklist_page(1),
            Input::Tap(x, y) if self.open.is_some() => self.note_tap(x, y),
            Input::Tap(x, y) => self.list_tap(x, y),
            Input::PageBack | Input::PageForward if !self.confirm_delete => {
                let delta = if input == Input::PageBack { -1 } else { 1 };
                let visible = (self.editor_rect().h / LINE_HEIGHT) as usize;
                if let Some((_, editor)) = &mut self.open {
                    editor.page(delta, COLUMN_COUNT, visible);
                    Change::Partial
                } else {
                    self.list_page(delta)
                }
            }
            _ => Change::None,
        }
    }

    fn save(&mut self) -> io::Result<()> {
        self.save_editor()?;
        if self.notes.has_pending_directory_sync() {
            with_storage(|| self.notes.sync_pending_directory())?;
        }
        Ok(())
    }

    fn storage_returned(&mut self) -> io::Result<()> {
        let result = with_storage(|| {
            let mut fresh = self.notes.refreshed()?;
            if let Some((file, editor)) = &mut self.open {
                let before = self.notes.note(file).map(|note| note.text.as_str());
                let after = fresh.note(file).map(|note| note.text.as_str());
                if before != after {
                    let draft = editor.editor.string();
                    if before != Some(draft.as_str()) && after != Some(draft.as_str()) {
                        *file = fresh.create_text(draft)?;
                        self.notice =
                            "Saved your draft as a new note. The original changed over USB.".into();
                    } else if let Some(text) = after {
                        *editor = primitive_ui::editor::Multiline::new(text, 100_000);
                        self.hit_lines.get_mut().clear();
                    } else {
                        self.open = None;
                        self.editing = false;
                    }
                }
            }
            self.checklist = self.checklist
                && self
                    .open
                    .as_ref()
                    .and_then(|(file, _)| fresh.note(file))
                    .is_some_and(|note| !checklist::items(&note.text).is_empty());
            self.checklist_page = 0;
            self.selected_item = None;
            self.notes = fresh;
            let _ = self.notes.sync_pending_directory();
            self.page = 0;
            Ok(())
        });
        if let Err(error) = &result {
            self.notice = format!("Could not reload notes. {error}");
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        path::Path,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn checklist_toggles_and_orders_on_disk_or_reloads_after_usb() -> io::Result<()> {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let folder = std::env::temp_dir().join(format!("primitive-notes-checklist-{stamp}"));
        let mut notes = Notes::open(folder.clone())?;
        let file = notes.create_text("Trip\n\n- [ ] Tickets\n- [ ] Camera".into())?;
        let mut app = View::new(notes);
        assert_eq!(app.input(Input::Tap(265.0, 340.0)), Change::Full);
        assert!(app.checklist);
        assert_eq!(app.input(Input::Tap(80.0, 355.0)), Change::Partial);
        assert_eq!(app.input(Input::Tap(245.0, 500.0)), Change::Partial);
        assert_eq!(app.input(Input::Tap(280.0, 1280.0)), Change::Partial);
        assert_eq!(
            fs::read_to_string(folder.join(&file))?,
            "Trip\n\n- [ ] Camera\n- [x] Tickets"
        );
        app.save()?;
        fs::write(folder.join(&file), "Changed over USB")?;
        app.storage_returned()?;
        assert!(!app.checklist);
        assert_eq!(
            app.open.as_ref().unwrap().1.editor.string(),
            "Changed over USB"
        );
        fs::remove_dir_all(folder)
    }

    #[test]
    fn templates_create_notes_in_the_existing_plain_text_library() -> io::Result<()> {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let folder = std::env::temp_dir().join(format!("primitive-notes-template-{stamp}"));
        let mut app = View::new(Notes::open(folder.clone())?);
        assert_eq!(app.input(Input::Tap(215.0, 1365.0)), Change::Full);
        assert_eq!(app.input(Input::Tap(240.0, 515.0)), Change::Full);
        let file = &app.open.as_ref().unwrap().0;
        assert_eq!(
            fs::read_to_string(folder.join(file))?,
            checklist::READING_JOURNAL_TEMPLATE
        );
        fs::remove_dir_all(folder)
    }

    #[test]
    fn eject_reloads_host_changes_without_overwriting_an_open_draft() -> io::Result<()> {
        let folder =
            std::env::temp_dir().join(format!("primitive-notes-usb-{}", std::process::id()));
        let mut notes = Notes::open(folder.clone())?;
        let file = notes.create_text("original".into())?;
        let mut view = View::new(notes);
        assert_eq!(view.view_note(file.clone()), Change::Full);
        view.open.as_mut().unwrap().1 =
            primitive_ui::editor::Multiline::new("Kindle draft", 100_000);
        fs::write(folder.join(&file), "macOS edit")?;
        fs::write(folder.join("Copied.txt"), "copied over USB")?;
        assert_eq!(view.save().unwrap_err().kind(), io::ErrorKind::WouldBlock);
        view.storage_returned()?;
        let (draft_file, editor) = view.open.as_ref().unwrap();
        assert_ne!(draft_file, &file);
        assert_eq!(editor.editor.string(), "Kindle draft");
        assert_eq!(fs::read_to_string(folder.join(draft_file))?, "Kindle draft");
        assert_eq!(fs::read_to_string(folder.join(&file))?, "macOS edit");
        assert_eq!(
            view.notes.note("Copied.txt").unwrap().text,
            "copied over USB"
        );
        fs::remove_dir_all(folder)
    }

    #[test]
    fn caret_hit_targets_follow_the_drawn_width_of_narrow_and_wide_letters() -> io::Result<()> {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let folder = std::env::temp_dir().join(format!("primitive-notes-hit-{stamp}"));
        let mut notes = Notes::open(folder.clone())?;
        let file = notes.create()?;
        notes.edit(&file, "iiiWWW".into())?;
        let mut view = View::new(notes);
        assert_eq!(view.view_note(file), Change::Full);
        let fonts = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../doannt-kindle-launcher/assets/fonts");
        let mut canvas = Canvas::new(1264, 1680, &primitive_app::font_paths(&fonts))
            .map_err(io::Error::other)?;
        primitive_app::frame(&view, &mut canvas, false);
        let prefix = canvas.measure("iii", NOTE_SIZE);
        let area = view.editor_rect();
        assert_eq!(
            view.input(Input::Tap((area.x + prefix) as f32, (area.y + 20) as f32)),
            Change::Partial
        );
        assert_eq!(view.open.as_ref().unwrap().1.editor.caret, 3);
        fs::remove_dir_all(folder)
    }
}
