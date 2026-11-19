use std::ops::Range;

pub const CHECKLIST_TEMPLATE: &str = "Checklist\n\n- [ ] ";
pub const READING_JOURNAL_TEMPLATE: &str = "Reading journal\n\nBook\n\nPage\n\nQuote\n\nTakeaway\n";

#[derive(Debug)]
pub struct Item<'a> {
    pub label: &'a str,
    pub checked: bool,
    line: Range<usize>,
    marker: usize,
}

pub fn items(text: &str) -> Vec<Item<'_>> {
    let mut offset = 0;
    text.split_inclusive('\n')
        .filter_map(|line| {
            let start = offset;
            offset += line.len();
            let line = line.trim_end_matches(['\r', '\n']);
            let body = line.trim_start_matches([' ', '\t']);
            let checkbox = body
                .strip_prefix("- ")
                .or_else(|| body.strip_prefix("* "))
                .or_else(|| body.strip_prefix("+ "))
                .unwrap_or(body);
            let [b'[', mark @ (b' ' | b'x' | b'X'), b']', rest @ ..] = checkbox.as_bytes() else {
                return None;
            };
            if rest.first().is_some_and(|c| !c.is_ascii_whitespace()) {
                return None;
            }
            Some(Item {
                label: checkbox[3..].trim(),
                checked: *mark != b' ',
                line: start..start + line.len(),
                marker: start + line.len() - checkbox.len() + 1,
            })
        })
        .collect()
}

pub fn toggle(text: &str, index: usize) -> Option<String> {
    let item = items(text).into_iter().nth(index)?;
    let mut updated = text.to_owned();
    updated.replace_range(
        item.marker..item.marker + 1,
        if item.checked { " " } else { "x" },
    );
    Some(updated)
}

pub fn move_item(text: &str, index: usize, direction: isize) -> Option<(String, usize)> {
    if !matches!(direction, -1 | 1) {
        return None;
    }
    let target = index.checked_add_signed(direction)?;
    let items = items(text);
    let first = &items.get(index.min(target))?.line;
    let second = &items.get(index.max(target))?.line;
    let mut updated = String::with_capacity(text.len());
    for part in [
        &text[..first.start],
        &text[second.clone()],
        &text[first.end..second.start],
        &text[first.clone()],
        &text[second.end..],
    ] {
        updated.push_str(part);
    }
    Some((updated, target))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_changes_only_marker_and_preserves_imported_utf8_and_newlines() {
        let text = "Đọc sách\r\n\t* [X] Chapter 1\r\n[ ] Take notes\r\n[ ]skip\r\n";
        let entries = items(text);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].label, "Chapter 1");
        assert!(entries[0].checked);
        let updated = toggle(text, 0).unwrap();
        assert_eq!(updated, text.replacen("[X]", "[ ]", 1));
        assert!(toggle(text, 2).is_none());
    }

    #[test]
    fn moving_items_preserves_non_checklist_content_and_last_newline() {
        let text = "Trip\n- [ ] Pack\nNotes on tomorrow\n[ x ] Camera\n+ [X] Tickets";
        let (updated, index) = move_item(text, 0, 1).unwrap();
        assert_eq!(index, 1);
        assert_eq!(
            updated,
            "Trip\n+ [X] Tickets\nNotes on tomorrow\n[ x ] Camera\n- [ ] Pack"
        );
        assert_eq!(move_item(&updated, index, -1).unwrap().0, text);
        assert!(move_item(text, 0, -1).is_none());
    }
}
