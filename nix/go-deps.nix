# Go dependencies for lorri's Go rewrite.
# Adapted from ~/kot/Profpatsch/users/Profpatsch/go-deps.nix
# Uses the local nix/buildGo instead of depot.nix.buildGo.
{ pkgs }:

let
  buildGo = import ./buildGo { inherit pkgs; };
in
rec {
  # ══════════════════════════════════════════════════════════
  # System libraries
  # ══════════════════════════════════════════════════════════

  # golang.org/x/sys - System calls and OS primitives
  golang-x-sys = buildGo.external {
    path = "golang.org/x/sys";
    src = pkgs.fetchFromGitHub {
      owner = "golang";
      repo = "sys";
      rev = "v0.37.0";
      sha256 = "sha256-fi0XHTjcaMTOtIB22XNsNa1cMZyVABFfZIylaIxFWDI=";
    };
  };

  # golang.org/x/sys/unix covers both Linux and Darwin
  golang-x-sys-unix = golang-x-sys.unix;

  # golang.org/x/exp - Experimental and deprecated packages
  golang-x-exp = buildGo.external {
    path = "golang.org/x/exp";
    src = pkgs.fetchFromGitHub {
      owner = "golang";
      repo = "exp";
      rev = "e25ba8c21ef6698001cda159e0c790527e430d33";
      sha256 = "1m7cq8i80p6njv8776ba0jwbglrkb2v1w00cg5ajwj9dk71d4hk1";
    };
  };

  # ══════════════════════════════════════════════════════════
  # Filesystem watching
  # ══════════════════════════════════════════════════════════

  # github.com/fsnotify/fsnotify - Cross-platform filesystem notifications
  fsnotify = buildGo.external {
    path = "github.com/fsnotify/fsnotify";
    src = pkgs.fetchFromGitHub {
      owner = "fsnotify";
      repo = "fsnotify";
      rev = "v1.9.0";
      sha256 = "sha256-WtpE1N6dpHwEvIub7Xp/CrWm0fd6PX7MKA4PV44rp2g=";
    };
    deps = [ golang-x-sys-unix ];
  };

  # ══════════════════════════════════════════════════════════
  # SQLite - Pure Go implementation (no CGO!)
  # ══════════════════════════════════════════════════════════

  # github.com/ncruces/julianday - Date/time utilities for go-sqlite3
  ncruces-julianday = buildGo.external {
    path = "github.com/ncruces/julianday";
    src = pkgs.fetchFromGitHub {
      owner = "ncruces";
      repo = "julianday";
      rev = "v1.0.0";
      sha256 = "sha256-EelxOI6DLHy0jYnAcTeda24wLWkGOYuFtoJvuDg874s=";
    };
  };

  # github.com/tetratelabs/wazero - WebAssembly runtime for go-sqlite3
  tetratelabs-wazero = buildGo.external {
    path = "github.com/tetratelabs/wazero";
    src = pkgs.fetchFromGitHub {
      owner = "tetratelabs";
      repo = "wazero";
      rev = "v1.9.0";
      sha256 = "sha256-yxnHLc0PFxh8NRBgK2hvhKaxRM1w3IZ9TnfJM0+uadg=";
    };
  };

  # github.com/ncruces/go-sqlite3 - Pure Go SQLite driver (no CGO!)
  ncruces-go-sqlite3 = buildGo.external {
    path = "github.com/ncruces/go-sqlite3";
    src = pkgs.fetchFromGitHub {
      owner = "ncruces";
      repo = "go-sqlite3";
      rev = "v0.30.0";
      sha256 = "sha256-HPECtUoMH0dDuovnZQssyn+m/U/DBNVP6lHV9cvdMY4=";
    };
    deps = [
      ncruces-julianday
      tetratelabs-wazero
      tetratelabs-wazero.api
      tetratelabs-wazero.experimental
      golang-x-sys-unix
    ];
  };

  # ══════════════════════════════════════════════════════════
  # Utilities for modernc.org packages
  # ══════════════════════════════════════════════════════════

  # github.com/google/uuid - UUID generation
  google-uuid = buildGo.external {
    path = "github.com/google/uuid";
    src = pkgs.fetchFromGitHub {
      owner = "google";
      repo = "uuid";
      rev = "v1.6.0";
      sha256 = "sha256-VWl9sqUzdOuhW0KzQlv0gwwUQClYkmZwSydHG2sALYw=";
    };
  };

  # github.com/dustin/go-humanize - Humanize values (filesizes, times, etc)
  dustin-go-humanize = buildGo.external {
    path = "github.com/dustin/go-humanize";
    src = pkgs.fetchFromGitHub {
      owner = "dustin";
      repo = "go-humanize";
      rev = "v1.0.1";
      sha256 = "1iyhd90pnmxh64nhsh6k02c1b1glpmhh4whga9jgb9g0i5hz3sya";
    };
  };

  # github.com/remyoudompheng/bigfft - Big integer FFT
  remyoudompheng-bigfft = buildGo.external {
    path = "github.com/remyoudompheng/bigfft";
    src = pkgs.fetchFromGitHub {
      owner = "remyoudompheng";
      repo = "bigfft";
      rev = "24d4a6f8daece64d3c9a7660d4ee0974c4e31021";
      sha256 = "0qxfda0jq70ank99zlgfz7iig2jpicbbxnpr7xcf1v9p474ak2dx";
    };
  };

  # ══════════════════════════════════════════════════════════
  # modernc.org/* - Pure Go implementations of various C libraries
  # ══════════════════════════════════════════════════════════

  # modernc.org/mathutil - Math utilities
  modernc-mathutil = buildGo.external {
    path = "modernc.org/mathutil";
    src = pkgs.fetchFromGitLab {
      owner = "cznic";
      repo = "mathutil";
      rev = "v1.6.0";
      sha256 = "0wafxarpfvys5p2wsamadkv8j54ahrv9dwmlba9xsxb85n4q9ywm";
    };
    deps = [ remyoudompheng-bigfft ];
  };

  # modernc.org/memory - Memory allocator
  modernc-memory = buildGo.external {
    path = "modernc.org/memory";
    src = pkgs.fetchFromGitLab {
      owner = "cznic";
      repo = "memory";
      rev = "v1.2.0";
      sha256 = "1xb9lxsppnzgzpf60iirax06jzlrkm32gc6kjcm50qj5ppi8fs8c";
    };
    deps = [ golang-x-sys-unix ];
  };

  # modernc.org/libc - C runtime library in pure Go
  modernc-libc = buildGo.external {
    path = "modernc.org/libc";
    src = pkgs.fetchFromGitLab {
      owner = "cznic";
      repo = "libc";
      rev = "v1.67.0";
      sha256 = "1pvj9v6gph85gd2njmbzl7sqi1grf8v56cy0ss42i5lxlvxl001b";
    };
    deps = [
      golang-x-sys-unix
      dustin-go-humanize
      google-uuid
      golang-x-exp.constraints
      modernc-mathutil
      modernc-memory
    ];
  };

  # modernc.org/sqlite - SQLite database engine in pure Go
  modernc-sqlite = buildGo.external {
    path = "modernc.org/sqlite";
    src = pkgs.fetchFromGitLab {
      owner = "cznic";
      repo = "sqlite";
      rev = "v1.40.1";
      sha256 = "0v0pnvvbkrwwilzzrkr6qn39vdzxxh1b7z5qnrwf75jvb32w79k9";
    };
    deps = [
      golang-x-sys-unix
      modernc-libc
      modernc-libc.sys.types
    ];
  };

  # zombiezen.com/go/sqlite - Low-level SQLite wrapper exposing blob I/O API
  # Wraps modernc.org/sqlite; does NOT provide a database/sql driver.
  zombiezen-go-sqlite = buildGo.external {
    path = "zombiezen.com/go/sqlite";
    src = pkgs.fetchFromGitHub {
      owner = "zombiezen";
      repo = "go-sqlite";
      rev = "v1.4.2";
      sha256 = "sha256-GvH50nFJiBWhwv1ArogmiPMvajySBrzyu/uOdcyrIv0=";
    };
    deps = [
      modernc-libc
      modernc-libc.sys.types
      modernc-sqlite.lib
    ];
  };
}
