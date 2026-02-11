use fontdb::{Database, Family, Query, Stretch, Style, Weight};
use rusttype::Font;
use std::{
    collections::{HashMap, HashSet},
    fs,
    sync::{Mutex, OnceLock},
};

#[derive(Clone, Eq, PartialEq, Hash)]
struct FontKey {
    family: String,
    bold: bool,
    italic: bool,
}

fn default_font() -> &'static Font<'static> {
    static FONT: OnceLock<Font<'static>> = OnceLock::new();
    FONT.get_or_init(|| {
        let font_data = include_bytes!("../../../assets/fonts/fira-code/FiraCode-Regular.ttf");
        Font::try_from_bytes(font_data as &[u8])
            .unwrap_or_else(|| panic!("bundled FiraCode font is invalid"))
    })
}

fn db() -> &'static Database {
    static DB: OnceLock<Database> = OnceLock::new();
    DB.get_or_init(|| {
        let mut db = Database::new();
        db.load_system_fonts();
        db
    })
}

pub fn list_font_families() -> Vec<String> {
    let mut set = HashSet::new();
    for face in db().faces() {
        for (name, _) in &face.families {
            set.insert(name.clone());
        }
    }
    let mut out: Vec<_> = set.into_iter().collect();
    out.sort();
    out
}

pub fn get_font_for(family: &str, bold: bool, italic: bool) -> &'static Font<'static> {
    static CACHE: OnceLock<Mutex<HashMap<FontKey, &'static Font<'static>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));

    let key = FontKey {
        family: family.to_string(),
        bold,
        italic,
    };

    if let Some(font) = cache.lock().unwrap_or_else(|p| p.into_inner()).get(&key) {
        return font;
    }

    let loaded = load_font_from_system(family, bold, italic);
    let font_ref: &'static Font<'static> = match loaded {
        Some(font) => Box::leak(Box::new(font)),
        None => return default_font(),
    };

    cache
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .insert(key, font_ref);
    font_ref
}

fn load_font_from_system(family: &str, bold: bool, italic: bool) -> Option<Font<'static>> {
    let families: Vec<Family<'_>> = match family.trim() {
        "" | "Sans" => vec![Family::SansSerif],
        "Serif" => vec![Family::Serif],
        "Monospace" => vec![Family::Monospace],
        other => vec![Family::Name(other)],
    };

    let query = Query {
        families: &families,
        weight: if bold { Weight::BOLD } else { Weight::NORMAL },
        stretch: Stretch::Normal,
        style: if italic { Style::Italic } else { Style::Normal },
    };

    let id = db().query(&query)?;
    let face = db().face(id)?;

    match &face.source {
        fontdb::Source::File(path) => {
            let bytes = fs::read(path).ok()?;
            Font::try_from_vec(bytes)
        }
        fontdb::Source::SharedFile(path, _) => {
            let bytes = fs::read(path).ok()?;
            Font::try_from_vec(bytes)
        }
        fontdb::Source::Binary(bytes) => Font::try_from_vec(bytes.as_ref().as_ref().to_vec()),
    }
}

/// Backwards-compatible default font.
pub fn get_font() -> &'static Font<'static> {
    default_font()
}
