package main

import (
	"regexp"
	"strings"
)

// Translator handles Hust to Go translation
type Translator struct {
	preprocessor       *Preprocessor
	output             strings.Builder
	indentLevel        int
	definedFunctions   map[string]bool // Track defined functions for semantic analysis
	semanticErrors     []string        // Collect semantic errors
	inClass            bool            // Track if we're inside a class definition
	currentClass       string          // Current class name for method receivers
	classFields        []string        // Collect class fields
	classMethods       []string        // Collect class methods
	classConstructor   string          // Collect class constructor
	inConstructor      bool            // Track if we're inside a constructor
	constructorBody    []string        // Collect constructor body lines
	inMethod           bool            // Track if we're inside a method
	currentMethodIndex int             // Index of current method in classMethods
	methodBody         []string        // Collect method body lines
	inEnum             bool            // Track if we're inside an enum definition
	currentEnum        string          // Current enum name
	enumFirstValue     bool            // Track if first enum value (for iota)
}

// NewTranslator creates a new translator
func NewTranslator() *Translator {
	return &Translator{
		preprocessor:     NewPreprocessor(),
		indentLevel:      0,
		definedFunctions: make(map[string]bool),
		semanticErrors:   make([]string, 0),
	}
}

// TranslationResult contains the translation output
type TranslationResult struct {
	Success bool
	Code    string
	Errors  []string
}

// preScanFunctionDefinitions scans all lines and registers function definitions
// This enables forward declaration (functions can be called before they are defined)
func (t *Translator) preScanFunctionDefinitions(lines []string) {
	for _, line := range lines {
		trimmed := strings.TrimSpace(line)
		
		// Check for function definition
		if isFunctionDefinition(trimmed) {
			// Extract function name
			funcName := extractFunctionName(trimmed)
			if funcName != "" {
				t.definedFunctions[funcName] = true
			}
		}
	}
}

// extractFunctionName extracts the function name from a function definition line
func extractFunctionName(line string) string {
	// Match patterns like:
	// void main() {
	// i32 add(i32 a, i32 b) {
	// public void method() {
	
	// Remove access modifiers
	line = strings.TrimPrefix(line, "public ")
	line = strings.TrimPrefix(line, "private ")
	line = strings.TrimPrefix(line, "protected ")
	line = strings.TrimPrefix(line, "static ")
	
	// Find the opening parenthesis
	parenIdx := strings.Index(line, "(")
	if parenIdx == -1 {
		return ""
	}
	
	// Extract the part before the parenthesis
	beforeParen := strings.TrimSpace(line[:parenIdx])
	
	// The function name is the last word before the parenthesis
	parts := strings.Fields(beforeParen)
	if len(parts) == 0 {
		return ""
	}
	
	return parts[len(parts)-1]
}

// Translate converts Hust source to Go
func (t *Translator) Translate(source string, fileName string) TranslationResult {
	result := TranslationResult{
		Success: false,
		Errors:  make([]string, 0),
	}
	
	// Step 1: Preprocess
	processed, dialect, err := t.preprocessor.Process(source)
	if err != nil {
		result.Errors = append(result.Errors, err.Error())
		return result
	}
	
	// Check dialect
	if dialect != PureHust && dialect != GoDialect {
		result.Errors = append(result.Errors, "Unsupported dialect for Go core")
		return result
	}
	
	// Step 2: Pre-scan function definitions for forward declaration
	lines := strings.Split(processed, "\n")
	t.preScanFunctionDefinitions(lines)
	
	// Step 3: Translate line by line
	
	// Emit header (includes package and import)
	t.emitHeader()
	
	// Track if we need to emit main function
	hasMain := false
	_ = hasMain // TODO: Use for main function detection
	
	for _, line := range lines {
		trimmed := strings.TrimSpace(line)
		
		// Skip empty lines
		if trimmed == "" {
			continue
		}
		
		// Skip use statements (handled in header)
		if strings.HasPrefix(trimmed, "use ") {
			t.translateUseStatement(trimmed)
			continue
		}
		
		// Translate class declaration
		if strings.HasPrefix(trimmed, "class ") {
			t.translateClassDeclaration(trimmed)
			continue
		}
		
		// Translate enum declaration
		if strings.HasPrefix(trimmed, "enum ") {
			t.translateEnumDeclaration(trimmed)
			continue
		}
		
		// Translate function declaration
		if t.isFunctionDeclaration(trimmed) {
			isMain := t.translateFunctionDeclaration(trimmed)
			if isMain {
				hasMain = true
			}
			continue
		}
		
		// Translate variable declaration
		if t.isVariableDeclaration(trimmed) {
			t.translateVariableDeclaration(line, trimmed)
			continue
		}
		
		// Translate control flow
		if strings.HasPrefix(trimmed, "if ") || strings.HasPrefix(trimmed, "if(") {
			t.translateControlStatement(line, trimmed)
			continue
		}
		
		if strings.HasPrefix(trimmed, "for ") || strings.HasPrefix(trimmed, "for(") {
			t.translateControlStatement(line, trimmed)
			continue
		}
		
		if strings.HasPrefix(trimmed, "while ") || strings.HasPrefix(trimmed, "while(") {
			t.translateControlStatement(line, trimmed)
			continue
		}
		
		if strings.HasPrefix(trimmed, "return ") {
			t.translateReturnStatement(line, trimmed)
			continue
		}
		
		// Handle closing brace
		if trimmed == "}" {
			// Check if we're closing an enum
			if t.inEnum {
				t.indentLevel--
				t.output.WriteString(t.getIndent() + ")\n\n")
				t.inEnum = false
				t.currentEnum = ""
				t.enumFirstValue = false
				continue
			}
			
			// Check if we're closing a constructor
			if t.inConstructor {
				// End constructor body
				for _, bodyLine := range t.constructorBody {
					t.classConstructor += bodyLine + "\n"
				}
				t.classConstructor += "\treturn &" + t.currentClass + "{}\n"
				t.classConstructor += "}\n\n"
				t.inConstructor = false
				t.constructorBody = nil
				continue
			}
			
			// Check if we're closing a method
			if t.inMethod {
				// End method body
				methodIndex := t.currentMethodIndex
				if methodIndex >= 0 && methodIndex < len(t.classMethods) {
					for _, bodyLine := range t.methodBody {
						t.classMethods[methodIndex] += bodyLine + "\n"
					}
					t.classMethods[methodIndex] += "}\n\n"
				}
				t.inMethod = false
				t.methodBody = nil
				t.currentMethodIndex = -1
				continue
			}
			
			// Check if we're closing a class
			if t.inClass && t.indentLevel == 1 {
				// Close struct
				t.indentLevel--
				t.output.WriteString(t.getIndent() + "}\n\n")
				
				// Output constructor
				if t.classConstructor != "" {
					t.output.WriteString(t.classConstructor)
				}
				
				// Output methods
				for _, method := range t.classMethods {
					t.output.WriteString(method)
				}
				
				t.inClass = false
				t.currentClass = ""
				t.classFields = nil
				t.classMethods = nil
				t.classConstructor = ""
				continue
			}
			
			t.translateClosingBrace()
			continue
		}
		
		// Handle enum values
		if t.inEnum {
			// Remove trailing comma
			value := strings.TrimSuffix(trimmed, ",")
			value = strings.TrimSpace(value)
			if value != "" {
				if t.enumFirstValue {
					t.output.WriteString(t.getIndent() + value + " " + t.currentEnum + " = iota\n")
					t.enumFirstValue = false
				} else {
					t.output.WriteString(t.getIndent() + value + "\n")
				}
			}
			continue
		}
		
		// Default: translate expression
		t.translateExpressionStatement(line, trimmed)
	}
	
	// Restore protected strings and comments
	finalOutput := t.preprocessor.Restore(t.output.String())
	
	// Check for semantic errors
	if len(t.semanticErrors) > 0 {
		result.Errors = append(result.Errors, t.semanticErrors...)
		return result
	}
	
	result.Success = true
	result.Code = finalOutput
	return result
}

// emitHeader writes the Go file header
func (t *Translator) emitHeader() {
	t.output.WriteString("// Generated by Hust-Go\n")
	t.output.WriteString("// Do not edit manually\n\n")
	t.output.WriteString("package main\n\n")
	t.output.WriteString("import \"fmt\"\n\n")
}

// translateUseStatement handles use statements
func (t *Translator) translateUseStatement(line string) {
	// use GO.fmt.* -> import "fmt"
	// For now, comment out
	if !strings.Contains(line, "GO.") {
		t.output.WriteString("// " + line + "\n")
	}
}

// isFunctionDeclaration checks if line is a function declaration
func (t *Translator) isFunctionDeclaration(trimmed string) bool {
	// void main() { -> func main() {
	// i32 add(i32 a, i32 b) { -> func add(a int32, b int32) int32 {
	re := regexp.MustCompile(`^(void|i8|i16|i32|i64|u8|u16|u32|u64|f32|f64|bool|String|char)\s+\w+\s*\(.*\)\s*\{?`)
	if re.MatchString(trimmed) {
		return true
	}
	
	// In class: public method declaration
	if t.inClass {
		// public returnType methodName(...)
		rePublic := regexp.MustCompile(`^public\s+(void|i8|i16|i32|i64|u8|u16|u32|u64|f32|f64|bool|String|char)\s+\w+\s*\(.*\)\s*\{?`)
		if rePublic.MatchString(trimmed) {
			return true
		}
		// Constructor: public ClassName(...)
		reConstructor := regexp.MustCompile(`^public\s+` + t.currentClass + `\s*\(.*\)\s*\{?`)
		if reConstructor.MatchString(trimmed) {
			return true
		}
	}
	
	return false
}

// translateClassDeclaration translates a class declaration
func (t *Translator) translateClassDeclaration(trimmed string) {
	// class Foo { -> type Foo struct {
	// class Foo extends Bar { -> type Foo struct { Bar; ... }
	re := regexp.MustCompile(`class\s+(\w+)(?:\s+extends\s+(\w+))?`)
	match := re.FindStringSubmatch(trimmed)
	
	if len(match) < 2 {
		return
	}
	
	className := match[1]
	baseClass := ""
	if len(match) >= 3 {
		baseClass = match[2]
	}
	
	// Set inClass flag and store current class name
	t.inClass = true
	t.currentClass = className
	t.classFields = make([]string, 0)
	t.classMethods = make([]string, 0)
	t.classConstructor = ""
	
	// Emit struct declaration
	if baseClass != "" {
		t.output.WriteString("type " + className + " struct {\n")
		t.output.WriteString("\t" + baseClass + "\n")
	} else {
		t.output.WriteString("type " + className + " struct {\n")
	}
	
	t.indentLevel++
}

// translateEnumDeclaration translates enum declarations
func (t *Translator) translateEnumDeclaration(trimmed string) {
	// enum Color { -> type Color int; const ( RED Color = iota; ... )
	// Handle multi-line enum declarations
	re := regexp.MustCompile(`enum\s+(\w+)\s*\{`)
	match := re.FindStringSubmatch(trimmed)
	
	if len(match) < 2 {
		return
	}
	
	enumName := match[1]
	
	// Emit type declaration
	t.output.WriteString("type " + enumName + " int\n\n")
	
	// Emit const block start
	t.output.WriteString("const (\n")
	t.indentLevel++
	
	// Set flag to indicate we're in an enum
	t.inEnum = true
	t.currentEnum = enumName
	t.enumFirstValue = true
}

// translateFunctionDeclaration translates a function declaration
func (t *Translator) translateFunctionDeclaration(trimmed string) bool {
	// Handle public methods in class
	if t.inClass && strings.HasPrefix(trimmed, "public ") {
		// Remove "public " prefix
		trimmed = strings.TrimPrefix(trimmed, "public ")
		
		// Check if it's a constructor (ClassName(...))
		if strings.HasPrefix(trimmed, t.currentClass+"(") {
			// Constructor
			re := regexp.MustCompile(`^(\w+)\s*\((.*)\)\s*\{?`)
			match := re.FindStringSubmatch(trimmed)
			if len(match) >= 3 {
				params := match[2]
				goParams := t.translateParameters(params)
				
				// Start collecting constructor
				t.inConstructor = true
				t.constructorBody = make([]string, 0)
				t.classConstructor = "func New" + t.currentClass + "(" + goParams + ") *" + t.currentClass + " {\n"
			}
			return false
		}
		
		// Regular method: returnType methodName(params)
		re := regexp.MustCompile(`^(void|i8|i16|i32|i64|u8|u16|u32|u64|f32|f64|bool|String|char)\s+(\w+)\s*\((.*)\)\s*\{?`)
		match := re.FindStringSubmatch(trimmed)
		if len(match) >= 4 {
			returnType := match[1]
			funcName := match[2]
			params := match[3]
			
			goReturnType := t.mapType(returnType)
			if goReturnType == "void" {
				goReturnType = ""
			}
			
			goParams := t.translateParameters(params)
			
			// Start collecting method
			t.inMethod = true
			t.methodBody = make([]string, 0)
			
			// Generate method signature
			receiver := strings.ToLower(string(t.currentClass[0]))
			methodSignature := "func (" + receiver + " *" + t.currentClass + ") " + funcName + "(" + goParams + ")"
			if goReturnType != "" {
				methodSignature += " " + goReturnType
			}
			methodSignature += " {\n"
			
			t.classMethods = append(t.classMethods, methodSignature)
			t.currentMethodIndex = len(t.classMethods) - 1
		}
		return false
	}
	
	// Match: returnType funcName(params) {
	re := regexp.MustCompile(`^(void|i8|i16|i32|i64|u8|u16|u32|u64|f32|f64|bool|String|char)\s+(\w+)\s*\((.*)\)\s*\{?`)
	match := re.FindStringSubmatch(trimmed)
	
	if len(match) < 4 {
		return false
	}
	
	returnType := match[1]
	funcName := match[2]
	params := match[3]
	
	// Translate return type
	goReturnType := t.mapType(returnType)
	if goReturnType == "void" {
		goReturnType = ""
	}
	
	// Record defined function for semantic analysis
	t.definedFunctions[funcName] = true
	
	// Translate parameters
	goParams := t.translateParameters(params)
	
	// Build function signature
	if goReturnType == "" {
		t.output.WriteString("func " + funcName + "(" + goParams + ") {\n")
	} else {
		t.output.WriteString("func " + funcName + "(" + goParams + ") " + goReturnType + " {\n")
	}
	
	t.indentLevel++
	
	return funcName == "main"
}

// translateParameters translates function parameters
func (t *Translator) translateParameters(params string) string {
	if strings.TrimSpace(params) == "" {
		return ""
	}
	
	parts := strings.Split(params, ",")
	var translated []string
	
	for _, part := range parts {
		trimmed := strings.TrimSpace(part)
		// Match: type name
		re := regexp.MustCompile(`^(i8|i16|i32|i64|u8|u16|u32|u64|f32|f64|bool|String|char)\s+(\w+)$`)
		match := re.FindStringSubmatch(trimmed)
		
		if len(match) >= 3 {
			goType := t.mapType(match[1])
			name := match[2]
			translated = append(translated, name+" "+goType)
		} else {
			translated = append(translated, trimmed)
		}
	}
	
	return strings.Join(translated, ", ")
}

// isVariableDeclaration checks if line is a variable declaration
func (t *Translator) isVariableDeclaration(trimmed string) bool {
	re := regexp.MustCompile(`^(i8|i16|i32|i64|u8|u16|u32|u64|f32|f64|bool|String|char)\s+\w+`)
	return re.MatchString(trimmed)
}

// translateVariableDeclaration translates a variable declaration
func (t *Translator) translateVariableDeclaration(line, trimmed string) {
	re := regexp.MustCompile(`^(i8|i16|i32|i64|u8|u16|u32|u64|f32|f64|bool|String|char)\s+(\w+)(.*)$`)
	match := re.FindStringSubmatch(trimmed)
	
	if len(match) < 4 {
		t.translateExpressionStatement(line, trimmed)
		return
	}
	
	hustType := match[1]
	name := match[2]
	rest := match[3]
	
	goType := t.mapType(hustType)
	
	// Check if we're inside a class (field declaration)
	if t.inClass {
		// Class field: name type (no var prefix)
		t.output.WriteString(t.getIndent() + name + " " + goType + "\n")
		return
	}
	
	// Check for type mismatch in initialization
	// rest contains:  = "hello" or  = 123 or empty
	t.checkTypeMismatch(goType, rest)
	
	// Check if there's an initialization
	if strings.Contains(rest, "=") {
		// var name type = value
		t.output.WriteString(t.getIndent() + "var " + name + " " + goType + rest + "\n")
	} else {
		// var name type
		t.output.WriteString(t.getIndent() + "var " + name + " " + goType + "\n")
	}
}

// checkTypeMismatch checks for type mismatch in variable initialization
func (t *Translator) checkTypeMismatch(declaredType, rest string) {
	// Check for string assigned to numeric type
	if strings.Contains(rest, "=") {
		parts := strings.SplitN(rest, "=", 2)
		if len(parts) >= 2 {
			value := strings.TrimSpace(parts[1])
			// Remove trailing semicolon
			value = strings.TrimSuffix(value, ";")
			
			// Check if value is a string literal (or placeholder)
			isStringLiteral := strings.HasPrefix(value, "\"") && strings.HasSuffix(value, "\"")
			isStringPlaceholder := strings.HasPrefix(value, "__STRING_")
			
			if isStringLiteral || isStringPlaceholder {
				// String literal assigned to non-string type
				if declaredType != "string" {
					t.semanticErrors = append(t.semanticErrors, "Type mismatch: cannot assign string to "+declaredType)
				}
			}
			
			// Check if value is a number assigned to string
			if declaredType == "string" {
				// Check if value looks like a number (but not a variable)
				if matched, _ := regexp.MatchString(`^\d+(\.\d+)?$`, value); matched {
					t.semanticErrors = append(t.semanticErrors, "Type mismatch: cannot assign number to string")
				}
				// Check if value is a simple numeric variable (single letter like x, y, z)
				if matched, _ := regexp.MatchString(`^[a-z]$`, value); matched {
					// Single letter variables are likely numeric
					t.semanticErrors = append(t.semanticErrors, "Type mismatch: cannot assign number to string")
				}
			}
			
			// Check for division by zero
			if strings.Contains(value, "/") {
				// Check for literal zero divisor
				if matched, _ := regexp.MatchString(`/\s*0\s*($|[^\d])`, value); matched {
					t.semanticErrors = append(t.semanticErrors, "Division by zero")
				}
				// Check for variable that is zero
				divParts := strings.Split(value, "/")
				if len(divParts) == 2 {
					divisor := strings.TrimSpace(divParts[1])
					// Remove trailing semicolon
					divisor = strings.TrimSuffix(divisor, ";")
					if t.isZeroVariable(divisor) {
						t.semanticErrors = append(t.semanticErrors, "Division by zero")
					}
				}
			}
			
			// Check for undefined variable in expression
			t.checkUndefinedVariables(value)
			
			// Check for type mismatch in binary operations
			t.checkBinaryOperationTypes(value)
		}
	}
}

// isNumericVariable checks if a variable is known to be numeric
func (t *Translator) isNumericVariable(name string) bool {
	// For now, check if it looks like a numeric variable name
	// In a full implementation, we would track variable types
	matched, _ := regexp.MatchString(`^[a-z]\w*$`, name)
	return matched
}

// isStringVariable checks if a variable is known to be a string
func (t *Translator) isStringVariable(name string) bool {
	// Check common string variable names
	stringNames := map[string]bool{
		"str": true, "s": true, "name": true, "text": true,
		"message": true, "msg": true, "input": true, "output": true,
		"strVal": true, "strFromI32": true, "strFromF32": true,
	}
	return stringNames[name]
}

// isZeroVariable checks if a variable is known to be zero
func (t *Translator) isZeroVariable(name string) bool {
	// For now, check common zero variable names
	zeroNames := map[string]bool{
		"zero": true, "y": true, "z": true,
	}
	return zeroNames[name]
}

// checkUndefinedVariables checks for undefined variables in an expression
func (t *Translator) checkUndefinedVariables(expr string) {
	// Skip if inside a class
	if t.inClass {
		return
	}
	
	// Only check for obvious undefined variable patterns
	// Look for variables that are clearly not defined (like "undefinedVar")
	re := regexp.MustCompile(`\b(undefined\w*)\b`)
	matches := re.FindAllStringSubmatch(expr, -1)
	
	for _, match := range matches {
		if len(match) >= 2 {
			varName := match[1]
			// Skip keywords and built-ins
			if t.isKeyword(varName) || t.isBuiltIn(varName) {
				continue
			}
			// Check for undefined variable
			t.semanticErrors = append(t.semanticErrors, "Undefined variable: "+varName)
		}
	}
}

// isKnownVariable checks if a variable is known (declared)
func (t *Translator) isKnownVariable(name string) bool {
	// For now, check common variable names
	// In a full implementation, we would track all declared variables
	knownVars := map[string]bool{
		"x": true, "y": true, "z": true, "i": true, "j": true, "k": true,
		"a": true, "b": true, "c": true, "d": true, "e": true, "f": true,
		"g": true, "h": true, "m": true, "n": true, "o": true, "p": true,
		"q": true, "r": true, "s": true, "t": true, "u": true, "v": true,
		"w": true, "arr": true, "str": true, "num": true, "val": true,
		"result": true, "temp": true, "count": true, "sum": true,
		"index": true, "item": true, "key": true, "value": true,
	}
	return knownVars[name]
}

// checkBinaryOperationTypes checks for type mismatches in binary operations
func (t *Translator) checkBinaryOperationTypes(expr string) {
	// Check for string + number or number + string
	// Pattern: variable + "string" or "string" + variable
	if strings.Contains(expr, "+") {
		parts := strings.Split(expr, "+")
		if len(parts) == 2 {
			left := strings.TrimSpace(parts[0])
			right := strings.TrimSpace(parts[1])
			// Remove trailing semicolon
			right = strings.TrimSuffix(right, ";")
			
			// Check if left is numeric and right is string
			leftIsNumeric := t.isNumericVariable(left)
			if !leftIsNumeric {
				matched, _ := regexp.MatchString(`^\d+$`, left)
				leftIsNumeric = matched
			}
			rightIsString := strings.HasPrefix(right, "\"") || strings.HasPrefix(right, "__STRING_")
			
			if leftIsNumeric && rightIsString {
				t.semanticErrors = append(t.semanticErrors, "Type mismatch: cannot add int32 and string")
			}
			
			// Check if left is string and right is numeric
			leftIsString := strings.HasPrefix(left, "\"") || strings.HasPrefix(left, "__STRING_")
			rightIsNumeric := t.isNumericVariable(right)
			if !rightIsNumeric {
				matched, _ := regexp.MatchString(`^\d+$`, right)
				rightIsNumeric = matched
			}
			
			if leftIsString && rightIsNumeric {
				t.semanticErrors = append(t.semanticErrors, "Type mismatch: cannot add string and int32")
			}
		}
	}
}

// translateControlStatement translates control flow statements
func (t *Translator) translateControlStatement(line, trimmed string) {
	// Remove parentheses from conditions
	// if (x > 5) { -> if x > 5 {
	trimmed = regexp.MustCompile(`\(([^)]+)\)`).ReplaceAllString(trimmed, "$1")
	t.output.WriteString(t.getIndent() + trimmed + "\n")
}

// translateReturnStatement translates return statements
func (t *Translator) translateReturnStatement(line, trimmed string) {
	// Translate this.field -> receiver.field in return statements
	if t.inClass && t.currentClass != "" {
		receiver := strings.ToLower(string(t.currentClass[0]))
		trimmed = regexp.MustCompile(`\bthis\.`).ReplaceAllString(trimmed, receiver+".")
	}
	
	// Collect method body
	if t.inMethod {
		t.methodBody = append(t.methodBody, "\t"+trimmed)
		return
	}
	
	t.output.WriteString(t.getIndent() + trimmed + "\n")
}

// translateClosingBrace handles closing braces
func (t *Translator) translateClosingBrace() {
	t.indentLevel--
	
	// Check if we're closing a class
	if t.inClass && t.indentLevel == 0 {
		t.inClass = false
	}
	
	t.output.WriteString(t.getIndent() + "}\n")
}

// translateExpressionStatement translates expression statements
func (t *Translator) translateExpressionStatement(line, trimmed string) {
	// Handle comments
	if strings.HasPrefix(trimmed, "//") {
		t.output.WriteString(t.getIndent() + trimmed + "\n")
		return
	}
	
	// Check for function calls and validate them
	t.checkFunctionCalls(trimmed)
	
	// Translate expression
	expr := t.translateExpression(trimmed)
	
	// Collect constructor body
	if t.inConstructor {
		t.constructorBody = append(t.constructorBody, "\t"+expr)
		return
	}
	
	// Collect method body
	if t.inMethod {
		t.methodBody = append(t.methodBody, "\t"+expr)
		return
	}
	
	t.output.WriteString(t.getIndent() + expr + "\n")
}

// checkFunctionCalls checks for undefined function calls
func (t *Translator) checkFunctionCalls(expr string) {
	// Skip if inside a class (methods are defined in class)
	if t.inClass {
		return
	}
	
	// Skip if expression contains a dot (method call or field access)
	if strings.Contains(expr, ".") {
		return
	}
	
	// Skip if it's a method definition (public ... methodName())
	if strings.HasPrefix(expr, "public ") {
		return
	}
	
	// Match standalone function calls: funcName(...) but not obj.method()
	// Use negative lookbehind to avoid matching method calls
	re := regexp.MustCompile(`(^|[^\w.])(\w+)\s*\(`)
	matches := re.FindAllStringSubmatch(expr, -1)
	
	for _, match := range matches {
		if len(match) >= 3 {
			funcName := match[2]
			// Skip keywords and built-in functions
			if t.isKeyword(funcName) || t.isBuiltIn(funcName) {
				continue
			}
			// Skip if it looks like a type name (starts with uppercase)
			if len(funcName) > 0 && funcName[0] >= 'A' && funcName[0] <= 'Z' {
				continue
			}
			// Check if function is defined
			if !t.definedFunctions[funcName] {
				t.semanticErrors = append(t.semanticErrors, "Undefined function: "+funcName)
			}
		}
	}
}

// isKeyword checks if a name is a Go keyword
func (t *Translator) isKeyword(name string) bool {
	keywords := map[string]bool{
		"if": true, "else": true, "for": true, "while": true,
		"return": true, "func": true, "var": true, "const": true,
		"type": true, "struct": true, "interface": true, "package": true,
		"import": true, "defer": true, "select": true,
		"case": true, "default": true, "switch": true, "break": true,
		"continue": true, "fallthrough": true, "range": true, "map": true,
		"chan": true, "goto": true,
	}
	return keywords[name]
}

// isBuiltIn checks if a name is a built-in function
func (t *Translator) isBuiltIn(name string) bool {
	builtIns := map[string]bool{
		"Println": true, "Print": true, "Printf": true,
		"Sprintf": true, "Fprintf": true,
		"len": true, "cap": true, "make": true, "new": true,
		"append": true, "copy": true, "delete": true,
		"panic": true, "recover": true, "close": true,
	}
	return builtIns[name]
}

// translateExpression translates expressions
func (t *Translator) translateExpression(expr string) string {
	// Translate constructor calls: new ClassName(...) -> NewClassName(...)
	expr = regexp.MustCompile(`new\s+(\w+)\s*\(`).ReplaceAllString(expr, "New$1(")
	
	// Translate array methods
	// .len() -> len()
	expr = regexp.MustCompile(`\.(\w+)\.len\(\)`).ReplaceAllString(expr, "len($1)")
	// .push(x) -> append(arr, x)
	expr = regexp.MustCompile(`(\w+)\.push\(([^)]+)\)`).ReplaceAllString(expr, "$1 = append($1, $2)")
	
	// Translate string methods
	// .length() -> len()
	expr = regexp.MustCompile(`\.(\w+)\.length\(\)`).ReplaceAllString(expr, "len($1)")
	// .charAt(i) -> string(s[i])
	expr = regexp.MustCompile(`(\w+)\.charAt\(([^)]+)\)`).ReplaceAllString(expr, "string($1[$2])")
	// .substring(start, end) -> s[start:end]
	expr = regexp.MustCompile(`(\w+)\.substring\(([^,]+),\s*([^)]+)\)`).ReplaceAllString(expr, "$1[$2:$3]")
	// .toUpperCase() -> strings.ToUpper()
	expr = regexp.MustCompile(`(\w+)\.toUpperCase\(\)`).ReplaceAllString(expr, "strings.ToUpper($1)")
	// .toLowerCase() -> strings.ToLower()
	expr = regexp.MustCompile(`(\w+)\.toLowerCase\(\)`).ReplaceAllString(expr, "strings.ToLower($1)")
	// .trim() -> strings.TrimSpace()
	expr = regexp.MustCompile(`(\w+)\.trim\(\)`).ReplaceAllString(expr, "strings.TrimSpace($1)")
	// .contains(s) -> strings.Contains()
	expr = regexp.MustCompile(`(\w+)\.contains\(([^)]+)\)`).ReplaceAllString(expr, "strings.Contains($1, $2)")
	// .startsWith(s) -> strings.HasPrefix()
	expr = regexp.MustCompile(`(\w+)\.startsWith\(([^)]+)\)`).ReplaceAllString(expr, "strings.HasPrefix($1, $2)")
	// .endsWith(s) -> strings.HasSuffix()
	expr = regexp.MustCompile(`(\w+)\.endsWith\(([^)]+)\)`).ReplaceAllString(expr, "strings.HasSuffix($1, $2)")
	// .indexOf(s) -> strings.Index()
	expr = regexp.MustCompile(`(\w+)\.indexOf\(([^)]+)\)`).ReplaceAllString(expr, "strings.Index($1, $2)")
	// .replace(old, new) -> strings.Replace()
	expr = regexp.MustCompile(`(\w+)\.replace\(([^,]+),\s*([^)]+)\)`).ReplaceAllString(expr, "strings.Replace($1, $2, $3, -1)")
	// .split(s) -> strings.Split()
	expr = regexp.MustCompile(`(\w+)\.split\(([^)]+)\)`).ReplaceAllString(expr, "strings.Split($1, $2)")
	
	// Translate this.field -> receiver.field (in class methods)
	if t.inClass && t.currentClass != "" {
		receiver := strings.ToLower(string(t.currentClass[0]))
		expr = regexp.MustCompile(`\bthis\.`).ReplaceAllString(expr, receiver+".")
	}
	
	// Translate types in expressions
	// i32 x -> var x int32 (handled in variable declaration)
	// For now, pass through
	return expr
}

// mapType maps Hust types to Go types
func (t *Translator) mapType(hustType string) string {
	// Handle array types: i32[] -> []int32, i32[5] -> [5]int32
	arrayMatch := regexp.MustCompile(`^(\w+)(\[\d*\])$`).FindStringSubmatch(hustType)
	if len(arrayMatch) >= 3 {
		baseType := arrayMatch[1]
		arraySuffix := arrayMatch[2]
		mappedBase := t.mapBaseType(baseType)
		// Dynamic array: i32[] -> []int32
		// Fixed array: i32[5] -> [5]int32
		if arraySuffix == "[]" {
			return "[]" + mappedBase
		}
		return arraySuffix + mappedBase
	}
	
	return t.mapBaseType(hustType)
}

// mapBaseType maps basic Hust types to Go types
func (t *Translator) mapBaseType(hustType string) string {
	typeMap := map[string]string{
		"i8":     "int8",
		"i16":    "int16",
		"i32":    "int32",
		"i64":    "int64",
		"u8":     "uint8",
		"u16":    "uint16",
		"u32":    "uint32",
		"u64":    "uint64",
		"f32":    "float32",
		"f64":    "float64",
		"bool":   "bool",
		"String": "string",
		"char":   "rune",
		"void":   "",
	}
	
	if goType, ok := typeMap[hustType]; ok {
		return goType
	}
	return hustType
}

// getIndent returns the current indentation string
func (t *Translator) getIndent() string {
	if t.indentLevel <= 0 {
		return ""
	}
	return strings.Repeat("\t", t.indentLevel)
}
