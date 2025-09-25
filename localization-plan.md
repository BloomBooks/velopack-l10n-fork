# Velopack Localization Plan

This document outlines the user-facing dialog boxes that users see during app insta## Build-Time Flow

1. [x] **Folder Layout** – App developers create a localization root (e.g. `./my-locales`) with subfolders per culture (`en`, `en-GB`, `fr`, etc.). Inside each folder is either a `dialogs.xliff`, `dialogs.po`, or culture-specific file like `fr.xliff` or `de.po`. Only the cultures supplied by the app are embedded.
2. [x] **CLI Flag** – `vpk pack` gains `--localization <path>`; when omitted we ship just the built-in English bundle.
3. [x] **Validation** – The pack command reads every supplied PO/XLIFF file, verifies required keys exist, and merges the provided `en` entries on top of Velopack's defaults to honor overrides.
4. [x] **Embedding** – The validated payload is serialized as individual XLIFF files and stored in the package manifest so every platform's installer can read the localization data.n and updates, along with the implemented PO and XLIFF-based localization system.

## Overview

This plan focuses specifically on the native setup/update dialogs that end users encounter when installing or updating applications built with Velopack. The system now supports industry-standard PO (GNU gettext) and XLIFF (XML Localization Interchange File Format) files.

## Implementation Status

- [x] **Core Localization System** - Rust localization framework with PO/XLIFF bundle loading and locale detection
- [x] **Windows Dialog Integration** - All Windows dialogs converted to use localization system
- [x] **C# Build Integration** - LocalizationBundleBuilder for build-time validation and bundle creation with PO/XLIFF support
- [x] **CLI Integration** - `--localization` flag support in pack command
- [x] **Default English Strings** - Complete dialogs.en.xliff with all required dialog templates
- [x] **PO File Support** - Parse GNU gettext PO files for localization strings
- [x] **XLIFF File Support** - Parse XLIFF 1.2 files for professional translation workflows
- [ ] **Multi-locale Testing** - Requires broader platform testing
- [x] **Documentation** - End-user localization guide

## Native Setup/Update Dialogs (Rust - `src/bins/`)

### Windows (`src/bins/src/shared/dialogs_windows.rs`)

#### Dialog Functions Requiring Localization:

- `show_restart_required()`

  - Dialog title: "{app.title} Setup {app.version}"
  - Main instruction: "Restart Required"
  - Content: "A restart is required before Setup can continue. Please restart your computer and try again."

- `show_update_missing_dependencies_dialog()`

  - Dialog title: "{app.title} Update {to_version}"
  - Main instruction: "Missing Dependencies"
  - Content: Template for dependency requirements and version updates
  - Button text: "Download & Install" / "Cancel"

- `show_setup_missing_dependencies_dialog()`

  - Dialog title: "{app.title} Setup {app.version}"
  - Main instruction: "Missing Dependencies"
  - Content: Template for dependency requirements
  - Button text: "Download & Install" / "Cancel"

- `show_uninstall_complete_with_errors_dialog()`

  - Dialog title: "{app.title} Uninstall"
  - Main instruction: "{app.title} uninstall has completed with errors."
  - Content: "There may be left-over files or directories on your system..."
  - Footer: Optional log file path display

- `show_processes_locking_folder_dialog()`

  - Dialog title: "{app.title} Update {app.version}"
  - Main instruction: "Application is currently running"
  - Content: Template for process names and update instructions
  - Button texts:
    - "Retry\nClose the program(s) and try again"
    - "Continue\nAttempt to close the program(s) automatically"
    - "Cancel\nThe update will not continue"

- `show_overwrite_repair_dialog()`
  - Dialog title: "{app.title} Setup {app.version}"
  - Dynamic main instructions based on version comparison:
    - "An older version of {app.title} is installed."
    - "A newer version of {app.title} is installed."
    - "{app.title} is already installed."
  - Dynamic content based on version scenarios:
    - Update: "Would you like to update from {old_version} to {new_version}?"
    - Downgrade: "You already have {old_version} installed. Would you like to downgrade this application to an older version?"
    - Repair: "This application is installed on your computer. If it is not functioning correctly, you can attempt to repair it."
  - Dynamic button texts:
    - "Update\nTo version {version}"
    - "Downgrade\nTo version {version}"
    - "Repair\nErase the application and re-install version {version}"
    - "Cancel\nBackup or save your work first"
  - Footer: Install directory path display

#### Generic Dialog Functions:

- `generate_confirm()` - Generic confirmation dialog
- `generate_alert()` - Generic alert dialog

### macOS (`src/bins/src/shared/dialogs_osx.rs`)

#### Dialog Functions:

- `generate_alert()` - macOS native alert dialogs
- `generate_confirm()` - macOS native confirmation dialogs
- Button text: "Ok", "Yes", "No"

### Linux (`src/bins/src/shared/dialogs_linux.rs`)

#### Dialog Functions:

- `generate_alert()` - Linux native message dialogs
- `generate_confirm()` - Linux native question dialogs
- Uses system dialog choices: "Cancel", "No", "Yes"

---

# Embedded Localization Plan

## Goals

- Accept any localization folder supplied to the CLI without hard-coded paths.
- Embed those files once in a platform-agnostic way so every installer format uses the same payload.
- Load the system locale at install time, fall back to the packaged English strings if that locale is missing.
- Allow app-provided `en` translations to override Velopack's built-in English defaults.
- Keep the data format simple (JSON today, could be something denser later) and note that the string set is tiny, so size growth is negligible.

## Build-Time Flow

1. **Folder Layout** – App developers create a localization root (e.g. `./my-locales`) with subfolders per culture (`en`, `en-GB`, `fr`, etc.). Inside each folder is a single `dialogs.json` file using the schema below. Only the cultures supplied by the app are embedded.
2. **CLI Flag** – `vpk pack` gains `--localization <path>`; when omitted we ship just the built-in English bundle.
3. **Validation** – The pack command reads every supplied JSON file, verifies required keys exist, and merges the provided `en` (or `en-US`) entries on top of Velopack's defaults to honor overrides.
4. **Embedding** – The validated payload is serialized into a single archive (JSON blob or gzipped JSON) and stored in the package manifest so every platform’s installer can read the identical binary chunk.

## Installer Runtime Flow

1. [x] **Locale Detection** – On startup, the installer reads the OS locale (Windows `GetUserDefaultLocaleName`, macOS `CFLocaleCopyCurrent`, Linux environment variables). No user override UI is provided.
2. [x] **Data Load** – The embedded archive is deserialized in-memory. Only the detected culture and the packaged English set are materialized; if the culture is missing we fall back to English.
3. [x] **Lookup** – Dialog builders request strings by key. If a key is missing in the active culture we immediately pull the English value.
4. [x] **Formatting** – Strings continue to use positional placeholders (`{0}`, `{1}`) so existing formatting calls remain unchanged.

## Supported File Formats

### XLIFF Format (Recommended for Professional Translation)

XLIFF (XML Localization Interchange File Format) is the preferred format for professional translation workflows. Example `dialogs.fr.xliff`:

```xml
<?xml version="1.0" encoding="utf-8"?>
<xliff xmlns="urn:oasis:names:tc:xliff:document:1.2" version="1.2">
  <file original="velopack.exe" source-language="en" target-language="fr" datatype="plaintext">
    <body>
      <trans-unit id="dialogs.restart_required.instruction">
        <source xml:lang="en">Restart Required</source>
        <target xml:lang="fr">Redémarrage Requis</target>
        <note>Restart required dialog instruction</note>
      </trans-unit>
      <trans-unit id="dialogs.restart_required.content">
        <source xml:lang="en">A restart is required before Setup can continue. Please restart your computer and try again.</source>
        <target xml:lang="fr">Un redémarrage est requis avant que l'installation puisse continuer. Veuillez redémarrer votre ordinateur et réessayer.</target>
        <note>Restart required dialog content</note>
      </trans-unit>
      <trans-unit id="buttons.ok">
        <source xml:lang="en">Ok</source>
        <target xml:lang="fr">D'accord</target>
        <note>Standard OK button</note>
      </trans-unit>
    </body>
  </file>
</xliff>
```

### PO Format (GNU gettext)

PO files are widely used in open source projects and support simple key-value translation. Example `dialogs.fr.po`:

```po
# French translation for Velopack dialogs
# Translators: Your Name <your.email@example.com>
msgid ""
msgstr ""
"Content-Type: text/plain; charset=UTF-8\n"
"Language: fr\n"

msgid "dialogs.restart_required.instruction"
msgstr "Redémarrage Requis"

msgid "dialogs.restart_required.content"
msgstr "Un redémarrage est requis avant que l'installation puisse continuer. Veuillez redémarrer votre ordinateur et réessayer."

msgid "buttons.ok"
msgstr "D'accord"
```

### File Structure

App developers can organize localization files in their project:

```
my-locales/
├── en/
│   └── dialogs.xliff          # English overrides (optional)
├── fr/
│   └── dialogs.xliff          # French translations
├── de/
│   └── dialogs.po             # German translations (PO format)
└── es/
    └── es.xliff               # Spanish translations (culture-named file)
```

## Notes and Constraints

- **Tiny Payload** – The dialog set is small (dozens of strings), so even with multiple locales the archive remains lightweight.
- **Single Implementation** – The embedding/reading code lives in shared Rust modules so Windows/macOS/Linux reuse it directly.
- **Backward Compat** – If no localization folder is provided we ship only the built-in English file, preserving current behavior.
- **Error Reporting** – Build fails fast with readable errors when JSON is invalid or required keys are missing.
