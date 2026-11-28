using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text;
using System.Text.RegularExpressions;

namespace HustCSharp;

/// <summary>
/// Preprocessor for Hust-CSharp
/// Handles comment/string protection, dialect checking, and block structure normalization
/// </summary>
public class Preprocessor
{
    private readonly List<string> _protectedStrings = new();
    private readonly List<string> _protectedComments = new();
    
    /// <summary>
    /// Dialect check result
    /// </summary>
    public class DialectResult
    {
        public bool IsValid { get; set; } = true;
        public List<string> Errors { get; } = new();
        public List<string> Warnings { get; } = new();
        public DialectType DetectedDialect { get; set; } = DialectType.PureHust;
    }
    
    public enum DialectType
    {
        PureHust,      // No system imports
        CSharpDialect, // C#.* imports
        MixedDialect,  // Multiple dialects (error)
        RustDialect,   // RUST.* imports (error for C# core)
        CDialect,      // C.* imports (error for C# core)
        GoDialect      // GO.* imports (error for C# core)
    }

    /// <summary>
    /// Preprocess source code: protect strings/comments, check dialect, normalize block structure
    /// Returns: (processed_source, dialect_result)
    /// </summary>
    public (string processed, DialectResult dialect) Process(string source)
    {
        var result = new DialectResult();
        
        // Step 1: Protect strings and comments
        var protectedSource = ProtectStringsAndComments(source);
        
        // Step 2: Check dialect
        CheckDialect(protectedSource, result);
        
        // Step 3: Normalize block structure
        var normalized = NormalizeBlockStructure(protectedSource);
        
        return (normalized, result);
    }
    
    /// <summary>
    /// Preprocess source code and return only the processed string
    /// </summary>
    public string ProcessToString(string source)
    {
        var (processed, _) = Process(source);
        return processed;
    }

    /// <summary>
    /// Protect strings and comments with placeholders
    /// </summary>
    private string ProtectStringsAndComments(string source)
    {
        var result = source;
        
        // Protect single-line comments
        result = Regex.Replace(result, @"//.*$", match =>
        {
            _protectedComments.Add(match.Value);
            return $"__COMMENT_{_protectedComments.Count - 1}__";
        }, RegexOptions.Multiline);
        
        // Protect multi-line comments
        result = Regex.Replace(result, @"/\*.*?\*/", match =>
        {
            _protectedComments.Add(match.Value);
            return $"__COMMENT_{_protectedComments.Count - 1}__";
        }, RegexOptions.Singleline);
        
        // Protect string literals
        result = Regex.Replace(result, @"""(?:[^""\\]|\\.)*""", match =>
        {
            _protectedStrings.Add(match.Value);
            return $"__STRING_{_protectedStrings.Count - 1}__";
        });
        
        return result;
    }

    /// <summary>
    /// Check dialect rules
    /// </summary>
    private void CheckDialect(string source, DialectResult result)
    {
        var lines = source.Split('\n');
        var hasCSharpImport = false;
        var hasRustImport = false;
        var hasCImport = false;
        var hasGoImport = false;
        
        foreach (var line in lines)
        {
            var trimmed = line.Trim();
            
            // Check for system imports
            if (trimmed.StartsWith("use "))
            {
                if (trimmed.Contains("CSHARP."))
                {
                    hasCSharpImport = true;
                }
                else if (trimmed.Contains("RUST."))
                {
                    hasRustImport = true;
                }
                else if (trimmed.Contains("GO."))
                {
                    hasGoImport = true;
                }
                else if (trimmed.Contains("C."))
                {
                    hasCImport = true;
                }
                else if (trimmed.Contains("HUST."))
                {
                    result.Errors.Add("HUST.* imports are not yet implemented");
                }
            }
        }
        
        // Determine dialect
        var dialectCount = (hasCSharpImport ? 1 : 0) + (hasRustImport ? 1 : 0) + (hasCImport ? 1 : 0) + (hasGoImport ? 1 : 0);
        
        if (dialectCount > 1)
        {
            result.DetectedDialect = DialectType.MixedDialect;
            result.IsValid = false;
            result.Errors.Add("Mixed dialects detected: a project can only use one language core");
        }
        else if (hasRustImport)
        {
            result.DetectedDialect = DialectType.RustDialect;
            result.IsValid = false;
            result.Errors.Add("RUST.* imports detected in C# core project");
        }
        else if (hasGoImport)
        {
            result.DetectedDialect = DialectType.GoDialect;
            result.IsValid = false;
            result.Errors.Add("GO.* imports detected in C# core project");
        }
        else if (hasCImport)
        {
            result.DetectedDialect = DialectType.CDialect;
            result.IsValid = false;
            result.Errors.Add("C.* imports detected in C# core project");
        }
        else if (hasCSharpImport)
        {
            result.DetectedDialect = DialectType.CSharpDialect;
        }
        else
        {
            result.DetectedDialect = DialectType.PureHust;
        }
        
        // Check for Rust-specific syntax (println!, etc.)
        if (Regex.IsMatch(source, @"\bprintln!\s*\("))
        {
            result.IsValid = false;
            result.Errors.Add("Rust macro 'println!' detected - use C#.Console.WriteLine instead");
        }
        
        // Check for C-specific syntax (printf without import)
        if (Regex.IsMatch(source, @"\bprintf\s*\(") && !hasCImport && !hasCSharpImport)
        {
            result.Warnings.Add("printf detected without C.* import - consider using C#.Console.WriteLine");
        }
    }

    /// <summary>
    /// Normalize block structure: ensure braces are properly paired and formatted
    /// Output: standard format with 2-space indentation per level (relative to current block)
    /// </summary>
    private string NormalizeBlockStructure(string source)
    {
        var lines = source.Split('\n');
        var result = new StringBuilder();
        var indentLevel = 0;
        const int IndentSize = 2;  // 2 spaces per level
        
        foreach (var line in lines)
        {
            var trimmed = line.Trim();
            
            // Skip empty lines
            if (string.IsNullOrWhiteSpace(trimmed))
            {
                result.AppendLine();
                continue;
            }
            
            // Handle closing brace first (before opening brace on same line)
            if (trimmed.StartsWith("}"))
            {
                indentLevel--;
                if (indentLevel < 0) indentLevel = 0;
                
                // Check if there's code after the closing brace
                var rest = trimmed.Substring(1).Trim();
                if (!string.IsNullOrEmpty(rest))
                {
                    // Line like "} else {" or "} else if (x) {"
                    result.AppendLine(new string(' ', indentLevel * IndentSize) + "} " + rest);
                    // Check if rest ends with opening brace
                    if (rest.EndsWith("{"))
                    {
                        indentLevel++;
                    }
                }
                else
                {
                    // Just "}"
                    result.AppendLine(new string(' ', indentLevel * IndentSize) + "}");
                }
                continue;
            }
            
            // Handle opening brace
            if (trimmed.EndsWith("{"))
            {
                // Line like "if (x > 5) {" or "class Foo {"
                var code = trimmed.Substring(0, trimmed.Length - 1).Trim();
                if (!string.IsNullOrEmpty(code))
                {
                    result.AppendLine(new string(' ', indentLevel * IndentSize) + code + " {");
                }
                else
                {
                    result.AppendLine(new string(' ', indentLevel * IndentSize) + "{");
                }
                indentLevel++;
                continue;
            }
            
            // Handle lines with only opening brace
            if (trimmed == "{")
            {
                result.AppendLine(new string(' ', indentLevel * IndentSize) + "{");
                indentLevel++;
                continue;
            }
            
            // Regular line: output with current indentation
            result.AppendLine(new string(' ', indentLevel * IndentSize) + trimmed);
        }
        
        return result.ToString();
    }

    /// <summary>
    /// Restore protected strings and comments
    /// </summary>
    public string Restore(string source)
    {
        var result = source;
        
        // Restore strings
        for (int i = 0; i < _protectedStrings.Count; i++)
        {
            result = result.Replace($"__STRING_{i}__", _protectedStrings[i]);
        }
        
        // Restore comments
        for (int i = 0; i < _protectedComments.Count; i++)
        {
            result = result.Replace($"__COMMENT_{i}__", _protectedComments[i]);
        }
        
        return result;
    }

    /// <summary>
    /// Verify no placeholders remain
    /// </summary>
    public bool VerifyNoPlaceholders(string source)
    {
        return !source.Contains("__STRING_") && !source.Contains("__COMMENT_");
    }
}
