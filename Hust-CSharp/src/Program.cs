using System;
using System.IO;
using System.Text.Json;

namespace HustCSharp;

/// <summary>
/// Hust-CSharp Core - Main entry point
/// Implements the Hust core contract: info, transpile, build
/// </summary>
class Program
{
    private const string CoreName = "csharp";
    private const string CoreVersion = "0.1.3.20261128125000";
    private const int ProtocolVersion = 1;

    static int Main(string[] args)
    {
        if (args.Length == 0)
        {
            PrintUsage();
            return 2;
        }

        var command = args[0].ToLowerInvariant();

        // Debug mode: print preprocessor output
        if (command == "debug" && args.Length > 1)
        {
            return HandleDebug(args[1]);
        }

        return command switch
        {
            "info" => HandleInfo(),
            "transpile" => HandleTranspile(args),
            "build" => HandleBuild(args),
            "--version" or "-v" => HandleVersion(),
            "--help" or "-h" => HandleHelp(),
            _ => HandleUnknownCommand(command)
        };
    }

    private static int HandleInfo()
    {
        var info = new
        {
            protocol = ProtocolVersion,
            name = CoreName,
            language = "csharp",
            transpile = true,
            build = true,
            toolchain = CheckToolchain()
        };

        Console.WriteLine(JsonSerializer.Serialize(info));
        return 0;
    }

    private static int HandleTranspile(string[] args)
    {
        if (args.Length < 2)
        {
            Console.Error.WriteLine("Error: transpile requires entry file");
            return 2;
        }

        var entry = ResolveEntry(args[1]);
        var outDir = GetOptionValue(args, "--out-dir") ?? "./out";

        if (entry == null || !File.Exists(entry))
        {
            Console.Error.WriteLine($"Error: entry file not found: {args[1]}");
            return 2;
        }

        try
        {
            Directory.CreateDirectory(outDir);

            // Read source file
            var source = File.ReadAllText(entry);
            var fileName = Path.GetFileName(entry);

            // Transpile
            var translator = new Translator();
            var result = translator.Translate(source, fileName);

            if (!result.Success)
            {
                foreach (var error in result.Errors)
                {
                    Console.Error.WriteLine($"Error: {error}");
                }
                return 4;
            }

            // Write output
            var outputFile = Path.Combine(outDir, Path.GetFileNameWithoutExtension(entry) + ".cs");
            File.WriteAllText(outputFile, result.Output);

            // Print warnings to stderr
            foreach (var warning in result.Warnings)
            {
                Console.Error.WriteLine($"Warning: {warning}");
            }

            return 0;
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"Transpilation failed: {ex.Message}");
            return 4;
        }
    }

    private static int HandleBuild(string[] args)
    {
        if (args.Length < 2)
        {
            Console.Error.WriteLine("Error: build requires entry file");
            return 2;
        }

        var entry = ResolveEntry(args[1]);
        var outDir = GetOptionValue(args, "--out-dir") ?? "./out";
        var isRelease = HasFlag(args, "--release");
        var isDebug = HasFlag(args, "--debug");

        if (entry == null || !File.Exists(entry))
        {
            Console.Error.WriteLine($"Error: entry file not found: {args[1]}");
            return 2;
        }

        var dotnet = FindDotnet();
        if (dotnet == null)
        {
            Console.Error.WriteLine("Error: dotnet toolchain not found");
            return 3;
        }

        try
        {
            // First transpile
            var transpileResult = HandleTranspile(args);
            if (transpileResult != 0)
            {
                return transpileResult;
            }

            var stem = Path.GetFileNameWithoutExtension(entry);
            var configuration = isDebug && !isRelease ? "Debug" : "Release";
            var publishDir = Path.GetFullPath(Path.Combine(outDir, "publish"));
            Directory.CreateDirectory(publishDir);

            // Generate a standalone project for the transpiled sources.
            var projectFile = Path.Combine(outDir, stem + ".csproj");
            File.WriteAllText(projectFile, $"""
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <OutputType>Exe</OutputType>
    <TargetFramework>net8.0</TargetFramework>
    <ImplicitUsings>enable</ImplicitUsings>
    <Nullable>disable</Nullable>
    <AssemblyName>{stem}</AssemblyName>
    <EnableDefaultCompileItems>true</EnableDefaultCompileItems>
    <InvariantGlobalization>true</InvariantGlobalization>
  </PropertyGroup>
</Project>
""");

            var psi = new System.Diagnostics.ProcessStartInfo
            {
                FileName = dotnet,
                RedirectStandardOutput = true,
                RedirectStandardError = true,
                UseShellExecute = false,
                CreateNoWindow = true
            };
            psi.ArgumentList.Add("publish");
            psi.ArgumentList.Add(projectFile);
            psi.ArgumentList.Add("-c");
            psi.ArgumentList.Add(configuration);
            psi.ArgumentList.Add("-o");
            psi.ArgumentList.Add(publishDir);
            psi.ArgumentList.Add("--nologo");
            psi.ArgumentList.Add("-v");
            psi.ArgumentList.Add("quiet");

            using var process = System.Diagnostics.Process.Start(psi)!;
            var stdout = process.StandardOutput.ReadToEnd();
            var stderr = process.StandardError.ReadToEnd();
            process.WaitForExit();

            if (process.ExitCode != 0)
            {
                if (!string.IsNullOrWhiteSpace(stdout)) Console.Error.Write(stdout);
                if (!string.IsNullOrWhiteSpace(stderr)) Console.Error.Write(stderr);
                Console.Error.WriteLine($"Error: dotnet publish failed with exit code {process.ExitCode}");
                return 5;
            }

            var dllPath = Path.Combine(publishDir, stem + ".dll");
            if (!File.Exists(dllPath))
            {
                Console.Error.WriteLine($"Error: build output not found: {dllPath}");
                return 5;
            }

            var exePath = Path.Combine(publishDir, OperatingSystem.IsWindows() ? stem + ".exe" : stem);
            if (!OperatingSystem.IsWindows())
            {
                // A .NET app needs its dll, deps.json, and runtimeconfig.json.
                // Put them in a sidecar directory so the conductor can copy the
                // launcher and its payload together when `-o` is used.
                var sidecar = Path.Combine(publishDir, stem + ".files");
                Directory.CreateDirectory(sidecar);
                foreach (var name in new[] { stem + ".dll", stem + ".deps.json", stem + ".runtimeconfig.json" })
                {
                    var source = Path.Combine(publishDir, name);
                    if (File.Exists(source))
                    {
                        File.Copy(source, Path.Combine(sidecar, name), overwrite: true);
                    }
                }

                // The apphost produced by `dotnet publish` cannot locate a
                // user-local runtime unless DOTNET_ROOT is set. A tiny launcher
                // makes the artifact relocatable and matches the core contract.
                var launcher = $$"""
#!/usr/bin/env bash
set -e
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PAYLOAD="$DIR/$(basename "$0").files"
if [ -f "$PAYLOAD/{{stem}}.dll" ]; then
  TARGET="$PAYLOAD/{{stem}}.dll"
else
  TARGET="$DIR/{{stem}}.dll"
fi
DOTNET="${DOTNET_CMD:-}"
if [ -z "$DOTNET" ] || [ ! -x "$DOTNET" ]; then
  if [ -x "$HOME/.dotnet/dotnet" ]; then
    DOTNET="$HOME/.dotnet/dotnet"
  else
    DOTNET="$(command -v dotnet || true)"
  fi
fi
[ -n "$DOTNET" ] || { echo "error: dotnet not found" >&2; exit 3; }
export DOTNET_ROOT="${DOTNET_ROOT:-$(dirname "$DOTNET")}"
exec "$DOTNET" "$TARGET" "$@"
""";
                File.WriteAllText(exePath, launcher);
                File.SetUnixFileMode(exePath,
                    UnixFileMode.UserRead | UnixFileMode.UserWrite | UnixFileMode.UserExecute |
                    UnixFileMode.GroupRead | UnixFileMode.GroupExecute |
                    UnixFileMode.OtherRead | UnixFileMode.OtherExecute);
            }

            // Contract: stdout carries only the executable line.
            Console.WriteLine($"executable={Path.GetFullPath(exePath)}");
            return 0;
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"Build failed: {ex.Message}");
            return 5;
        }
    }

    /// <summary>
    /// Resolves a file entry or a project directory (settings.config / main.hust).
    /// </summary>
    private static string? ResolveEntry(string raw)
    {
        if (File.Exists(raw))
        {
            return raw;
        }

        if (!Directory.Exists(raw))
        {
            return null;
        }

        var config = Path.Combine(raw, "settings.config");
        if (File.Exists(config))
        {
            foreach (var line in File.ReadAllLines(config))
            {
                var trimmed = line.Trim();
                if (trimmed.StartsWith("#") || trimmed.StartsWith(";") || trimmed.StartsWith("["))
                {
                    continue;
                }
                var eq = trimmed.IndexOf('=');
                if (eq <= 0) continue;
                var key = trimmed[..eq].Trim();
                var value = trimmed[(eq + 1)..].Trim().Trim('"');
                if (key == "entry" && value.Length > 0)
                {
                    var candidate = Path.Combine(raw, value);
                    if (File.Exists(candidate)) return candidate;
                }
            }
        }

        var main = Path.Combine(raw, "main.hust");
        return File.Exists(main) ? main : null;
    }

    private static string? FindDotnet()
    {
        var env = Environment.GetEnvironmentVariable("DOTNET_CMD");
        if (!string.IsNullOrEmpty(env) && File.Exists(env))
        {
            return env;
        }

        var exeName = OperatingSystem.IsWindows() ? "dotnet.exe" : "dotnet";
        var home = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
        if (!string.IsNullOrEmpty(home))
        {
            var local = Path.Combine(home, ".dotnet", exeName);
            if (File.Exists(local)) return local;
        }

        var path = Environment.GetEnvironmentVariable("PATH") ?? "";
        foreach (var dir in path.Split(Path.PathSeparator))
        {
            if (string.IsNullOrWhiteSpace(dir)) continue;
            var candidate = Path.Combine(dir.Trim(), exeName);
            if (File.Exists(candidate)) return candidate;
        }

        return null;
    }

    private static int HandleDebug(string entry)
    {
        if (!File.Exists(entry))
        {
            Console.Error.WriteLine($"Error: entry file not found: {entry}");
            return 2;
        }

        try
        {
            var source = File.ReadAllText(entry);
            var preprocessor = new Preprocessor();
            var processed = preprocessor.ProcessToString(source);
            
            Console.WriteLine("=== Preprocessed Source ===");
            Console.WriteLine(processed);
            Console.WriteLine("=== End ===");
            
            return 0;
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"Debug failed: {ex.Message}");
            return 4;
        }
    }

    private static int HandleVersion()
    {
        Console.WriteLine($"hust_csharp {CoreVersion}");
        return 0;
    }

    private static int HandleHelp()
    {
        PrintUsage();
        return 0;
    }

    private static int HandleUnknownCommand(string command)
    {
        Console.Error.WriteLine($"Error: unknown command '{command}'");
        PrintUsage();
        return 2;
    }

    private static void PrintUsage()
    {
        Console.WriteLine("Hust-CSharp Core - Hust language core for C#");
        Console.WriteLine();
        Console.WriteLine("Usage:");
        Console.WriteLine("  hust_csharp info                              Show core information");
        Console.WriteLine("  hust_csharp transpile <entry> --out-dir <dir> Transpile to C#");
        Console.WriteLine("  hust_csharp build <entry> --out-dir <dir>     Build executable");
        Console.WriteLine("  hust_csharp --version                         Show version");
        Console.WriteLine("  hust_csharp --help                            Show this help");
        Console.WriteLine();
        Console.WriteLine("Options:");
        Console.WriteLine("  --project          Treat entry as project directory");
        Console.WriteLine("  --release          Build in release mode (default)");
        Console.WriteLine("  --debug            Build in debug mode");
    }

    private static bool CheckToolchain()
    {
        var dotnet = FindDotnet();
        if (dotnet == null)
        {
            return false;
        }

        try
        {
            var process = new System.Diagnostics.Process
            {
                StartInfo = new System.Diagnostics.ProcessStartInfo
                {
                    FileName = dotnet,
                    Arguments = "--version",
                    RedirectStandardOutput = true,
                    RedirectStandardError = true,
                    UseShellExecute = false,
                    CreateNoWindow = true
                }
            };
            process.Start();
            process.WaitForExit();
            return process.ExitCode == 0;
        }
        catch
        {
            return false;
        }
    }

    private static string? GetOptionValue(string[] args, string option)
    {
        for (int i = 0; i < args.Length - 1; i++)
        {
            if (args[i] == option)
            {
                return args[i + 1];
            }
        }
        return null;
    }

    private static bool HasFlag(string[] args, string flag)
    {
        return args.Contains(flag);
    }
}
