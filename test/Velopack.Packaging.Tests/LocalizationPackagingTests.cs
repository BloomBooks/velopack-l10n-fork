using System.IO.Compression;
using System.Threading.Tasks;
using Velopack.Packaging;
using Velopack.Util;
using Velopack.Core.Abstractions;

namespace Velopack.Packaging.Tests;

public class LocalizationPackagingTests
{
  [Fact]
  public async Task Packaging_Includes_Localization_Files_In_Correct_Layout()
  {
    using var _ = TempUtil.GetTempDirectory(out var temp);
    var appDir = Path.Combine(temp, "app");
    Directory.CreateDirectory(appDir);

    // Minimal app content
    File.WriteAllText(Path.Combine(appDir, "MyApp.exe"), "stub");

    // Create localization root with culture subfolders
    var locRoot = Path.Combine(temp, "localization");
    Directory.CreateDirectory(Path.Combine(locRoot, "fr"));
    Directory.CreateDirectory(Path.Combine(locRoot, "es"));

    // French PO
    File.WriteAllText(Path.Combine(locRoot, "fr", "velopack.po"), "msgid \"buttons.install\"\nmsgstr \"Installer\"");
    // Spanish XLIFF (preferred)
    File.WriteAllText(Path.Combine(locRoot, "es", "velopack.xlf"), "<?xml version=\"1.0\"?><xliff version=\"1.2\" xmlns=\"urn:oasis:names:tc:xliff:document:1.2\"><file source-language=\"en\" target-language=\"es\"><body><trans-unit id=\"buttons.install\"><source>Install</source><target>Instalar</target></trans-unit></body></file></xliff>");

    // Build a release package using a concrete builder: use WindowsPackOptions for deterministic behavior
    var releases = Path.Combine(temp, "releases");
    Directory.CreateDirectory(releases);

    var options = new Velopack.Packaging.Windows.Commands.WindowsPackOptions {
      ReleaseDir = new DirectoryInfo(releases),
      PackDirectory = appDir,
      PackId = "MyApp",
      PackVersion = "1.0.0",
      Channel = "stable",
      TargetRuntime = Velopack.RID.Parse("win-x64"),
      EntryExecutableName = "MyApp.exe",
      LocalizationDirectory = locRoot,
      NoInst = true, // skip installer to keep test fast
      NoPortable = true,
    };

    // Use Core LoggerConsole which implements IFancyConsole/Progress
    var logFactory = LoggerFactory.Create(b => b.SetMinimumLevel(LogLevel.Information));
    var logger = logFactory.CreateLogger("test");
    var console = new Velopack.Core.LoggerConsole(logger);
    var runner = new Velopack.Packaging.Windows.Commands.WindowsPackCommandRunner(logger, console);

    await runner.Run(options);

    // Find the full nupkg
    var nupkg = Directory.GetFiles(releases, "*.nupkg").FirstOrDefault(f => f.Contains("full"));
    Assert.NotNull(nupkg);

    using var zip = ZipFile.OpenRead(nupkg!);
    // Assert Spanish XLIFF present
    Assert.NotNull(zip.GetEntry("localization/es/velopack.xlf"));
    // Assert French PO present
    Assert.NotNull(zip.GetEntry("localization/fr/velopack.po"));
  }
}

// No helpers needed; using LoggerConsole from Core for IFancyConsole
