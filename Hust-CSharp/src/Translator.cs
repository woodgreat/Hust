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
    private int _classDepth = -1;
    private int _methodDepth = -1;
    private int _interfaceDepth = -1;
    private bool _inClass = false;      // Inside a class declaration
    private bool _inInterface = false;  // Inside an interface declaration
    private bool _inMethod = false;     // Inside a method
    private string _currentClass = "";  // Current class name (constructors)
    private HashSet<string> _knownClasses = new();
    private HashSet<string> _defaultCtorClasses = new();

    // Type pattern shared by declarations and methods. Class/enum names are
    // capitalized; primitives cover Hust and C# spellings.
    private const string PrimitiveTypes =
        "i8|i16|i32|i64|u8|u16|u32|u64|f32|f64|bool|String|char|void|" +
        "int|long|short|byte|sbyte|uint|ulong|ushort|float|double|string|object";
    private const string TypePattern = @"(?:" + PrimitiveTypes + @"|[A-Z]\w*)(?:\[\d*\])*";
    
    // Semantic analysis
    private readonly HashSet<string> _definedFunctions = new();  // Track defined functions
    private readonly List<string> _semanticErrors = new();       // Collect semantic errors
    
    // Indent constants (2 spaces per level)
    private const string INDENT_1 = "  ";      // 1 level: Program class body
    private const string INDENT_2 = "    ";    // 2 levels: class members / top-level method body
    private const string INDENT_3 = "      ";  // 3 levels: class method body
    
    // Hust to C# type mapping
    private static readonly Dictionary<string, string> TypeMap = new()
    {
        { "i8", "sbyte" },
        { "i16", "short" },
        { "i32", "int" },
        { "i64", "long" },
        { "u8", "byte" },
        { "u16", "ushort" },
        { "u32", "uint" },
        { "u64", "ulong" },
        { "f32", "float" },
        { "f64", "double" },
        { "bool", "bool" },
        { "String", "string" },
        { "char", "char" },
        { "void", "void" }
    };
    
    private string MapType(string hustType)
    {
        // Handle array types: i32[] -> int[], i32[5] -> int[5], Person[][] -> Person[][]
        var arrayMatch = Regex.Match(hustType, @"^(\w+)((?:\[\d*\])*)$");
        if (arrayMatch.Success)
        {
            var baseType = arrayMatch.Groups[1].Value;
            var arraySuffix = arrayMatch.Groups[2].Value;
            var mappedBase = TypeMap.ContainsKey(baseType) ? TypeMap[baseType] : baseType;
            return mappedBase + arraySuffix;
        }

        // Handle simple types
        return TypeMap.ContainsKey(hustType) ? TypeMap[hustType] : hustType;
    }

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

            if (!dialect.IsValid || dialect.Errors.Count > 0)
            {
                result.Success = false;
                result.Errors.AddRange(dialect.Errors);
                return result;
            }

            if (dialect.DetectedDialect != Preprocessor.DialectType.PureHust &&
                dialect.DetectedDialect != Preprocessor.DialectType.CSharpDialect)
            {
                result.Success = false;
                var errorMsg = dialect.DetectedDialect switch
                {
                    Preprocessor.DialectType.CDialect => "C.* imports detected in C# core project",
                    Preprocessor.DialectType.RustDialect => "RUST.* imports detected in C# core project",
                    Preprocessor.DialectType.GoDialect => "GO.* imports detected in C# core project",
                    Preprocessor.DialectType.MixedDialect => "Mixed dialects detected",
                    _ => $"Unsupported dialect: {dialect.DetectedDialect}"
                };
                result.Errors.Add(errorMsg);
                return result;
            }
            
            // Step 2: Translate line by line
            var lines = ExpandInlineBlocks(processed.Split('\n'));

            // Pre-scan class names and parameterless constructors so derived
            // classes can rely on an implicit base() call.
            _knownClasses = Regex.Matches(processed, @"\bclass\s+(\w+)")
                .Select(m => m.Groups[1].Value)
                .ToHashSet();
            _defaultCtorClasses = Regex.Matches(processed, @"\b(?:public\s+)?(\w+)\s*\(\s*\)\s*\{")
                .Select(m => m.Groups[1].Value)
                .Where(name => _knownClasses.Contains(name))
                .ToHashSet();
            
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
                if (trimmed.StartsWith("interface "))
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

    /// <summary>
    /// Splits `void main() { ... }` and `if (x) { ... }` so the line-based
    /// translator can handle one-line blocks.
    /// </summary>
    private static List<string> ExpandInlineBlocks(IEnumerable<string> lines)
    {
        var expanded = new List<string>();

        foreach (var line in lines)
        {
            var trimmed = line.Trim();
            if (trimmed.StartsWith("class ") || trimmed.StartsWith("interface ") ||
                trimmed.StartsWith("enum ") || trimmed.StartsWith("using ") ||
                trimmed.StartsWith("use "))
            {
                expanded.Add(line);
                continue;
            }

            var open = trimmed.IndexOf('{');
            var close = trimmed.LastIndexOf('}');
            if (open < 0 || close <= open)
            {
                expanded.Add(line);
                continue;
            }

            var header = trimmed.Substring(0, open).Trim();
            // Only expand blocks: a method/if/for/while header contains "()".
            if (!header.Contains('(') || !header.Contains(')'))
            {
                expanded.Add(line);
                continue;
            }

            var body = trimmed.Substring(open + 1, close - open - 1).Trim();
            expanded.Add(header + " {");
            foreach (var statement in SplitTopLevelStatements(body))
            {
                expanded.Add(statement);
            }
            expanded.Add("}");
        }

        return expanded;
    }

    private static IEnumerable<string> SplitTopLevelStatements(string body)
    {
        var start = 0;
        var depth = 0;

        for (var i = 0; i < body.Length; i++)
        {
            var ch = body[i];
            if (ch == '{' || ch == '(' || ch == '[')
            {
                depth++;
            }
            else if (ch == '}' || ch == ')' || ch == ']')
            {
                depth--;
            }
            else if (ch == ';' && depth == 0)
            {
                var statement = body.Substring(start, i - start + 1).Trim();
                if (statement.Length > 0)
                {
                    yield return statement;
                }
                start = i + 1;
            }
        }

        var tail = body.Substring(Math.Min(start, body.Length)).Trim();
        if (tail.Length > 0)
        {
            yield return tail;
        }
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
        // class Dog extends Animal { -> public class Dog : Animal {
        // class Circle implements Shape { -> public class Circle : Shape {
        // class Dog extends Animal implements Drawable { -> public class Dog : Animal, Drawable {
        
        // First try: class Name extends Base implements Interface1, Interface2
        var matchExtImpl = Regex.Match(trimmed, @"class\s+(\w+)\s+extends\s+(\w+)\s+implements\s+([\w,\s]+)");
        if (matchExtImpl.Success)
        {
            var className = matchExtImpl.Groups[1].Value;
            var baseClass = matchExtImpl.Groups[2].Value;
            var interfaces = matchExtImpl.Groups[3].Value;
            var bases = new List<string> { baseClass };
            bases.AddRange(interfaces.Split(',').Select(i => i.Trim()));
            _output.AppendLine($"{INDENT_1}public class {className} : {string.Join(", ", bases)}");
            _output.AppendLine($"{INDENT_1}{{");
            OpenClass(className);
            return;
        }
        
        // Second try: class Name implements Interface1, Interface2
        var matchImpl = Regex.Match(trimmed, @"class\s+(\w+)\s+implements\s+([\w,\s]+)");
        if (matchImpl.Success)
        {
            var className = matchImpl.Groups[1].Value;
            var interfaces = matchImpl.Groups[2].Value;
            var bases = interfaces.Split(',').Select(i => i.Trim()).ToArray();
            _output.AppendLine($"{INDENT_1}public class {className} : {string.Join(", ", bases)}");
            _output.AppendLine($"{INDENT_1}{{");
            OpenClass(className);
            return;
        }
        
        // Third try: class Name extends Base
        var matchExt = Regex.Match(trimmed, @"class\s+(\w+)\s+extends\s+(\w+)");
        if (matchExt.Success)
        {
            var className = matchExt.Groups[1].Value;
            var baseClass = matchExt.Groups[2].Value;
            _output.AppendLine($"{INDENT_1}public class {className} : {baseClass}");
            _output.AppendLine($"{INDENT_1}{{");
            OpenClass(className);
            return;
        }
        
        // Fourth try: class Name (no inheritance)
        var matchSimple = Regex.Match(trimmed, @"class\s+(\w+)");
        if (matchSimple.Success)
        {
            var className = matchSimple.Groups[1].Value;
            _output.AppendLine($"{INDENT_1}public class {className}");
            _output.AppendLine($"{INDENT_1}{{");
            OpenClass(className);
            return;
        }
    }

    private void OpenClass(string className)
    {
        _inClass = true;
        _currentClass = className;
        _braceDepth++;
        _classDepth = _braceDepth;

        // C# requires a parameterless base constructor for derived classes.
        // Add one when the Hust class has only parameterized constructors.
        if (!_defaultCtorClasses.Contains(className))
        {
            _output.AppendLine($"{INDENT_2}public {className}() {{ }}");
        }
    }

    private void TranslateInterfaceDeclaration(string trimmed)
    {
        // interface Foo { -> public interface Foo {
        // Hust 只有一种接口：interface
        // 形状接口是接口的一种形态（无类型参数），不是独立关键字
        var match = Regex.Match(trimmed, @"interface\s+(\w+)");
        if (match.Success)
        {
            var interfaceName = match.Groups[1].Value;
            _output.AppendLine($"{INDENT_1}public interface {interfaceName}");
            _output.AppendLine($"{INDENT_1}{{");
            _inInterface = true;
            _braceDepth++;
            _interfaceDepth = _braceDepth;
        }
    }

    private bool IsMethodDeclaration(string trimmed)
    {
        // Constructors: ClassName(...) or public ClassName(...)
        if (_inClass && !string.IsNullOrEmpty(_currentClass))
        {
            var ctorProbe = trimmed.StartsWith("public ") ? trimmed.Substring(7).Trim() : trimmed;
            if (Regex.IsMatch(ctorProbe, $@"^{Regex.Escape(_currentClass)}\s*\("))
                return true;
        }

        // Regular method: Type name(...) with an optional access modifier.
        if (Regex.IsMatch(trimmed, $@"^(public\s+|private\s+|protected\s+)?{TypePattern}\s+\w+\s*\("))
            return true;

        // Shape interface method: draw(); or public draw();
        if (_inInterface)
        {
            var trimmedForShape = trimmed;
            if (trimmedForShape.StartsWith("public "))
            {
                trimmedForShape = trimmedForShape.Substring(7).Trim();
            }
            if (!Regex.IsMatch(trimmedForShape, $@"^{TypePattern}\s+") &&
                Regex.IsMatch(trimmedForShape, @"^\w+\s*\(.*?\)(\s*;)?$"))
                return true;
        }

        // Shape method implementation inside a class: public draw() { ... }
        if (_inClass && !string.IsNullOrWhiteSpace(_currentClass))
        {
            var candidate = trimmed.StartsWith("public ") ? trimmed.Substring(7).Trim() : trimmed;
            if (!Regex.IsMatch(candidate, @"^(if|for|while|switch|catch|using|return|else|do)\b") &&
                Regex.IsMatch(candidate, @"^\w+\s*\([^)]*\)\s*\{?$"))
                return true;
        }

        return false;
    }

    private bool TranslateMethodDeclaration(string trimmed)
    {
        // Constructors: Person(...) or public Person(...)
        if (_inClass && !string.IsNullOrEmpty(_currentClass))
        {
            var ctorProbe = trimmed.StartsWith("public ") ? trimmed.Substring(7).Trim() : trimmed;
            var ctorMatch = Regex.Match(ctorProbe, $@"^{Regex.Escape(_currentClass)}\s*\((.*?)\)");
            if (ctorMatch.Success)
            {
                _definedFunctions.Add(_currentClass);
                _output.AppendLine($"{INDENT_2}public {_currentClass}({TranslateParameters(ctorMatch.Groups[1].Value)})");
                _output.AppendLine($"{INDENT_2}{{");
                _inMethod = true;
                _braceDepth++;
                _methodDepth = _braceDepth;
                return false;
            }
        }

        // Shape interface method: draw(); or public draw();
        if (_inInterface)
        {
            var trimmedForShape = trimmed;
            if (trimmedForShape.StartsWith("public "))
            {
                trimmedForShape = trimmedForShape.Substring(7).Trim();
            }

            if (!Regex.IsMatch(trimmedForShape, $@"^{TypePattern}\s+") &&
                Regex.IsMatch(trimmedForShape, @"^\w+\s*\(.*?\)(\s*;)?$"))
            {
                var shapeMatchWithReturn = Regex.Match(trimmedForShape, @"^(\w+)\s+(\w+)\s*\((.*?)\)(\s*;)?$");
                if (shapeMatchWithReturn.Success)
                {
                    var shapeMethodName = shapeMatchWithReturn.Groups[2].Value;
                    var shapeParameters = shapeMatchWithReturn.Groups[3].Value;
                    _definedFunctions.Add(shapeMethodName);
                    _output.AppendLine($"{INDENT_2}object {shapeMethodName}({TranslateShapeParameters(shapeParameters)});");
                    return false;
                }

                var shapeMatch = Regex.Match(trimmedForShape, @"^(\w+)\s*\((.*?)\)(\s*;)?$");
                if (shapeMatch.Success)
                {
                    var shapeMethodName = shapeMatch.Groups[1].Value;
                    var shapeParameters = shapeMatch.Groups[2].Value;
                    _definedFunctions.Add(shapeMethodName);
                    _output.AppendLine($"{INDENT_2}void {shapeMethodName}({TranslateShapeParameters(shapeParameters)});");
                    return false;
                }
            }
        }

        // Shape method implementation inside a class: public draw() { ... }
        if (_inClass && !string.IsNullOrWhiteSpace(_currentClass))
        {
            var candidate = trimmed.StartsWith("public ") ? trimmed.Substring(7).Trim() : trimmed;
            if (!Regex.IsMatch(candidate, @"^(if|for|while|switch|catch|using|return|else|do)\b") &&
                !Regex.IsMatch(candidate, $@"^{TypePattern}\s+"))
            {
                var shapeImpl = Regex.Match(candidate, @"^(\w+)\s*\((.*?)\)\s*\{?$");
                if (shapeImpl.Success && shapeImpl.Groups[1].Value != _currentClass)
                {
                    var shapeMethodName = shapeImpl.Groups[1].Value;
                    _definedFunctions.Add(shapeMethodName);
                    _output.AppendLine($"{INDENT_2}public void {shapeMethodName}({TranslateParameters(shapeImpl.Groups[2].Value)})");
                    _output.AppendLine($"{INDENT_2}{{");
                    _inMethod = true;
                    _braceDepth++;
                    _methodDepth = _braceDepth;
                    return false;
                }
            }
        }

        // Regular method with a return type.
        var trimmedForParse = trimmed;
        if (trimmedForParse.StartsWith("public "))
        {
            trimmedForParse = trimmedForParse.Substring(7).Trim();
        }

        var match = Regex.Match(trimmedForParse, $@"^({TypePattern})\s+(\w+)\s*\((.*?)\)(\s*;)?");
        if (!match.Success) return false;

        var returnType = TranslateType(match.Groups[1].Value);
        var methodName = match.Groups[2].Value;
        var parameters = match.Groups[3].Value;
        var isInterfaceMethod = match.Groups[4].Success;
        var isMain = methodName.ToLower() == "main";

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
            _output.AppendLine($"{INDENT_2}{returnType} {methodName}({TranslateParameters(parameters)});");
            return false;
        }
        else if (_inClass)
        {
            _output.AppendLine($"{INDENT_2}public {returnType} {methodName}({TranslateParameters(parameters)})");
            _output.AppendLine($"{INDENT_2}{{");
            _inMethod = true;
        }
        else
        {
            _output.AppendLine($"{INDENT_1}public static {returnType} {methodName}({TranslateParameters(parameters)})");
            _output.AppendLine($"{INDENT_1}{{");
            _inMethod = true;
        }

        _braceDepth++;
        _methodDepth = _braceDepth;
        return isMain;
    }

    private string TranslateShapeParameters(string parameters)
    {
        // Shape interface parameters: just names, no types
        // e.g., "item" -> "object item", "key, value" -> "object key, object value"
        // Special case: "object factor" -> "object factor" (already has type)
        if (string.IsNullOrWhiteSpace(parameters))
            return "";
        
        // Add object type for each parameter (unless already typed)
        var parts = parameters.Split(',');
        var translated = parts.Select(p =>
        {
            var trimmed = p.Trim();
            // Check if already has type (e.g., "object factor")
            if (trimmed.StartsWith("object ") || trimmed.StartsWith("dynamic "))
                return trimmed;
            return $"object {trimmed}";
        });
        return string.Join(", ", translated);
    }

    private bool IsVariableDeclaration(string trimmed)
    {
        var candidate = trimmed.StartsWith("const ") ? trimmed.Substring(6).Trim() : trimmed;
        if (candidate.StartsWith("return ") || candidate.StartsWith("new "))
        {
            return false;
        }
        return Regex.IsMatch(candidate, $@"^{TypePattern}\s+\w+");
    }

    private void TranslateVariableDeclaration(string line, string trimmed)
    {
        var isConst = trimmed.StartsWith("const ");
        var candidate = isConst ? trimmed.Substring(6).Trim() : trimmed;

        var match = Regex.Match(candidate, $@"^({TypePattern})\s+(\w+)(.*)$");
        if (!match.Success)
        {
            TranslateExpressionStatement(line, trimmed);
            return;
        }

        var hustType = match.Groups[1].Value;
        var type = MapType(hustType);
        var name = match.Groups[2].Value;
        var rest = match.Groups[3].Value;

        // Semantic check: type mismatch
        CheckTypeMismatch(type, rest);

        // Map types and methods inside the initializer: new i32[3] -> new int[3].
        rest = TranslateExpression(rest);
        rest = TranslateArrayInitializer(hustType, rest);

        var prefix = isConst ? "const " : "";

        string indent;
        if (_inClass && !_inMethod)
        {
            indent = INDENT_2;
            _output.AppendLine($"{indent}public {prefix}{type} {name}{rest}");
        }
        else if (_inMethod)
        {
            indent = _inClass ? INDENT_3 : INDENT_2;
            _output.AppendLine($"{indent}{prefix}{type} {name}{rest}");
        }
        else
        {
            indent = INDENT_1;
            _output.AppendLine($"{indent}{prefix}{type} {name}{rest}");
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
        AdjustDepth(trimmed);
    }

    private void TranslateReturnStatement(string line, string trimmed)
    {
        var indent = _inClass ? INDENT_3 : INDENT_2;
        _output.AppendLine($"{indent}{TranslateExpression(trimmed)}");
    }

    private void TranslateClosingBrace()
    {
        _braceDepth--;

        if (_inMethod && _braceDepth < _methodDepth)
        {
            var indent = _inClass ? INDENT_2 : INDENT_1;
            _output.AppendLine($"{indent}}}");
            _inMethod = false;
            _methodDepth = -1;
        }
        else if (_inInterface && _braceDepth < _interfaceDepth)
        {
            _output.AppendLine($"{INDENT_1}}}");
            _inInterface = false;
            _interfaceDepth = -1;
        }
        else if (_inClass && _braceDepth < _classDepth)
        {
            _output.AppendLine($"{INDENT_1}}}");
            _inClass = false;
            _currentClass = "";
            _classDepth = -1;
        }
        else
        {
            // Inner block (if/for/while/switch) inside a method or class.
            var indent = _inMethod ? (_inClass ? INDENT_3 : INDENT_2) : INDENT_2;
            _output.AppendLine($"{indent}}}");
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
        
        // Enums are nested inside Program; make them public so public methods
        // can expose them.
        if (trimmed.StartsWith("enum ") && !trimmed.StartsWith("public "))
        {
            trimmed = "public " + trimmed;
        }

        // Check for undefined function calls
        CheckUndefinedFunction(trimmed);
        
        // Method body statements: fixed indent, ignore preprocessor indent
        var stmtIndent = _inClass ? INDENT_3 : INDENT_2;
        _output.AppendLine($"{stmtIndent}{TranslateExpression(trimmed)}");
        AdjustDepth(trimmed);
    }

    private void AdjustDepth(string text)
    {
        _braceDepth += text.Count(c => c == '{') - text.Count(c => c == '}');
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
        // Use the comprehensive type mapping
        return MapType(type);
    }

    private string TranslateParameters(string parameters)
    {
        if (string.IsNullOrWhiteSpace(parameters))
            return "";
        
        // Simple parameter translation: i32 x, String y -> int x, string y
        var parts = parameters.Split(',');
        var translated = parts.Select(p =>
        {
            var trimmed = p.Trim();
            var match = Regex.Match(trimmed, $@"^({TypePattern})\s+(\w+)$");
            if (match.Success)
            {
                return $"{MapType(match.Groups[1].Value)} {match.Groups[2].Value}";
            }
            // Untyped shape-interface parameter: use object.
            if (Regex.IsMatch(trimmed, @"^\w+$"))
            {
                return $"object {trimmed}";
            }
            return trimmed;
        });
        
        return string.Join(", ", translated);
    }

    /// <summary>
    /// C# requires "new T[] { ... }" for nested array initializers, while Hust
    /// uses the shorter "{ ... }" form.
    /// </summary>
    private string TranslateArrayInitializer(string hustType, string rest)
    {
        var literalMatch = Regex.Match(rest, @"^\s*=\s*(\{.*\})\s*;\s*$", RegexOptions.Singleline);
        if (!literalMatch.Success)
        {
            return rest;
        }

        var typeMatch = Regex.Match(hustType, @"^(\w+)((?:\[\d*\])*)$");
        if (!typeMatch.Success)
        {
            return rest;
        }

        var dims = Regex.Matches(typeMatch.Groups[2].Value, @"\[").Count;
        if (dims == 0)
        {
            return rest;
        }

        var baseType = MapType(typeMatch.Groups[1].Value);
        return $"= {BuildArrayInitializer(literalMatch.Groups[1].Value, baseType, dims)};";
    }

    private string BuildArrayInitializer(string literal, string baseType, int dims)
    {
        var text = literal.Trim();
        if (!text.StartsWith("{") || !text.EndsWith("}"))
        {
            return text;
        }

        var body = text.Substring(1, text.Length - 2).Trim();
        if (dims == 1)
        {
            return $"new {baseType}[] {text}";
        }

        var elements = SplitTopLevel(body);
        var rendered = elements
            .Where(e => !string.IsNullOrWhiteSpace(e))
            .Select(e => BuildArrayInitializer(e.Trim(), baseType, dims - 1));
        var arrayType = baseType + string.Concat(Enumerable.Repeat("[]", dims));
        return $"new {arrayType} {{ {string.Join(", ", rendered)} }}";
    }

    private static List<string> SplitTopLevel(string text)
    {
        var parts = new List<string>();
        var current = new System.Text.StringBuilder();
        var depth = 0;

        foreach (var ch in text)
        {
            if (ch == '{' || ch == '(' || ch == '[')
            {
                depth++;
            }
            else if (ch == '}' || ch == ')' || ch == ']')
            {
                depth--;
            }

            if (ch == ',' && depth == 0)
            {
                parts.Add(current.ToString());
                current.Clear();
            }
            else
            {
                current.Append(ch);
            }
        }

        if (current.Length > 0)
        {
            parts.Add(current.ToString());
        }
        return parts;
    }

    private string TranslateExpression(string expr)
    {
        // Translate Hust types to C# types in expressions.
        expr = Regex.Replace(expr, @"\b(i8|i16|i32|i64|u8|u16|u32|u64|f32|f64|bool|String|char)\s+(?=\w)",
            match => MapType(match.Groups[1].Value) + " ");

        // Translate array types: i32[] -> int[], new i32[3] -> new int[3]
        expr = Regex.Replace(expr, @"\b(i8|i16|i32|i64|u8|u16|u32|u64|f32|f64|bool|String|char)((?:\[\d*\])+)",
            match => MapType(match.Groups[1].Value) + match.Groups[2].Value);

        // Array/string length: arr.len() -> arr.Length
        expr = Regex.Replace(expr, @"\.(?:len|length)\(\)", ".Length");

        // String methods
        expr = Regex.Replace(expr, @"([\w.]+)\.charAt\(([^)]+)\)", "$1[$2]");
        expr = Regex.Replace(expr, @"([\w.]+)\.substring\(([^,]+),\s*([^)]+)\)",
            match => $"{match.Groups[1].Value}.Substring({match.Groups[2].Value}, {match.Groups[3].Value} - {match.Groups[2].Value})");
        expr = Regex.Replace(expr, @"\.toUpperCase\(\)", ".ToUpper()");
        expr = Regex.Replace(expr, @"\.toLowerCase\(\)", ".ToLower()");
        expr = Regex.Replace(expr, @"\.trim\(\)", ".Trim()");
        expr = Regex.Replace(expr, @"\.trimLeft\(\)", ".TrimStart()");
        expr = Regex.Replace(expr, @"\.trimRight\(\)", ".TrimEnd()");
        expr = Regex.Replace(expr, @"\.contains\(", ".Contains(");
        expr = Regex.Replace(expr, @"\.startsWith\(", ".StartsWith(");
        expr = Regex.Replace(expr, @"\.endsWith\(", ".EndsWith(");
        expr = Regex.Replace(expr, @"\.indexOf\(", ".IndexOf(");
        expr = Regex.Replace(expr, @"\.replace\(", ".Replace(");
        expr = Regex.Replace(expr, @"\.split\(", ".Split(");

        // Fix float literals: add F suffix for C#
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
