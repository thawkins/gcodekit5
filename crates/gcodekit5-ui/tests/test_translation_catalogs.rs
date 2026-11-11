//! Translation catalog guards: the template must hold every string the UI
//! translates through a variable, and every language must translate every
//! template string (PR #98 review).

use gcodekit5_ui::ui::gtk::config_settings::SETTING_CATEGORIES;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// Repository `po/` directory, independent of the test's working directory.
fn po_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../po")
}

/// Decodes one quoted PO string (C escapes used by gettext).
fn unquote(s: &str) -> String {
    let inner = &s[1..s.len() - 1];
    let mut out = String::new();
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some(other) => out.push(other),
                None => {}
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Parses the active (non-obsolete) entries of a PO file: msgid -> (msgstr, fuzzy).
fn parse_po(name: &str) -> HashMap<String, (String, bool)> {
    let text = std::fs::read_to_string(po_dir().join(name)).expect("read po file");
    let mut entries = HashMap::new();
    for block in text.split("\n\n") {
        let mut fuzzy = false;
        let mut field: Option<&str> = None;
        let (mut id, mut msgstr) = (String::new(), String::new());
        for line in block.lines() {
            if line.starts_with("#,") && line.contains("fuzzy") {
                fuzzy = true;
            } else if let Some(rest) = line.strip_prefix("msgid ") {
                field = Some("id");
                id = unquote(rest);
            } else if let Some(rest) = line.strip_prefix("msgstr ") {
                field = Some("str");
                msgstr = unquote(rest);
            } else if line.starts_with('"') {
                match field {
                    Some("id") => id.push_str(&unquote(line)),
                    Some("str") => msgstr.push_str(&unquote(line)),
                    _ => {}
                }
            }
        }
        if field.is_some() && !id.is_empty() {
            entries.insert(id, (msgstr, fuzzy));
        }
    }
    entries
}

#[test]
fn test_translation_catalogs_template_has_setting_categories() {
    let template = parse_po("gcodekit5.pot");
    let missing: Vec<&str> = SETTING_CATEGORIES
        .iter()
        .copied()
        .filter(|c| !template.contains_key(*c))
        .collect();
    assert!(
        missing.is_empty(),
        "categories missing from .pot: {missing:?}"
    );
}

#[test]
fn test_translation_catalogs_every_language_is_complete() {
    let template: HashSet<String> = parse_po("gcodekit5.pot").into_keys().collect();
    for lang in ["de", "es", "fr", "it", "pt"] {
        let po = parse_po(&format!("{lang}.po"));
        let ids: HashSet<String> = po.keys().cloned().collect();
        let missing: Vec<&String> = template.difference(&ids).collect();
        let extra: Vec<&String> = ids.difference(&template).collect();
        let untranslated: Vec<&String> = po
            .iter()
            .filter(|(_, (s, fuzzy))| s.is_empty() || *fuzzy)
            .map(|(id, _)| id)
            .collect();
        assert!(missing.is_empty(), "{lang}: not in .po: {missing:?}");
        assert!(extra.is_empty(), "{lang}: not in .pot: {extra:?}");
        assert!(
            untranslated.is_empty(),
            "{lang}: untranslated or fuzzy: {untranslated:?}"
        );
    }
}
