//! Translations. Each language is one Fluent file in `locales/`, compiled
//! into the program. Missing messages fall back to English.
//!
//! Adding a language: copy `locales/en.ftl` to `locales/<code>.ftl`,
//! translate it, and add it to [`LANGUAGES`] and [`source`].

use std::cell::RefCell;

use fluent_bundle::{FluentArgs, FluentBundle, FluentResource};
use unic_langid::LanguageIdentifier;

pub use fluent_bundle::FluentValue;

/// (code, name in that language, right-to-left)
pub const LANGUAGES: &[(&str, &str, bool)] = &[
    ("en", "English", false),
    ("ar", "العربية", true),
    ("fr", "Français", false),
    ("es", "Español", false),
    ("de", "Deutsch", false),
    ("tr", "Türkçe", false),
    ("ur", "اردو", true),
    ("id", "Bahasa Indonesia", false),
    ("pt", "Português", false),
    ("ru", "Русский", false),
    ("zh", "中文（简体）", false),
];

fn source(code: &str) -> Option<&'static str> {
    match code {
        "en" => Some(include_str!("../../../locales/en.ftl")),
        "ar" => Some(include_str!("../../../locales/ar.ftl")),
        "fr" => Some(include_str!("../../../locales/fr.ftl")),
        "es" => Some(include_str!("../../../locales/es.ftl")),
        "de" => Some(include_str!("../../../locales/de.ftl")),
        "tr" => Some(include_str!("../../../locales/tr.ftl")),
        "ur" => Some(include_str!("../../../locales/ur.ftl")),
        "id" => Some(include_str!("../../../locales/id.ftl")),
        "pt" => Some(include_str!("../../../locales/pt.ftl")),
        "ru" => Some(include_str!("../../../locales/ru.ftl")),
        "zh" => Some(include_str!("../../../locales/zh.ftl")),
        _ => None,
    }
}

struct Translator {
    code: &'static str,
    rtl: bool,
    bundle: FluentBundle<FluentResource>,
    english: FluentBundle<FluentResource>,
}

thread_local! {
    static TRANSLATOR: RefCell<Option<Translator>> = const { RefCell::new(None) };
}

fn bundle(code: &str) -> FluentBundle<FluentResource> {
    let lang: LanguageIdentifier = code.parse().unwrap_or_default();
    let mut bundle = FluentBundle::new(vec![lang]);
    let res = FluentResource::try_new(source(code).unwrap_or_default().to_string())
        .unwrap_or_else(|(res, errors)| {
            eprintln!("al-dir: {} translation errors in {code}.ftl", errors.len());
            res
        });
    let _ = bundle.add_resource(res);
    bundle
}

/// The best supported language for a locale string like "ar_SA.UTF-8".
pub fn match_language(locale: &str) -> &'static str {
    let primary = locale.split(['_', '-', '.', '@']).next().unwrap_or("").to_ascii_lowercase();
    LANGUAGES.iter().find(|(c, _, _)| *c == primary).map_or("en", |(c, _, _)| c)
}

/// The user's language from the environment (LANGUAGE, LC_ALL, LC_MESSAGES, LANG).
pub fn system_language() -> &'static str {
    for var in ["LANGUAGE", "LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(v) = std::env::var(var) {
            for part in v.split(':').filter(|p| !p.is_empty() && *p != "C" && *p != "POSIX") {
                let code = match_language(part);
                if code != "en" || part.starts_with("en") {
                    return code;
                }
            }
        }
    }
    "en"
}

/// Selects the language for this thread (the UI thread).
pub fn init(code: &str) {
    let (code, _, rtl) = *LANGUAGES.iter().find(|(c, _, _)| *c == code).unwrap_or(&LANGUAGES[0]);
    TRANSLATOR.with(|t| {
        *t.borrow_mut() = Some(Translator { code, rtl, bundle: bundle(code), english: bundle("en") });
    });
}

pub fn current() -> &'static str {
    TRANSLATOR.with(|t| t.borrow().as_ref().map_or("en", |t| t.code))
}

pub fn is_rtl() -> bool {
    TRANSLATOR.with(|t| t.borrow().as_ref().is_some_and(|t| t.rtl))
}

fn format(key: &str, args: Option<&FluentArgs>) -> String {
    TRANSLATOR.with(|t| {
        let t = t.borrow();
        let Some(t) = t.as_ref() else { return key.to_string() };
        for bundle in [&t.bundle, &t.english] {
            if let Some(pattern) = bundle.get_message(key).and_then(|m| m.value()) {
                let mut errors = Vec::new();
                return bundle.format_pattern(pattern, args, &mut errors).into_owned();
            }
        }
        key.to_string()
    })
}

/// A translated message.
pub fn t(key: &str) -> String {
    format(key, None)
}

/// A translated message with arguments: `ta("rules-count", &[("count", 3.into())])`.
pub fn ta(key: &str, args: &[(&str, FluentValue)]) -> String {
    let mut fa = FluentArgs::new();
    for (k, v) in args {
        fa.set(*k, v.clone());
    }
    format(key, Some(&fa))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every message in every language must also exist in English, and
    /// English must have every message any language has.
    #[test]
    fn languages_have_the_same_messages() {
        let ids = |code: &str| -> Vec<String> {
            let mut v: Vec<String> = source(code)
                .unwrap()
                .lines()
                .filter_map(|l| l.split_once(" =").map(|(k, _)| k.trim().to_string()))
                .filter(|k| !k.is_empty() && !k.starts_with('#') && !k.starts_with(' ') && !k.starts_with('['))
                .collect();
            v.sort();
            v
        };
        let en = ids("en");
        for (code, _, _) in LANGUAGES {
            assert_eq!(ids(code), en, "{code}.ftl differs from en.ftl");
            assert!(source(code).is_some());
            let res = FluentResource::try_new(source(code).unwrap().to_string());
            assert!(res.is_ok(), "{code}.ftl has syntax errors");
        }
    }

    #[test]
    fn formats_and_falls_back() {
        init("ar");
        assert!(is_rtl());
        assert_ne!(t("app-name"), "app-name");
        assert_eq!(t("no-such-message"), "no-such-message");
        init("en");
        assert!(!is_rtl());
        let s = ta("rules-count", &[("count", 1.into())]);
        assert!(s.contains('1'), "{s}");
    }

    #[test]
    fn locale_matching() {
        assert_eq!(match_language("ar_SA.UTF-8"), "ar");
        assert_eq!(match_language("en_GB.UTF-8"), "en");
        assert_eq!(match_language("nb_NO.UTF-8"), "en");
    }
}
