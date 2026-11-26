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
    private const string CoreVersion = "0.1.0";
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

        var entry = args[1];
        var outDir = GetOptionValue(args, "--out-dir") ?? "./out";
        var isProject = HasFlag(args, "--project");

        if (!File.Exists(entry))
        {
            Console.Error.WriteLine($"Error: entry file not found: {entry}");
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

        var entry = args[1];
        var outDir = GetOptionValue(args, "--out-dir") ?? "./out";
        var isProject = HasFlag(args, "--project");
        var isRelease = HasFlag(args, "--release");
        var isDebug = HasFlag(args, "--debug");

        if (!File.Exists(entry))
        {
            Console.Error.WriteLine($"Error: entry file not found: {entry}");
            return 2;
        }

        try
        {
            // First transpile
            var transpileResult = HandleTranspile(args);
            if (transpileResult != 0)
            {
                return transpileResult;
            }

            // TODO: Implement build with dotnet
            Console.Error.WriteLine($"Building {entry}...");
            Console.Error.WriteLine("Warning: C# core build not yet implemented");
            
            // Placeholder: return dummy executable path
            var exePath = Path.GetFullPath(Path.Combine(outDir, "app.exe"));
            Console.WriteLine($"executable={exePath}");
            
            return 0;
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"Build failed: {ex.Message}");
            return 5;
        }
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
        try
        {
            var process = new System.Diagnostics.Process
            {
                StartInfo = new System.Diagnostics.ProcessStartInfo
                {
                    FileName = "dotnet",
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
