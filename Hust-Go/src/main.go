package main

import (
	"fmt"
	"os"
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
		if len(os.Args) < 3 {
			fmt.Fprintf(os.Stderr, "Error: transpile requires entry file\n")
			os.Exit(2)
		}
		entry := os.Args[2]
		fmt.Printf("Transpiling %s...\n", entry)
		// TODO: Implement transpile
	case "build":
		if len(os.Args) < 3 {
			fmt.Fprintf(os.Stderr, "Error: build requires entry file\n")
			os.Exit(2)
		}
		entry := os.Args[2]
		fmt.Printf("Building %s...\n", entry)
		// TODO: Implement build
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

func printUsage() {
	fmt.Fprintf(os.Stderr, "Usage:\n")
	fmt.Fprintf(os.Stderr, "  hust_go transpile <entry> [--project] --out-dir <dir>\n")
	fmt.Fprintf(os.Stderr, "  hust_go build <entry> [--project] --out-dir <dir> [--release|--debug]\n")
	fmt.Fprintf(os.Stderr, "  hust_go info\n")
	fmt.Fprintf(os.Stderr, "  hust_go --version\n")
	fmt.Fprintf(os.Stderr, "  hust_go --help\n")
}
