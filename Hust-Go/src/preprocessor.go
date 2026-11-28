package main

import (
	"fmt"
	"strings"
)

// DialectType is decided by the imported system modules, not by syntax alone.
type DialectType int

const (
	PureHust DialectType = iota
	GoDialect
	RustDialect
	CDialect
	CSharpDialect
	MixedDialect
)

// Preprocessor strips comments and protects string/char literals so the
// lexer never has to reason about escapes or comment markers.
type Preprocessor struct {
	strings []string
	chars   []string
}

func NewPreprocessor() *Preprocessor {
	return &Preprocessor{}
}

// Process returns protected source plus the detected dialect.
func (p *Preprocessor) Process(source string) (string, DialectType, error) {
	protected, err := p.protect(source)
	if err != nil {
		return "", PureHust, err
	}

	dialect := detectDialect(protected)
	return protected, dialect, nil
}

func (p *Preprocessor) protect(src string) (string, error) {
	var b strings.Builder
	b.Grow(len(src))

	for i := 0; i < len(src); {
		c := src[i]

		// Line comment
		if c == '/' && i+1 < len(src) && src[i+1] == '/' {
			for i < len(src) && src[i] != '\n' {
				i++
			}
			continue
		}

		// Block comment: keep newlines so line numbers stay stable.
		if c == '/' && i+1 < len(src) && src[i+1] == '*' {
			i += 2
			closed := false
			for i < len(src) {
				if src[i] == '*' && i+1 < len(src) && src[i+1] == '/' {
					i += 2
					closed = true
					break
				}
				if src[i] == '\n' {
					b.WriteByte('\n')
				}
				i++
			}
			if !closed {
				return "", fmt.Errorf("unterminated block comment")
			}
			continue
		}

		// String literal
		if c == '"' {
			start := i
			i++
			for i < len(src) {
				if src[i] == '\\' {
					i += 2
					continue
				}
				if src[i] == '"' {
					i++
					break
				}
				i++
			}
			if i > len(src) || src[i-1] != '"' {
				return "", fmt.Errorf("unterminated string literal")
			}
			literal := src[start:i]
			idx := len(p.strings)
			p.strings = append(p.strings, literal)
			fmt.Fprintf(&b, "__HUST_STR_%d__", idx)
			continue
		}

		// Char literal
		if c == '\'' {
			start := i
			i++
			for i < len(src) {
				if src[i] == '\\' {
					i += 2
					continue
				}
				if src[i] == '\'' {
					i++
					break
				}
				i++
			}
			if i > len(src) || src[i-1] != '\'' {
				return "", fmt.Errorf("unterminated char literal")
			}
			literal := src[start:i]
			idx := len(p.chars)
			p.chars = append(p.chars, literal)
			fmt.Fprintf(&b, "__HUST_CHR_%d__", idx)
			continue
		}

		b.WriteByte(c)
		i++
	}

	return b.String(), nil
}

// Restore is kept for the three-layer contract and for debugging output.
func (p *Preprocessor) Restore(s string) string {
	for i, literal := range p.strings {
		s = strings.ReplaceAll(s, fmt.Sprintf("__HUST_STR_%d__", i), literal)
	}
	for i, literal := range p.chars {
		s = strings.ReplaceAll(s, fmt.Sprintf("__HUST_CHR_%d__", i), literal)
	}
	return s
}

func detectDialect(src string) DialectType {
	hasGo, hasRust, hasC, hasCSharp := false, false, false, false

	for _, line := range strings.Split(src, "\n") {
		trimmed := strings.TrimSpace(line)
		if !strings.HasPrefix(trimmed, "use ") {
			continue
		}
		path := strings.TrimSpace(strings.TrimSuffix(strings.TrimPrefix(trimmed, "use "), ";"))
		root := strings.SplitN(path, ".", 2)[0]
		switch root {
		case "GO":
			hasGo = true
		case "RUST":
			hasRust = true
		case "C":
			hasC = true
		case "CSHARP":
			hasCSharp = true
		}
	}

	count := 0
	for _, v := range []bool{hasGo, hasRust, hasC, hasCSharp} {
		if v {
			count++
		}
	}
	if count > 1 {
		return MixedDialect
	}
	switch {
	case hasGo:
		return GoDialect
	case hasRust:
		return RustDialect
	case hasC:
		return CDialect
	case hasCSharp:
		return CSharpDialect
	default:
		return PureHust
	}
}
