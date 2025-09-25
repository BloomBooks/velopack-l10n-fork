# Velopack Localization Guide

We have made this fork so that we can localize while waiting for the official Velopack solution. See https://github.com/velopack/velopack/issues/60.

## Overview

Velopack supports localizing installer dialogs (setup, update, error messages, etc.) to provide a native experience for users in different regions. The installer automatically detects the system locale at runtime and displays the appropriate language.

## Platform Support

**Supported Platforms:**

- ✅ Windows with .NET 8.0 or later installers
- ❌ macOS (not currently supported)
- ❌ Linux (not currently supported)

**Note:** Localization is currently only available for Windows installers built with .NET 8.0 or later. macOS and Linux support may be added in future versions.

## Supported File Formats

Velopack supports two standard localization file formats:

- **PO format:** `.po` extension (GNU gettext Portable Object)
- **XLIFF format:** `.xlf` extension (XML Localization Interchange File Format)

**Important:**

- Each culture directory should contain either a `velopack.po` OR `velopack.xlf` file (not both)
- XLIFF files take precedence over PO files if both exist in the same directory
- File names must be exactly `velopack.po` or `velopack.xlf`

## Getting Started

### 1. Create Default Localization Files

Copy the default English localization file from the Velopack source to use as translation templates, either xlf or po. Put this in your project such that you have something like

```
localization/
├── en/
│   ├── velopack.po      # English PO format (optional override)
├── fr/
│   └── velopack.xlf   # French translations (XLIFF format)
├── de/
│   └── velopack.po      # German translations
```

### 3. Use in Packaging

Add the `--localization` flag when packaging your application:

```bash
vpk pack -u MyApp -v 1.0.0 -p ./publish --localization ./localization
```

## Runtime Behavior

### Locale Detection

The installer automatically detects the user's system locale using Windows' display language settings. The detection follows this priority:

1. Current user's display language
2. System default language
3. Fallback to English if no translation exists

### Testing Different Locales

To test your localized installer on a system with a different language:

#### Method 1: Change Windows Display Language

1. Open Windows Settings → Time & Language → Language & Region
2. Add your target language and set it as display language
3. Sign out and sign back in
4. Run your installer

#### Method 2: PowerShell Locale Simulation

```powershell
# Set culture to French and launch installer
[System.Globalization.CultureInfo]::CurrentUICulture = [System.Globalization.CultureInfo]::new("fr-FR")
[System.Threading.Thread]::CurrentThread.CurrentUICulture = [System.Globalization.CultureInfo]::new("fr-FR")
Start-Process "path\to\your-installer.exe"
```

#### Method 3: Environment Variables

```batch
set LANG=fr-FR
set LC_ALL=fr-FR
your-installer.exe
```

**Note:** The PowerShell and environment variable methods may not work for all installer types. Changing the Windows display language is the most reliable testing method.

## Troubleshooting

### Localization Not Working

1. **Check file structure:** Ensure files are in correct `culture/velopack.[po|xlf]` structure
2. **Verify file encoding:** Must be UTF-8
3. **Check culture codes:** Use standard codes like `fr`, `de`, `es-ES`
4. **Test on target locale:** Change Windows display language for testing

### Build Errors

- Ensure the localization directory exists before packaging
- Check for syntax errors in PO/XLIFF files
- Verify all required translation keys are present

### Missing Translations

- If a translation key is missing, English text will be displayed
- Check console output during packaging for validation warnings
- Ensure all strings from the template file are translated
