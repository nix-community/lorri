// Copyright 2019 Google LLC.
// SPDX-License-Identifier: Apache-2.0

// This tool analyses external (i.e. not built with `buildGo.nix`) Go
// packages to determine a build plan that Nix can import.
package main

import (
	"encoding/json"
	"flag"
	"fmt"
	"go/build"
	"log"
	"os"
	"path"
	"path/filepath"
	"strings"
)

// Path to a JSON file describing all standard library import paths.
// This file is generated and set here by Nix during the build
// process.
var stdlibList string

// pkg describes a single Go package within the specified source
// directory.
//
// Return information includes the local (relative from project root)
// and external (none-stdlib) dependencies of this package.
type pkg struct {
	Name        string       `json:"name"`
	Locator     []string     `json:"locator"`
	Files       []string     `json:"files"`
	SFiles      []string     `json:"sfiles"`
	LocalDeps   [][]string   `json:"localDeps"`
	ForeignDeps []foreignDep `json:"foreignDeps"`
	IsCommand   bool         `json:"isCommand"`
}

type foreignDep struct {
	Path string `json:"path"`
	// filename, column and line number of the import, if known
	Position string `json:"position"`
}

// findGoDirs returns a filepath.WalkFunc that identifies all
// directories that contain Go source code in a certain tree.
func findGoDirs(at string) ([]string, error) {
	dirSet := make(map[string]bool)

	err := filepath.Walk(at, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return err
		}

		name := info.Name()
		// Skip folders that are guaranteed to not be relevant
		if info.IsDir() && (name == "testdata" || name == ".git") {
			return filepath.SkipDir
		}

		// If the current file is a Go file, then the directory is popped
		// (i.e. marked as a Go directory).
		if !info.IsDir() && strings.HasSuffix(name, ".go") && !strings.HasSuffix(name, "_test.go") {
			dirSet[filepath.Dir(path)] = true
		}

		return nil
	})

	if err != nil {
		return nil, err
	}

	goDirs := []string{}
	for goDir := range dirSet {
		goDirs = append(goDirs, goDir)
	}

	return goDirs, nil
}

// analysePackage loads and analyses the imports of a single Go
// package, returning the data that is required by the Nix code to
// generate a derivation for this package.
func analysePackage(root, source, importpath string, stdlib map[string]bool) (pkg, error) {
	ctx := build.Default
	ctx.CgoEnabled = false

	p, err := ctx.ImportDir(source, build.IgnoreVendor)
	if err != nil {
		return pkg{}, err
	}

	local := [][]string{}
	foreign := []foreignDep{}

	for _, i := range p.Imports {
		if stdlib[i] {
			continue
		}

		if i == importpath {
			local = append(local, []string{})
		} else if strings.HasPrefix(i, importpath+"/") {
			local = append(local, strings.Split(strings.TrimPrefix(i, importpath+"/"), "/"))
		} else {
			// The import positions is a map keyed on the import name.
			// The value is a list, presumably because an import can appear
			// multiple times in a package. Let’s just take the first one,
			// should be enough for a good error message.
			firstPos := p.ImportPos[i][0].String()
			foreign = append(foreign, foreignDep{Path: i, Position: firstPos})
		}
	}

	prefix := strings.TrimPrefix(source, root+"/")

	locator := []string{}
	if len(prefix) != len(source) {
		locator = strings.Split(prefix, "/")
	} else {
		// Otherwise, the locator is empty since its the root package and
		// no prefix should be added to files.
		prefix = ""
	}

	files := []string{}
	for _, f := range p.GoFiles {
		files = append(files, path.Join(prefix, f))
	}

	sfiles := []string{}
	for _, f := range p.SFiles {
		sfiles = append(sfiles, path.Join(prefix, f))
	}

	return pkg{
		Name:        path.Join(importpath, prefix),
		Locator:     locator,
		Files:       files,
		SFiles:      sfiles,
		LocalDeps:   local,
		ForeignDeps: foreign,
		IsCommand:   p.IsCommand(),
	}, nil
}

func loadStdlibPkgs(from string) (pkgs map[string]bool, err error) {
	f, err := os.ReadFile(from)
	if err != nil {
		return
	}

	err = json.Unmarshal(f, &pkgs)
	return
}

func main() {
	source := flag.String("source", "", "path to directory with sources to process")
	path := flag.String("path", "", "import path for the package")

	flag.Parse()

	if *source == "" {
		log.Fatalf("-source flag must be specified")
	}

	stdlibPkgs, err := loadStdlibPkgs(stdlibList)
	if err != nil {
		log.Fatalf("failed to load standard library index from %q: %s\n", stdlibList, err)
	}

	goDirs, err := findGoDirs(*source)
	if err != nil {
		log.Fatalf("failed to walk source directory '%s': %s", *source, err)
	}

	all := []pkg{}
	for _, d := range goDirs {
		analysed, err := analysePackage(*source, d, *path, stdlibPkgs)

		// If the Go source analysis returned "no buildable Go files",
		// that directory should be skipped.
		//
		// This might be due to `+build` flags on the platform and other
		// reasons (such as test files).
		if _, ok := err.(*build.NoGoError); ok {
			continue
		}

		if err != nil {
			log.Fatalf("failed to analyse package at %q: %s", d, err)
		}
		all = append(all, analysed)
	}

	// Fail if no packages were found
	if len(all) == 0 {
		log.Printf("ERROR: buildGo analyser found no Go packages\n")
		log.Printf("\n")
		log.Printf("Import path: %s\n", *path)
		log.Printf("Source directory: %s\n", *source)
		log.Printf("\n")
		log.Printf("Discovered %d directories with .go files, but none formed valid packages.\n", len(goDirs))

		if len(goDirs) > 0 {
			log.Printf("\nDirectories examined:\n")
			for _, d := range goDirs {
				log.Printf("  - %s\n", d)
			}
		}

		log.Printf("\nCommon causes:\n")
		log.Printf("  - Source tarball extracts to a subdirectory (e.g., repo-version/)\n")
		log.Printf("  - Import path doesn't match directory structure\n")
		log.Printf("  - All .go files are test files (*_test.go)\n")
		log.Printf("  - Build constraints exclude all files for this platform\n")
		log.Printf("\n")
		log.Printf("Suggestion: Use fetchFromGitLab/fetchFromGitHub instead of fetchurl\n")
		log.Printf("            or check the tarball structure with: tar -tzf <tarball>\n")

		log.Fatalf("\nAnalysis failed: no buildable packages found")
	}

	// Log success summary showing what packages were found
	log.Printf("Successfully analyzed %s\n", *path)

	// Check if there's a root package (locator is empty)
	hasRoot := false
	subpackages := []string{}
	for _, pkg := range all {
		if len(pkg.Locator) == 0 {
			hasRoot = true
		} else {
			subpackages = append(subpackages, strings.Join(pkg.Locator, "/"))
		}
	}

	if hasRoot {
		log.Printf("Root package: yes\n")
	} else {
		log.Printf("Root package: no\n")
	}

	if len(subpackages) > 0 {
		log.Printf("Subpackages: %s\n", strings.Join(subpackages, ", "))
	}

	j, _ := json.Marshal(all)
	fmt.Println(string(j))
}
