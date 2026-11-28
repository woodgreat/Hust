package main

import (
	"fmt"
	"strconv"
	"strings"
)

type Parser struct {
	tokens []Token
	pos    int
}

func NewParser(tokens []Token) *Parser {
	return &Parser{tokens: tokens}
}

func (p *Parser) cur() Token  { return p.tokens[p.pos] }
func (p *Parser) peek() Token { return p.peekN(1) }

func (p *Parser) peekN(n int) Token {
	i := p.pos + n
	if i >= len(p.tokens) {
		return p.tokens[len(p.tokens)-1]
	}
	return p.tokens[i]
}

func (p *Parser) advance() Token {
	t := p.tokens[p.pos]
	if p.pos < len(p.tokens)-1 {
		p.pos++
	}
	return t
}

func (p *Parser) at(text string) bool { return p.cur().is(text) }

func (p *Parser) accept(text string) bool {
	if p.at(text) {
		p.advance()
		return true
	}
	return false
}

func (p *Parser) expect(text string) error {
	if !p.at(text) {
		return fmt.Errorf("line %d: expected %q, got %q", p.cur().Line, text, p.cur().Text)
	}
	p.advance()
	return nil
}

func (p *Parser) errorf(format string, args ...interface{}) error {
	return fmt.Errorf("line %d: %s", p.cur().Line, fmt.Sprintf(format, args...))
}

func (p *Parser) ParseProgram() (*Program, error) {
	prog := &Program{}

	for p.cur().Kind != TEOF {
		if p.accept(";") {
			continue
		}
		switch {
		case p.at("use"):
			use, err := p.parseUse()
			if err != nil {
				return nil, err
			}
			prog.Uses = append(prog.Uses, use)
		case p.at("class"):
			class, err := p.parseClass()
			if err != nil {
				return nil, err
			}
			prog.Classes = append(prog.Classes, class)
		case p.at("interface"):
			iface, err := p.parseInterface()
			if err != nil {
				return nil, err
			}
			prog.Interfaces = append(prog.Interfaces, iface)
		case p.at("enum"):
			enum, err := p.parseEnum()
			if err != nil {
				return nil, err
			}
			prog.Enums = append(prog.Enums, enum)
		default:
			if fn, ok, err := p.tryFuncDecl(""); err != nil {
				return nil, err
			} else if ok {
				prog.Funcs = append(prog.Funcs, fn)
				continue
			}
			if vd, ok, err := p.tryVarDecl(); err != nil {
				return nil, err
			} else if ok {
				prog.Globals = append(prog.Globals, vd)
				continue
			}
			return nil, p.errorf("unexpected top-level token %q", p.cur().Text)
		}
	}

	return prog, nil
}

func (p *Parser) parseUse() (*UseDecl, error) {
	line := p.cur().Line
	p.advance() // use
	var parts strings.Builder
	for p.cur().Kind != TEOF && !p.at(";") {
		parts.WriteString(p.advance().Text)
	}
	if err := p.expect(";"); err != nil {
		return nil, err
	}
	return &UseDecl{Path: parts.String(), Line: line}, nil
}

func (p *Parser) parseClass() (*ClassDecl, error) {
	line := p.cur().Line
	p.advance() // class

	if p.cur().Kind != TIdent {
		return nil, p.errorf("expected class name")
	}
	class := &ClassDecl{Name: p.advance().Text, Line: line}

	if p.accept("extends") {
		if p.cur().Kind != TIdent {
			return nil, p.errorf("expected base class name")
		}
		class.Extends = p.advance().Text
	}
	if p.accept("implements") {
		for {
			if p.cur().Kind != TIdent {
				return nil, p.errorf("expected interface name")
			}
			class.Implements = append(class.Implements, p.advance().Text)
			if !p.accept(",") {
				break
			}
		}
	}

	if err := p.expect("{"); err != nil {
		return nil, err
	}
	for !p.at("}") && p.cur().Kind != TEOF {
		if p.accept(";") {
			continue
		}

		public := false
		for p.at("public") || p.at("private") || p.at("protected") || p.at("static") {
			if p.at("public") {
				public = true
			}
			p.advance()
		}

		// Constructor: ClassName(...)
		if p.cur().Kind == TIdent && p.cur().Text == class.Name && p.peek().is("(") {
			name := p.advance().Text
			fn, err := p.parseFuncRest(name, nil, public, class.Name)
			if err != nil {
				return nil, err
			}
			class.Ctors = append(class.Ctors, fn)
			continue
		}

		// Method without an explicit return type: name(...)
		if p.cur().Kind == TIdent && p.peek().is("(") {
			name := p.advance().Text
			fn, err := p.parseFuncRest(name, nil, public, class.Name)
			if err != nil {
				return nil, err
			}
			class.Methods = append(class.Methods, fn)
			continue
		}

		ret, err := p.parseType()
		if err != nil {
			return nil, err
		}
		if p.cur().Kind != TIdent {
			return nil, p.errorf("expected member name in class %s", class.Name)
		}
		name := p.advance().Text

		if p.at("(") {
			fn, err := p.parseFuncRest(name, ret, public, class.Name)
			if err != nil {
				return nil, err
			}
			class.Methods = append(class.Methods, fn)
			continue
		}

		field := &VarDecl{Name: name, Type: ret, Line: p.cur().Line}
		if p.accept("=") {
			init, err := p.parseExpr()
			if err != nil {
				return nil, err
			}
			field.Init = init
		}
		if err := p.expect(";"); err != nil {
			return nil, err
		}
		class.Fields = append(class.Fields, field)
	}
	if err := p.expect("}"); err != nil {
		return nil, err
	}

	return class, nil
}

func (p *Parser) parseInterface() (*InterfaceDecl, error) {
	line := p.cur().Line
	p.advance() // interface

	if p.cur().Kind != TIdent {
		return nil, p.errorf("expected interface name")
	}
	iface := &InterfaceDecl{Name: p.advance().Text, Line: line}

	if err := p.expect("{"); err != nil {
		return nil, err
	}
	for !p.at("}") && p.cur().Kind != TEOF {
		if p.accept(";") {
			continue
		}
		for p.at("public") || p.at("private") || p.at("protected") || p.at("static") {
			p.advance()
		}

		sig := &FuncSignature{Line: p.cur().Line}
		if p.cur().Kind == TIdent && p.peek().is("(") {
			sig.Name = p.advance().Text
		} else {
			ret, err := p.parseType()
			if err != nil {
				return nil, err
			}
			sig.Ret = ret
			if p.cur().Kind != TIdent {
				return nil, p.errorf("expected interface method name")
			}
			sig.Name = p.advance().Text
		}

		params, err := p.parseParams()
		if err != nil {
			return nil, err
		}
		sig.Params = params
		if err := p.expect(";"); err != nil {
			return nil, err
		}
		iface.Methods = append(iface.Methods, sig)
	}
	if err := p.expect("}"); err != nil {
		return nil, err
	}
	return iface, nil
}

func (p *Parser) parseEnum() (*EnumDecl, error) {
	line := p.cur().Line
	p.advance() // enum
	if p.cur().Kind != TIdent {
		return nil, p.errorf("expected enum name")
	}
	enum := &EnumDecl{Name: p.advance().Text, Line: line}
	if err := p.expect("{"); err != nil {
		return nil, err
	}
	for !p.at("}") && p.cur().Kind != TEOF {
		if p.cur().Kind != TIdent {
			return nil, p.errorf("expected enum value")
		}
		enum.Values = append(enum.Values, p.advance().Text)
		if !p.accept(",") {
			break
		}
	}
	if err := p.expect("}"); err != nil {
		return nil, err
	}
	return enum, nil
}

func (p *Parser) parseType() (*Type, error) {
	if p.cur().Kind != TIdent {
		return nil, p.errorf("expected type, got %q", p.cur().Text)
	}
	t := &Type{Name: p.advance().Text}
	for p.at("[") {
		p.advance()
		if p.accept("]") {
			t.Dims = append(t.Dims, -1)
			continue
		}
		if p.cur().Kind != TInt {
			return nil, p.errorf("expected array size")
		}
		n, err := strconv.Atoi(p.cur().Text)
		if err != nil {
			return nil, p.errorf("bad array size %q", p.cur().Text)
		}
		p.advance()
		if err := p.expect("]"); err != nil {
			return nil, err
		}
		t.Dims = append(t.Dims, n)
	}
	return t, nil
}

func (p *Parser) parseParams() ([]*Param, error) {
	var params []*Param
	if err := p.expect("("); err != nil {
		return nil, err
	}
	if p.accept(")") {
		return params, nil
	}

	for {
		line := p.cur().Line

		// Try "Type name" first, then fall back to a bare name.
		save := p.pos
		typ, err := p.parseType()
		if err == nil && p.cur().Kind == TIdent {
			params = append(params, &Param{Name: p.advance().Text, Type: typ, Line: line})
		} else {
			p.pos = save
			if p.cur().Kind != TIdent {
				return nil, p.errorf("expected parameter")
			}
			params = append(params, &Param{Name: p.advance().Text, Line: line})
		}

		if !p.accept(",") {
			break
		}
	}

	if err := p.expect(")"); err != nil {
		return nil, err
	}
	return params, nil
}

// tryFuncDecl parses a top-level or class-level function. className is empty
// for top-level functions.
func (p *Parser) tryFuncDecl(className string) (*FuncDecl, bool, error) {
	save := p.pos

	public := false
	for p.at("public") || p.at("private") || p.at("protected") || p.at("static") {
		if p.at("public") {
			public = true
		}
		p.advance()
	}

	// No explicit return type: name(...)
	if p.cur().Kind == TIdent && p.peek().is("(") && p.cur().Text != className {
		name := p.advance().Text
		fn, err := p.parseFuncRest(name, nil, public, className)
		if err != nil {
			return nil, false, err
		}
		return fn, true, nil
	}

	ret, err := p.parseType()
	if err != nil {
		p.pos = save
		return nil, false, nil
	}
	if p.cur().Kind != TIdent || !p.peek().is("(") {
		p.pos = save
		return nil, false, nil
	}
	name := p.advance().Text
	fn, err := p.parseFuncRest(name, ret, public, className)
	if err != nil {
		return nil, false, err
	}
	return fn, true, nil
}

// parseFuncRest assumes the name is consumed and the current token is "(".
func (p *Parser) parseFuncRest(name string, ret *Type, public bool, className string) (*FuncDecl, error) {
	line := p.cur().Line
	params, err := p.parseParams()
	if err != nil {
		return nil, err
	}
	if !p.at("{") {
		return nil, p.errorf("expected function body for %s", name)
	}
	body, err := p.parseBlock()
	if err != nil {
		return nil, err
	}
	return &FuncDecl{Name: name, Ret: ret, Params: params, Body: body, Public: public, Line: line}, nil
}

func (p *Parser) tryVarDecl() (*VarDecl, bool, error) {
	save := p.pos
	isConst := p.accept("const")

	typ, err := p.parseType()
	if err != nil {
		p.pos = save
		return nil, false, nil
	}
	if p.cur().Kind != TIdent {
		p.pos = save
		return nil, false, nil
	}
	name := p.advance().Text

	if !p.at("=") && !p.at(";") && !p.at(",") {
		p.pos = save
		return nil, false, nil
	}

	vd := &VarDecl{Name: name, Type: typ, Const: isConst, Line: p.cur().Line}
	if p.accept("=") {
		init, err := p.parseExpr()
		if err != nil {
			return nil, false, err
		}
		vd.Init = init
	}

	// Additional declarators share the type: `i32 a = 1, b = 2;`
	for p.accept(",") {
		if p.cur().Kind != TIdent {
			return nil, false, p.errorf("expected declarator name")
		}
		extra := &VarDecl{Name: p.advance().Text, Type: typ.clone(), Const: isConst, Line: p.cur().Line}
		if p.accept("=") {
			init, err := p.parseExpr()
			if err != nil {
				return nil, false, err
			}
			extra.Init = init
		}
		vd.More = append(vd.More, extra)
	}

	if err := p.expect(";"); err != nil {
		return nil, false, err
	}
	return vd, true, nil
}

func (p *Parser) parseBlock() (*Block, error) {
	line := p.cur().Line
	if err := p.expect("{"); err != nil {
		return nil, err
	}
	block := &Block{Line: line}
	for !p.at("}") && p.cur().Kind != TEOF {
		stmt, err := p.parseStmt()
		if err != nil {
			return nil, err
		}
		if stmt != nil {
			block.Stmts = append(block.Stmts, stmt)
		}
	}
	if err := p.expect("}"); err != nil {
		return nil, err
	}
	return block, nil
}

func (p *Parser) parseStmt() (Stmt, error) {
	if p.accept(";") {
		return nil, nil
	}
	if p.at("{") {
		return p.parseBlock()
	}

	switch {
	case p.at("if"):
		return p.parseIf()
	case p.at("while"):
		return p.parseWhile()
	case p.at("for"):
		return p.parseFor()
	case p.at("do"):
		return p.parseDoWhile()
	case p.at("switch"):
		return p.parseSwitch()
	case p.at("return"):
		line := p.advance().Line
		var value Expr
		if !p.at(";") && !p.at("}") {
			expr, err := p.parseExpr()
			if err != nil {
				return nil, err
			}
			value = expr
		}
		p.accept(";")
		return &ReturnStmt{Value: value, Line: line}, nil
	case p.at("break"):
		line := p.advance().Line
		p.accept(";")
		return &BreakStmt{Line: line}, nil
	case p.at("continue"):
		line := p.advance().Line
		p.accept(";")
		return &ContinueStmt{Line: line}, nil
	}

	if vd, ok, err := p.tryVarDecl(); err != nil {
		return nil, err
	} else if ok {
		return vd, nil
	}

	line := p.cur().Line
	expr, err := p.parseExpr()
	if err != nil {
		return nil, err
	}
	p.accept(";")
	return &ExprStmt{X: expr, Line: line}, nil
}

func (p *Parser) parseBody() (*Block, error) {
	if p.at("{") {
		return p.parseBlock()
	}
	stmt, err := p.parseStmt()
	if err != nil {
		return nil, err
	}
	return &Block{Stmts: []Stmt{stmt}, Line: exprLineOfStmt(stmt)}, nil
}

func (p *Parser) parseIf() (Stmt, error) {
	line := p.advance().Line // if
	if err := p.expect("("); err != nil {
		return nil, err
	}
	cond, err := p.parseExpr()
	if err != nil {
		return nil, err
	}
	if err := p.expect(")"); err != nil {
		return nil, err
	}
	then, err := p.parseBody()
	if err != nil {
		return nil, err
	}

	var elseStmt Stmt
	if p.accept("else") {
		if p.at("if") {
			elseStmt, err = p.parseIf()
			if err != nil {
				return nil, err
			}
		} else {
			elseStmt, err = p.parseBody()
			if err != nil {
				return nil, err
			}
		}
	}

	return &IfStmt{Cond: cond, Then: then, Else: elseStmt, Line: line}, nil
}

func (p *Parser) parseWhile() (Stmt, error) {
	line := p.advance().Line
	if err := p.expect("("); err != nil {
		return nil, err
	}
	cond, err := p.parseExpr()
	if err != nil {
		return nil, err
	}
	if err := p.expect(")"); err != nil {
		return nil, err
	}
	body, err := p.parseBody()
	if err != nil {
		return nil, err
	}
	return &WhileStmt{Cond: cond, Body: body, Line: line}, nil
}

func (p *Parser) parseFor() (Stmt, error) {
	line := p.advance().Line
	if err := p.expect("("); err != nil {
		return nil, err
	}

	var init Stmt
	if !p.at(";") {
		if vd, ok, err := p.tryVarDeclWithoutSemi(); err != nil {
			return nil, err
		} else if ok {
			init = vd
		} else {
			expr, err := p.parseExpr()
			if err != nil {
				return nil, err
			}
			init = &ExprStmt{X: expr, Line: exprLine(expr)}
		}
	}
	if err := p.expect(";"); err != nil {
		return nil, err
	}

	var cond Expr
	if !p.at(";") {
		expr, err := p.parseExpr()
		if err != nil {
			return nil, err
		}
		cond = expr
	}
	if err := p.expect(";"); err != nil {
		return nil, err
	}

	var post Stmt
	if !p.at(")") {
		expr, err := p.parseExpr()
		if err != nil {
			return nil, err
		}
		post = &ExprStmt{X: expr, Line: exprLine(expr)}
	}
	if err := p.expect(")"); err != nil {
		return nil, err
	}

	body, err := p.parseBody()
	if err != nil {
		return nil, err
	}
	return &ForStmt{Init: init, Cond: cond, Post: post, Body: body, Line: line}, nil
}

// tryVarDeclWithoutSemi is used inside a for-init where ";" is consumed by the
// caller.
func (p *Parser) tryVarDeclWithoutSemi() (*VarDecl, bool, error) {
	save := p.pos
	isConst := p.accept("const")

	typ, err := p.parseType()
	if err != nil {
		p.pos = save
		return nil, false, nil
	}
	if p.cur().Kind != TIdent {
		p.pos = save
		return nil, false, nil
	}
	name := p.advance().Text
	if !p.at("=") {
		p.pos = save
		return nil, false, nil
	}
	p.advance()
	init, err := p.parseExpr()
	if err != nil {
		return nil, false, err
	}
	return &VarDecl{Name: name, Type: typ, Init: init, Const: isConst, Line: p.cur().Line}, true, nil
}

func (p *Parser) parseDoWhile() (Stmt, error) {
	line := p.advance().Line
	body, err := p.parseBlock()
	if err != nil {
		return nil, err
	}
	if !p.accept("while") {
		return nil, p.errorf("expected 'while' after do block")
	}
	if err := p.expect("("); err != nil {
		return nil, err
	}
	cond, err := p.parseExpr()
	if err != nil {
		return nil, err
	}
	if err := p.expect(")"); err != nil {
		return nil, err
	}
	p.accept(";")
	return &DoWhileStmt{Body: body, Cond: cond, Line: line}, nil
}

func (p *Parser) parseSwitch() (Stmt, error) {
	line := p.advance().Line
	if err := p.expect("("); err != nil {
		return nil, err
	}
	value, err := p.parseExpr()
	if err != nil {
		return nil, err
	}
	if err := p.expect(")"); err != nil {
		return nil, err
	}
	if err := p.expect("{"); err != nil {
		return nil, err
	}

	stmt := &SwitchStmt{Value: value, Line: line}
	for !p.at("}") && p.cur().Kind != TEOF {
		clause := &CaseClause{Line: p.cur().Line}
		if p.accept("case") {
			v, err := p.parseExpr()
			if err != nil {
				return nil, err
			}
			clause.Value = v
		} else if p.accept("default") {
			clause.Default = true
		} else {
			return nil, p.errorf("expected case or default")
		}
		if err := p.expect(":"); err != nil {
			return nil, err
		}

		for !p.at("case") && !p.at("default") && !p.at("}") && p.cur().Kind != TEOF {
			bodyStmt, err := p.parseStmt()
			if err != nil {
				return nil, err
			}
			if bodyStmt != nil {
				clause.Body = append(clause.Body, bodyStmt)
			}
		}
		stmt.Cases = append(stmt.Cases, clause)
	}
	if err := p.expect("}"); err != nil {
		return nil, err
	}
	return stmt, nil
}

func (p *Parser) parseExpr() (Expr, error) {
	return p.parseAssign()
}

func (p *Parser) parseAssign() (Expr, error) {
	left, err := p.parseTernary()
	if err != nil {
		return nil, err
	}

	switch p.cur().Text {
	case "=", "+=", "-=", "*=", "/=", "%=", "<<=", ">>=", "&=", "|=", "^=":
		op := p.advance().Text
		right, err := p.parseAssign()
		if err != nil {
			return nil, err
		}
		return &AssignExpr{Op: op, Target: left, Value: right, Line: exprLine(left)}, nil
	}
	return left, nil
}

func (p *Parser) parseTernary() (Expr, error) {
	cond, err := p.parseBinary(0)
	if err != nil {
		return nil, err
	}
	if p.accept("?") {
		thenExpr, err := p.parseExpr()
		if err != nil {
			return nil, err
		}
		if err := p.expect(":"); err != nil {
			return nil, err
		}
		elseExpr, err := p.parseExpr()
		if err != nil {
			return nil, err
		}
		return &TernaryExpr{Cond: cond, Then: thenExpr, Else: elseExpr, Line: exprLine(cond)}, nil
	}
	return cond, nil
}

var binaryLevels = [][]string{
	{"||"},
	{"&&"},
	{"|"},
	{"^"},
	{"&"},
	{"==", "!="},
	{"<", "<=", ">", ">="},
	{"<<", ">>"},
	{"+", "-"},
	{"*", "/", "%"},
}

func (p *Parser) parseBinary(level int) (Expr, error) {
	if level >= len(binaryLevels) {
		return p.parseUnary()
	}

	left, err := p.parseBinary(level + 1)
	if err != nil {
		return nil, err
	}
	for {
		op := ""
		for _, candidate := range binaryLevels[level] {
			if p.at(candidate) {
				op = candidate
				break
			}
		}
		if op == "" {
			return left, nil
		}
		p.advance()
		right, err := p.parseBinary(level + 1)
		if err != nil {
			return nil, err
		}
		left = &BinaryExpr{Op: op, L: left, R: right, Line: exprLine(left)}
	}
}

func (p *Parser) parseUnary() (Expr, error) {
	if p.at("!") || p.at("~") || p.at("-") || p.at("+") || p.at("++") || p.at("--") {
		op := p.advance().Text
		operand, err := p.parseUnary()
		if err != nil {
			return nil, err
		}
		return &UnaryExpr{Op: op, X: operand, Line: exprLine(operand)}, nil
	}
	return p.parsePostfix()
}

func (p *Parser) parsePostfix() (Expr, error) {
	expr, err := p.parsePrimary()
	if err != nil {
		return nil, err
	}

	for {
		switch {
		case p.at("("):
			line := p.cur().Line
			args, err := p.parseArgs()
			if err != nil {
				return nil, err
			}
			expr = &CallExpr{Callee: expr, Args: args, Line: line}
		case p.at("["):
			line := p.cur().Line
			p.advance()
			var low Expr
			if !p.at(":") {
				parsed, err := p.parseExpr()
				if err != nil {
					return nil, err
				}
				low = parsed
			}
			if p.accept(":") {
				var high Expr
				if !p.at("]") {
					parsed, err := p.parseExpr()
					if err != nil {
						return nil, err
					}
					high = parsed
				}
				if err := p.expect("]"); err != nil {
					return nil, err
				}
				expr = &SliceExpr{X: expr, Low: low, High: high, Line: line}
			} else {
				if err := p.expect("]"); err != nil {
					return nil, err
				}
				expr = &IndexExpr{X: expr, Index: low, Line: line}
			}
		case p.at("."):
			line := p.cur().Line
			p.advance()
			if p.cur().Kind != TIdent {
				return nil, p.errorf("expected member name after '.'")
			}
			name := p.advance().Text
			member := &MemberExpr{X: expr, Name: name, Line: line}
			if p.at("(") {
				args, err := p.parseArgs()
				if err != nil {
					return nil, err
				}
				expr = &CallExpr{Callee: member, Args: args, Line: line}
			} else {
				expr = member
			}
		case p.at("++") || p.at("--"):
			op := p.advance().Text
			expr = &PostfixExpr{Op: op, X: expr, Line: exprLine(expr)}
		default:
			return expr, nil
		}
	}
}

func (p *Parser) parseArgs() ([]Expr, error) {
	if err := p.expect("("); err != nil {
		return nil, err
	}
	var args []Expr
	if p.accept(")") {
		return args, nil
	}
	for {
		arg, err := p.parseExpr()
		if err != nil {
			return nil, err
		}
		args = append(args, arg)
		if !p.accept(",") {
			break
		}
	}
	if err := p.expect(")"); err != nil {
		return nil, err
	}
	return args, nil
}

func (p *Parser) parsePrimary() (Expr, error) {
	tok := p.cur()

	switch tok.Kind {
	case TInt:
		p.advance()
		return &Literal{Kind: "int", Text: tok.Text, Line: tok.Line}, nil
	case TFloat:
		p.advance()
		return &Literal{Kind: "float", Text: tok.Text, Line: tok.Line}, nil
	case TString:
		p.advance()
		return &Literal{Kind: "string", Text: tok.Text, Line: tok.Line}, nil
	case TChar:
		p.advance()
		return &Literal{Kind: "char", Text: tok.Text, Line: tok.Line}, nil
	}

	if tok.is("(") {
		p.advance()
		expr, err := p.parseExpr()
		if err != nil {
			return nil, err
		}
		if err := p.expect(")"); err != nil {
			return nil, err
		}
		return &ParenExpr{X: expr, Line: tok.Line}, nil
	}

	if tok.is("{") {
		line := p.advance().Line
		lit := &ArrayLit{Line: line}
		if p.accept("}") {
			return lit, nil
		}
		for {
			elem, err := p.parseExpr()
			if err != nil {
				return nil, err
			}
			lit.Elems = append(lit.Elems, elem)
			if !p.accept(",") {
				break
			}
		}
		if err := p.expect("}"); err != nil {
			return nil, err
		}
		return lit, nil
	}

	if tok.Kind == TIdent {
		switch tok.Text {
		case "true", "false":
			p.advance()
			return &Literal{Kind: "bool", Text: tok.Text, Line: tok.Line}, nil
		case "null":
			p.advance()
			return &Literal{Kind: "null", Text: "null", Line: tok.Line}, nil
		case "this":
			p.advance()
			return &ThisExpr{Line: tok.Line}, nil
		case "new":
			p.advance()
			typ, err := p.parseType()
			if err != nil {
				return nil, err
			}
			var args []Expr
			if p.at("(") {
				args, err = p.parseArgs()
				if err != nil {
					return nil, err
				}
			}
			return &NewExpr{Type: typ, Args: args, Line: tok.Line}, nil
		default:
			p.advance()
			return &Ident{Name: tok.Text, Line: tok.Line}, nil
		}
	}

	return nil, p.errorf("unexpected token %q in expression", tok.Text)
}

func exprLineOfStmt(s Stmt) int {
	switch v := s.(type) {
	case *VarDecl:
		return v.Line
	case *ExprStmt:
		return v.Line
	case *IfStmt:
		return v.Line
	case *WhileStmt:
		return v.Line
	case *ForStmt:
		return v.Line
	case *DoWhileStmt:
		return v.Line
	case *ReturnStmt:
		return v.Line
	case *BreakStmt:
		return v.Line
	case *ContinueStmt:
		return v.Line
	case *SwitchStmt:
		return v.Line
	case *Block:
		return v.Line
	}
	return 0
}

func exprLine(e Expr) int {
	switch v := e.(type) {
	case *Ident:
		return v.Line
	case *Literal:
		return v.Line
	case *ThisExpr:
		return v.Line
	case *BinaryExpr:
		return v.Line
	case *UnaryExpr:
		return v.Line
	case *PostfixExpr:
		return v.Line
	case *AssignExpr:
		return v.Line
	case *CallExpr:
		return v.Line
	case *IndexExpr:
		return v.Line
	case *MemberExpr:
		return v.Line
	case *NewExpr:
		return v.Line
	case *ArrayLit:
		return v.Line
	case *TernaryExpr:
		return v.Line
	case *ParenExpr:
		return v.Line
	default:
		return 0
	}
}
