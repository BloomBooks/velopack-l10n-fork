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
- **XLIFF format:** `.xlf`/`.xliff` extension (XML Localization Interchange File Format)

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

### 2. Use in Packaging

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
