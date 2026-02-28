use crate::i18n_bundle;
use greentic_types::ChannelMessageEnvelope;
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::OnceLock;

type Catalog = BTreeMap<String, String>;
type Catalogs = BTreeMap<String, Catalog>;

static CATALOGS: OnceLock<Catalogs> = OnceLock::new();

fn catalogs() -> &'static Catalogs {
    CATALOGS.get_or_init(|| {
        let mut result = BTreeMap::new();
        for locale in i18n_bundle::supported_locales() {
            if let Some(raw) = i18n_bundle::locale_json(locale) {
                let parsed: Catalog = serde_json::from_str(raw).unwrap_or_default();
                result.insert((*locale).to_string(), parsed);
            }
        }
        result
    })
}

fn normalize(raw: &str) -> Option<String> {
    let mut cleaned = raw.trim();
    if cleaned.is_empty() {
        return None;
    }
    if let Some((head, _)) = cleaned.split_once('.') {
        cleaned = head;
    }
    if let Some((head, _)) = cleaned.split_once('@') {
        cleaned = head;
    }
    let cleaned = cleaned.replace('_', "-");
    if cleaned.is_empty() {
        return None;
    }

    let mut out = Vec::new();
    for (idx, part) in cleaned.split('-').enumerate() {
        if part.is_empty() {
            continue;
        }
        if idx == 0 {
            out.push(part.to_ascii_lowercase());
        } else {
            out.push(part.to_ascii_uppercase());
        }
    }
    if out.is_empty() {
        return None;
    }
    Some(out.join("-"))
}

fn base_language(tag: &str) -> Option<String> {
    tag.split('-').next().map(|s| s.to_ascii_lowercase())
}

fn resolve_supported(candidate: &str) -> Option<String> {
    let normalized = normalize(candidate)?;
    let catalogs = catalogs();

    if catalogs.contains_key(&normalized) {
        return Some(normalized);
    }

    let normalized_lower = normalized.to_ascii_lowercase();
    if let Some((found, _)) = catalogs
        .iter()
        .find(|(key, _)| key.to_ascii_lowercase() == normalized_lower)
    {
        return Some(found.clone());
    }

    let base = base_language(&normalized)?;
    if catalogs.contains_key(&base) {
        return Some(base);
    }

    None
}

pub(crate) fn select_locale(config: &Value, msg: &ChannelMessageEnvelope) -> String {
    let candidates = [
        config
            .get("locale")
            .and_then(Value::as_str)
            .map(str::to_string),
        config
            .get("templates")
            .and_then(Value::as_object)
            .and_then(|obj| obj.get("locale"))
            .and_then(Value::as_str)
            .map(str::to_string),
        msg.metadata.get("locale").cloned(),
        msg.metadata.get("lang").cloned(),
    ];

    for candidate in candidates.into_iter().flatten() {
        if let Some(resolved) = resolve_supported(&candidate) {
            return resolved;
        }
    }

    "en".to_string()
}

fn lookup(locale: &str, key: &str) -> Option<String> {
    let catalogs = catalogs();
    catalogs.get(locale).and_then(|m| m.get(key).cloned())
}

pub(crate) fn t(locale: &str, key: &str) -> String {
    if let Some(value) = lookup(locale, key) {
        return value;
    }
    if let Some(base) = base_language(locale)
        && let Some(value) = lookup(&base, key)
    {
        return value;
    }
    if let Some(value) = lookup("en", key) {
        return value;
    }
    key.to_string()
}

pub(crate) fn tf(locale: &str, key: &str, args: &[(&str, String)]) -> String {
    let mut text = t(locale, key);
    for (name, value) in args {
        let pattern = format!("{{{}}}", name);
        text = text.replace(&pattern, value);
    }
    text
}

pub(crate) fn all_en_keys() -> Vec<String> {
    let mut keys: Vec<String> = catalogs()
        .get("en")
        .map(|cat| cat.keys().cloned().collect())
        .unwrap_or_default();
    keys.sort();
    keys
}
