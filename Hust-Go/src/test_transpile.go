package main

import (
	"fmt"
)

func main() {
	source := `use GO.fmt.*;

void main() {
    i32 x = "hello";
    fmt.Println(x);
}
`
	
	translator := NewTranslator()
	result := translator.Translate(source, "test.hust")
	
	fmt.Printf("Success: %v\n", result.Success)
	fmt.Printf("Errors: %v\n", result.Errors)
	fmt.Printf("Code:\n%s\n", result.Code)
}
