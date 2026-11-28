package main

import (
	"fmt"
	"strconv"
	"strings"
)

type TokKind int

const (
	TEOF TokKind = iota
	TIdent
	TInt
	TFloat
	TString
	TChar
	TOp
)

type Token struct {
	Kind TokKind
	Text string
	Line int
}

func (t Token) is(op string) bool {
	return (t.Kind == TOp || t.Kind == TIdent) && t.Text == op
}

type Lexer struct {
	src     string
	pos     int
	line    int
	strings []string
	chars   []string
}

func NewLexer(src string, p *Preprocessor) *Lexer {
	return &Lexer{
		src:     src,
		line:    1,
		strings: p.strings,
		chars:   p.chars,
	}
}

func (l *Lexer) Tokens() ([]Token, error) {
	var out []Token
	for {
		tok, err := l.nextToken()
		if err != nil {
			return nil, err
		}
		out = append(out, tok)
		if tok.Kind == TEOF {
			return out, nil
		}
	}
}

func (l *Lexer) nextToken() (Token, error) {
	for l.pos < len(l.src) {
		c := l.src[l.pos]
		if c == '\n' {
			l.line++
			l.pos++
			continue
		}
		if c == ' ' || c == '\t' || c == '\r' {
			l.pos++
			continue
		}
		break
	}

	if l.pos >= len(l.src) {
		return Token{Kind: TEOF, Line: l.line}, nil
	}

	start := l.pos
	line := l.line
	c := l.src[l.pos]

	// Identifier or placeholder
	if isIdentStart(c) {
		l.pos++
		for l.pos < len(l.src) && isIdentPart(l.src[l.pos]) {
			l.pos++
		}
		text := l.src[start:l.pos]

		if strings.HasPrefix(text, "__HUST_STR_") && strings.HasSuffix(text, "__") {
			idx := strings.TrimSuffix(strings.TrimPrefix(text, "__HUST_STR_"), "__")
			n, err := strconv.Atoi(idx)
			if err != nil || n < 0 || n >= len(l.strings) {
				return Token{}, fmt.Errorf("line %d: bad string placeholder %q", line, text)
			}
			return Token{Kind: TString, Text: l.strings[n], Line: line}, nil
		}
		if strings.HasPrefix(text, "__HUST_CHR_") && strings.HasSuffix(text, "__") {
			idx := strings.TrimSuffix(strings.TrimPrefix(text, "__HUST_CHR_"), "__")
			n, err := strconv.Atoi(idx)
			if err != nil || n < 0 || n >= len(l.chars) {
				return Token{}, fmt.Errorf("line %d: bad char placeholder %q", line, text)
			}
			return Token{Kind: TChar, Text: l.chars[n], Line: line}, nil
		}
		return Token{Kind: TIdent, Text: text, Line: line}, nil
	}

	// Number
	if isDigit(c) {
		l.pos++
		if c == '0' && l.pos < len(l.src) && (l.src[l.pos] == 'x' || l.src[l.pos] == 'X') {
			l.pos++
			for l.pos < len(l.src) && isHexDigit(l.src[l.pos]) {
				l.pos++
			}
			return Token{Kind: TInt, Text: l.src[start:l.pos], Line: line}, nil
		}
		if c == '0' && l.pos < len(l.src) && (l.src[l.pos] == 'b' || l.src[l.pos] == 'B') {
			l.pos++
			for l.pos < len(l.src) && (l.src[l.pos] == '0' || l.src[l.pos] == '1') {
				l.pos++
			}
			return Token{Kind: TInt, Text: l.src[start:l.pos], Line: line}, nil
		}

		isFloat := false
		for l.pos < len(l.src) && isDigit(l.src[l.pos]) {
			l.pos++
		}
		if l.pos < len(l.src) && l.src[l.pos] == '.' && l.pos+1 < len(l.src) && isDigit(l.src[l.pos+1]) {
			isFloat = true
			l.pos++
			for l.pos < len(l.src) && isDigit(l.src[l.pos]) {
				l.pos++
			}
		}
		if l.pos < len(l.src) && (l.src[l.pos] == 'e' || l.src[l.pos] == 'E') {
			isFloat = true
			l.pos++
			if l.pos < len(l.src) && (l.src[l.pos] == '+' || l.src[l.pos] == '-') {
				l.pos++
			}
			for l.pos < len(l.src) && isDigit(l.src[l.pos]) {
				l.pos++
			}
		}
		kind := TInt
		if isFloat {
			kind = TFloat
		}
		return Token{Kind: kind, Text: l.src[start:l.pos], Line: line}, nil
	}

	// Raw string/char (normally already protected by the preprocessor)
	if c == '"' {
		l.pos++
		for l.pos < len(l.src) {
			if l.src[l.pos] == '\\' {
				l.pos += 2
				continue
			}
			if l.src[l.pos] == '"' {
				l.pos++
				break
			}
			l.pos++
		}
		return Token{Kind: TString, Text: l.src[start:l.pos], Line: line}, nil
	}
	if c == '\'' {
		l.pos++
		for l.pos < len(l.src) {
			if l.src[l.pos] == '\\' {
				l.pos += 2
				continue
			}
			if l.src[l.pos] == '\'' {
				l.pos++
				break
			}
			l.pos++
		}
		return Token{Kind: TChar, Text: l.src[start:l.pos], Line: line}, nil
	}

	// Multi-character operators, longest first.
	for _, op := range []string{"<<=", ">>=", "==", "!=", "<=", ">=", "&&", "||", "++", "--", "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=", "<<", ">>"} {
		if strings.HasPrefix(l.src[l.pos:], op) {
			l.pos += len(op)
			return Token{Kind: TOp, Text: op, Line: line}, nil
		}
	}

	single := string(c)
	l.pos++
	return Token{Kind: TOp, Text: single, Line: line}, nil
}

func isIdentStart(c byte) bool {
	return c == '_' || (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z')
}

func isIdentPart(c byte) bool {
	return isIdentStart(c) || isDigit(c)
}

func isDigit(c byte) bool { return c >= '0' && c <= '9' }

func isHexDigit(c byte) bool {
	return isDigit(c) || (c >= 'a' && c <= 'f') || (c >= 'A' && c <= 'F')
}
