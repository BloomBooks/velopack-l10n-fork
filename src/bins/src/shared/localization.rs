use std::collections::HashMap;
use std::sync::RwLock;

use lazy_static::lazy_static;
use velopack::bundle::BundleZip;
use quick_xml::de::from_str as xml_from_str;
use serde::{Deserialize, Serialize};

const DEFAULT_PO_PATH: &str = include_str!("../../../localization/en/velopack.po");

#[derive(Debug, Deserialize, Serialize)]
struct XliffFile {
    #[serde(rename = "@original")]
    original: String,
    #[serde(rename = "@source-language")]
    source_language: String,
    #[serde(rename = "@target-language")]
    target_language: Option<String>,
    body: XliffBody,
}

#[derive(Debug, Deserialize, Serialize)]
struct XliffBody {
    #[serde(rename = "trans-unit")]
    trans_units: Vec<XliffTransUnit>,
}

#[derive(Debug, Deserialize, Serialize)]
struct XliffTransUnit {
    #[serde(rename = "@id")]
    id: String,
    source: XliffText,
    target: Option<XliffText>,
}

#[derive(Debug, Deserialize, Serialize)]
struct XliffText {
    #[serde(rename = "@xml:lang")]
    lang: Option<String>,
    #[serde(rename = "$text")]
    content: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct Xliff {
    file: XliffFile,
}

lazy_static! {
    static ref STATE: RwLock<LocalizationState> = RwLock::new(LocalizationState::new());
}

#[derive(Clone)]
struct LocalizationState {
    default_english: HashMap<String, String>,
    english: HashMap<String, String>,
    active: HashMap<String, String>,
    active_key: String,
    available: Vec<String>,
}

impl LocalizationState {
    fn new() -> Self {
        let base = parse_po_content(DEFAULT_PO_PATH).expect("default localization is valid po");
        Self {
            default_english: base.clone(),
            english: base.clone(),
            active: base,
            active_key: "en".to_string(),
            available: Vec::new(),
        }
    }

    fn reset(&mut self) {
        self.english = self.default_english.clone();
        self.active = self.default_english.clone();
        self.active_key = "en".to_string();
        self.available.clear();
    }

    fn load_bundle(&mut self, bundle_data: Option<&HashMap<String, String>>) {
        self.reset();

        let mut cultures: HashMap<String, HashMap<String, String>> = HashMap::new();

        if let Some(data) = bundle_data {
            for (locale, content) in data {
                let norm = normalize_locale_tag(locale);
                
                // Try to parse as XLIFF first, then PO
                if let Some(strings) = parse_xliff_content(content) {
                    cultures.insert(norm, strings);
                } else if let Some(strings) = parse_po_content(content) {
                    cultures.insert(norm, strings);
                } else {
                    warn!("Failed to parse localization data for locale '{}'. Skipping.", locale);
                }
            }
        }

        if let Some(en_override) = cultures.get("en") {
            self.english = merge_string_maps(&self.english, en_override);
        }

        let detected = detect_system_locale().unwrap_or_else(|| "en".to_string());
        let candidates = candidate_locales(&detected);

        let mut chosen_key = "en".to_string();
        let mut chosen = self.english.clone();

        for candidate in candidates {
            if candidate == "en" {
                chosen_key = "en".to_string();
                chosen = self.english.clone();
                break;
            }

            if let Some(value) = cultures.get(&candidate) {
                chosen_key = candidate.clone();
                chosen = merge_string_maps(&self.english, value);
                break;
            }
        }

        self.available = cultures.keys().cloned().collect();
        self.active_key = chosen_key.clone();
        self.active = chosen;

        if bundle_data.is_some() {
            if self.active_key != "en" {
                info!("Loaded localization bundle for locale '{}'. Available locales: {:?}", self.active_key, self.available);
            } else {
                info!("Localization bundle supplied but active locale '{}' not found. Falling back to English. Available locales: {:?}", detected, self.available);
            }
        } else {
            debug!("No localization bundle present. Using built-in English strings.");
        }
    }

    fn get(&self, path: &str) -> Option<String> {
        self.active.get(path).cloned()
            .or_else(|| self.english.get(path).cloned())
    }

    fn english(&self, path: &str) -> Option<String> {
        self.english.get(path).cloned()
    }
}

pub fn initialize_from_bundle(bundle: &BundleZip) {
    let bundle_data = load_localization_files_from_bundle(bundle);

    let mut state = STATE.write().expect("localization state poisoned");
    state.load_bundle(bundle_data.as_ref());
}

pub fn text(path: &str) -> Option<String> {
    let state = STATE.read().expect("localization state poisoned");
    state.get(path)
}

pub fn text_with(path: &str, replacements: &[(&str, &str)]) -> Option<String> {
    text(path).map(|base| apply_replacements(base, replacements))
}

pub fn text_or_default(path: &str, default: &str) -> String {
    text(path).unwrap_or_else(|| default.to_string())
}

pub fn text_with_or(path: &str, replacements: &[(&str, &str)], default: &str) -> String {
    text_with(path, replacements).unwrap_or_else(|| apply_replacements(default.to_string(), replacements))
}

pub fn english_text(path: &str) -> Option<String> {
    let state = STATE.read().expect("localization state poisoned");
    state.english(path)
}

pub fn active_locale() -> String {
    let state = STATE.read().expect("localization state poisoned");
    state.active_key.clone()
}

fn merge_string_maps(base: &HashMap<String, String>, overlay: &HashMap<String, String>) -> HashMap<String, String> {
    let mut result = base.clone();
    for (key, value) in overlay {
        result.insert(key.clone(), value.clone());
    }
    result
}

fn load_localization_files_from_bundle(bundle: &BundleZip) -> Option<HashMap<String, String>> {
    let mut files = HashMap::new();

    // Look for XLIFF files first (preferred format) - .xlf extension only
    let xlf_files = find_localization_files(bundle, ".xlf");
    
    for (name, content) in xlf_files {
        if let Some(locale) = extract_locale_from_filename(&name) {
            files.insert(locale, content);
        }
    }

    // Then look for PO files if no XLIFF found
    if files.is_empty() {
        let po_files = find_localization_files(bundle, ".po");
        for (name, content) in po_files {
            if let Some(locale) = extract_locale_from_filename(&name) {
                files.insert(locale, content);
            }
        }
    }



    if files.is_empty() {
        None
    } else {
        Some(files)
    }
}

fn find_localization_files(bundle: &BundleZip, extension: &str) -> Vec<(String, String)> {
    let mut files = Vec::new();
    
    // Dynamically discover all localization files in the bundle
    if let Ok(file_names) = bundle.get_file_names() {
        for file_name in file_names {
            // Look for files in localization/ directory with the specified extension
            if file_name.starts_with("localization/") && file_name.ends_with(extension) {
                if let Ok(content) = bundle.read_zip_predicate_to_string(|name| name == file_name) {
                    files.push((file_name, content));
                }
            }
        }
    }
    
    files
}

fn extract_locale_from_filename(filename: &str) -> Option<String> {
    use std::path::Component;
    let path = std::path::Path::new(filename);

    // 1) Support folder layout: localization/<culture>/<file>
    // Find "localization" in the path components and take the next component as the culture folder if present
    let mut comps = path.components().peekable();
    while let Some(comp) = comps.next() {
        if let Component::Normal(os) = comp {
            if os == "localization" {
                if let Some(Component::Normal(locale_os)) = comps.peek() {
                    if let Some(locale_str) = locale_os.to_str() {
                        // Only treat as a culture if it doesn't look like a filename (no dot)
                        if !locale_str.contains('.') && !locale_str.is_empty() {
                            return Some(normalize_locale_tag(locale_str));
                        }
                    }
                }
                break;
            }
        }
    }

    // 2) Fallback to filename-based detection:
    //    - dialogs.en.xliff -> en
    //    - en.po -> en
    if let Some(stem) = path.file_stem() {
        if let Some(stem_str) = stem.to_str() {
            if let Some(dot_pos) = stem_str.rfind('.') {
                let locale = &stem_str[dot_pos + 1..];
                if !locale.is_empty() {
                    return Some(normalize_locale_tag(locale));
                }
            }
            // If no dot, assume the stem is the locale (e.g., "en.xliff")
            return Some(normalize_locale_tag(stem_str));
        }
    }
    None
}

fn parse_xliff_content(content: &str) -> Option<HashMap<String, String>> {
    match xml_from_str::<Xliff>(content) {
        Ok(xliff) => {
            let mut strings = HashMap::new();
            for trans_unit in xliff.file.body.trans_units {
                let text = if let Some(target) = trans_unit.target {
                    target.content
                } else {
                    trans_unit.source.content
                };
                strings.insert(trans_unit.id, text);
            }
            Some(strings)
        }
        Err(err) => {
            warn!("Failed to parse XLIFF content: {err:?}");
            None
        }
    }
}

fn parse_po_content(content: &str) -> Option<HashMap<String, String>> {
    // Simple PO file parser - parse msgid/msgstr pairs
    let mut strings = HashMap::new();
    let mut current_msgid = String::new();
    let mut current_msgstr = String::new();
    let mut in_msgid = false;
    let mut in_msgstr = false;
    
    for line in content.lines() {
        let line = line.trim();
        
        if line.starts_with("msgid ") {
            // Save previous entry if we have one
            if !current_msgid.is_empty() {
                strings.insert(current_msgid.clone(), if current_msgstr.is_empty() { current_msgid.clone() } else { current_msgstr.clone() });
            }
            
            current_msgid = parse_po_string(&line[6..]);
            current_msgstr.clear();
            in_msgid = true;
            in_msgstr = false;
        } else if line.starts_with("msgstr ") {
            current_msgstr = parse_po_string(&line[7..]);
            in_msgid = false;
            in_msgstr = true;
        } else if line.starts_with("\"") && (in_msgid || in_msgstr) {
            let parsed = parse_po_string(line);
            if in_msgid {
                current_msgid.push_str(&parsed);
            } else if in_msgstr {
                current_msgstr.push_str(&parsed);
            }
        } else if line.is_empty() || line.starts_with('#') {
            in_msgid = false;
            in_msgstr = false;
        }
    }
    
    // Save the last entry
    if !current_msgid.is_empty() {
        strings.insert(current_msgid.clone(), if current_msgstr.is_empty() { current_msgid } else { current_msgstr });
    }
    
    if strings.is_empty() {
        None
    } else {
        Some(strings)
    }
}

fn parse_po_string(quoted: &str) -> String {
    let trimmed = quoted.trim();
    if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
        let content = &trimmed[1..trimmed.len()-1];
        // Basic unescape - handle \\\\ FIRST, then \\n, \\t, \\"
        content.replace("\\\\", "\\")
               .replace("\\n", "\n")
               .replace("\\t", "\t")
               .replace("\\\"", "\"")
    } else {
        trimmed.to_string()
    }
}



fn detect_system_locale() -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::Globalization::GetUserDefaultLocaleName;

        const MAX_LOCALE_LENGTH: usize = 85;
        let mut buffer = [0u16; MAX_LOCALE_LENGTH];
        let len = unsafe { GetUserDefaultLocaleName(&mut buffer) };
        if len > 0 {
            let slice = &buffer[..(len as usize - 1)];
            if let Ok(s) = String::from_utf16(slice) {
                return Some(normalize_locale_tag(&s));
            }
        }
        None
    }
    #[cfg(not(target_os = "windows"))]
    {
        for key in ["LC_ALL", "LC_MESSAGES", "LANG"] {
            if let Ok(val) = std::env::var(key) {
                if !val.is_empty() {
                    return Some(normalize_locale_tag(&val));
                }
            }
        }
        None
    }
}

fn normalize_locale_tag(input: &str) -> String {
    let lowered = input.trim().to_lowercase();
    let primary = lowered
        .split('.').next().unwrap_or("")
        .split('@').next().unwrap_or("")
        .replace('_', "-");

    let value = if primary == "c" || primary == "posix" || primary.is_empty() {
        "en".to_string()
    } else {
        primary
    };

    value
}

fn candidate_locales(locale: &str) -> Vec<String> {
    let mut candidates = Vec::new();
    if !locale.is_empty() {
        candidates.push(locale.to_string());
        if let Some((base, _)) = locale.split_once('-') {
            if !candidates.iter().any(|c| c == base) {
                candidates.push(base.to_string());
            }
        }
    }
    if !candidates.iter().any(|c| c == "en") {
        candidates.push("en".to_string());
    }
    candidates
}

fn apply_replacements(mut value: String, replacements: &[(&str, &str)]) -> String {
    for (key, replacement) in replacements {
        let token = format!("{{{}}}", key);
        value = value.replace(&token, replacement);
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_po_content() {
        let po_content = r#"# Comment
msgid ""
msgstr ""
"Content-Type: text/plain; charset=UTF-8\n"

msgid "restart_required.title"
msgstr "Restart Required"

msgid "restart_required.instruction"
msgstr "Please restart your application"

msgid "multiline"
msgstr ""
"This is a "
"multiline string"

msgid "escape_test"
msgstr "Line 1\nLine 2\tTabbed"
"#;

        let result = parse_po_content(po_content).unwrap();
        
        assert_eq!(result.get("restart_required.title"), Some(&"Restart Required".to_string()));
        assert_eq!(result.get("restart_required.instruction"), Some(&"Please restart your application".to_string()));
        assert_eq!(result.get("multiline"), Some(&"This is a multiline string".to_string()));
        assert_eq!(result.get("escape_test"), Some(&"Line 1\nLine 2\tTabbed".to_string()));
    }

    #[test]
    fn test_parse_xliff_content() {
        let xliff_content = r#"<?xml version="1.0" encoding="utf-8"?>
<xliff xmlns="urn:oasis:names:tc:xliff:document:1.2" version="1.2">
  <file original="test" source-language="en" target-language="fr">
    <body>
      <trans-unit id="restart_required.title">
        <source xml:lang="en">Restart Required</source>
        <target xml:lang="fr">Redémarrage requis</target>
      </trans-unit>
      <trans-unit id="restart_required.instruction">
        <source xml:lang="en">Please restart</source>
        <target xml:lang="fr">Veuillez redémarrer</target>
      </trans-unit>
    </body>
  </file>
</xliff>"#;

        let result = parse_xliff_content(xliff_content).unwrap();
        
        assert_eq!(result.get("restart_required.title"), Some(&"Redémarrage requis".to_string()));
        assert_eq!(result.get("restart_required.instruction"), Some(&"Veuillez redémarrer".to_string()));
    }

    #[test]
    fn test_parse_xliff_content_without_target() {
        let xliff_content = r#"<?xml version="1.0" encoding="utf-8"?>
<xliff xmlns="urn:oasis:names:tc:xliff:document:1.2" version="1.2">
  <file original="test" source-language="en">
    <body>
      <trans-unit id="test.key">
        <source xml:lang="en">Source Text</source>
      </trans-unit>
    </body>
  </file>
</xliff>"#;

        let result = parse_xliff_content(xliff_content).unwrap();
        assert_eq!(result.get("test.key"), Some(&"Source Text".to_string()));
    }

    #[test]
    fn test_extract_locale_from_filename() {
        // filename-based detection
        assert_eq!(extract_locale_from_filename("localization/en/velopack.po"), Some("en".to_string()));
        assert_eq!(extract_locale_from_filename("localization/dialogs.fr-CA.xliff"), Some("fr-ca".to_string()));
        assert_eq!(extract_locale_from_filename("localization/de.po"), Some("de".to_string()));
        assert_eq!(extract_locale_from_filename("localization/dialogs.json"), Some("dialogs".to_string()));
        assert_eq!(extract_locale_from_filename("invalid"), Some("invalid".to_string()));
        assert_eq!(extract_locale_from_filename(""), None);

        // folder-based detection: localization/<culture>/file
        assert_eq!(extract_locale_from_filename("localization/en/velopack.xlf"), Some("en".to_string()));
        assert_eq!(extract_locale_from_filename("localization/fr/velopack.po"), Some("fr".to_string()));
        assert_eq!(extract_locale_from_filename("x/localization/pt-BR/velopack.xlf"), Some("pt-br".to_string()));
    }

    #[test]
    fn test_normalize_locale_tag() {
        assert_eq!(normalize_locale_tag("en_US"), "en-us");
        assert_eq!(normalize_locale_tag("fr_CA.UTF-8"), "fr-ca");
        assert_eq!(normalize_locale_tag("C"), "en");
        assert_eq!(normalize_locale_tag("POSIX"), "en");
        assert_eq!(normalize_locale_tag("zh-CN"), "zh-cn");
        assert_eq!(normalize_locale_tag("  EN_GB  "), "en-gb");
    }

    #[test]
    fn test_candidate_locales() {
        assert_eq!(candidate_locales("fr-ca"), vec!["fr-ca", "fr", "en"]);
        assert_eq!(candidate_locales("en"), vec!["en"]);
        assert_eq!(candidate_locales("de"), vec!["de", "en"]);
        assert_eq!(candidate_locales(""), vec!["en"]);
    }

    #[test]
    fn test_parse_po_string() {
        assert_eq!(parse_po_string("\"Hello World\""), "Hello World");
        assert_eq!(parse_po_string("\"Line 1\\nLine 2\""), "Line 1\nLine 2");
        assert_eq!(parse_po_string("\"Tab\\tTest\""), "Tab\tTest");
        assert_eq!(parse_po_string("\"Quote \\\"test\\\"\""), "Quote \"test\"");
        // Test that \\\\ in PO becomes \\ in output (literal backslash)
        assert_eq!(parse_po_string(r#""Backslash \\est""#), "Backslash \\est");
        assert_eq!(parse_po_string("unquoted"), "unquoted");
    }

    #[test]
    fn test_apply_replacements() {
        assert_eq!(
            apply_replacements("Hello {name}!".to_string(), &[("name", "World")]),
            "Hello World!"
        );
        assert_eq!(
            apply_replacements("{app} version {version}".to_string(), &[("app", "MyApp"), ("version", "1.0.0")]),
            "MyApp version 1.0.0"
        );
        assert_eq!(
            apply_replacements("No replacements".to_string(), &[]),
            "No replacements"
        );
    }


}
