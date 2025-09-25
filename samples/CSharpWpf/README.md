# VeloWpfSample

_Prerequisites: vpk command line tool installed_

This app demonstrates how to use WPF to provide a desktop UI, installer, and updates for Windows only.

You can run this sample by executing the build script with a version number: `build.bat 1.0.0`. Once built, you can install the app - build more updates, and then test updates and so forth. The sample app will check the local release dir for new update packages.

This sample also demonstrates **localization** - the installer dialogs will appear in French when run on a French system, thanks to the `--localization` flag in the build scripts and the French translation files in the `localization/` folder.

In your production apps, you should deploy your updates to some kind of update server instead.

## Localization

This sample demonstrates Velopack's localization feature with translations for installer dialogs in multiple languages and formats. The localization files are organized by culture in the `localization/` folder:

- `localization/fr/velopack.po` - French installer dialogs (PO format)
- `localization/es/velopack.xlf` - Spanish installer dialogs (XLIFF format)

The build scripts use the `--localization` flag to include these translations in the installer package. When users install or update the app on a French or Spanish system, they'll see localized installer dialogs instead of English.

### Quick Verification

```bash
# Check that localization is set up correctly
bash verify-localization.sh
```

### Testing Localization

**Method 1 (Recommended):** Change Windows display language to French or Spanish in Settings > Time & Language > Language, then run the installer.

**Method 2:** Use PowerShell culture override (may not work for all installer types):

```powershell
[System.Globalization.CultureInfo]::CurrentUICulture = 'fr-FR'
Start-Process '.\releases\VelopackCSharpWpf-win-Setup.exe'
```

**Expected Results:**

- **French:** Window title "Installation de CSharpWpf", buttons "Installer", "Oui", "Non"
- **Spanish:** Window title "Instalación de CSharpWpf", buttons "Instalar", "Sí", "No"

### Complete Documentation

For comprehensive localization documentation including supported formats, platform requirements, and translation workflows, see: **[README-localization.md](../../README-localization.md)**

## WPF Implementation Notes

WPF generates a `Program.Main(argv[])` method automatically for you, so it requires a couple of extra steps to get Velopack working with WPF.

1. You need to create your own `Program.cs` class, and add a static `Main()` method.
2. In order for dotnet to execute this new Main() method instead of the default WPF one, you need to add the following to your .csproj:
   ```xml
   <PropertyGroup>
     <StartupObject>YourNamespace.Program</StartupObject>
   </PropertyGroup>
   ```
3. You should run the `VelopackApp` builder before starting WPF as usual.
   ```cs
   [STAThread]
   public static void Main(string[] args)
   {
       VelopackApp.Build().Run();
       var application = new App();
       application.InitializeComponent();
       application.Run();
   }
   ```
