package main

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
)

const Version = "0.1.1.20261128125000"

func main() {
	if len(os.Args) < 2 {
		printUsage()
		os.Exit(2)
	}

	switch os.Args[1] {
	case "transpile":
		os.Exit(handleTranspile(os.Args[2:]))
	case "build":
		os.Exit(handleBuild(os.Args[2:]))
	case "info":
		fmt.Printf(`{"protocol":1,"name":"go","language":"go","transpile":true,"build":true,"toolchain":%t}`+"\n", findGo() != "")
	case "--version", "-V":
		fmt.Printf("hust_go %s\n", Version)
	case "--help", "-h":
		printUsage()
	default:
		fmt.Fprintf(os.Stderr, "Error: unknown command '%s'\n", os.Args[1])
		printUsage()
		os.Exit(2)
	}
}

type options struct {
	entry   string
	outDir  string
	release bool
	debug   bool
	project bool
}

func parseOptions(args []string) (options, error) {
	opts := options{outDir: ".", release: true}

	for i := 0; i < len(args); i++ {
		switch args[i] {
		case "--out-dir":
			if i+1 >= len(args) {
				return opts, fmt.Errorf("--out-dir requires a value")
			}
			i++
			opts.outDir = args[i]
		case "--release":
			opts.release = true
			opts.debug = false
		case "--debug":
			opts.debug = true
			opts.release = false
		case "--project":
			opts.project = true
		default:
			if strings.HasPrefix(args[i], "-") {
				return opts, fmt.Errorf("unknown option '%s'", args[i])
			}
			if opts.entry != "" {
				return opts, fmt.Errorf("unexpected argument '%s'", args[i])
			}
			opts.entry = args[i]
		}
	}

	if opts.entry == "" {
		return opts, fmt.Errorf("missing entry file")
	}

	resolved, err := resolveEntry(opts.entry)
	if err != nil {
		return opts, err
	}
	opts.entry = resolved
	return opts, nil
}

func resolveEntry(raw string) (string, error) {
	info, err := os.Stat(raw)
	if err == nil && !info.IsDir() {
		return raw, nil
	}
	if err == nil && info.IsDir() {
		config := filepath.Join(raw, "settings.config")
		if data, err := os.ReadFile(config); err == nil {
			for _, line := range strings.Split(string(data), "\n") {
				line = strings.TrimSpace(line)
				if line == "" || strings.HasPrefix(line, "#") || strings.HasPrefix(line, ";") || strings.HasPrefix(line, "[") {
					continue
				}
				key, value, ok := strings.Cut(line, "=")
				if !ok {
					continue
				}
				if strings.TrimSpace(key) == "entry" {
					candidate := filepath.Join(raw, strings.Trim(strings.TrimSpace(value), `"`))
					if _, statErr := os.Stat(candidate); statErr == nil {
						return candidate, nil
					}
				}
			}
		}
		main := filepath.Join(raw, "main.hust")
		if _, statErr := os.Stat(main); statErr == nil {
			return main, nil
		}
	}
	return "", fmt.Errorf("entry file not found: %s", raw)
}

func handleTranspile(args []string) int {
	opts, err := parseOptions(args)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error: %v\n", err)
		return 2
	}

	source, err := os.ReadFile(opts.entry)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error: cannot read entry file: %v\n", err)
		return 2
	}

	result := NewTranslator().Translate(string(source), filepath.Base(opts.entry))
	if !result.Success {
		for _, message := range result.Errors {
			fmt.Fprintf(os.Stderr, "Error: %s\n", message)
		}
		return 4
	}

	if err := os.MkdirAll(opts.outDir, 0o755); err != nil {
		fmt.Fprintf(os.Stderr, "Error: cannot create output directory: %v\n", err)
		return 2
	}

	outFile := outputFile(opts.outDir, opts.entry)
	if err := os.WriteFile(outFile, []byte(result.Code), 0o644); err != nil {
		fmt.Fprintf(os.Stderr, "Error: cannot write output file: %v\n", err)
		return 2
	}
	return 0
}

func handleBuild(args []string) int {
	opts, err := parseOptions(args)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error: %v\n", err)
		return 2
	}

	goBin := findGo()
	if goBin == "" {
		fmt.Fprintf(os.Stderr, "Error: go toolchain not found (set GO_BIN or add go to PATH)\n")
		return 3
	}

	source, err := os.ReadFile(opts.entry)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error: cannot read entry file: %v\n", err)
		return 2
	}

	result := NewTranslator().Translate(string(source), filepath.Base(opts.entry))
	if !result.Success {
		for _, message := range result.Errors {
			fmt.Fprintf(os.Stderr, "Error: %s\n", message)
		}
		return 4
	}

	if err := os.MkdirAll(opts.outDir, 0o755); err != nil {
		fmt.Fprintf(os.Stderr, "Error: cannot create output directory: %v\n", err)
		return 2
	}

	goFile := outputFile(opts.outDir, opts.entry)
	if err := os.WriteFile(goFile, []byte(result.Code), 0o644); err != nil {
		fmt.Fprintf(os.Stderr, "Error: cannot write output file: %v\n", err)
		return 2
	}

	stem := strings.TrimSuffix(filepath.Base(opts.entry), filepath.Ext(opts.entry))
	executable := filepath.Join(opts.outDir, stem)
	if runtime.GOOS == "windows" {
		executable += ".exe"
	}

	buildArgs := []string{"build", "-o", executable}
	if opts.release {
		buildArgs = append(buildArgs, "-ldflags=-s -w")
	}
	buildArgs = append(buildArgs, goFile)

	cmd := exec.Command(goBin, buildArgs...)
	cmd.Stdout = os.Stderr
	cmd.Stderr = os.Stderr
	if err := cmd.Run(); err != nil {
		fmt.Fprintf(os.Stderr, "Error: go build failed: %v\n", err)
		return 5
	}

	abs, err := filepath.Abs(executable)
	if err != nil {
		abs = executable
	}
	// Contract: stdout carries only the executable line.
	fmt.Printf("executable=%s\n", abs)
	return 0
}

func outputFile(outDir, entry string) string {
	// Always use main.go: a source named `foo_test.hust` would otherwise
	// produce `foo_test.go`, which the Go toolchain ignores as a test file.
	return filepath.Join(outDir, "main.go")
}

func findGo() string {
	if candidate := os.Getenv("GO_BIN"); candidate != "" {
		if isExecutable(candidate) {
			return candidate
		}
	}
	if path, err := exec.LookPath("go"); err == nil {
		return path
	}
	if home, err := os.UserHomeDir(); err == nil {
		candidate := filepath.Join(home, "go-sdk", "go", "bin", "go")
		if isExecutable(candidate) {
			return candidate
		}
	}
	return ""
}

func isExecutable(path string) bool {
	info, err := os.Stat(path)
	if err != nil || info.IsDir() {
		return false
	}
	if runtime.GOOS == "windows" {
		return true
	}
	return info.Mode()&0o111 != 0
}

func printUsage() {
	fmt.Fprintf(os.Stderr, "Hust-Go core %s\n", Version)
	fmt.Fprintf(os.Stderr, "Usage:\n")
	fmt.Fprintf(os.Stderr, "  hust_go transpile <entry> [--project] --out-dir <dir>\n")
	fmt.Fprintf(os.Stderr, "  hust_go build <entry> [--project] --out-dir <dir> [--release|--debug]\n")
	fmt.Fprintf(os.Stderr, "  hust_go info\n")
	fmt.Fprintf(os.Stderr, "  hust_go --version\n")
	fmt.Fprintf(os.Stderr, "  hust_go --help\n")
}
