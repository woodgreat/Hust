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
