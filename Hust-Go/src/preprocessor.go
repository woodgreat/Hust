package main

import (
	"regexp"
	"strings"
)

// Preprocessor handles comment/string protection and dialect checking
type Preprocessor struct {
	stringTable  []string
	commentTable []string
}

// NewPreprocessor creates a new preprocessor
func NewPreprocessor() *Preprocessor {
	return &Preprocessor{
		stringTable:  make([]string, 0),
		commentTable: make([]string, 0),
	}
}

// Process performs preprocessing on source code
// Returns: processed code, detected dialect, error
func (p *Preprocessor) Process(source string) (string, DialectType, error) {
	// Step 1: Protect strings
	protected := p.protectStrings(source)
	
	// Step 2: Protect comments
	protected = p.protectComments(protected)
	
	// Step 3: Check dialect
	dialect := p.checkDialect(protected)
	
	// Step 4: Normalize blocks (brace matching, indent standardization)
	protected = p.normalizeBlocks(protected)
	
	// Step 5: Move function definitions to top (forward declaration)
	protected = p.moveFunctionsToTop(protected)
	
	return protected, dialect, nil
}

// DialectType represents the detected dialect
type DialectType int

const (
	PureHust DialectType = iota
	GoDialect
	RustDialect
	CDialect
	CSharpDialect
	MixedDialect
)

// protectStrings replaces string literals with placeholders
func (p *Preprocessor) protectStrings(source string) string {
	// Match string literals: "..." with escape support
	re := regexp.MustCompile(`"(?:[^"\\]|\\.)*"`)
	
	return re.ReplaceAllStringFunc(source, func(match string) string {
		placeholder := "__STRING_" + string(rune(len(p.stringTable))) + "__"
		p.stringTable = append(p.stringTable, match)
		return placeholder
	})
}

// protectComments replaces comments with placeholders
func (p *Preprocessor) protectComments(source string) string {
	lines := strings.Split(source, "\n")
	var result []string
	
	for _, line := range lines {
		// Check for line comment
		if idx := strings.Index(line, "//"); idx >= 0 {
			// Check if // is inside a string (odd number of quotes before it)
			before := line[:idx]
			quoteCount := strings.Count(before, "\"")
			if quoteCount%2 == 0 {
				// Not inside string, protect comment
				comment := line[idx:]
				placeholder := "__COMMENT_" + string(rune(len(p.commentTable))) + "__"
				p.commentTable = append(p.commentTable, comment)
				line = line[:idx] + placeholder
			}
		}
		result = append(result, line)
	}
	
	return strings.Join(result, "\n")
}

// checkDialect detects the dialect from imports
func (p *Preprocessor) checkDialect(source string) DialectType {
	hasGoImport := strings.Contains(source, "use GO.")
	hasRustImport := strings.Contains(source, "use RUST.")
	hasCImport := strings.Contains(source, "use C.")
	hasCSharpImport := strings.Contains(source, "use CSHARP.")
	
	count := 0
	if hasGoImport {
		count++
	}
	if hasRustImport {
		count++
	}
	if hasCImport {
		count++
	}
	if hasCSharpImport {
		count++
	}
	
	if count > 1 {
		return MixedDialect
	}
	if hasGoImport {
		return GoDialect
	}
	if hasRustImport {
		return RustDialect
	}
	if hasCImport {
		return CDialect
	}
	if hasCSharpImport {
		return CSharpDialect
	}
	
	return PureHust
}

// normalizeBlocks normalizes brace structure and indentation
func (p *Preprocessor) normalizeBlocks(source string) string {
	// TODO: Implement block normalization
	// For now, return as-is
	return source
}

// Restore restores protected strings and comments
func (p *Preprocessor) Restore(source string) string {
	// Restore strings
	for i, s := range p.stringTable {
		placeholder := "__STRING_" + string(rune(i)) + "__"
		source = strings.ReplaceAll(source, placeholder, s)
	}
	
	// Restore comments
	for i, c := range p.commentTable {
		placeholder := "__COMMENT_" + string(rune(i)) + "__"
		source = strings.ReplaceAll(source, placeholder, c)
	}
	
	return source
}

// moveFunctionsToTop moves function definitions to the top of the file
// This enables forward declaration (functions can be called before they are defined)
func (p *Preprocessor) moveFunctionsToTop(source string) string {
	lines := strings.Split(source, "\n")
	
	// Collect function definitions and other code separately
	var functions []string
	var otherCode []string
	var currentFunction []string
	inFunction := false
	braceCount := 0
	
	for _, line := range lines {
		trimmed := strings.TrimSpace(line)
		
		// Check if this is a function definition
		if !inFunction && isFunctionDefinition(trimmed) {
			inFunction = true
			currentFunction = []string{line}
			braceCount = strings.Count(line, "{") - strings.Count(line, "}")
			continue
		}
		
		if inFunction {
			currentFunction = append(currentFunction, line)
			braceCount += strings.Count(line, "{") - strings.Count(line, "}")
			
			// Function ends when brace count returns to 0
			if braceCount == 0 {
				inFunction = false
				functions = append(functions, strings.Join(currentFunction, "\n"))
				currentFunction = nil
			}
			continue
		}
		
		// Not in a function, add to other code
		otherCode = append(otherCode, line)
	}
	
	// If we were still in a function at the end, add it
	if inFunction && len(currentFunction) > 0 {
		functions = append(functions, strings.Join(currentFunction, "\n"))
	}
	
	// Reconstruct: functions first, then other code
	var result []string
	result = append(result, functions...)
	result = append(result, otherCode...)
	
	return strings.Join(result, "\n")
}

// isFunctionDefinition checks if a line is a function definition
func isFunctionDefinition(line string) bool {
	// Match patterns like:
	// void main() {
	// i32 add(i32 a, i32 b) {
	// String greet(String name) {
	// public void method() { (class method)
	// private i32 helper() { (class method)
	
	// Skip class/enum declarations
	if strings.HasPrefix(line, "class ") || strings.HasPrefix(line, "enum ") {
		return false
	}
	
	// Skip control flow statements
	if strings.HasPrefix(line, "if ") || strings.HasPrefix(line, "for ") ||
		strings.HasPrefix(line, "while ") || strings.HasPrefix(line, "do ") ||
		strings.HasPrefix(line, "switch ") || strings.HasPrefix(line, "} else") {
		return false
	}
	
	// Check for function definition pattern: returnType name(params) {
	// Must contain ( and ) and end with {
	if !strings.Contains(line, "(") || !strings.Contains(line, ")") {
		return false
	}
	
	// Must end with { (possibly with whitespace)
	if !strings.HasSuffix(strings.TrimSpace(line), "{") {
		return false
	}
	
	// Check for return type at the beginning
	// Common return types: void, i32, i64, f32, f64, bool, String, etc.
	returnTypePattern := regexp.MustCompile(`^(public\s+|private\s+|protected\s+)?(static\s+)?(void|i8|i16|i32|i64|u8|u16|u32|u64|f32|f64|bool|String|[A-Z][a-zA-Z0-9]*)\s+\w+\s*\(`)
	
	return returnTypePattern.MatchString(line)
}
