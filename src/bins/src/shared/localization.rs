use std::collections::HashMap;
use std::sync::RwLock;

use lazy_static::lazy_static;
use quick_xml::de::from_str as xml_from_str;
use serde::{Deserialize, Serialize};
use velopack::bundle::BundleZip;

const DEFAULT_PO_CONTENT: &str = include_str!("../../../localization/en/velopack.po");

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
    // Built-in string tables that ship with the binary (currently only English).
    static ref FACTORY_LOCALES: HashMap<String, HashMap<String, String>> = {
        let mut map = HashMap::new();
        let english = parse_po_content(DEFAULT_PO_CONTENT)
            .expect("default localization is valid po");
        map.insert("en".to_string(), english);
        map
    };
    static ref STATE: RwLock<LocalizationState> = RwLock::new(LocalizationState::new());
}

#[cfg(test)]
lazy_static! {
    static ref TEST_LOCALE_OVERRIDE: RwLock<Option<String>> = RwLock::new(None);
}

#[cfg(test)]
fn set_test_locale_override(locale: Option<&str>) {
    let mut guard = TEST_LOCALE_OVERRIDE
        .write()
        .expect("test locale override poisoned");
    *guard = locale.map(|value| normalize_locale_tag(value));
}

#[derive(Clone)]
struct LocalizationState {
    default_english_strings: HashMap<String, String>,
    english_strings: HashMap<String, String>,
    active_strings: HashMap<String, String>,
    active_locale: String,
    available_locales: Vec<String>,
}

impl LocalizationState {
    fn new() -> Self {
        let base = FACTORY_LOCALES.get("en").expect("factory locales always contain english").clone();
        Self {
            default_english_strings: base.clone(),
            english_strings: base.clone(),
            active_strings: base,
            active_locale: "en".to_string(),
            available_locales: Vec::new(),
        }
    }

    fn reset(&mut self) {
        self.english_strings = self.default_english_strings.clone();
        self.active_strings = self.default_english_strings.clone();
        self.active_locale = "en".to_string();
        self.available_locales.clear();
    }

    /// Refreshes the localization state from an optional map of bundle strings.
    ///
    /// * `bundle_data` is an optional `HashMap` keyed by normalized locale tag where each value is the raw
    ///   contents of a `.po`, `.xlf`, or `.xliff` localization file shipped with the installer bundle.
    ///
    /// The algorithm performs these steps:
    /// 1. Reset the state to the built-in English strings.
    /// 2. Collect the list of available locale tags surfaced by the bundle, always including `en`.
    /// 3. Allow bundled English strings to override the factory defaults.
    /// 4. Detect the current system locale and walk the candidate locale list (locale, language fallback, then `en`).
    /// 5. For each candidate, merge factory-provided strings and bundle-provided strings, choosing the first locale
    ///    that supplies translations; otherwise fall back to English.
    /// 6. Record the active locale, available locales, and the merged string tables for later lookups, logging the
    ///    outcome for observability.
    fn load_bundle(&mut self, bundle_data: Option<&HashMap<String, String>>) {
        self.reset();

        // Build the set of locale tags the caller supplied so UI can surface the choices later.
        let mut available_locale_list: Vec<String> = bundle_data.map(|data| data.keys().cloned().collect()).unwrap_or_default();
        if !available_locale_list.iter().any(|locale| locale == "en") {
            available_locale_list.push("en".to_string());
        }
        available_locale_list.sort();
        available_locale_list.dedup();

        // Allow the bundle to override built-in English strings before we merge any other locale.
        if let Some(en_override) = load_ui_strings(bundle_data, "en") {
            self.english_strings = merge_string_maps(&self.english_strings, &en_override);
        }

        let detected_locale = detect_system_locale().unwrap_or_else(|| "en".to_string());
        let candidate_locale_list = candidate_locales(&detected_locale);

        let mut selected_locale = "en".to_string();
        let mut selected_strings = self.english_strings.clone();

        for candidate_locale in candidate_locale_list {
            // A literal "en" candidate is a hard stop: we already have the English baseline.
            if candidate_locale == "en" {
                selected_locale = "en".to_string();
                selected_strings = self.english_strings.clone();
                break;
            }

            let mut candidate_strings = self.english_strings.clone();

            // First merge the bundled-in factory strings, if we ship any for this locale.
            if let Some(factory_strings) = load_factory_locale(&candidate_locale) {
                candidate_strings = merge_string_maps(&candidate_strings, &factory_strings);
            }

            // Then merge user-provided strings from the bundle. If present we take this locale.
            if let Some(bundle_strings) = load_ui_strings(bundle_data, &candidate_locale) {
                candidate_strings = merge_string_maps(&candidate_strings, &bundle_strings);
                selected_locale = candidate_locale.clone();
                selected_strings = candidate_strings;
                break;
            } else if candidate_strings != self.english_strings {
                // No bundle strings, but we do have factory strings – use them as a fallback.
                selected_locale = candidate_locale.clone();
                selected_strings = candidate_strings;
                break;
            }
        }

        self.available_locales = available_locale_list;
        self.active_locale = selected_locale;
        self.active_strings = selected_strings;

        if bundle_data.is_some() {
            if self.active_locale != "en" {
                info!("Loaded localization bundle for locale '{}'. Available locales: {:?}", self.active_locale, self.available_locales);
            } else {
                info!(
                    "Localization bundle supplied but active locale '{}' not found. Falling back to English. Available locales: {:?}",
                    detected_locale, self.available_locales
                );
            }
        } else {
            debug!("No localization bundle present. Using built-in English strings.");
        }
    }

    fn get(&self, string_id: &str) -> Option<String> {
        self
            .active_strings
            .get(string_id)
            .cloned()
            .or_else(|| self.english_strings.get(string_id).cloned())
    }
}

/// Initializes the global localization state by extracting localization files from the provided bundle and
/// selecting the most appropriate locale for the current system.
///
/// * `bundle` is the installer bundle whose embedded localization files will be parsed and applied.
pub fn initialize_from_bundle(bundle: &BundleZip) {
    let bundle_data = load_localization_files_from_bundle(bundle);

    let mut state = STATE.write().expect("localization state poisoned");
    state.load_bundle(bundle_data.as_ref());
}

/// Returns the localized string for the given resource key using the active locale, falling back to English if
/// necessary. Returns `None` if the key does not exist in either table.
pub fn text(string_id: &str) -> Option<String> {
    let state = STATE.read().expect("localization state poisoned");
    state.get(string_id)
}

/// Retrieves a localized string and applies placeholder replacements shaped like `{key}` using the provided
/// `(key, value)` pairs. Returns `None` if the resource key is unknown.
pub fn text_with(string_id: &str, replacements: &[(&str, &str)]) -> Option<String> {
    text(string_id).map(|base| apply_replacements(base, replacements))
}

/// Fetches the localized string for `string_id`, or returns the supplied default when the key is missing in both the
/// active and English tables.
pub fn text_or_default(string_id: &str, default: &str) -> String {
    text(string_id).unwrap_or_else(|| default.to_string())
}

/// Retrieves the localized string for `string_id`, applies replacements, and falls back to the provided default (with the
/// same replacements) when the key is absent.
pub fn text_with_default(string_id: &str, replacements: &[(&str, &str)], default: &str) -> String {
    text_with(string_id, replacements).unwrap_or_else(|| apply_replacements(default.to_string(), replacements))
}

fn merge_string_maps(base: &HashMap<String, String>, overlay: &HashMap<String, String>) -> HashMap<String, String> {
    let mut result = base.clone();
    result.extend(overlay.iter().map(|(key, value)| (key.clone(), value.clone())));
    result
}

fn load_ui_strings(bundle_data: Option<&HashMap<String, String>>, locale: &str) -> Option<HashMap<String, String>> {
    let content = bundle_data?.get(locale)?;
    match parse_ui_string_content(content) {
        Some(strings) => Some(strings),
        None => {
            warn!("Failed to parse localization data for locale '{}'. Skipping.", locale);
            None
        }
    }
}

fn load_factory_locale(locale: &str) -> Option<HashMap<String, String>> {
    // Fetch a pre-parsed factory string table if we ship one for the requested locale.
    FACTORY_LOCALES.get(locale).cloned()
}

fn parse_ui_string_content(content: &str) -> Option<HashMap<String, String>> {
    // Try XLIFF first because it preserves context, then fall back to PO parsing.
    parse_xliff_content(content).or_else(|| parse_po_content(content))
}

fn load_localization_files_from_bundle(bundle: &BundleZip) -> Option<HashMap<String, String>> {
    let mut files = HashMap::new();

    for extension in [".xlf", ".xliff"] {
        for (name, content) in find_localization_files(bundle, &[extension]) {
            if let Some(locale) = extract_locale_from_filename(&name) {
                files.insert(locale, content);
            }
        }
    }

    for (name, content) in find_localization_files(bundle, &[".po"]) {
        if let Some(locale) = extract_locale_from_filename(&name) {
            files.entry(locale).or_insert(content);
        }
    }

    if files.is_empty() {
        None
    } else {
        Some(files)
    }
}

fn find_localization_files(bundle: &BundleZip, extensions: &[&str]) -> Vec<(String, String)> {
    let mut files = Vec::new();

    if let Ok(file_names) = bundle.get_file_names() {
        for file_name in file_names {
            if file_name.starts_with("localization/") {
                let lower_name = file_name.to_ascii_lowercase();
                if extensions.iter().any(|extension| lower_name.ends_with(extension)) {
                    if let Ok(content) = bundle.read_zip_predicate_to_string(|name| name == file_name) {
                        files.push((file_name, content));
                    }
                }
            }
        }
    }

    files
}

fn extract_locale_from_filename(filename: &str) -> Option<String> {
    use std::path::Component;
    let path = std::path::Path::new(filename);

    let extension = path.extension().and_then(|ext| ext.to_str()).map(|ext| ext.to_ascii_lowercase())?;

    if !matches!(extension.as_str(), "po" | "xlf" | "xliff") {
        return None;
    }

    // 1) Support folder layout: localization/<culture>/<file>
    // Find "localization" in the path components and take the next component as the culture folder if present
    let mut comps = path.components().peekable();
    while let Some(comp) = comps.next() {
        if let Component::Normal(os) = comp {
            if os == "localization" {
                if let Some(Component::Normal(locale_os)) = comps.peek() {
                    if let Some(locale_str) = locale_os.to_str() {
                        if !locale_str.contains('.') && !locale_str.is_empty() {
                            let normalized = normalize_locale_tag(locale_str);
                            if is_valid_locale_tag(&normalized) {
                                return Some(normalized);
                            }
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
                    let normalized = normalize_locale_tag(locale);
                    if is_valid_locale_tag(&normalized) {
                        return Some(normalized);
                    }
                }
            }

            let normalized = normalize_locale_tag(stem_str);
            if is_valid_locale_tag(&normalized) {
                return Some(normalized);
            }
        }
    }
    None
}

fn parse_xliff_content(content: &str) -> Option<HashMap<String, String>> {
    match xml_from_str::<Xliff>(content) {
        Ok(xliff) => {
            let mut strings = HashMap::new();
            for trans_unit in xliff.file.body.trans_units {
                let text = if let Some(target) = trans_unit.target { target.content } else { trans_unit.source.content };
                // Convert literal \n sequences to actual newlines
                let processed_text = text.replace("\\n", "\n");
                strings.insert(trans_unit.id, processed_text);
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
                let final_text = if current_msgstr.is_empty() { current_msgid.clone() } else { current_msgstr.clone() };
                // Convert literal \n sequences to actual newlines
                let processed_text = final_text.replace("\\n", "\n");
                strings.insert(current_msgid.clone(), processed_text);
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
        let final_text = if current_msgstr.is_empty() { current_msgid } else { current_msgstr };
        // Convert literal \n sequences to actual newlines. The ui uses this for multi-line strings.
        let processed_text = final_text.replace("\\n", "\n");
        strings.insert(current_msgid.clone(), processed_text);
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
        let content = &trimmed[1..trimmed.len() - 1];
        // Basic unescape - handle \\\\ FIRST, then \\n, \\t, \\"
        content.replace("\\\\", "\\").replace("\\n", "\n").replace("\\t", "\t").replace("\\\"", "\"")
    } else {
        trimmed.to_string()
    }
}

fn detect_system_locale() -> Option<String> {
    #[cfg(test)]
    {
        if let Some(locale) = TEST_LOCALE_OVERRIDE
            .read()
            .expect("test locale override poisoned")
            .clone()
        {
            return Some(locale);
        }
    }

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
    let primary = lowered.split('.').next().unwrap_or("").split('@').next().unwrap_or("").replace('_', "-");

    let value = if primary == "c" || primary == "posix" || primary.is_empty() { "en".to_string() } else { primary };

    value
}

fn is_valid_locale_tag(tag: &str) -> bool {
    let mut parts = tag.split('-').peekable();
    let Some(first) = parts.next() else {
        return false;
    };

    let first_valid = if (first.len() == 2 || first.len() == 3) && first.chars().all(|c| c.is_ascii_lowercase()) {
        true
    } else if first.len() == 4 && first.chars().all(|c| c.is_ascii_lowercase()) && parts.peek().is_some() {
        true
    } else {
        false
    };

    if !first_valid {
        return false;
    }

    for part in parts {
        if part.is_empty() || part.len() > 8 || !part.chars().all(|c| c.is_ascii_alphanumeric()) {
            return false;
        }
    }

    true
}

fn candidate_locales(locale: &str) -> Vec<String> {
    let mut candidate_locale_list = Vec::new();
    if !locale.is_empty() {
        candidate_locale_list.push(locale.to_string());
        if let Some((base, _)) = locale.split_once('-') {
            if !candidate_locale_list.iter().any(|existing_locale| existing_locale == base) {
                candidate_locale_list.push(base.to_string());
            }
        }
    }
    if !candidate_locale_list.iter().any(|existing_locale| existing_locale == "en") {
        candidate_locale_list.push("en".to_string());
    }
    candidate_locale_list
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
    use std::collections::HashMap;
    use std::io::Write;
    use velopack::bundle::{load_bundle_from_memory, BundleZip};
    use zip::write::SimpleFileOptions;

    struct LocaleOverrideGuard;

    impl LocaleOverrideGuard {
        fn new(locale: &str) -> Self {
            set_test_locale_override(Some(locale));
            Self
        }
    }

    impl Drop for LocaleOverrideGuard {
        fn drop(&mut self) {
            set_test_locale_override(None);
        }
    }

    struct GlobalStateGuard {
        original: Option<LocalizationState>,
    }

    impl GlobalStateGuard {
        fn replace(state: LocalizationState) -> Self {
            let original = {
                let mut guard = STATE.write().expect("localization state poisoned");
                let original = guard.clone();
                *guard = state;
                original
            };

            Self {
                original: Some(original),
            }
        }
    }

    impl Drop for GlobalStateGuard {
        fn drop(&mut self) {
            if let Some(original) = self.original.take() {
                let mut guard = STATE.write().expect("localization state poisoned");
                *guard = original;
            }
        }
    }

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
        assert_eq!(extract_locale_from_filename("localization/es/FILE.XLF"), Some("es".to_string()));

        assert_eq!(extract_locale_from_filename("invalid"), None);
        assert_eq!(extract_locale_from_filename(""), None);

        // folder-based detection: localization/<culture>/file
        assert_eq!(extract_locale_from_filename("localization/en/velopack.xlf"), Some("en".to_string()));
        assert_eq!(extract_locale_from_filename("localization/fr/velopack.po"), Some("fr".to_string()));
        assert_eq!(extract_locale_from_filename("x/localization/pt-BR/velopack.xlf"), Some("pt-br".to_string()));
    }

    fn bundle_from_entries(entries: &[(&str, &str)]) -> BundleZip<'static> {
        let mut cursor = std::io::Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut cursor);
            for (name, content) in entries {
                writer.start_file(*name, SimpleFileOptions::default()).unwrap();
                writer.write_all(content.as_bytes()).unwrap();
            }
            writer.finish().unwrap();
        }

        let bytes = cursor.into_inner();
        let leaked = Box::leak(bytes.into_boxed_slice());
        load_bundle_from_memory(leaked).unwrap()
    }

    #[test]
    fn test_load_localization_files_supports_xliff_extension() {
        let xliff_content = "<xliff stub fr";
        let bundle = bundle_from_entries(&[("localization/fr/app.xliff", xliff_content)]);

        let files = load_localization_files_from_bundle(&bundle).unwrap();

        assert_eq!(files.get("fr"), Some(&xliff_content.to_string()));
    }

    #[test]
    fn test_load_localization_files_merges_xliff_and_po() {
        let xliff_content = "<xliff fr";
        let po_content = "msgid \"hello\"\nmsgstr \"bonjour\"";
        let bundle = bundle_from_entries(&[("localization/fr/app.xlf", xliff_content), ("localization/de/app.po", po_content)]);

        let files = load_localization_files_from_bundle(&bundle).unwrap();

        assert_eq!(files.get("fr"), Some(&xliff_content.to_string()));
        assert_eq!(files.get("de"), Some(&po_content.to_string()));
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
        assert_eq!(apply_replacements("Hello {name}!".to_string(), &[("name", "World")]), "Hello World!");
        assert_eq!(
            apply_replacements("{app} version {version}".to_string(), &[("app", "MyApp"), ("version", "1.0.0")]),
            "MyApp version 1.0.0"
        );
        assert_eq!(apply_replacements("No replacements".to_string(), &[]), "No replacements");
    }

    #[test]
    fn test_load_bundle_selects_best_matching_locale() {
        let _override_guard = LocaleOverrideGuard::new("fr-CA");

        let mut bundle_data = HashMap::new();
        bundle_data.insert(
            "en".to_string(),
            r#"msgid "dialogs.restart_required.instruction"
msgstr "Restart Required (Override)""#
                .to_string(),
        );
        bundle_data.insert(
            "fr".to_string(),
            r#"msgid "dialogs.restart_required.instruction"
msgstr "Redémarrage requis""#
                .to_string(),
        );

        let mut state = LocalizationState::new();
        state.load_bundle(Some(&bundle_data));

        assert_eq!(state.active_locale, "fr");
        assert_eq!(state.available_locales, vec!["en".to_string(), "fr".to_string()]);
        assert_eq!(
            state
                .english_strings
                .get("dialogs.restart_required.instruction"),
            Some(&"Restart Required (Override)".to_string())
        );
        assert_eq!(
            state
                .active_strings
                .get("dialogs.restart_required.instruction"),
            Some(&"Redémarrage requis".to_string())
        );
    }

    #[test]
    fn test_text_helpers_use_global_state() {
        let mut default_map = HashMap::new();
        default_map.insert("welcome".to_string(), "Welcome {name}".to_string());
        default_map.insert("fallback".to_string(), "Default Fallback".to_string());

        let mut active_map = HashMap::new();
        active_map.insert("welcome".to_string(), "Bonjour {name}".to_string());

        let mut state = LocalizationState::new();
        state.default_english_strings = default_map.clone();
        state.english_strings = default_map.clone();
        state.active_strings = active_map;
        state.available_locales = vec!["en".to_string(), "fr".to_string()];
        state.active_locale = "fr".to_string();

        let _state_guard = GlobalStateGuard::replace(state);

        assert_eq!(text("welcome"), Some("Bonjour {name}".to_string()));
        assert_eq!(text_with("welcome", &[("name", "Alice")]), Some("Bonjour Alice".to_string()));
        assert_eq!(text("fallback"), Some("Default Fallback".to_string()));
        assert_eq!(text_or_default("unknown", "Hi"), "Hi".to_string());
        assert_eq!(
            text_with_default("unknown", &[("name", "Alice")], "Hello {name}"),
            "Hello Alice".to_string()
        );
    }

    #[test]
    fn test_load_ui_strings_returns_none_for_invalid_content() {
        let mut bundle_data = HashMap::new();
        bundle_data.insert("fr".to_string(), "invalid content".to_string());

        assert!(load_ui_strings(Some(&bundle_data), "fr").is_none());
    }

    #[test]
    fn test_merge_string_maps_prefers_overlay() {
        let mut base = HashMap::new();
        base.insert("shared".to_string(), "base".to_string());
        base.insert("only_base".to_string(), "only base".to_string());

        let mut overlay = HashMap::new();
        overlay.insert("shared".to_string(), "overlay".to_string());
        overlay.insert("only_overlay".to_string(), "only overlay".to_string());

        let merged = merge_string_maps(&base, &overlay);

        assert_eq!(merged.get("shared"), Some(&"overlay".to_string()));
        assert_eq!(merged.get("only_base"), Some(&"only base".to_string()));
        assert_eq!(merged.get("only_overlay"), Some(&"only overlay".to_string()));
    }
}
