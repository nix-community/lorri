buildGo.nix
===========

This is an alternative [Nix][] build system for [Go][]. It supports building Go
libraries and programs.

*Note:* This will probably end up being folded into [Nixery][].

## Background

Most language-specific Nix tooling outsources the build to existing
language-specific build tooling, which essentially means that Nix ends up being
a wrapper around all sorts of external build systems.

However, systems like [Bazel][] take an alternative approach in which the
compiler is invoked directly and the composition of programs and libraries stays
within a single homogeneous build system.

Users don't need to learn per-language build systems and especially for
companies with large monorepo-setups ([like Google][]) this has huge
productivity impact.

This project is an attempt to prove that Nix can be used in a similar style to
build software directly, rather than shelling out to other build systems.

## Example

Given a program layout like this:

```
.
├── lib          <-- some library component
│   ├── bar.go
│   └── foo.go
├── main.go      <-- program implementation
└── default.nix  <-- build instructions
```

The contents of `default.nix` could look like this:

```nix
{ buildGo }:

let
  lib = buildGo.package {
    name = "somelib";
    srcs = [
      ./lib/bar.go
      ./lib/foo.go
    ];
  };
in buildGo.program {
  name = "my-program";
  deps = [ lib ];

  srcs = [
    ./main.go
  ];
}
```

(If you don't know how to read Nix, check out [nix-1p][])

## Usage

`buildGo` exposes five different functions:

* `buildGo.program`: Build a Go binary out of the specified source files.

  | parameter | type                    | use                                            | required? |
  |-----------|-------------------------|------------------------------------------------|-----------|
  | `name`    | `string`                | Name of the program (and resulting executable) | yes       |
  | `srcs`    | `list<path>`            | List of paths to source files                  | yes       |
  | `deps`    | `list<drv>`             | List of dependencies (i.e. other Go libraries) | no        |
  | `x_defs`  | `attrs<string, string>` | Attribute set of linker vars (i.e. `-X`-flags) | no        |

* `buildGo.package`: Build a Go library out of the specified source files.

  | parameter | type         | use                                            | required? |
  |-----------|--------------|------------------------------------------------|-----------|
  | `name`    | `string`     | Name of the library                            | yes       |
  | `srcs`    | `list<path>` | List of paths to source files                  | yes       |
  | `deps`    | `list<drv>`  | List of dependencies (i.e. other Go libraries) | no        |
  | `path`    | `string`     | Go import path for the resulting library       | no        |

* `buildGo.external`: Build an externally defined Go library or program.

  This function performs analysis on the supplied source code (which
  can use the standard Go tooling layout) and creates a tree of all
  the packages contained within.

  This exists for compatibility with external libraries that were not
  defined using buildGo.

  | parameter | type           | use                                           | required? |
  |-----------|----------------|-----------------------------------------------|-----------|
  | `path`    | `string`       | Go import path for the resulting package      | yes       |
  | `src`     | `path`         | Path to the source **directory**              | yes       |
  | `deps`    | `list<drv>`    | List of dependencies (i.e. other Go packages) | no        |

  **How `buildGo.external` works:**

  `buildGo.external` analyzes a Go repository and creates a nested attribute set
  matching the package structure. For each package found, it creates a `gopkg`
  attribute containing the compiled library.

  For example, `github.com/varlink/go` creates:
  ```nix
  {
    gopkg = <root-package-drv>;
    varlink = {
      gopkg = <varlink-subpackage-drv>;
      idl = {
        gopkg = <idl-subpackage-drv>;
      };
    };
  }
  ```

  **Using external packages as dependencies:**

  When an external package is a single-package repository (like `github.com/ncruces/julianday`),
  it will have `gopkg` at the root:
  ```nix
  julianday = buildGo.external {
    path = "github.com/ncruces/julianday";
    src = fetchFromGitHub { ... };
  };
  # Use as: deps = [ julianday ];
  ```

  When an external package contains multiple packages (like `github.com/tetratelabs/wazero`),
  you must navigate to the specific subpackages needed:
  ```nix
  wazero = buildGo.external {
    path = "github.com/tetratelabs/wazero";
    src = fetchFromGitHub { ... };
  };
  # Use as: deps = [ wazero wazero.api wazero.experimental ];
  ```

  Some repositories have no root package, only subpackages (like `golang.org/x/sys`):
  ```nix
  golang-x-sys = buildGo.external {
    path = "golang.org/x/sys";
    src = fetchFromGitHub { ... };
  };
  # Use as: deps = [ golang-x-sys.unix golang-x-sys.windows ];
  ```

  **Dependency resolution:**

  When you pass `deps` to `buildGo.external`, the analyser extracts each dependency's
  `goImportPath` and `gopkg` attributes to build a dependency map. When Go code imports
  a foreign package (e.g., `import "github.com/ncruces/julianday"`), buildGo looks up
  the import path in this map to find the correct package.

  This means:
  - Always pass complete external results or their subpackages with `gopkg` attributes
  - buildGo automatically finds the right subpackage based on import statements
  - You don't need to manually wire up internal dependencies within an external package

## Working with External Dependencies

For projects with multiple external dependencies, it's recommended to organize them
in a separate `go-deps.nix` file:

```nix
# go-deps.nix
{ depot, pkgs, ... }:

rec {
  # Single-package repository - gopkg at root
  varlink-go = depot.nix.buildGo.external {
    path = "github.com/varlink/go";
    src = pkgs.fetchFromGitHub {
      owner = "varlink";
      repo = "go";
      rev = "ec4cb63960126b1102b3c578fa245c90614ad4f4";
      sha256 = "sha256-rIL/ka0mxxsddeEM0cf7HApeLFpLN/ljSP12od/HZCo=";
    };
  };

  # Multi-package repository - navigate to subpackages
  wazero = depot.nix.buildGo.external {
    path = "github.com/tetratelabs/wazero";
    src = pkgs.fetchFromGitHub {
      owner = "tetratelabs";
      repo = "wazero";
      rev = "v1.9.0";
      sha256 = "sha256-yxnHLc0PFxh8NRBgK2hvhKaxRM1w3IZ9TnfJM0+uadg=";
    };
  };
}
```

Then use them in your `default.nix`:

```nix
# default.nix
{ depot, pkgs, ... }:

let
  goDeps = import ./go-deps.nix { inherit depot pkgs; };
in
depot.nix.buildGo.program {
  name = "my-program";
  srcs = [ ./main.go ];
  deps = [
    goDeps.varlink-go.varlink      # Navigate to varlink subpackage
    goDeps.varlink-go.varlink.idl  # Navigate to idl subpackage
    goDeps.wazero                  # Root package
    goDeps.wazero.api              # api subpackage
  ];
}
```

## go:embed Support

buildGo fully supports `//go:embed` directives for embedding files into Go binaries.
The build system automatically detects embed directives using `go list` and generates
the necessary embedcfg configuration for the compiler.

Example:

```go
package main

import (
    _ "embed"
    "fmt"
)

//go:embed config.txt
var config string

func main() {
    fmt.Println(config)
}
```

This works transparently with both `buildGo.program` and `buildGo.package`. Embedded
files must be present in the same directory as the source files that reference them.

## Current status

This project is work-in-progress. It is still lacking the following features:

* feature flag parity with Bazel's Go rules
* documentation building
* test execution

There are still some open questions around how to structure some of those
features in Nix.

[Nix]: https://nixos.org/nix/
[Go]: https://golang.org/
[Nixery]: https://github.com/google/nixery
[Bazel]: https://bazel.build/
[like Google]: https://ai.google/research/pubs/pub45424
[nix-1p]: https://github.com/tazjin/nix-1p
