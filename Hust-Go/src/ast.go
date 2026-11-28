package main

// Type is a Hust type: a base name plus array dimensions.
// A dimension of -1 means a dynamic array ([]), a positive number means a
// fixed-size array ([N]).
type Type struct {
	Name string
	Dims []int
}

func (t *Type) clone() *Type {
	if t == nil {
		return nil
	}
	out := &Type{Name: t.Name}
	out.Dims = append(out.Dims, t.Dims...)
	return out
}

func (t *Type) isArray() bool { return t != nil && len(t.Dims) > 0 }

// elem returns the element type of an array, or nil when t is not an array.
func (t *Type) elem() *Type {
	if !t.isArray() {
		return nil
	}
	out := &Type{Name: t.Name}
	out.Dims = append(out.Dims, t.Dims[:len(t.Dims)-1]...)
	return out
}

// Decl is a top-level declaration.
type Decl interface{ declNode() }

type UseDecl struct {
	Path string
	Line int
}

func (*UseDecl) declNode() {}

type Program struct {
	Uses       []*UseDecl
	Classes    []*ClassDecl
	Interfaces []*InterfaceDecl
	Enums      []*EnumDecl
	Funcs      []*FuncDecl
	Globals    []*VarDecl
}

type EnumDecl struct {
	Name   string
	Values []string
	Line   int
}

func (*EnumDecl) declNode() {}

type InterfaceDecl struct {
	Name    string
	Methods []*FuncSignature
	Line    int
}

func (*InterfaceDecl) declNode() {}

type FuncSignature struct {
	Name   string
	Ret    *Type
	Params []*Param
	Line   int
}

type Param struct {
	Name string
	Type *Type
	Line int
}

type ClassDecl struct {
	Name       string
	Extends    string
	Implements []string
	Fields     []*VarDecl
	Methods    []*FuncDecl
	Ctors      []*FuncDecl
	Line       int
}

func (*ClassDecl) declNode() {}

type FuncDecl struct {
	Name   string
	Ret    *Type
	Params []*Param
	Body   *Block
	Public bool
	Line   int
}

func (*FuncDecl) declNode() {}

// Stmt is a statement node.
type Stmt interface{ stmtNode() }

type Block struct {
	Stmts []Stmt
	Line  int
}

func (*Block) stmtNode() {}

type VarDecl struct {
	Name  string
	Type  *Type
	Init  Expr
	Const bool
	More  []*VarDecl
	Line  int
}

func (*VarDecl) stmtNode()  {}
func (*VarDecl) declNode()  {}

type ExprStmt struct {
	X    Expr
	Line int
}

func (*ExprStmt) stmtNode() {}

type IfStmt struct {
	Cond Expr
	Then *Block
	Else Stmt // *Block or *IfStmt
	Line int
}

func (*IfStmt) stmtNode() {}

type WhileStmt struct {
	Cond Expr
	Body *Block
	Line int
}

func (*WhileStmt) stmtNode() {}

type ForStmt struct {
	Init Stmt
	Cond Expr
	Post Stmt
	Body *Block
	Line int
}

func (*ForStmt) stmtNode() {}

type DoWhileStmt struct {
	Body *Block
	Cond Expr
	Line int
}

func (*DoWhileStmt) stmtNode() {}

type ReturnStmt struct {
	Value Expr
	Line  int
}

func (*ReturnStmt) stmtNode() {}

type BreakStmt struct{ Line int }

func (*BreakStmt) stmtNode() {}

type ContinueStmt struct{ Line int }

func (*ContinueStmt) stmtNode() {}

type SwitchStmt struct {
	Value Expr
	Cases []*CaseClause
	Line  int
}

func (*SwitchStmt) stmtNode() {}

type CaseClause struct {
	Value   Expr
	Default bool
	Body    []Stmt
	Line    int
}

// Expr is an expression node.
type Expr interface{ exprNode() }

type Ident struct {
	Name string
	Line int
}

func (*Ident) exprNode() {}

type Literal struct {
	Kind string // int, float, string, char, bool, null
	Text string
	Line int
}

func (*Literal) exprNode() {}

type ThisExpr struct{ Line int }

func (*ThisExpr) exprNode() {}

type BinaryExpr struct {
	Op   string
	L, R Expr
	Line int
}

func (*BinaryExpr) exprNode() {}

type UnaryExpr struct {
	Op   string
	X    Expr
	Line int
}

func (*UnaryExpr) exprNode() {}

type PostfixExpr struct {
	Op   string
	X    Expr
	Line int
}

func (*PostfixExpr) exprNode() {}

type AssignExpr struct {
	Op     string
	Target Expr
	Value  Expr
	Line   int
}

func (*AssignExpr) exprNode() {}

type CallExpr struct {
	Callee Expr
	Args   []Expr
	Line   int
}

func (*CallExpr) exprNode() {}

type IndexExpr struct {
	X     Expr
	Index Expr
	Line  int
}

func (*IndexExpr) exprNode() {}

type SliceExpr struct {
	X    Expr
	Low  Expr
	High Expr
	Line int
}

func (*SliceExpr) exprNode() {}

type MemberExpr struct {
	X    Expr
	Name string
	Line int
}

func (*MemberExpr) exprNode() {}

type NewExpr struct {
	Type *Type
	Args []Expr
	Line int
}

func (*NewExpr) exprNode() {}

type ArrayLit struct {
	Elems []Expr
	Line  int
}

func (*ArrayLit) exprNode() {}

type TernaryExpr struct {
	Cond, Then, Else Expr
	Line             int
}

func (*TernaryExpr) exprNode() {}

type ParenExpr struct {
	X    Expr
	Line int
}

func (*ParenExpr) exprNode() {}
