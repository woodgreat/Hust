using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text;
using System.Text.RegularExpressions;

namespace HustCSharp;

/// <summary>
/// Hust to C# translator with simplified indent handling.
/// 
/// Indent strategy:
/// - Preprocessor outputs relative indent (2 spaces per level, starting from 0)
/// - Translator adds absolute indent based on context:
///   - Program class wrapper: +1 level (2 spaces)
///   - Class inside Program: +2 levels (4 spaces) for members
///   - Method inside class: +3 levels (6 spaces) for body
/// </summary>
public class Translator
{
    private readonly StringBuilder _output = new();
    private readonly Preprocessor _preprocessor = new();
    
    // Context tracking
    private int _braceDepth = 0;        // Track { } depth
    private bool _inClass = false;      // Inside a class declaration
    private bool _inInterface = false;  // Inside an interface declaration
    private bool _inMethod = false;     // Inside a method
    
    // Semantic analysis
    private readonly HashSet<string> _definedFunctions = new();  // Track defined functions
    private readonly List<string> _semanticErrors = new();       // Collect semantic errors
    
    // Indent constants (2 spaces per level)
    private const string INDENT_1 = "  ";      // 1 level: Program class body
    private const string INDENT_2 = "    ";    // 2 levels: class members / top-level method body
    private const string INDENT_3 = "      ";  // 3 levels: class method body

    public TranslationResult Translate(string source, string fileName = "output")
    {
        var result = new TranslationResult();
        
        try
        {
            // Reset state
            _definedFunctions.Clear();
            _semanticErrors.Clear();
            _inClass = false;
            _inMethod = false;
            _braceDepth = 0;
            
            // Step 1: Preprocess (protect strings/comments, normalize blocks)
            var (processed, dialect) = _preprocessor.Process(source);
            
            if (dialect.DetectedDialect != Preprocessor.DialectType.PureHust && 
                dialect.DetectedDialect != Preprocessor.DialectType.CSharpDialect)
            {
                result.Success = false;
                var errorMsg = dialect.DetectedDialect switch
                {
                    Preprocessor.DialectType.CDialect => "C.* imports detected in C# core project",
                    Preprocessor.DialectType.RustDialect => "RUST.* imports detected in C# core project",
                    Preprocessor.DialectType.MixedDialect => "Mixed dialects detected",
                    _ => $"Unsupported dialect: {dialect.DetectedDialect}"
                };
                result.Errors.Add(errorMsg);
                return result;
            }
            
            // Step 2: Translate line by line
            var lines = processed.Split('\n');
            
            // Emit header
            EmitHeader();
            
            // Emit Program class start
            _output.AppendLine("public class Program");
            _output.AppendLine("{");
            _output.AppendLine();
            
            // Track if we need to emit Main method
            bool hasMainMethod = false;
            
            foreach (var line in lines)
            {
                var trimmed = line.Trim();
                
                // Skip empty lines
                if (string.IsNullOrWhiteSpace(trimmed))
                {
                    continue;
                }
                
                // Skip use statements (handled in header)
                if (trimmed.StartsWith("use "))
                {
                    TranslateUseStatement(trimmed);
                    continue;
                }
                
                // Translate class declaration
                if (trimmed.StartsWith("class "))
                {
                    TranslateClassDeclaration(trimmed);
                    continue;
                }
                
                // Translate interface declaration
                if (trimmed.StartsWith("interface ") || trimmed.StartsWith("shape "))
                {
                    TranslateInterfaceDeclaration(trimmed);
                    continue;
                }
                
                // Translate method declaration
                if (IsMethodDeclaration(trimmed))
                {
                    var isMain = TranslateMethodDeclaration(trimmed);
                    if (isMain) hasMainMethod = true;
                    continue;
                }
                
                // Translate variable declaration
                if (IsVariableDeclaration(trimmed))
                {
                    TranslateVariableDeclaration(line, trimmed);
                    continue;
                }
                
                // Translate control flow statements
                if (trimmed.StartsWith("if ") || trimmed.StartsWith("if("))
                {
                    TranslateControlStatement(line, trimmed);
                    continue;
                }
                
                if (trimmed.StartsWith("for ") || trimmed.StartsWith("for("))
                {
                    TranslateControlStatement(line, trimmed);
                    continue;
                }
                
                if (trimmed.StartsWith("while ") || trimmed.StartsWith("while("))
                {
                    TranslateControlStatement(line, trimmed);
                    continue;
                }
                
                if (trimmed.StartsWith("return "))
                {
                    TranslateReturnStatement(line, trimmed);
                    continue;
                }
                
                // Handle closing brace
                if (trimmed == "}")
                {
                    TranslateClosingBrace();
                    continue;
                }
                
                // Default: translate expression
                TranslateExpressionStatement(line, trimmed);
            }
            
            // Emit Program class end
            _output.AppendLine();
            _output.AppendLine("}");
            
            // Restore protected strings and comments
            var finalOutput = _preprocessor.Restore(_output.ToString());
            
            // Check for semantic errors
            if (_semanticErrors.Count > 0)
            {
                result.Success = false;
                result.Errors.AddRange(_semanticErrors);
                return result;
            }
            
            result.Success = true;
            result.Code = finalOutput;
            result.HasMainMethod = hasMainMethod;
        }
        catch (Exception ex)
        {
            result.Success = false;
            result.Errors.Add($"Translation error: {ex.Message}");
        }
        
        return result;
    }

    private void EmitHeader()
    {
        _output.AppendLine("// Generated by Hust-CSharp");
        _output.AppendLine("// Do not edit manually");
        _output.AppendLine();
        _output.AppendLine("using System;");
        _output.AppendLine("using System.Collections.Generic;");
        _output.AppendLine("using System.Linq;");
        _output.AppendLine();
    }

    private void TranslateUseStatement(string line)
    {
        // use CSHARP.Console.* -> already in header
        // Other imports -> comment for now
        if (!line.Contains("CSHARP."))
        {
            _output.AppendLine($"// {line}");
        }
    }

    private void TranslateClassDeclaration(string trimmed)
    {
        // class Foo { -> public class Foo {
        // class Dog : Animal { -> public class Dog : Animal {
        // class Circle : Shape { -> public class Circle : Shape {
        var match = Regex.Match(trimmed, @"class\s+(\w+)(?:\s*:\s*([\w,\s]+))?");
        if (match.Success)
        {
            var className = match.Groups[1].Value;
            var baseList = match.Groups[2].Success ? match.Groups[2].Value : null;
            
            if (baseList != null)
            {
                // Split multiple interfaces/base classes
                var bases = baseList.Split(',').Select(b => b.Trim()).ToArray();
                _output.AppendLine($"{INDENT_1}public class {className} : {string.Join(", ", bases)}");
            }
            else
            {
                _output.AppendLine($"{INDENT_1}public class {className}");
            }
            _output.AppendLine($"{INDENT_1}{{");
            _inClass = true;
            _braceDepth++;
        }
    }

    private void TranslateInterfaceDeclaration(string trimmed)
    {
        // interface Foo { -> public interface Foo {
        // shape Foo { -> public interface Foo { (shape is Hust's interface keyword)
        var match = Regex.Match(trimmed, @"(?:interface|shape)\s+(\w+)");
        if (match.Success)
        {
            var interfaceName = match.Groups[1].Value;
            _output.AppendLine($"{INDENT_1}public interface {interfaceName}");
            _output.AppendLine($"{INDENT_1}{{");
            _inInterface = true;
            _braceDepth++;
        }
    }

    private bool IsMethodDeclaration(string trimmed)
    {
        return Regex.IsMatch(trimmed, @"^(void|int|float|double|string|bool)\s+\w+\s*\(");
    }

    private bool TranslateMethodDeclaration(string trimmed)
    {
        // void main() { -> public static void Main() {
        // void draw(); -> void draw(); (interface method, no body)
        var match = Regex.Match(trimmed, @"^(void|int|float|double|string|bool)\s+(\w+)\s*\((.*?)\)(\s*;)?");
        if (!match.Success) return false;
        
        var returnType = TranslateType(match.Groups[1].Value);
        var methodName = match.Groups[2].Value;
        var parameters = match.Groups[3].Value;
        var isInterfaceMethod = match.Groups[4].Success; // Ends with ;
        var isMain = methodName.ToLower() == "main";
        
        // Record defined function for semantic analysis
        _definedFunctions.Add(methodName);
        
        if (isMain)
        {
            methodName = "Main";
            _output.AppendLine($"{INDENT_1}public static {returnType} {methodName}({TranslateParameters(parameters)})");
            _output.AppendLine($"{INDENT_1}{{");
            _inMethod = true;
        }
        else if (_inInterface)
        {
            // Interface method: no body, just signature
            _output.AppendLine($"{INDENT_2}{returnType} {methodName}({TranslateParameters(parameters)});");
            // Don't set _inMethod for interface methods
        }
        else if (_inClass)
        {
            _output.AppendLine($"{INDENT_2}public {returnType} {methodName}({TranslateParameters(parameters)})");
            _output.AppendLine($"{INDENT_2}{{");
            _inMethod = true;
        }
        else
        {
            _output.AppendLine($"{INDENT_1}public {returnType} {methodName}({TranslateParameters(parameters)})");
            _output.AppendLine($"{INDENT_1}{{");
            _inMethod = true;
        }
        
        _braceDepth++;
        return isMain;
    }

    private bool IsVariableDeclaration(string trimmed)
    {
        return Regex.IsMatch(trimmed, @"^(int|float|double|string|bool)\s+\w+");
    }

    private void TranslateVariableDeclaration(string line, string trimmed)
    {
        var match = Regex.Match(trimmed, @"^(int|float|double|string|bool)\s+(\w+)(.*)$");
        if (!match.Success)
        {
            TranslateExpressionStatement(line, trimmed);
            return;
        }
        
        var type = TranslateType(match.Groups[1].Value);
        var name = match.Groups[2].Value;
        var rest = match.Groups[3].Value;
        
        // Semantic check: type mismatch
        CheckTypeMismatch(type, rest);
        
        string indent;
        if (_inClass && !_inMethod)
        {
            // Class field: 2 levels
            indent = INDENT_2;
            _output.AppendLine($"{indent}public {type} {name}{rest}");
        }
        else if (_inMethod)
        {
            // Method local: fixed indent, ignore preprocessor indent
            indent = _inClass ? INDENT_3 : INDENT_2;
            _output.AppendLine($"{indent}{type} {name}{rest}");
        }
        else
        {
            // Top level (shouldn't happen)
            indent = INDENT_1;
            _output.AppendLine($"{indent}{type} {name}{rest}");
        }
    }

    private void CheckTypeMismatch(string declaredType, string rest)
    {
        // Check for type mismatch in initialization
        // e.g., int x = "hello"; -> error
        if (string.IsNullOrWhiteSpace(rest)) return;
        
        // Match: = value;
        var match = Regex.Match(rest, @"^\s*=\s*(.+);$");
        if (!match.Success) return;
        
        var value = match.Groups[1].Value.Trim();
        
        // Check string literal assigned to non-string type
        // Note: preprocessor protects strings as __STRING_N__
        if (declaredType != "string" && 
            (value.StartsWith("\"") && value.EndsWith("\"") || value.StartsWith("__STRING_")))
        {
            _semanticErrors.Add($"Type mismatch: cannot assign string to {declaredType}");
        }
        
        // Check numeric literal assigned to string
        if (declaredType == "string" && Regex.IsMatch(value, @"^\d+$"))
        {
            _semanticErrors.Add($"Type mismatch: cannot assign number to string");
        }
    }

    private void TranslateControlStatement(string line, string trimmed)
    {
        // Method body statements: fixed indent, ignore preprocessor indent
        var indent = _inClass ? INDENT_3 : INDENT_2;
        _output.AppendLine($"{indent}{TranslateExpression(trimmed)}");
    }

    private void TranslateReturnStatement(string line, string trimmed)
    {
        var indent = _inClass ? INDENT_3 : INDENT_2;
        _output.AppendLine($"{indent}{TranslateExpression(trimmed)}");
    }

    private void TranslateClosingBrace()
    {
        _braceDepth--;
        
        if (_inMethod)
        {
            // End of method
            var indent = _inClass ? INDENT_2 : INDENT_1;
            _output.AppendLine($"{indent}}}");
            _inMethod = false;
        }
        else if (_inInterface)
        {
            // End of interface
            _output.AppendLine($"{INDENT_1}}}");
            _inInterface = false;
        }
        else if (_inClass)
        {
            // End of class
            _output.AppendLine($"{INDENT_1}}}");
            _inClass = false;
        }
        else
        {
            // End of Program (shouldn't happen here)
            _output.AppendLine("}");
        }
    }

    private void TranslateExpressionStatement(string line, string trimmed)
    {
        // Handle comments: preserve but fix indent
        if (trimmed.StartsWith("//"))
        {
            var indent = _inClass ? INDENT_2 : INDENT_1;
            _output.AppendLine($"{indent}{trimmed}");
            return;
        }
        
        // Check for undefined function calls
        CheckUndefinedFunction(trimmed);
        
        // Method body statements: fixed indent, ignore preprocessor indent
        var stmtIndent = _inClass ? INDENT_3 : INDENT_2;
        _output.AppendLine($"{stmtIndent}{TranslateExpression(trimmed)}");
    }

    private void CheckUndefinedFunction(string trimmed)
    {
        // Check for function calls: identifier(...)
        // Skip known patterns: Console.WriteLine, new, etc.
        var match = Regex.Match(trimmed, @"^(\w+)\s*\(");
        if (!match.Success) return;
        
        var funcName = match.Groups[1].Value;
        
        // Skip known built-in functions and keywords
        var knownFunctions = new HashSet<string> { "Console", "new", "if", "for", "while", "return", "switch", "catch" };
        if (knownFunctions.Contains(funcName)) return;
        
        // Skip if it's a method call on an object (contains .)
        if (trimmed.Contains(".")) return;
        
        // Check if function is defined
        if (!_definedFunctions.Contains(funcName))
        {
            _semanticErrors.Add($"Undefined function: {funcName}");
        }
    }

    private string GetOriginalIndent(string line)
    {
        // Get the relative indent from preprocessor output
        var match = Regex.Match(line, @"^(\s*)");
        return match.Success ? match.Groups[1].Value : "";
    }

    private string TranslateType(string type)
    {
        return type switch
        {
            "int" => "int",
            "float" => "float",
            "double" => "double",
            "string" => "string",
            "bool" => "bool",
            "void" => "void",
            _ => type
        };
    }

    private string TranslateParameters(string parameters)
    {
        if (string.IsNullOrWhiteSpace(parameters))
            return "";
        
        // Simple parameter translation: int x, string y -> int x, string y
        var parts = parameters.Split(',');
        var translated = parts.Select(p =>
        {
            var trimmed = p.Trim();
            var match = Regex.Match(trimmed, @"^(int|float|double|string|bool)\s+(\w+)$");
            if (match.Success)
            {
                return $"{TranslateType(match.Groups[1].Value)} {match.Groups[2].Value}";
            }
            return trimmed;
        });
        
        return string.Join(", ", translated);
    }

    private string TranslateExpression(string expr)
    {
        // Translate Hust expressions to C#
        // For now, mostly pass-through with some replacements
        
        // Console.WriteLine -> Console.WriteLine (already correct)
        // new Person() -> new Person() (already correct)
        
        // Fix float literals: add F suffix for C#
        // Match: number.number (not followed by F/f)
        expr = Regex.Replace(expr, @"(\d+\.\d+)(?![Ff\d])", "$1f");
        
        return expr;
    }

    public class TranslationResult
    {
        public bool Success { get; set; }
        public string Code { get; set; } = "";
        public string Output => Code;  // Alias for compatibility
        public List<string> Errors { get; set; } = new();
        public List<string> Warnings { get; set; } = new();
        public bool HasMainMethod { get; set; }
    }
}
