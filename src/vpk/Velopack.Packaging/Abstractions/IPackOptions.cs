using Velopack.Packaging.Compression;

#nullable enable

namespace Velopack.Packaging.Abstractions;

public interface IPackOptions : INugetPackCommand, IPlatformOptions
{
    string Channel { get; set; }
    DeltaMode DeltaMode { get; set; }
    string EntryExecutableName { get; set; }
    string Icon { get; set; }
    string Exclude { get; set; }
    bool NoPortable { get; set; }
    bool NoInst { get; set; }

    /// <summary>
    /// Path to directory containing localized dialog files grouped by culture (e.g., 'en/velopack.po', 'fr/velopack.xlf')
    /// </summary>
    string? LocalizationDirectory { get; set; }
}
