// Copyright 2024 Profpatsch
// SPDX-License-Identifier: Apache-2.0
//
// mkembedcfg generates embedcfg JSON from go list output.
// This enables buildGo to support //go:embed directives.

package main

import (
	"encoding/json"
	"flag"
	"fmt"
	"io"
	"log"
	"os"
	"path/filepath"
)

// GoListOutput represents the relevant fields from `go list -json`
type GoListOutput struct {
	Dir           string   `json:"Dir"`
	EmbedPatterns []string `json:"EmbedPatterns"`
	EmbedFiles    []string `json:"EmbedFiles"`
}

// EmbedCfg represents the embedcfg format expected by `go tool compile -embedcfg`
type EmbedCfg struct {
	Patterns map[string][]string `json:"Patterns"`
	Files    map[string]string   `json:"Files"`
}

func main() {
	srcDir := flag.String("srcdir", "", "source directory (absolute path)")
	flag.Parse()

	if *srcDir == "" {
		log.Fatal("error: -srcdir is required")
	}

	// Read go list JSON from stdin
	data, err := io.ReadAll(os.Stdin)
	if err != nil {
		log.Fatalf("error reading stdin: %v", err)
	}

	var listOutput GoListOutput
	if err := json.Unmarshal(data, &listOutput); err != nil {
		log.Fatalf("error parsing go list output: %v", err)
	}

	// Build embedcfg
	cfg := EmbedCfg{
		Patterns: make(map[string][]string),
		Files:    make(map[string]string),
	}

	// If no embed patterns, output empty config
	if len(listOutput.EmbedPatterns) == 0 {
		outputJSON(cfg)
		return
	}

	// Map each pattern to the files it matches using filepath.Match.
	// Patterns may be plain paths (e.g. "templates/index.html") or globs
	// (e.g. "templates/*.html"). Directory patterns include all files below.
	for _, pattern := range listOutput.EmbedPatterns {
		var matched []string
		for _, file := range listOutput.EmbedFiles {
			ok, err := filepath.Match(pattern, file)
			if err != nil {
				log.Fatalf("bad pattern %q: %v", pattern, err)
			}
			if ok {
				matched = append(matched, file)
				continue
			}
			// Also match if the file is inside a directory named by the pattern.
			inDir, _ := filepath.Match(pattern+"/*", file)
			if inDir {
				matched = append(matched, file)
				continue
			}
			// Plain path equality (no wildcards).
			if file == pattern {
				matched = append(matched, file)
			}
		}
		if len(matched) == 0 {
			// Fallback: map pattern to all files (original behaviour).
			matched = listOutput.EmbedFiles
		}
		cfg.Patterns[pattern] = matched
	}

	// Map each relative file path to its absolute path.
	for _, file := range listOutput.EmbedFiles {
		absPath := filepath.Join(*srcDir, file)
		cfg.Files[file] = absPath
	}

	outputJSON(cfg)
}

func outputJSON(cfg EmbedCfg) {
	output, err := json.MarshalIndent(cfg, "", "  ")
	if err != nil {
		log.Fatalf("error marshaling JSON: %v", err)
	}
	fmt.Println(string(output))
}
