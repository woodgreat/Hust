package main

import (
	"fmt"
	"os"
	"path/filepath"
)

const (
	Version = "0.1.0.20261127084900"
)

func main() {
	if len(os.Args) < 2 {
		printUsage()
		os.Exit(1)
	}

	command := os.Args[1]

	switch command {
	case "transpile":
		handleTranspile()
	case "build":
		handleBuild()
	case "info":
		fmt.Printf(`{"protocol":1,"name":"go","language":"go","transpile":true,"build":true,"toolchain":true}`)
	case "--version", "-V":
		fmt.Printf("hust_go %s\n", Version)
	case "--help", "-h":
		printUsage()
	default:
		fmt.Fprintf(os.Stderr, "Error: unknown command '%s'\n", command)
		printUsage()
		os.Exit(2)
	}
}

func handleTranspile() {
	if len(os.Args) < 3 {
		fmt.Fprintf(os.Stderr, "Error: transpile requires entry file\n")
		os.Exit(2)
	}
	
	entry := os.Args[2]
	outDir := "."
	
	// Parse --out-dir
	for i := 3; i < len(os.Args); i++ {
		if os.Args[i] == "--out-dir" && i+1 < len(os.Args) {
			outDir = os.Args[i+1]
			i++
		}
	}
	
	// Read source file
	source, err := os.ReadFile(entry)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error: cannot read entry file: %v\n", err)
		os.Exit(2)
	}
	
	// Transpile
	translator := NewTranslator()
	result := translator.Translate(string(source), filepath.Base(entry))
	
	if !result.Success {
		for _, err := range result.Errors {
			fmt.Fprintf(os.Stderr, "Error: %s\n", err)
		}
		os.Exit(4)
	}
	
	// Write output
	outputFile := filepath.Join(outDir, filepath.Base(entry[:len(entry)-5])+".go")
	if err := os.MkdirAll(outDir, 0755); err != nil {
		fmt.Fprintf(os.Stderr, "Error: cannot create output directory: %v\n", err)
		os.Exit(2)
	}
	
	if err := os.WriteFile(outputFile, []byte(result.Code), 0644); err != nil {
		fmt.Fprintf(os.Stderr, "Error: cannot write output file: %v\n", err)
		os.Exit(2)
	}
	
	fmt.Fprintf(os.Stderr, "Transpiled %s -> %s\n", entry, outputFile)
}

func handleBuild() {
	if len(os.Args) < 3 {
		fmt.Fprintf(os.Stderr, "Error: build requires entry file\n")
		os.Exit(2)
	}
	
	entry := os.Args[2]
	outDir := "."
	release := true
	
	// Parse flags
	for i := 3; i < len(os.Args); i++ {
		switch os.Args[i] {
		case "--out-dir":
			if i+1 < len(os.Args) {
				outDir = os.Args[i+1]
				i++
			}
		case "--debug":
			release = false
		case "--release":
			release = true
		}
	}
	
	// First transpile
	source, err := os.ReadFile(entry)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error: cannot read entry file: %v\n", err)
		os.Exit(2)
	}
	
	translator := NewTranslator()
	result := translator.Translate(string(source), filepath.Base(entry))
	
	if !result.Success {
		for _, err := range result.Errors {
			fmt.Fprintf(os.Stderr, "Error: %s\n", err)
		}
		os.Exit(4)
	}
	
	// Write transpiled file
	if err := os.MkdirAll(outDir, 0755); err != nil {
		fmt.Fprintf(os.Stderr, "Error: cannot create output directory: %v\n", err)
		os.Exit(2)
	}
	
	goFile := filepath.Join(outDir, "main.go")
	if err := os.WriteFile(goFile, []byte(result.Code), 0644); err != nil {
		fmt.Fprintf(os.Stderr, "Error: cannot write Go file: %v\n", err)
		os.Exit(2)
	}
	
	// Build with go
	executable := filepath.Join(outDir, "main")
	if runtime.GOOS == "windows" {
		executable += ".exe"
	}
	
	// Determine build mode
	buildMode := "-ldflags=-s -w" // release: strip debug info
	if !release {
		buildMode = "" // debug: keep debug info
	}
	
	// Run go build
	cmd := exec.Command("go", "build", "-o", executable, goFile)
	if buildMode != "" {
		cmd = exec.Command("go", "build", "-ldflags=-s -w", "-o", executable, goFile)
	}
	
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr
	
	if err := cmd.Run(); err != nil {
		fmt.Fprintf(os.Stderr, "Error: go build failed: %v\n", err)
		os.Exit(5)
	}
	
	// Output executable path (contract requirement)
	absPath, err := filepath.Abs(executable)
	if err != nil {
		absPath = executable
	}
	fmt.Printf("executable=%s\n", absPath)
}

func printUsage() {
	fmt.Fprintf(os.Stderr, "Usage:\n")
	fmt.Fprintf(os.Stderr, "  hust_go transpile <entry> [--project] --out-dir <dir>\n")
	fmt.Fprintf(os.Stderr, "  hust_go build <entry> [--project] --out-dir <dir> [--release|--debug]\n")
	fmt.Fprintf(os.Stderr, "  hust_go info\n")
	fmt.Fprintf(os.Stderr, "  hust_go --version\n")
	fmt.Fprintf(os.Stderr, "  hust_go --help\n")
}
