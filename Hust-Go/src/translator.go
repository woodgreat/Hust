package main

import (
	"fmt"
	"sort"
	"strings"
)

// TranslationResult is the core's public output.
type TranslationResult struct {
	Success bool
	Code    string
	Errors  []string
}

type Symbol struct {
	Type *Type
	Zero bool
}

type Scope struct {
	vars   map[string]*Symbol
	parent *Scope
}

func newScope(parent *Scope) *Scope {
	return &Scope{vars: map[string]*Symbol{}, parent: parent}
}

func (s *Scope) lookup(name string) *Symbol {
	for cur := s; cur != nil; cur = cur.parent {
		if sym, ok := cur.vars[name]; ok {
			return sym
		}
	}
	return nil
}

func (s *Scope) declare(name string, sym *Symbol) {
	s.vars[name] = sym
}

type ClassInfo struct {
	Decl        *ClassDecl
	Fields      map[string]*VarDecl
	FieldOrder  []string
	Methods     map[string]*FuncDecl
	MethodOrder []string
	Ctor        *FuncDecl
	Subclasses  []string
	Instantiated bool
	IsInterface bool
	Flattened    bool
}

type Translator struct {
	program    *Program
	classes    map[string]*ClassInfo
	funcs      map[string]*FuncDecl
	enums      map[string]*EnumDecl
	interfaces map[string]*InterfaceDecl

	scope        *Scope
	currentClass *ClassInfo
	currentFile  string

	returnTypes map[*FuncDecl]*Type
	methodOwner map[*FuncDecl]*ClassInfo

	body    strings.Builder
	indent  int
	reports map[string]bool
	errors  []string
	suppress bool

	needsFmt        bool
	needsStrings    bool
	needsMath       bool
	needsPop        bool
	needsInsert     bool
	needsRemoveAt   bool
	needsTernary    bool
	needsCapitalize bool

	currentReturn *Type
}

func NewTranslator() *Translator {
	return &Translator{
		classes:     map[string]*ClassInfo{},
		funcs:       map[string]*FuncDecl{},
		enums:       map[string]*EnumDecl{},
		interfaces:  map[string]*InterfaceDecl{},
		returnTypes: map[*FuncDecl]*Type{},
		methodOwner: map[*FuncDecl]*ClassInfo{},
		reports:     map[string]bool{},
		scope:       newScope(nil),
	}
}

func (t *Translator) Translate(source, fileName string) TranslationResult {
	result := TranslationResult{}

	pre := NewPreprocessor()
	protected, dialect, err := pre.Process(source)
	if err != nil {
		result.Errors = append(result.Errors, err.Error())
		return result
	}

	if dialect != PureHust && dialect != GoDialect {
		result.Errors = append(result.Errors, "Unsupported dialect for Go core")
		return result
	}
	if strings.Contains(protected, "println!") {
		result.Errors = append(result.Errors, "println! is Rust-dialect syntax and cannot run on the Go core")
		return result
	}

	tokens, err := NewLexer(protected, pre).Tokens()
	if err != nil {
		result.Errors = append(result.Errors, err.Error())
		return result
	}
	program, err := NewParser(tokens).ParseProgram()
	if err != nil {
		result.Errors = append(result.Errors, err.Error())
		return result
	}

	t.program = program
	t.currentFile = fileName
	t.collect(program)
	t.inferReturnTypes()

	body := t.emitProgram()
	if len(t.errors) > 0 {
		result.Errors = append(result.Errors, t.errors...)
		return result
	}

	result.Success = true
	result.Code = t.assemble(body)
	return result
}

// ---------------------------------------------------------------- collection

func (t *Translator) collect(program *Program) {
	for _, enum := range program.Enums {
		t.enums[enum.Name] = enum
	}
	for _, iface := range program.Interfaces {
		t.interfaces[iface.Name] = iface
	}
	for _, fn := range program.Funcs {
		t.funcs[fn.Name] = fn
	}

	for _, class := range program.Classes {
		info := &ClassInfo{
			Decl:    class,
			Fields:  map[string]*VarDecl{},
			Methods: map[string]*FuncDecl{},
		}
		if len(class.Ctors) > 0 {
			info.Ctor = class.Ctors[0]
		}
		t.classes[class.Name] = info
	}

	// Register direct subclasses.
	for _, class := range program.Classes {
		if class.Extends != "" {
			if base, ok := t.classes[class.Extends]; ok {
				base.Subclasses = append(base.Subclasses, class.Name)
			}
		}
	}

	// Flatten fields and methods along the inheritance chain.
	for _, class := range program.Classes {
		t.flattenClass(t.classes[class.Name], map[string]bool{})
	}

	// Which classes are actually instantiated with new?
	for _, expr := range walkProgramExprs(program) {
		if n, ok := expr.(*NewExpr); ok {
			if info, exists := t.classes[n.Type.Name]; exists {
				info.Instantiated = true
			}
		}
	}

	for _, info := range t.classes {
		info.IsInterface = len(info.Subclasses) > 0 && !info.Instantiated
	}

	// Global declarations live in the root scope.
	for _, g := range program.Globals {
		t.scope.declare(g.Name, &Symbol{Type: g.Type, Zero: isZeroLiteral(g.Init)})
	}
}

func (t *Translator) flattenClass(info *ClassInfo, visiting map[string]bool) {
	if info == nil || info.Flattened || visiting[info.Decl.Name] {
		return
	}
	visiting[info.Decl.Name] = true

	if info.Decl.Extends != "" {
		if base, ok := t.classes[info.Decl.Extends]; ok {
			t.flattenClass(base, visiting)
			for _, name := range base.FieldOrder {
				info.Fields[name] = base.Fields[name]
				info.FieldOrder = append(info.FieldOrder, name)
			}
			for _, name := range base.MethodOrder {
				m := base.Methods[name]
				info.Methods[name] = m
				info.MethodOrder = append(info.MethodOrder, name)
				t.methodOwner[m] = info
			}
		}
	}

	for _, field := range info.Decl.Fields {
		if _, exists := info.Fields[field.Name]; !exists {
			info.FieldOrder = append(info.FieldOrder, field.Name)
		}
		info.Fields[field.Name] = field
	}
	for _, method := range info.Decl.Methods {
		if _, exists := info.Methods[method.Name]; !exists {
			info.MethodOrder = append(info.MethodOrder, method.Name)
		}
		info.Methods[method.Name] = method
		t.methodOwner[method] = info
	}

	visiting[info.Decl.Name] = false
	info.Flattened = true
}

func (t *Translator) inferReturnTypes() {
	for _, info := range t.classes {
		for _, name := range info.MethodOrder {
			fd := info.Methods[name]
			if fd.Ret == nil {
				t.returnTypes[fd] = t.inferReturn(fd, info)
			}
		}
	}
	for _, fd := range t.funcs {
		if fd.Ret == nil {
			t.returnTypes[fd] = t.inferReturn(fd, nil)
		}
	}
}

func (t *Translator) inferReturn(fd *FuncDecl, owner *ClassInfo) *Type {
	prevClass := t.currentClass
	prevScope := t.scope
	prevSuppress := t.suppress

	t.currentClass = owner
	t.scope = newScope(nil)
	t.suppress = true
	for _, p := range fd.Params {
		t.scope.declare(p.Name, &Symbol{Type: paramType(p)})
	}

	var inferred *Type
	var visit func(stmts []Stmt)
	visit = func(stmts []Stmt) {
		for _, stmt := range stmts {
			if inferred != nil {
				return
			}
			switch s := stmt.(type) {
			case *ReturnStmt:
				if s.Value != nil {
					inferred = t.typeOf(s.Value)
				}
			case *Block:
				visit(s.Stmts)
			case *IfStmt:
				visit(s.Then.Stmts)
				if b, ok := s.Else.(*Block); ok {
					visit(b.Stmts)
				} else if nested, ok := s.Else.(*IfStmt); ok {
					visit([]Stmt{nested})
				}
			case *WhileStmt:
				visit(s.Body.Stmts)
			case *ForStmt:
				visit(s.Body.Stmts)
			case *DoWhileStmt:
				visit(s.Body.Stmts)
			case *SwitchStmt:
				for _, c := range s.Cases {
					visit(c.Body)
				}
			}
		}
	}
	visit(fd.Body.Stmts)

	t.currentClass = prevClass
	t.scope = prevScope
	t.suppress = prevSuppress
	return inferred
}

// ------------------------------------------------------------------ emission

func (t *Translator) emitProgram() string {
	t.body.Reset()

	// Enums first.
	for _, enum := range t.program.Enums {
		t.emitEnum(enum)
	}

	// Declared interfaces are contracts; emit them for documentation and for
	// code that uses them as types.
	for _, iface := range t.program.Interfaces {
		t.emitInterfaceDecl(iface)
	}

	// Class-based interfaces (a base class used polymorphically).
	for _, info := range t.sortedClasses() {
		if info.IsInterface {
			t.emitClassInterface(info)
		}
	}

	// Structs, constructors, methods.
	for _, info := range t.sortedClasses() {
		if info.IsInterface {
			continue
		}
		t.emitStruct(info)
		t.emitConstructor(info)
		for _, name := range info.MethodOrder {
			t.emitMethod(info, info.Methods[name])
		}
	}

	for _, g := range t.program.Globals {
		t.emitGlobal(g)
	}

	for _, fd := range t.sortedFuncs() {
		t.emitFunc(fd)
	}

	t.emitHelpers()

	return t.body.String()
}

func (t *Translator) sortedClasses() []*ClassInfo {
	names := make([]string, 0, len(t.classes))
	for name := range t.classes {
		names = append(names, name)
	}
	sort.Strings(names)
	out := make([]*ClassInfo, 0, len(names))
	for _, name := range names {
		out = append(out, t.classes[name])
	}
	return out
}

func (t *Translator) sortedFuncs() []*FuncDecl {
	names := make([]string, 0, len(t.funcs))
	for name := range t.funcs {
		names = append(names, name)
	}
	sort.Strings(names)
	out := make([]*FuncDecl, 0, len(names))
	for _, name := range names {
		out = append(out, t.funcs[name])
	}
	return out
}

func (t *Translator) assemble(body string) string {
	var out strings.Builder
	out.WriteString("// Generated by Hust-Go\n")
	out.WriteString("// Do not edit manually\n\n")
	out.WriteString("package main\n\n")

	if t.needsFmt || t.needsStrings || t.needsMath {
		out.WriteString("import (\n")
		if t.needsFmt {
			out.WriteString("\t\"fmt\"\n")
		}
		if t.needsMath {
			out.WriteString("\t\"math\"\n")
		}
		if t.needsStrings {
			out.WriteString("\t\"strings\"\n")
		}
		out.WriteString(")\n\n")
	}

	out.WriteString(body)
	return out.String()
}

func (t *Translator) emitHelpers() {
	if t.needsPop {
		t.body.WriteString("func __hust_pop[T any](s *[]T) T {\n")
		t.body.WriteString("\told := *s\n")
		t.body.WriteString("\tn := len(old)\n")
		t.body.WriteString("\tv := old[n-1]\n")
		t.body.WriteString("\t*s = old[:n-1]\n")
		t.body.WriteString("\treturn v\n")
		t.body.WriteString("}\n\n")
	}
	if t.needsInsert {
		t.body.WriteString("func __hust_insert[T any](s *[]T, i int, v T) {\n")
		t.body.WriteString("\told := *s\n")
		t.body.WriteString("\told = append(old, v)\n")
		t.body.WriteString("\tcopy(old[i+1:], old[i:])\n")
		t.body.WriteString("\told[i] = v\n")
		t.body.WriteString("\t*s = old\n")
		t.body.WriteString("}\n\n")
	}
	if t.needsRemoveAt {
		t.body.WriteString("func __hust_remove_at[T any](s *[]T, i int) T {\n")
		t.body.WriteString("\told := *s\n")
		t.body.WriteString("\tv := old[i]\n")
		t.body.WriteString("\t*s = append(old[:i], old[i+1:]...)\n")
		t.body.WriteString("\treturn v\n")
		t.body.WriteString("}\n\n")
	}
	if t.needsTernary {
		t.body.WriteString("func __hust_ternary[T any](cond bool, a, b T) T {\n")
		t.body.WriteString("\tif cond {\n\t\treturn a\n\t}\n\treturn b\n")
		t.body.WriteString("}\n\n")
	}
	if t.needsCapitalize {
		t.needsStrings = true
		t.body.WriteString("func __hust_capitalize(s string) string {\n")
		t.body.WriteString("\tif s == \"\" {\n\t\treturn s\n\t}\n")
		t.body.WriteString("\tr := []rune(s)\n")
		t.body.WriteString("\treturn strings.ToUpper(string(r[0])) + string(r[1:])\n")
		t.body.WriteString("}\n\n")
	}
}

func (t *Translator) line(format string, args ...interface{}) {
	t.body.WriteString(strings.Repeat("\t", t.indent))
	fmt.Fprintf(&t.body, format, args...)
	t.body.WriteByte('\n')
}

func (t *Translator) emitEnum(enum *EnumDecl) {
	t.line("type %s int32", enum.Name)
	if len(enum.Values) == 0 {
		t.line("")
		return
	}
	t.line("")
	t.line("const (")
	t.indent++
	for i, value := range enum.Values {
		if i == 0 {
			t.line("%s%s %s = iota", enum.Name, value, enum.Name)
		} else {
			t.line("%s%s", enum.Name, value)
		}
	}
	t.indent--
	t.line(")")
	t.line("")
}

func (t *Translator) emitInterfaceDecl(iface *InterfaceDecl) {
	t.line("type %s interface {", iface.Name)
	t.indent++
	for _, sig := range iface.Methods {
		t.line("%s", t.signatureString(sig.Name, sig.Params, sig.Ret, true))
	}
	t.indent--
	t.line("}")
	t.line("")
}

func (t *Translator) emitClassInterface(info *ClassInfo) {
	t.line("type %s interface {", info.Decl.Name)
	t.indent++
	for _, name := range info.MethodOrder {
		fd := info.Methods[name]
		ret := fd.Ret
		if ret == nil {
			ret = t.returnTypes[fd]
		}
		t.line("%s", t.signatureString(fd.Name, fd.Params, ret, false))
	}
	t.indent--
	t.line("}")
	t.line("")
}

func (t *Translator) signatureString(name string, params []*Param, ret *Type, allowUntyped bool) string {
	var parts []string
	for _, p := range params {
		if p.Type == nil {
			parts = append(parts, goName(p.Name)+" any")
		} else {
			parts = append(parts, goName(p.Name)+" "+t.goType(p.Type))
		}
	}
	sig := fmt.Sprintf("%s(%s)", goName(name), strings.Join(parts, ", "))
	if ret != nil && ret.Name != "void" {
		sig += " " + t.goType(ret)
	}
	return sig
}

func (t *Translator) emitStruct(info *ClassInfo) {
	if len(info.FieldOrder) == 0 {
		t.line("type %s struct{}", info.Decl.Name)
		t.line("")
		return
	}
	t.line("type %s struct {", info.Decl.Name)
	t.indent++
	for _, name := range info.FieldOrder {
		field := info.Fields[name]
		t.checkType(field.Type, field.Line)
		t.line("%s %s", goName(field.Name), t.goType(field.Type))
	}
	t.indent--
	t.line("}")
	t.line("")
}

func (t *Translator) emitConstructor(info *ClassInfo) {
	if info.IsInterface {
		return
	}

	fd := info.Ctor
	hasInitializers := false
	for _, name := range info.FieldOrder {
		if info.Fields[name].Init != nil {
			hasInitializers = true
			break
		}
	}
	if fd == nil && !hasInitializers {
		return
	}

	className := info.Decl.Name
	var params []string
	if fd != nil {
		for _, p := range fd.Params {
			t.checkType(p.Type, p.Line)
			params = append(params, goName(p.Name)+" "+t.goType(p.Type))
		}
	}

	t.line("func New%s(%s) *%s {", className, strings.Join(params, ", "), className)
	t.indent++

	prevClass := t.currentClass
	prevScope := t.scope
	t.currentClass = info
	t.scope = newScope(t.scope)
	if fd != nil {
		for _, p := range fd.Params {
			t.scope.declare(p.Name, &Symbol{Type: p.Type})
		}
	}

	t.line("self := &%s{}", className)
	for _, name := range info.FieldOrder {
		field := info.Fields[name]
		if field.Init != nil {
			t.line("self.%s = %s", goName(field.Name), t.expr(field.Init, field.Type))
		}
	}

	returned := false
	if fd != nil {
		for _, stmt := range fd.Body.Stmts {
			if _, ok := stmt.(*ReturnStmt); ok {
				returned = true
			}
			t.emitStmt(stmt)
		}
	}
	if !returned {
		t.line("return self")
	}

	t.scope = prevScope
	t.currentClass = prevClass

	t.indent--
	t.line("}")
	t.line("")
}

func (t *Translator) emitMethod(info *ClassInfo, fd *FuncDecl) {
	ret := fd.Ret
	if ret == nil {
		ret = t.returnTypes[fd]
	}

	var params []string
	for _, p := range fd.Params {
		t.checkType(p.Type, p.Line)
		ptype := "any"
		if p.Type != nil {
			ptype = t.goType(p.Type)
		}
		params = append(params, goName(p.Name)+" "+ptype)
	}
	t.checkType(ret, fd.Line)

	sig := fmt.Sprintf("func (self *%s) %s(%s)", info.Decl.Name, goName(fd.Name), strings.Join(params, ", "))
	if ret != nil && ret.Name != "void" {
		sig += " " + t.goType(ret)
	}
	t.line("%s {", sig)
	t.indent++

	prevClass := t.currentClass
	prevScope := t.scope
	prevReturn := t.currentReturn
	t.currentClass = info
	t.currentReturn = ret
	t.scope = newScope(t.scope)
	for _, p := range fd.Params {
		t.scope.declare(p.Name, &Symbol{Type: paramType(p)})
	}
	for _, stmt := range fd.Body.Stmts {
		t.emitStmt(stmt)
	}
	t.scope = prevScope
	t.currentClass = prevClass
	t.currentReturn = prevReturn

	t.indent--
	t.line("}")
	t.line("")
}

func (t *Translator) emitFunc(fd *FuncDecl) {
	ret := fd.Ret
	if ret == nil {
		ret = t.returnTypes[fd]
	}

	var params []string
	for _, p := range fd.Params {
		t.checkType(p.Type, p.Line)
		params = append(params, goName(p.Name)+" "+t.goType(paramType(p)))
	}
	t.checkType(ret, fd.Line)

	sig := fmt.Sprintf("func %s(%s)", goName(fd.Name), strings.Join(params, ", "))
	if ret != nil && ret.Name != "void" {
		sig += " " + t.goType(ret)
	}
	t.line("%s {", sig)
	t.indent++

	prevScope := t.scope
	prevReturn := t.currentReturn
	t.currentReturn = ret
	t.scope = newScope(t.scope)
	for _, p := range fd.Params {
		t.scope.declare(p.Name, &Symbol{Type: paramType(p)})
	}
	for _, stmt := range fd.Body.Stmts {
		t.emitStmt(stmt)
	}
	t.scope = prevScope
	t.currentReturn = prevReturn

	t.indent--
	t.line("}")
	t.line("")
}

func (t *Translator) emitGlobal(g *VarDecl) {
	value := ""
	if g.Init != nil {
		value = " = " + t.coerce(t.expr(g.Init, g.Type), t.typeOf(g.Init), g.Type)
		t.checkAssign(g.Type, g.Init, g.Line)
	}
	if g.Const {
		t.line("const %s %s%s", goName(g.Name), t.goType(g.Type), value)
	} else {
		t.line("var %s %s%s", goName(g.Name), t.goType(g.Type), value)
	}
}

// ----------------------------------------------------------------- statements

func (t *Translator) emitStmt(stmt Stmt) {
	switch s := stmt.(type) {
	case *Block:
		t.line("{")
		t.indent++
		t.emitBlockContents(s)
		t.indent--
		t.line("}")

	case *VarDecl:
		t.emitVarDecl(s)

	case *ExprStmt:
		t.emitExprStmt(s.X)

	case *IfStmt:
		t.emitIf(s)

	case *WhileStmt:
		t.line("for %s {", t.expr(s.Cond, &Type{Name: "bool"}))
		t.emitBlockContents(s.Body)
		t.line("}")

	case *ForStmt:
		t.emitFor(s)

	case *DoWhileStmt:
		t.line("for {")
		t.emitBlockContents(s.Body)
		t.indent++
		t.line("if !(%s) {", t.expr(s.Cond, &Type{Name: "bool"}))
		t.indent++
		t.line("break")
		t.indent--
		t.line("}")
		t.indent--
		t.line("}")

	case *ReturnStmt:
		if s.Value == nil {
			t.line("return")
		} else {
			value := t.expr(s.Value, t.currentReturn)
			value = t.coerce(value, t.typeOf(s.Value), t.currentReturn)
			t.line("return %s", value)
		}

	case *BreakStmt:
		t.line("break")
	case *ContinueStmt:
		t.line("continue")

	case *SwitchStmt:
		t.emitSwitch(s)

	default:
		t.line("// unsupported statement")
	}
}

func (t *Translator) emitVarDecl(v *VarDecl) {
	t.emitOneVar(v)
	for _, extra := range v.More {
		t.emitOneVar(extra)
	}
}

func (t *Translator) emitOneVar(v *VarDecl) {
	t.checkType(v.Type, v.Line)
	t.scope.declare(v.Name, &Symbol{Type: v.Type, Zero: isZeroLiteral(v.Init)})

	if v.Init != nil && !v.Const && t.emitIncrementDeclaration(v) {
		return
	}

	var value string
	if v.Init != nil {
		value = t.coerce(t.expr(v.Init, v.Type), t.typeOf(v.Init), v.Type)
		t.checkAssign(v.Type, v.Init, v.Line)
	}

	if v.Const {
		if v.Init == nil {
			t.report(v.Line, "constant declaration requires an initializer")
			return
		}
		t.line("const %s %s = %s", goName(v.Name), t.goType(v.Type), value)
		return
	}

	if v.Init != nil {
		t.line("var %s %s = %s", goName(v.Name), t.goType(v.Type), value)
	} else {
		t.line("var %s %s", goName(v.Name), t.goType(v.Type))
	}
	// Go rejects unused local variables; Hust does not.
	t.line("_ = %s", goName(v.Name))
}

func (t *Translator) emitExprStmt(expr Expr) {
	// Array mutation is a statement-level operation in Hust.
	if call, ok := expr.(*CallExpr); ok {
		if member, ok := call.Callee.(*MemberExpr); ok {
			if id, isPkg := member.X.(*Ident); isPkg && id.Name == "fmt" {
				// Package call, not a receiver.
			} else if recvType := t.typeOf(member.X); recvType != nil && recvType.isArray() {
				switch member.Name {
				case "push":
					if len(call.Args) == 1 {
						recv := t.expr(member.X, nil)
						value := t.expr(call.Args[0], recvType.elem())
						t.line("%s = append(%s, %s)", recv, recv, value)
						return
					}
				case "insert":
					if len(call.Args) == 2 {
						recv := t.expr(member.X, nil)
						if t.isAddressable(member.X) {
							t.needsInsert = true
							index := t.coerce(t.expr(call.Args[0], &Type{Name: "i32"}), t.typeOf(call.Args[0]), &Type{Name: "i32"})
							t.line("__hust_insert(&%s, int(%s), %s)", recv, index,
								t.expr(call.Args[1], recvType.elem()))
							return
						}
					}
				case "removeAt":
					if len(call.Args) == 1 {
						recv := t.expr(member.X, nil)
						if t.isAddressable(member.X) {
							t.needsRemoveAt = true
							index := t.coerce(t.expr(call.Args[0], &Type{Name: "i32"}), t.typeOf(call.Args[0]), &Type{Name: "i32"})
							t.line("__hust_remove_at(&%s, int(%s))", recv, index)
							return
						}
					}
				case "clear":
					if len(call.Args) == 0 && t.isAddressable(member.X) {
						recv := t.expr(member.X, nil)
						t.line("%s = %s[:0]", recv, recv)
						return
					}
				case "pop":
					if len(call.Args) == 0 {
						recv := t.expr(member.X, nil)
						if t.isAddressable(member.X) {
							t.needsPop = true
							t.line("__hust_pop(&%s)", recv)
							return
						}
					}
				}
			}
		}
	}

	if assign, ok := expr.(*AssignExpr); ok {
		t.emitAssign(assign)
		return
	}
	if post, ok := expr.(*PostfixExpr); ok {
		t.line("%s%s", t.expr(post.X, nil), post.Op)
		return
	}
	t.line("%s", t.expr(expr, nil))
}

// emitIncrementDeclaration rewrites `T x = y++;` and `T x = ++y;` because Go
// has no increment expression.
func (t *Translator) emitIncrementDeclaration(v *VarDecl) bool {
	switch init := v.Init.(type) {
	case *PostfixExpr:
		if init.Op != "++" && init.Op != "--" {
			return false
		}
		value := t.coerce(t.expr(init.X, nil), t.typeOf(init.X), v.Type)
		t.line("var %s %s = %s", goName(v.Name), t.goType(v.Type), value)
		t.line("%s%s", t.expr(init.X, nil), init.Op)
		return true
	case *UnaryExpr:
		if init.Op != "++" && init.Op != "--" {
			return false
		}
		t.line("%s%s", t.expr(init.X, nil), init.Op)
		value := t.coerce(t.expr(init.X, nil), t.typeOf(init.X), v.Type)
		t.line("var %s %s = %s", goName(v.Name), t.goType(v.Type), value)
		return true
	}
	return false
}

func (t *Translator) emitAssign(assign *AssignExpr) {
	targetType := t.typeOf(assign.Target)

	// `x = y++;` / `x = ++y;`
	if assign.Op == "=" {
		switch valueExpr := assign.Value.(type) {
		case *PostfixExpr:
			if valueExpr.Op == "++" || valueExpr.Op == "--" {
				value := t.coerce(t.expr(valueExpr.X, nil), t.typeOf(valueExpr.X), targetType)
				t.line("%s = %s", t.expr(assign.Target, nil), value)
				t.line("%s%s", t.expr(valueExpr.X, nil), valueExpr.Op)
				return
			}
		case *UnaryExpr:
			if valueExpr.Op == "++" || valueExpr.Op == "--" {
				t.line("%s%s", t.expr(valueExpr.X, nil), valueExpr.Op)
				value := t.coerce(t.expr(valueExpr.X, nil), t.typeOf(valueExpr.X), targetType)
				t.line("%s = %s", t.expr(assign.Target, nil), value)
				return
			}
		}
	}

	var value string
	if targetType != nil && targetType.Name != "error" {
		value = t.coerce(t.expr(assign.Value, targetType), t.typeOf(assign.Value), targetType)
		t.checkAssign(targetType, assign.Value, assign.Line)
	} else {
		value = t.expr(assign.Value, nil)
	}

	target := t.expr(assign.Target, nil)
	t.line("%s %s %s", target, assign.Op, value)

	if assign.Op == "=" {
		if id, ok := assign.Target.(*Ident); ok {
			if sym := t.scope.lookup(id.Name); sym != nil {
				sym.Zero = isZeroLiteral(assign.Value)
			}
		}
	}
}

func (t *Translator) emitIf(stmt *IfStmt) {
	t.line("if %s {", t.expr(stmt.Cond, &Type{Name: "bool"}))
	t.emitBlockContents(stmt.Then)

	cur := stmt
	for {
		if cur.Else == nil {
			t.line("}")
			return
		}
		if next, ok := cur.Else.(*IfStmt); ok {
			t.line("} else if %s {", t.expr(next.Cond, &Type{Name: "bool"}))
			t.emitBlockContents(next.Then)
			cur = next
			continue
		}
		if block, ok := cur.Else.(*Block); ok {
			t.line("} else {")
			t.emitBlockContents(block)
		}
		t.line("}")
		return
	}
}

func (t *Translator) emitFor(stmt *ForStmt) {
	prevScope := t.scope
	t.scope = newScope(t.scope)

	init := ""
	if stmt.Init != nil {
		init = t.emitForInit(stmt.Init)
	}
	cond := ""
	if stmt.Cond != nil {
		cond = t.expr(stmt.Cond, &Type{Name: "bool"})
	}
	post := ""
	if stmt.Post != nil {
		post = t.emitForPost(stmt.Post)
	}

	t.line("for %s; %s; %s {", init, cond, post)
	t.emitBlockContents(stmt.Body)
	t.line("}")

	t.scope = prevScope
}

func (t *Translator) emitForInit(stmt Stmt) string {
	if vd, ok := stmt.(*VarDecl); ok {
		t.scope.declare(vd.Name, &Symbol{Type: vd.Type, Zero: isZeroLiteral(vd.Init)})
		if vd.Init == nil {
			return "var " + goName(vd.Name) + " " + t.goType(vd.Type)
		}
		value := t.expr(vd.Init, vd.Type)
		t.checkAssign(vd.Type, vd.Init, vd.Line)
		from := t.typeOf(vd.Init)
		value = t.coerce(value, from, vd.Type)
		if isNumericType(vd.Type) && isUntyped(from) {
			// Short declarations would infer int/float64; force the Hust type.
			value = fmt.Sprintf("%s(%s)", t.goType(vd.Type), value)
		}
		return goName(vd.Name) + " := " + value
	}
	if es, ok := stmt.(*ExprStmt); ok {
		return t.expr(es.X, nil)
	}
	return ""
}

func (t *Translator) emitForPost(stmt Stmt) string {
	if es, ok := stmt.(*ExprStmt); ok {
		return t.expr(es.X, nil)
	}
	return ""
}

func (t *Translator) emitSwitch(stmt *SwitchStmt) {
	t.line("switch %s {", t.expr(stmt.Value, nil))
	t.indent++
	for _, clause := range stmt.Cases {
		if clause.Default {
			t.line("default:")
		} else {
			t.line("case %s:", t.expr(clause.Value, t.typeOf(stmt.Value)))
		}
		t.indent++
		for _, bodyStmt := range clause.Body {
			t.emitStmt(bodyStmt)
		}
		t.indent--
	}
	t.indent--
	t.line("}")
}

func (t *Translator) emitBlockContents(block *Block) {
	if block == nil {
		return
	}
	t.indent++
	prevScope := t.scope
	t.scope = newScope(t.scope)
	for _, stmt := range block.Stmts {
		t.emitStmt(stmt)
	}
	t.scope = prevScope
	t.indent--
}

// ---------------------------------------------------------------- expressions

func (t *Translator) expr(e Expr, expected *Type) string {
	switch v := e.(type) {
	case *ParenExpr:
		return "(" + t.expr(v.X, expected) + ")"

	case *Literal:
		switch v.Kind {
		case "null":
			return "nil"
		default:
			return v.Text
		}

	case *Ident:
		return goName(v.Name)

	case *ThisExpr:
		return "self"

	case *BinaryExpr:
		return t.emitBinary(v, expected)

	case *UnaryExpr:
		op := v.Op
		if op == "++" || op == "--" {
			// Go has no prefix increment expression; statements only need the
			// postfix form.
			return t.expr(v.X, nil) + op
		}
		if op == "~" {
			op = "^"
		}
		return op + t.expr(v.X, nil)

	case *PostfixExpr:
		return t.expr(v.X, nil) + v.Op

	case *AssignExpr:
		return fmt.Sprintf("%s %s %s", t.expr(v.Target, nil), v.Op, t.expr(v.Value, t.typeOf(v.Target)))

	case *CallExpr:
		return t.emitCall(v, expected)

	case *IndexExpr:
		return fmt.Sprintf("%s[%s]", t.expr(v.X, nil), t.expr(v.Index, &Type{Name: "i32"}))

	case *SliceExpr:
		low, high := "", ""
		if v.Low != nil {
			low = t.expr(v.Low, &Type{Name: "i32"})
		}
		if v.High != nil {
			high = t.expr(v.High, &Type{Name: "i32"})
		}
		return fmt.Sprintf("%s[%s:%s]", t.expr(v.X, nil), low, high)

	case *MemberExpr:
		if enum, ok := t.enumMember(v); ok {
			_ = enum
			return v.X.(*Ident).Name + v.Name
		}
		return t.expr(v.X, nil) + "." + goName(v.Name)

	case *NewExpr:
		return t.emitNew(v)

	case *ArrayLit:
		return t.emitArrayLit(v, expected)

	case *TernaryExpr:
		t.needsTernary = true
		cond := t.expr(v.Cond, &Type{Name: "bool"})
		common := promote(t.typeOf(v.Then), t.typeOf(v.Else))
		thenExpr := t.expr(v.Then, common)
		elseExpr := t.expr(v.Else, common)
		return fmt.Sprintf("__hust_ternary(%s, %s, %s)", cond, thenExpr, elseExpr)
	}

	return ""
}

func (t *Translator) emitBinary(v *BinaryExpr, expected *Type) string {
	resultType := t.typeOf(v)

	// String repetition: "ab" * 3.
	if v.Op == "*" && resultType != nil && resultType.Name == "String" && len(resultType.Dims) == 0 {
		t.needsStrings = true
		lt := t.typeOf(v.L)
		if lt != nil && lt.Name == "String" {
			return fmt.Sprintf("strings.Repeat(%s, int(%s))",
				t.expr(v.L, nil), t.expr(v.R, &Type{Name: "i32"}))
		}
		return fmt.Sprintf("strings.Repeat(%s, int(%s))",
			t.expr(v.R, nil), t.expr(v.L, &Type{Name: "i32"}))
	}

	if v.Op == "+" && resultType != nil && resultType.Name == "String" && len(resultType.Dims) == 0 {
		t.needsFmt = true
		var parts []Expr
		t.flattenConcat(v, &parts)
		var rendered []string
		for _, part := range parts {
			text := t.expr(part, nil)
			if pt := t.typeOf(part); pt != nil && pt.Name == "char" {
				text = "string(" + text + ")"
			}
			rendered = append(rendered, text)
		}
		return "fmt.Sprint(" + strings.Join(rendered, ", ") + ")"
	}

	// A short-circuited right-hand side is still emitted, but its compile-time
	// diagnostics (notably division by a known-zero variable) must not fire.
	skipRight := false
	if v.Op == "&&" {
		if value, ok := t.constBool(v.L); ok && !value {
			skipRight = true
		}
	} else if v.Op == "||" {
		if value, ok := t.constBool(v.L); ok && value {
			skipRight = true
		}
	}

	lt := t.typeOf(v.L)
	var rt *Type
	if skipRight {
		prev := t.suppress
		t.suppress = true
		rt = t.typeOf(v.R)
		t.suppress = prev
	} else {
		rt = t.typeOf(v.R)
	}

	if (v.Op == "/" || v.Op == "%") && !skipRight && t.isZeroExpr(v.R) {
		t.report(v.Line, "Division by zero")
	}

	if v.Op == "+" && lt != nil && lt.isArray() && rt != nil && rt.isArray() {
		arrayType := t.goType(lt)
		return fmt.Sprintf("append(append(%s{}, %s...), %s...)",
			arrayType, t.expr(v.L, nil), t.expr(v.R, nil))
	}

	switch v.Op {
	case "==", "!=", "<", "<=", ">", ">=":
		common := promote(lt, rt)
		return fmt.Sprintf("%s %s %s", t.coerce(t.expr(v.L, common), lt, common), v.Op,
			t.coerce(t.expr(v.R, common), rt, common))
	case "&&", "||":
		left := t.expr(v.L, &Type{Name: "bool"})
		prev := t.suppress
		if skipRight {
			t.suppress = true
		}
		right := t.expr(v.R, &Type{Name: "bool"})
		t.suppress = prev
		return fmt.Sprintf("%s %s %s", left, v.Op, right)
	case "&", "|", "^", "<<", ">>":
		common := promote(lt, rt)
		return fmt.Sprintf("%s %s %s", t.coerce(t.expr(v.L, common), lt, common), v.Op,
			t.coerce(t.expr(v.R, common), rt, common))
	default:
		common := promote(lt, rt)
		if expected != nil && isNumericType(expected) && !isUntyped(common) && !sameType(common, expected) {
			common = expected
		}
		return fmt.Sprintf("%s %s %s", t.coerce(t.expr(v.L, common), lt, common), v.Op,
			t.coerce(t.expr(v.R, common), rt, common))
	}
}

func (t *Translator) constBool(e Expr) (bool, bool) {
	switch v := e.(type) {
	case *Literal:
		if v.Kind == "bool" {
			return v.Text == "true", true
		}
	case *ParenExpr:
		return t.constBool(v.X)
	case *UnaryExpr:
		if v.Op == "!" {
			if value, ok := t.constBool(v.X); ok {
				return !value, true
			}
		}
	case *BinaryExpr:
		switch v.Op {
		case "&&", "||":
			left, okLeft := t.constBool(v.L)
			right, okRight := t.constBool(v.R)
			if okLeft && okRight {
				if v.Op == "&&" {
					return left && right, true
				}
				return left || right, true
			}
		case "==", "!=", ">", "<", ">=", "<=":
			if leftZero, ok := t.zeroOperand(v.L); ok {
				if rightZero, ok := t.zeroOperand(v.R); ok {
					_ = rightZero
					switch v.Op {
					case "==":
						return true, true
					case "!=":
						return false, true
					case ">", "<":
						return false, true
					case ">=", "<=":
						return true, true
					}
				}
				_ = leftZero
			}
		}
	}
	return false, false
}

func (t *Translator) zeroOperand(e Expr) (bool, bool) {
	if t.isZeroExpr(e) {
		return true, true
	}
	return false, false
}

func (t *Translator) flattenConcat(e Expr, out *[]Expr) {
	if b, ok := e.(*BinaryExpr); ok && b.Op == "+" {
		if bt := t.typeOf(b); bt != nil && bt.Name == "String" {
			t.flattenConcat(b.L, out)
			t.flattenConcat(b.R, out)
			return
		}
	}
	*out = append(*out, e)
}

func (t *Translator) emitCall(call *CallExpr, expected *Type) string {
	if member, ok := call.Callee.(*MemberExpr); ok {
		// Package calls.
		if id, ok := member.X.(*Ident); ok && id.Name == "fmt" {
			t.needsFmt = true
			var args []string
			for _, arg := range call.Args {
				rendered := t.expr(arg, nil)
				if (member.Name == "Println" || member.Name == "Print") && t.typeOf(arg) != nil && t.typeOf(arg).Name == "char" {
					rendered = "string(" + rendered + ")"
				}
				args = append(args, rendered)
			}
			return fmt.Sprintf("fmt.%s(%s)", member.Name, strings.Join(args, ", "))
		}

		recvType := t.typeOf(member.X)
		recv := t.expr(member.X, nil)

		// Array methods.
		if recvType != nil && recvType.isArray() {
			switch member.Name {
			case "len", "length":
				return "int32(len(" + recv + "))"
			case "pop":
				t.needsPop = true
				return "__hust_pop(&" + recv + ")"
			case "insert":
				t.needsInsert = true
				var args []string
				for i, arg := range call.Args {
					if i == 0 {
						args = append(args, "int("+t.coerce(t.expr(arg, &Type{Name: "i32"}), t.typeOf(arg), &Type{Name: "i32"})+")")
					} else {
						args = append(args, t.expr(arg, recvType.elem()))
					}
				}
				return fmt.Sprintf("__hust_insert(&%s, %s)", recv, strings.Join(args, ", "))
			case "removeAt":
				t.needsRemoveAt = true
				var args []string
				for i, arg := range call.Args {
					if i == 0 {
						args = append(args, "int("+t.coerce(t.expr(arg, &Type{Name: "i32"}), t.typeOf(arg), &Type{Name: "i32"})+")")
					}
				}
				return fmt.Sprintf("__hust_remove_at(&%s, %s)", recv, strings.Join(args, ", "))
			case "clear":
				return recv + "[:0]"
			case "push":
				t.report(call.Line, "push() cannot be used as an expression")
				return recv
			}
		}

		// String methods.
		if recvType != nil && recvType.Name == "String" {
			return t.emitStringMethod(recv, member.Name, call)
		}

		// Class / interface methods.
		if recvType != nil {
			if info, ok := t.classes[recvType.Name]; ok {
				if fd, ok := info.Methods[member.Name]; ok {
					if len(call.Args) != len(fd.Params) {
						t.report(call.Line, fmt.Sprintf("Wrong number of arguments for %s: expected %d, got %d",
							member.Name, len(fd.Params), len(call.Args)))
					}
					var args []string
					for i, arg := range call.Args {
						var expected *Type
						if i < len(fd.Params) {
							expected = paramType(fd.Params[i])
						}
						rendered := t.expr(arg, expected)
						args = append(args, t.coerce(rendered, t.typeOf(arg), expected))
					}
					return fmt.Sprintf("%s.%s(%s)", recv, goName(member.Name), strings.Join(args, ", "))
				}
			}
			if iface, ok := t.interfaces[recvType.Name]; ok {
				for _, sig := range iface.Methods {
					if sig.Name != member.Name {
						continue
					}
					if len(call.Args) != len(sig.Params) {
						t.report(call.Line, fmt.Sprintf("Wrong number of arguments for %s: expected %d, got %d",
							member.Name, len(sig.Params), len(call.Args)))
					}
					var args []string
					for i, arg := range call.Args {
						var expected *Type
						if i < len(sig.Params) {
							expected = paramType(sig.Params[i])
						}
						rendered := t.expr(arg, expected)
						args = append(args, t.coerce(rendered, t.typeOf(arg), expected))
					}
					return fmt.Sprintf("%s.%s(%s)", recv, goName(member.Name), strings.Join(args, ", "))
				}
			}
		}

		t.report(call.Line, "Undefined method: "+member.Name)
		return recv + "." + goName(member.Name) + "()"
	}

	if id, ok := call.Callee.(*Ident); ok {
		if fd, ok := t.funcs[id.Name]; ok {
			if len(call.Args) != len(fd.Params) {
				t.report(call.Line, fmt.Sprintf("Wrong number of arguments for %s: expected %d, got %d",
					id.Name, len(fd.Params), len(call.Args)))
			}
			var args []string
			for i, arg := range call.Args {
				var expected *Type
				if i < len(fd.Params) {
					expected = paramType(fd.Params[i])
				}
				rendered := t.expr(arg, expected)
				args = append(args, t.coerce(rendered, t.typeOf(arg), expected))
			}
			return fmt.Sprintf("%s(%s)", goName(id.Name), strings.Join(args, ", "))
		}
		if mathName, ok := mathBuiltin(id.Name); ok {
			t.needsMath = true
			var args []string
			for _, arg := range call.Args {
				rendered := t.expr(arg, nil)
				args = append(args, t.coerce(rendered, t.typeOf(arg), &Type{Name: "f64"}))
			}
			rendered := fmt.Sprintf("math.%s(%s)", mathName, strings.Join(args, ", "))
			if expected != nil && expected.Name == "f32" {
				rendered = "float32(" + rendered + ")"
			}
			return rendered
		}
		if _, ok := t.classes[id.Name]; ok {
			t.report(call.Line, "Undefined function: "+id.Name+" (use new "+id.Name+"())")
			return goName(id.Name) + "()"
		}
		t.report(call.Line, "Undefined function: "+id.Name)
		return goName(id.Name) + "()"
	}

	return t.expr(call.Callee, nil) + "()"
}

func (t *Translator) emitStringMethod(recv, name string, call *CallExpr) string {
	var args []string
	for _, arg := range call.Args {
		args = append(args, t.expr(arg, nil))
	}

	switch name {
	case "len", "length":
		return "int32(len(" + recv + "))"
	case "charAt":
		return fmt.Sprintf("[]rune(%s)[%s]", recv, strings.Join(args, ", "))
	case "substring":
		if len(args) == 2 {
			return fmt.Sprintf("string([]rune(%s)[%s:%s])", recv, args[0], args[1])
		}
	case "toUpperCase", "toUpper":
		t.needsStrings = true
		return "strings.ToUpper(" + recv + ")"
	case "toLowerCase", "toLower":
		t.needsStrings = true
		return "strings.ToLower(" + recv + ")"
	case "capitalize":
		t.needsCapitalize = true
		return "__hust_capitalize(" + recv + ")"
	case "trim":
		t.needsStrings = true
		return "strings.TrimSpace(" + recv + ")"
	case "trimLeft":
		t.needsStrings = true
		return "strings.TrimLeft(" + recv + ", \" \\t\\n\\r\")"
	case "trimRight":
		t.needsStrings = true
		return "strings.TrimRight(" + recv + ", \" \\t\\n\\r\")"
	case "contains":
		t.needsStrings = true
		return fmt.Sprintf("strings.Contains(%s, %s)", recv, strings.Join(args, ", "))
	case "startsWith":
		t.needsStrings = true
		return fmt.Sprintf("strings.HasPrefix(%s, %s)", recv, strings.Join(args, ", "))
	case "endsWith":
		t.needsStrings = true
		return fmt.Sprintf("strings.HasSuffix(%s, %s)", recv, strings.Join(args, ", "))
	case "indexOf":
		t.needsStrings = true
		return fmt.Sprintf("int32(strings.Index(%s, %s))", recv, strings.Join(args, ", "))
	case "replace":
		t.needsStrings = true
		if len(args) == 2 {
			return fmt.Sprintf("strings.ReplaceAll(%s, %s, %s)", recv, args[0], args[1])
		}
	case "split":
		t.needsStrings = true
		return fmt.Sprintf("strings.Split(%s, %s)", recv, strings.Join(args, ", "))
	}

	t.report(call.Line, "Undefined string method: "+name)
	return recv
}

func (t *Translator) emitNew(v *NewExpr) string {
	if v.Type.isArray() {
		t.report(v.Line, "new array expressions are not supported; use an array literal")
		return "nil"
	}
	info, ok := t.classes[v.Type.Name]
	if !ok {
		t.report(v.Line, "Unknown type: "+v.Type.Name)
		return "nil"
	}
	if info.IsInterface {
		t.report(v.Line, "Cannot instantiate "+v.Type.Name+"; it is used as a polymorphic base")
		return "nil"
	}
	if info.Ctor != nil {
		if len(v.Args) != len(info.Ctor.Params) {
			t.report(v.Line, fmt.Sprintf("Wrong number of arguments for %s: expected %d, got %d",
				v.Type.Name, len(info.Ctor.Params), len(v.Args)))
		}
		var args []string
		for i, arg := range v.Args {
			var expected *Type
			if i < len(info.Ctor.Params) {
				expected = info.Ctor.Params[i].Type
			}
			rendered := t.expr(arg, expected)
			args = append(args, t.coerce(rendered, t.typeOf(arg), expected))
		}
		return fmt.Sprintf("New%s(%s)", v.Type.Name, strings.Join(args, ", "))
	}
	if len(v.Args) == 0 {
		return "&" + v.Type.Name + "{}"
	}
	t.report(v.Line, "No constructor accepts arguments for "+v.Type.Name)
	return "nil"
}

func (t *Translator) emitArrayLit(lit *ArrayLit, expected *Type) string {
	if expected == nil || !expected.isArray() {
		// Best effort: infer from the first element.
		var elem *Type
		if len(lit.Elems) > 0 {
			elem = t.typeOf(lit.Elems[0])
		}
		if elem == nil {
			t.report(lit.Line, "Cannot infer array literal type")
			return "[]interface{}{}"
		}
		expected = &Type{Name: elem.Name, Dims: append([]int{-1}, elem.Dims...)}
	}

	container := t.goType(expected)
	if len(lit.Elems) == 0 {
		return container + "{}"
	}

	elemType := expected.elem()
	var parts []string
	for _, elem := range lit.Elems {
		rendered := t.expr(elem, elemType)
		parts = append(parts, t.coerce(rendered, t.typeOf(elem), elemType))
	}
	return container + "{" + strings.Join(parts, ", ") + "}"
}

func (t *Translator) isAddressable(e Expr) bool {
	switch e.(type) {
	case *Ident, *MemberExpr, *IndexExpr:
		return true
	default:
		return false
	}
}

// ---------------------------------------------------------------- type system

func (t *Translator) typeOf(e Expr) *Type {
	switch v := e.(type) {
	case *ParenExpr:
		return t.typeOf(v.X)

	case *Literal:
		switch v.Kind {
		case "int":
			return &Type{Name: "untyped-int"}
		case "float":
			return &Type{Name: "untyped-float"}
		case "string":
			return &Type{Name: "String"}
		case "char":
			return &Type{Name: "char"}
		case "bool":
			return &Type{Name: "bool"}
		case "null":
			return &Type{Name: "null"}
		}
		return errorType()

	case *Ident:
		if sym := t.scope.lookup(v.Name); sym != nil {
			return sym.Type
		}
		if t.currentClass != nil {
			if field, ok := t.currentClass.Fields[v.Name]; ok {
				return field.Type
			}
		}
		if _, ok := t.funcs[v.Name]; ok {
			// Function used as a value is not supported.
			return errorType()
		}
		t.report(v.Line, "Undefined variable: "+v.Name)
		return errorType()

	case *ThisExpr:
		if t.currentClass != nil {
			return &Type{Name: t.currentClass.Decl.Name}
		}
		return errorType()

	case *BinaryExpr:
		lt := t.typeOf(v.L)
		var rt *Type
		skipRight := false
		if v.Op == "&&" {
			if value, ok := t.constBool(v.L); ok && !value {
				skipRight = true
			}
		} else if v.Op == "||" {
			if value, ok := t.constBool(v.L); ok && value {
				skipRight = true
			}
		}
		if skipRight {
			prev := t.suppress
			t.suppress = true
			rt = t.typeOf(v.R)
			t.suppress = prev
		} else {
			rt = t.typeOf(v.R)
		}
		if lt != nil && lt.Name == "error" || rt != nil && rt.Name == "error" {
			return errorType()
		}
		switch v.Op {
		case "==", "!=", "<", "<=", ">", ">=", "&&", "||":
			return &Type{Name: "bool"}
		case "+":
			if (lt != nil && lt.Name == "String" && len(lt.Dims) == 0) || (rt != nil && rt.Name == "String" && len(rt.Dims) == 0) {
				return &Type{Name: "String"}
			}
			return promote(lt, rt)
		default:
			return promote(lt, rt)
		}

	case *UnaryExpr:
		if v.Op == "!" {
			return &Type{Name: "bool"}
		}
		return t.typeOf(v.X)

	case *PostfixExpr:
		return t.typeOf(v.X)

	case *AssignExpr:
		return t.typeOf(v.Target)

	case *CallExpr:
		return t.typeOfCall(v)

	case *IndexExpr:
		xt := t.typeOf(v.X)
		if xt != nil && xt.isArray() {
			return xt.elem()
		}
		if xt != nil && xt.Name == "String" {
			return &Type{Name: "char"}
		}
		return errorType()

	case *SliceExpr:
		return t.typeOf(v.X)

	case *MemberExpr:
		return t.typeOfMember(v)

	case *NewExpr:
		if v.Type.isArray() {
			return v.Type
		}
		if _, ok := t.classes[v.Type.Name]; ok {
			return &Type{Name: v.Type.Name}
		}
		if _, ok := t.enums[v.Type.Name]; ok {
			return &Type{Name: v.Type.Name}
		}
		return errorType()

	case *ArrayLit:
		return &Type{Name: "array-literal"}

	case *TernaryExpr:
		return promote(t.typeOf(v.Then), t.typeOf(v.Else))
	}

	return errorType()
}

func mathBuiltin(name string) (string, bool) {
	switch name {
	case "sqrt":
		return "Sqrt", true
	case "pow":
		return "Pow", true
	case "abs":
		return "Abs", true
	case "floor":
		return "Floor", true
	case "ceil":
		return "Ceil", true
	case "sin":
		return "Sin", true
	case "cos":
		return "Cos", true
	case "tan":
		return "Tan", true
	}
	return "", false
}

func (t *Translator) typeOfCall(call *CallExpr) *Type {
	if member, ok := call.Callee.(*MemberExpr); ok {
		if id, ok := member.X.(*Ident); ok && id.Name == "fmt" {
			return &Type{Name: "void"}
		}

		recvType := t.typeOf(member.X)
		if recvType == nil {
			return errorType()
		}
		if recvType.Name == "error" {
			return errorType()
		}

		if recvType.isArray() {
			switch member.Name {
			case "len", "length":
				return &Type{Name: "i32"}
			case "pop", "removeAt":
				return recvType.elem()
			case "insert", "push", "clear":
				return &Type{Name: "void"}
			}
		}

		if recvType.Name == "String" {
			switch member.Name {
			case "len", "length", "indexOf":
				return &Type{Name: "i32"}
			case "charAt":
				return &Type{Name: "char"}
			case "substring", "toUpperCase", "toUpper", "toLowerCase", "toLower",
				"trim", "trimLeft", "trimRight", "replace", "capitalize":
				return &Type{Name: "String"}
			case "contains", "startsWith", "endsWith":
				return &Type{Name: "bool"}
			case "split":
				return &Type{Name: "String", Dims: []int{-1}}
			}
		}

		if info, ok := t.classes[recvType.Name]; ok {
			if fd, ok := info.Methods[member.Name]; ok {
				if fd.Ret != nil {
					return fd.Ret
				}
				if ret, ok := t.returnTypes[fd]; ok {
					return ret
				}
				return &Type{Name: "void"}
			}
		}
		if iface, ok := t.interfaces[recvType.Name]; ok {
			for _, sig := range iface.Methods {
				if sig.Name == member.Name {
					if sig.Ret != nil {
						return sig.Ret
					}
					return &Type{Name: "void"}
				}
			}
		}

		return errorType()
	}

	if id, ok := call.Callee.(*Ident); ok {
		if fd, ok := t.funcs[id.Name]; ok {
			if fd.Ret != nil {
				return fd.Ret
			}
			if ret, ok := t.returnTypes[fd]; ok {
				return ret
			}
			return &Type{Name: "void"}
		}
		if _, ok := mathBuiltin(id.Name); ok {
			return &Type{Name: "f64"}
		}
		if _, ok := t.classes[id.Name]; ok {
			return &Type{Name: "void"}
		}
		t.report(call.Line, "Undefined function: "+id.Name)
		return errorType()
	}

	return errorType()
}

func (t *Translator) typeOfMember(m *MemberExpr) *Type {
	if _, ok := t.enumMember(m); ok {
		if id, ok := m.X.(*Ident); ok {
			return &Type{Name: id.Name}
		}
	}

	xt := t.typeOf(m.X)
	if xt == nil || xt.Name == "error" {
		return errorType()
	}
	if info, ok := t.classes[xt.Name]; ok {
		if field, ok := info.Fields[m.Name]; ok {
			return field.Type
		}
		if fd, ok := info.Methods[m.Name]; ok {
			if fd.Ret != nil {
				return fd.Ret
			}
			return t.returnTypes[fd]
		}
	}
	if iface, ok := t.interfaces[xt.Name]; ok {
		for _, sig := range iface.Methods {
			if sig.Name == m.Name && sig.Ret != nil {
				return sig.Ret
			}
		}
	}
	t.report(m.Line, "Undefined member: "+m.Name)
	return errorType()
}

func (t *Translator) enumMember(m *MemberExpr) (*EnumDecl, bool) {
	id, ok := m.X.(*Ident)
	if !ok {
		return nil, false
	}
	enum, ok := t.enums[id.Name]
	if !ok {
		return nil, false
	}
	for _, value := range enum.Values {
		if value == m.Name {
			return enum, true
		}
	}
	return nil, false
}

func (t *Translator) checkAssign(target *Type, value Expr, line int) {
	if target == nil {
		return
	}
	vt := t.typeOf(value)
	if vt == nil || vt.Name == "error" || vt.Name == "array-literal" {
		return
	}

	if target.Name == "String" && len(target.Dims) == 0 && isNumericType(vt) && vt.Name != "char" {
		t.report(line, "Type mismatch: cannot assign number to string")
		return
	}
	if isNumericType(target) && vt.Name == "String" && len(vt.Dims) == 0 {
		t.report(line, "Type mismatch: cannot assign string to "+t.goType(target))
		return
	}
	if target.Name == "bool" && isNumericType(vt) {
		t.report(line, "Type mismatch: cannot assign number to bool")
		return
	}
	if target.Name == "bool" && vt.Name == "String" {
		t.report(line, "Type mismatch: cannot assign string to bool")
	}
}

func (t *Translator) isZeroExpr(e Expr) bool {
	if lit, ok := e.(*Literal); ok {
		return lit.Kind == "int" && strings.TrimLeft(lit.Text, "0") == "" ||
			lit.Kind == "float" && isZeroFloat(lit.Text)
	}
	if id, ok := e.(*Ident); ok {
		if sym := t.scope.lookup(id.Name); sym != nil {
			return sym.Zero
		}
	}
	return false
}

func (t *Translator) report(line int, message string) {
	if t.suppress {
		return
	}
	key := fmt.Sprintf("%d:%s", line, message)
	if t.reports[key] {
		return
	}
	t.reports[key] = true
	t.errors = append(t.errors, message)
}

func (t *Translator) checkType(typ *Type, line int) {
	if typ == nil || typ.Name == "" {
		return
	}
	switch typ.Name {
	case "i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64", "f32", "f64",
		"bool", "String", "char", "void", "object", "any", "error", "array-literal",
		"untyped-int", "untyped-float":
		return
	}
	if _, ok := t.classes[typ.Name]; ok {
		return
	}
	if _, ok := t.enums[typ.Name]; ok {
		return
	}
	if _, ok := t.interfaces[typ.Name]; ok {
		return
	}
	t.report(line, "Unknown type: "+typ.Name)
}

func (t *Translator) goType(typ *Type) string {
	if typ == nil {
		return ""
	}
	base := mapBaseType(typ.Name)
	if info, ok := t.classes[typ.Name]; ok {
		if info.IsInterface {
			base = typ.Name
		} else {
			base = "*" + typ.Name
		}
	}
	if typ.Name == "object" {
		base = "any"
	}
	for i := len(typ.Dims) - 1; i >= 0; i-- {
		if typ.Dims[i] < 0 {
			base = "[]" + base
		} else {
			base = fmt.Sprintf("[%d]%s", typ.Dims[i], base)
		}
	}
	return base
}

func (t *Translator) coerce(rendered string, from, to *Type) string {
	if from == nil || to == nil {
		return rendered
	}
	if from.Name == "error" || to.Name == "error" {
		return rendered
	}
	if sameType(from, to) {
		return rendered
	}
	if isUntyped(from) {
		if to.Name == "any" {
			if isFloatType(from) {
				return "float32(" + rendered + ")"
			}
			return "int32(" + rendered + ")"
		}
		return rendered
	}
	if from.Name == "any" && isNumericType(to) {
		return rendered + ".(" + t.goType(to) + ")"
	}
	if to.Name == "String" && len(to.Dims) == 0 && from.Name == "char" && len(from.Dims) == 0 {
		return "string(" + rendered + ")"
	}
	if isNumericType(from) && isNumericType(to) {
		target := t.goType(to)
		if isUntyped(to) {
			if isFloatType(to) {
				target = "float64"
			} else {
				target = "int"
			}
		}
		return fmt.Sprintf("%s(%s)", target, rendered)
	}
	if to.Name == "String" && len(to.Dims) == 0 && from.Name != "String" && len(from.Dims) == 0 {
		t.needsFmt = true
		return "fmt.Sprint(" + rendered + ")"
	}
	return rendered
}

var goReserved = map[string]bool{
	"break": true, "case": true, "chan": true, "const": true, "continue": true,
	"default": true, "defer": true, "else": true, "fallthrough": true, "for": true,
	"func": true, "go": true, "goto": true, "if": true, "import": true,
	"interface": true, "map": true, "package": true, "range": true, "return": true,
	"select": true, "struct": true, "switch": true, "type": true, "var": true,
}

// goName avoids Go keywords when a Hust identifier happens to collide.
func goName(name string) string {
	if goReserved[name] {
		return name + "_"
	}
	return name
}

func mapBaseType(name string) string {
	switch name {
	case "i8":
		return "int8"
	case "i16":
		return "int16"
	case "i32":
		return "int32"
	case "i64":
		return "int64"
	case "u8":
		return "uint8"
	case "u16":
		return "uint16"
	case "u32":
		return "uint32"
	case "u64":
		return "uint64"
	case "f32":
		return "float32"
	case "f64":
		return "float64"
	case "bool":
		return "bool"
	case "String":
		return "string"
	case "char":
		return "rune"
	case "void":
		return ""
	case "object":
		return "any"
	case "untyped-int":
		return "int"
	case "untyped-float":
		return "float64"
	default:
		return name
	}
}

func errorType() *Type { return &Type{Name: "error"} }

// paramType defaults untyped shape-interface parameters to Go's `any`.
func paramType(p *Param) *Type {
	if p.Type != nil {
		return p.Type
	}
	return &Type{Name: "any"}
}

func isUntyped(t *Type) bool {
	return t != nil && (t.Name == "untyped-int" || t.Name == "untyped-float")
}

func isNumericType(t *Type) bool {
	if t == nil || len(t.Dims) > 0 {
		return false
	}
	switch t.Name {
	case "i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64", "f32", "f64", "char", "untyped-int", "untyped-float":
		return true
	}
	return false
}

func isFloatType(t *Type) bool {
	return t != nil && len(t.Dims) == 0 && (t.Name == "f32" || t.Name == "f64" || t.Name == "untyped-float")
}

func numericRank(t *Type) int {
	switch t.Name {
	case "untyped-int":
		return 1
	case "i8":
		return 2
	case "u8":
		return 3
	case "i16":
		return 4
	case "u16":
		return 5
	case "i32":
		return 6
	case "char":
		return 6
	case "u32":
		return 7
	case "i64":
		return 8
	case "u64":
		return 9
	case "untyped-float":
		return 10
	case "f32":
		return 11
	case "f64":
		return 12
	}
	return 0
}

func promote(a, b *Type) *Type {
	if a == nil {
		return b
	}
	if b == nil {
		return a
	}
	if a.Name == "error" || b.Name == "error" {
		return errorType()
	}
	if a.Name == "String" || b.Name == "String" {
		if len(a.Dims) == 0 && len(b.Dims) == 0 {
			return &Type{Name: "String"}
		}
		return a
	}
	if sameType(a, b) {
		return a
	}
	if isUntyped(a) && isUntyped(b) {
		if isFloatType(a) || isFloatType(b) {
			return &Type{Name: "untyped-float"}
		}
		return &Type{Name: "untyped-int"}
	}
	if isUntyped(a) {
		if isFloatType(a) && isNumericType(b) && !isFloatType(b) {
			return &Type{Name: "f64"}
		}
		return b
	}
	if isUntyped(b) {
		if isFloatType(b) && isNumericType(a) && !isFloatType(a) {
			return &Type{Name: "f64"}
		}
		return a
	}
	if isFloatType(a) || isFloatType(b) {
		if numericRank(a) >= numericRank(b) {
			return a
		}
		return b
	}
	if isNumericType(a) && isNumericType(b) {
		if numericRank(a) >= numericRank(b) {
			return a
		}
		return b
	}
	return a
}

func sameType(a, b *Type) bool {
	if a == nil || b == nil {
		return false
	}
	if a.Name != b.Name || len(a.Dims) != len(b.Dims) {
		return false
	}
	for i := range a.Dims {
		if a.Dims[i] != b.Dims[i] {
			return false
		}
	}
	return true
}

func isZeroLiteral(e Expr) bool {
	lit, ok := e.(*Literal)
	if !ok {
		return false
	}
	if lit.Kind == "int" {
		return strings.TrimLeft(lit.Text, "0") == ""
	}
	if lit.Kind == "float" {
		return isZeroFloat(lit.Text)
	}
	return false
}

func isZeroFloat(text string) bool {
	trimmed := strings.TrimSpace(text)
	trimmed = strings.TrimSuffix(strings.TrimSuffix(trimmed, "0"), ".")
	return trimmed == "" || trimmed == "-0"
}

// walkProgramExprs visits every expression in the program.
func walkProgramExprs(program *Program) []Expr {
	var out []Expr
	var walkExpr func(Expr)
	var walkStmt func(Stmt)

	walkExpr = func(e Expr) {
		if e == nil {
			return
		}
		out = append(out, e)
		switch v := e.(type) {
		case *BinaryExpr:
			walkExpr(v.L)
			walkExpr(v.R)
		case *UnaryExpr:
			walkExpr(v.X)
		case *PostfixExpr:
			walkExpr(v.X)
		case *AssignExpr:
			walkExpr(v.Target)
			walkExpr(v.Value)
		case *CallExpr:
			walkExpr(v.Callee)
			for _, a := range v.Args {
				walkExpr(a)
			}
		case *IndexExpr:
			walkExpr(v.X)
			walkExpr(v.Index)
		case *SliceExpr:
			walkExpr(v.X)
			walkExpr(v.Low)
			walkExpr(v.High)
		case *MemberExpr:
			walkExpr(v.X)
		case *NewExpr:
			for _, a := range v.Args {
				walkExpr(a)
			}
		case *ArrayLit:
			for _, a := range v.Elems {
				walkExpr(a)
			}
		case *TernaryExpr:
			walkExpr(v.Cond)
			walkExpr(v.Then)
			walkExpr(v.Else)
		case *ParenExpr:
			walkExpr(v.X)
		}
	}

	walkStmt = func(s Stmt) {
		if s == nil {
			return
		}
		switch v := s.(type) {
		case *Block:
			for _, inner := range v.Stmts {
				walkStmt(inner)
			}
		case *VarDecl:
			walkExpr(v.Init)
		case *ExprStmt:
			walkExpr(v.X)
		case *IfStmt:
			walkExpr(v.Cond)
			walkStmt(v.Then)
			walkStmt(v.Else)
		case *WhileStmt:
			walkExpr(v.Cond)
			walkStmt(v.Body)
		case *ForStmt:
			walkStmt(v.Init)
			walkExpr(v.Cond)
			walkStmt(v.Post)
			walkStmt(v.Body)
		case *DoWhileStmt:
			walkStmt(v.Body)
			walkExpr(v.Cond)
		case *ReturnStmt:
			walkExpr(v.Value)
		case *SwitchStmt:
			walkExpr(v.Value)
			for _, c := range v.Cases {
				walkExpr(c.Value)
				for _, inner := range c.Body {
					walkStmt(inner)
				}
			}
		}
	}

	for _, g := range program.Globals {
		walkExpr(g.Init)
	}
	for _, fn := range program.Funcs {
		walkStmt(fn.Body)
	}
	for _, class := range program.Classes {
		for _, field := range class.Fields {
			walkExpr(field.Init)
		}
		for _, method := range class.Methods {
			walkStmt(method.Body)
		}
		for _, ctor := range class.Ctors {
			walkStmt(ctor.Body)
		}
	}
	return out
}
