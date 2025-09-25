#!/bin/bash

echo "=== Velopack Localization Verification ==="
echo ""

installer_path="releases/VelopackCSharpWpf-win-Setup.exe"
en_po_file="localization/en/velopack.po"
en_xlf_file="localization/en/velopack.xlf"
fr_po_file="localization/fr/velopack.po"
fr_xlf_file="localization/fr/velopack.xlf"
es_po_file="localization/es/velopack.po"
es_xlf_file="localization/es/velopack.xlf"

echo "Checking localization setup..."

if [ -f "$installer_path" ]; then
    echo "  ✓ Installer exists: $(basename "$installer_path")"
else
    echo "  ✗ Installer missing - run build.bat 2.0.x first"
    exit 1
fi

if [ -f "$en_po_file" ] || [ -f "$en_xlf_file" ]; then
    if [ -f "$en_po_file" ]; then
        echo "  ✓ English localization: $en_po_file"
    else
        echo "  ✓ English localization: $en_xlf_file"
    fi
else
    echo "  ✓ English localization: Using built-in defaults"
fi

if [ -f "$fr_po_file" ] || [ -f "$fr_xlf_file" ]; then
    if [ -f "$fr_po_file" ]; then
        echo "  ✓ French localization: $fr_po_file (PO format)"
        # Show a sample of French content from PO file
        sample=$(grep -A1 'msgid "buttons.install"' "$fr_po_file" | grep msgstr | cut -d'"' -f2)
        if [ -n "$sample" ]; then
            echo "    Sample French text: '$sample'"
        fi
    elif [ -f "$fr_xlf_file" ]; then
        echo "  ✓ French localization: $fr_xlf_file (XLIFF format)"
        # Show a sample of French content from XLF file
        sample=$(grep -A1 'id="buttons.install"' "$fr_xlf_file" | grep '<target' | sed 's/.*<target[^>]*>\([^<]*\)<\/target>.*/\1/')
        if [ -n "$sample" ]; then
            echo "    Sample French text: '$sample'"
        fi
    fi
else
    echo "  ✗ French localization missing"
fi

if [ -f "$es_po_file" ] || [ -f "$es_xlf_file" ]; then
    if [ -f "$es_po_file" ]; then
        echo "  ✓ Spanish localization: $es_po_file (PO format)"
        # Show a sample of Spanish content from PO file
        sample=$(grep -A1 'msgid "buttons.install"' "$es_po_file" | grep msgstr | cut -d'"' -f2)
        if [ -n "$sample" ]; then
            echo "    Sample Spanish text: '$sample'"
        fi
    elif [ -f "$es_xlf_file" ]; then
        echo "  ✓ Spanish localization: $es_xlf_file (XLIFF format)"
        # Show a sample of Spanish content from XLF file
        sample=$(grep -A1 'id="buttons.install"' "$es_xlf_file" | grep '<target' | sed 's/.*<target[^>]*>\([^<]*\)<\/target>.*/\1/')
        if [ -n "$sample" ]; then
            echo "    Sample Spanish text: '$sample'"
        fi
    fi
else
    echo "  ✗ Spanish localization missing"
fi

echo ""
echo "✓ This sample demonstrates Velopack localization with multiple formats"
echo "✓ French (PO format) and Spanish (XLIFF format) translations will be embedded"
echo ""

echo "Testing Instructions:"
echo ""
echo "Method 1 - Change Windows Display Language (Recommended):"
echo "  1. Open Settings > Time and Language > Language and Region"
echo "  2. Add French or Spanish language and set as display language"
echo "  3. Sign out and sign back in"  
echo "  4. Run: ./releases/VelopackCSharpWpf-win-Setup.exe"
echo ""
echo "Expected Translations:"
echo "  • French: 'Installation de CSharpWpf', 'Installer'"
echo "  • Spanish: 'Instalación de CSharpWpf', 'Instalar'"
echo ""
echo "For more information, see: ../../README-localization.md"