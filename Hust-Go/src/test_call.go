package main

import (
	"fmt"
)

func main() {
	translator := NewTranslator()
	
	// Test translateVariableDeclaration directly
	translator.translateVariableDeclaration("i32 x = \"hello\";", "i32 x = \"hello\";")
	
	fmt.Printf("SemanticErrors after direct call: %v\n", translator.semanticErrors)
}
