{ lib, buildRustCrate, buildRustCrateHelpers }:
with buildRustCrateHelpers;
let inherit (lib.lists) fold;
    inherit (lib.attrsets) recursiveUpdate;
in
rec {

# aho-corasick-0.7.12

  crates.aho_corasick."0.7.12" = deps: { features?(features_.aho_corasick."0.7.12" deps {}) }: buildRustCrate {
    crateName = "aho-corasick";
    version = "0.7.12";
    description = "Fast multiple substring searching.";
    authors = [ "Andrew Gallant <jamslam@gmail.com>" ];
    sha256 = "1aa870hbl1m93bqibpdbdkv0sv43njkvl363va0ji7n70zbd41k2";
    libName = "aho_corasick";
    dependencies = mapFeatures features ([
      (crates."memchr"."${deps."aho_corasick"."0.7.12"."memchr"}" deps)
    ]);
    features = mkFeatures (features."aho_corasick"."0.7.12" or {});
  };
  features_.aho_corasick."0.7.12" = deps: f: updateFeatures f (rec {
    aho_corasick = fold recursiveUpdate {} [
      { "0.7.12"."std" =
        (f.aho_corasick."0.7.12"."std" or false) ||
        (f.aho_corasick."0.7.12".default or false) ||
        (aho_corasick."0.7.12"."default" or false); }
      { "0.7.12".default = (f.aho_corasick."0.7.12".default or true); }
    ];
    memchr = fold recursiveUpdate {} [
      { "${deps.aho_corasick."0.7.12".memchr}"."use_std" =
        (f.memchr."${deps.aho_corasick."0.7.12".memchr}"."use_std" or false) ||
        (aho_corasick."0.7.12"."std" or false) ||
        (f."aho_corasick"."0.7.12"."std" or false); }
      { "${deps.aho_corasick."0.7.12".memchr}".default = (f.memchr."${deps.aho_corasick."0.7.12".memchr}".default or false); }
    ];
  }) [
    (features_.memchr."${deps."aho_corasick"."0.7.12"."memchr"}" deps)
  ];


# end
# ansi_term-0.11.0

  crates.ansi_term."0.11.0" = deps: { features?(features_.ansi_term."0.11.0" deps {}) }: buildRustCrate {
    crateName = "ansi_term";
    version = "0.11.0";
    description = "Library for ANSI terminal colours and styles (bold, underline)";
    authors = [ "ogham@bsago.me" "Ryan Scheel (Havvy) <ryan.havvy@gmail.com>" "Josh Triplett <josh@joshtriplett.org>" ];
    sha256 = "08fk0p2xvkqpmz3zlrwnf6l8sj2vngw464rvzspzp31sbgxbwm4v";
    dependencies = (if kernel == "windows" then mapFeatures features ([
      (crates."winapi"."${deps."ansi_term"."0.11.0"."winapi"}" deps)
    ]) else []);
  };
  features_.ansi_term."0.11.0" = deps: f: updateFeatures f (rec {
    ansi_term."0.11.0".default = (f.ansi_term."0.11.0".default or true);
    winapi = fold recursiveUpdate {} [
      { "${deps.ansi_term."0.11.0".winapi}"."consoleapi" = true; }
      { "${deps.ansi_term."0.11.0".winapi}"."errhandlingapi" = true; }
      { "${deps.ansi_term."0.11.0".winapi}"."processenv" = true; }
      { "${deps.ansi_term."0.11.0".winapi}".default = true; }
    ];
  }) [
    (features_.winapi."${deps."ansi_term"."0.11.0"."winapi"}" deps)
  ];


# end
# anyhow-1.0.38

  crates.anyhow."1.0.38" = deps: { features?(features_.anyhow."1.0.38" deps {}) }: buildRustCrate {
    crateName = "anyhow";
    version = "1.0.38";
    description = "Flexible concrete Error type built on std::error::Error";
    authors = [ "David Tolnay <dtolnay@gmail.com>" ];
    edition = "2018";
    sha256 = "0q3m3swj42403wq1y5djbi9804i3bk7z42mp2khdp2mxi9agc7rn";
    features = mkFeatures (features."anyhow"."1.0.38" or {});
  };
  features_.anyhow."1.0.38" = deps: f: updateFeatures f (rec {
    anyhow = fold recursiveUpdate {} [
      { "1.0.38"."std" =
        (f.anyhow."1.0.38"."std" or false) ||
        (f.anyhow."1.0.38".default or false) ||
        (anyhow."1.0.38"."default" or false); }
      { "1.0.38".default = (f.anyhow."1.0.38".default or true); }
    ];
  }) [];


# end
# anymap-0.12.1

  crates.anymap."0.12.1" = deps: { features?(features_.anymap."0.12.1" deps {}) }: buildRustCrate {
    crateName = "anymap";
    version = "0.12.1";
    description = "A safe and convenient store for one value of each type";
    authors = [ "Chris Morgan <me@chrismorgan.info>" ];
    sha256 = "08l2xa6ini8nbd4y997cayr0ibh23209il5zn6j516q1pcb1wiwi";
    features = mkFeatures (features."anymap"."0.12.1" or {});
  };
  features_.anymap."0.12.1" = deps: f: updateFeatures f (rec {
    anymap."0.12.1".default = (f.anymap."0.12.1".default or true);
  }) [];


# end
# arc-swap-0.4.7

  crates.arc_swap."0.4.7" = deps: { features?(features_.arc_swap."0.4.7" deps {}) }: buildRustCrate {
    crateName = "arc-swap";
    version = "0.4.7";
    description = "Atomically swappable Arc";
    authors = [ "Michal 'vorner' Vaner <vorner@vorner.cz>" ];
    sha256 = "1x69rg4b6sjzz4hkjbs3wkyqha7l00x044bn87j4l4prmc9dkxh7";
    features = mkFeatures (features."arc_swap"."0.4.7" or {});
  };
  features_.arc_swap."0.4.7" = deps: f: updateFeatures f (rec {
    arc_swap."0.4.7".default = (f.arc_swap."0.4.7".default or true);
  }) [];


# end
# arrayref-0.3.6

  crates.arrayref."0.3.6" = deps: { features?(features_.arrayref."0.3.6" deps {}) }: buildRustCrate {
    crateName = "arrayref";
    version = "0.3.6";
    description = "Macros to take array references of slices";
    authors = [ "David Roundy <roundyd@physics.oregonstate.edu>" ];
    sha256 = "0s5k9qc9rq1yd6idrn79jwp4lhc9mp7dydcqbz492nxnyfpv4044";
  };
  features_.arrayref."0.3.6" = deps: f: updateFeatures f (rec {
    arrayref."0.3.6".default = (f.arrayref."0.3.6".default or true);
  }) [];


# end
# arrayvec-0.5.1

  crates.arrayvec."0.5.1" = deps: { features?(features_.arrayvec."0.5.1" deps {}) }: buildRustCrate {
    crateName = "arrayvec";
    version = "0.5.1";
    description = "A vector with fixed capacity, backed by an array (it can be stored on the stack too). Implements fixed capacity ArrayVec and ArrayString.";
    authors = [ "bluss" ];
    edition = "2018";
    sha256 = "01fc06ab7zh75z26m2l4a0fc7zy4zpr962qazdcp9hl4fgdwbj6v";
    dependencies = mapFeatures features ([
]);
    features = mkFeatures (features."arrayvec"."0.5.1" or {});
  };
  features_.arrayvec."0.5.1" = deps: f: updateFeatures f (rec {
    arrayvec = fold recursiveUpdate {} [
      { "0.5.1"."std" =
        (f.arrayvec."0.5.1"."std" or false) ||
        (f.arrayvec."0.5.1".default or false) ||
        (arrayvec."0.5.1"."default" or false); }
      { "0.5.1".default = (f.arrayvec."0.5.1".default or true); }
    ];
  }) [];


# end
# atomicwrites-0.2.5

  crates.atomicwrites."0.2.5" = deps: { features?(features_.atomicwrites."0.2.5" deps {}) }: buildRustCrate {
    crateName = "atomicwrites";
    version = "0.2.5";
    description = "Atomic file-writes.";
    authors = [ "Markus Unterwaditzer <markus@unterwaditzer.net>" ];
    sha256 = "117276ag68iyfs5c90vfhb10klmwzs7rqx6clvh124wh4717fgvd";
    dependencies = mapFeatures features ([
      (crates."tempdir"."${deps."atomicwrites"."0.2.5"."tempdir"}" deps)
    ])
      ++ (if (kernel == "linux" || kernel == "darwin") then mapFeatures features ([
      (crates."nix"."${deps."atomicwrites"."0.2.5"."nix"}" deps)
    ]) else [])
      ++ (if kernel == "windows" then mapFeatures features ([
      (crates."winapi"."${deps."atomicwrites"."0.2.5"."winapi"}" deps)
    ]) else []);
  };
  features_.atomicwrites."0.2.5" = deps: f: updateFeatures f (rec {
    atomicwrites."0.2.5".default = (f.atomicwrites."0.2.5".default or true);
    nix."${deps.atomicwrites."0.2.5".nix}".default = true;
    tempdir."${deps.atomicwrites."0.2.5".tempdir}".default = true;
    winapi = fold recursiveUpdate {} [
      { "${deps.atomicwrites."0.2.5".winapi}"."winbase" = true; }
      { "${deps.atomicwrites."0.2.5".winapi}".default = true; }
    ];
  }) [
    (features_.tempdir."${deps."atomicwrites"."0.2.5"."tempdir"}" deps)
    (features_.nix."${deps."atomicwrites"."0.2.5"."nix"}" deps)
    (features_.winapi."${deps."atomicwrites"."0.2.5"."winapi"}" deps)
  ];


# end
# atty-0.2.14

  crates.atty."0.2.14" = deps: { features?(features_.atty."0.2.14" deps {}) }: buildRustCrate {
    crateName = "atty";
    version = "0.2.14";
    description = "A simple interface for querying atty";
    authors = [ "softprops <d.tangren@gmail.com>" ];
    sha256 = "18x3dv3clg1qyf0skj16b9zd9679dav2r81in85zdmb5aqd25564";
    dependencies = (if kernel == "hermit" then mapFeatures features ([
      (crates."hermit_abi"."${deps."atty"."0.2.14"."hermit_abi"}" deps)
    ]) else [])
      ++ (if (kernel == "linux" || kernel == "darwin") then mapFeatures features ([
      (crates."libc"."${deps."atty"."0.2.14"."libc"}" deps)
    ]) else [])
      ++ (if kernel == "windows" then mapFeatures features ([
      (crates."winapi"."${deps."atty"."0.2.14"."winapi"}" deps)
    ]) else []);
  };
  features_.atty."0.2.14" = deps: f: updateFeatures f (rec {
    atty."0.2.14".default = (f.atty."0.2.14".default or true);
    hermit_abi."${deps.atty."0.2.14".hermit_abi}".default = true;
    libc."${deps.atty."0.2.14".libc}".default = (f.libc."${deps.atty."0.2.14".libc}".default or false);
    winapi = fold recursiveUpdate {} [
      { "${deps.atty."0.2.14".winapi}"."consoleapi" = true; }
      { "${deps.atty."0.2.14".winapi}"."minwinbase" = true; }
      { "${deps.atty."0.2.14".winapi}"."minwindef" = true; }
      { "${deps.atty."0.2.14".winapi}"."processenv" = true; }
      { "${deps.atty."0.2.14".winapi}"."winbase" = true; }
      { "${deps.atty."0.2.14".winapi}".default = true; }
    ];
  }) [
    (features_.hermit_abi."${deps."atty"."0.2.14"."hermit_abi"}" deps)
    (features_.libc."${deps."atty"."0.2.14"."libc"}" deps)
    (features_.winapi."${deps."atty"."0.2.14"."winapi"}" deps)
  ];


# end
# autocfg-1.0.0

  crates.autocfg."1.0.0" = deps: { features?(features_.autocfg."1.0.0" deps {}) }: buildRustCrate {
    crateName = "autocfg";
    version = "1.0.0";
    description = "Automatic cfg for Rust compiler features";
    authors = [ "Josh Stone <cuviper@gmail.com>" ];
    sha256 = "1hhgqh551gmws22z9rxbnsvlppwxvlj0nszj7n1x56pqa3j3swy7";
  };
  features_.autocfg."1.0.0" = deps: f: updateFeatures f (rec {
    autocfg."1.0.0".default = (f.autocfg."1.0.0".default or true);
  }) [];


# end
# backtrace-0.3.44

  crates.backtrace."0.3.44" = deps: { features?(features_.backtrace."0.3.44" deps {}) }: buildRustCrate {
    crateName = "backtrace";
    version = "0.3.44";
    description = "A library to acquire a stack trace (backtrace) at runtime in a Rust program.\n";
    authors = [ "The Rust Project Developers" ];
    edition = "2018";
    sha256 = "19i5ary8nwk14k0z7gwdwlhs6h3ha9s44942qdy54xi0sbmwqnv0";
    dependencies = mapFeatures features ([
      (crates."cfg_if"."${deps."backtrace"."0.3.44"."cfg_if"}" deps)
      (crates."libc"."${deps."backtrace"."0.3.44"."libc"}" deps)
      (crates."rustc_demangle"."${deps."backtrace"."0.3.44"."rustc_demangle"}" deps)
    ]
      ++ (if features.backtrace."0.3.44".backtrace-sys or false then [ (crates.backtrace_sys."${deps."backtrace"."0.3.44".backtrace_sys}" deps) ] else []))
      ++ (if kernel == "windows" then mapFeatures features ([
]) else []);
    features = mkFeatures (features."backtrace"."0.3.44" or {});
  };
  features_.backtrace."0.3.44" = deps: f: updateFeatures f (rec {
    backtrace = fold recursiveUpdate {} [
      { "0.3.44"."addr2line" =
        (f.backtrace."0.3.44"."addr2line" or false) ||
        (f.backtrace."0.3.44".gimli-symbolize or false) ||
        (backtrace."0.3.44"."gimli-symbolize" or false); }
      { "0.3.44"."backtrace-sys" =
        (f.backtrace."0.3.44"."backtrace-sys" or false) ||
        (f.backtrace."0.3.44".libbacktrace or false) ||
        (backtrace."0.3.44"."libbacktrace" or false); }
      { "0.3.44"."compiler_builtins" =
        (f.backtrace."0.3.44"."compiler_builtins" or false) ||
        (f.backtrace."0.3.44".rustc-dep-of-std or false) ||
        (backtrace."0.3.44"."rustc-dep-of-std" or false); }
      { "0.3.44"."core" =
        (f.backtrace."0.3.44"."core" or false) ||
        (f.backtrace."0.3.44".rustc-dep-of-std or false) ||
        (backtrace."0.3.44"."rustc-dep-of-std" or false); }
      { "0.3.44"."dbghelp" =
        (f.backtrace."0.3.44"."dbghelp" or false) ||
        (f.backtrace."0.3.44".default or false) ||
        (backtrace."0.3.44"."default" or false); }
      { "0.3.44"."dladdr" =
        (f.backtrace."0.3.44"."dladdr" or false) ||
        (f.backtrace."0.3.44".default or false) ||
        (backtrace."0.3.44"."default" or false); }
      { "0.3.44"."findshlibs" =
        (f.backtrace."0.3.44"."findshlibs" or false) ||
        (f.backtrace."0.3.44".gimli-symbolize or false) ||
        (backtrace."0.3.44"."gimli-symbolize" or false); }
      { "0.3.44"."goblin" =
        (f.backtrace."0.3.44"."goblin" or false) ||
        (f.backtrace."0.3.44".gimli-symbolize or false) ||
        (backtrace."0.3.44"."gimli-symbolize" or false); }
      { "0.3.44"."libbacktrace" =
        (f.backtrace."0.3.44"."libbacktrace" or false) ||
        (f.backtrace."0.3.44".default or false) ||
        (backtrace."0.3.44"."default" or false); }
      { "0.3.44"."libunwind" =
        (f.backtrace."0.3.44"."libunwind" or false) ||
        (f.backtrace."0.3.44".default or false) ||
        (backtrace."0.3.44"."default" or false); }
      { "0.3.44"."memmap" =
        (f.backtrace."0.3.44"."memmap" or false) ||
        (f.backtrace."0.3.44".gimli-symbolize or false) ||
        (backtrace."0.3.44"."gimli-symbolize" or false); }
      { "0.3.44"."rustc-serialize" =
        (f.backtrace."0.3.44"."rustc-serialize" or false) ||
        (f.backtrace."0.3.44".serialize-rustc or false) ||
        (backtrace."0.3.44"."serialize-rustc" or false); }
      { "0.3.44"."serde" =
        (f.backtrace."0.3.44"."serde" or false) ||
        (f.backtrace."0.3.44".serialize-serde or false) ||
        (backtrace."0.3.44"."serialize-serde" or false); }
      { "0.3.44"."std" =
        (f.backtrace."0.3.44"."std" or false) ||
        (f.backtrace."0.3.44".default or false) ||
        (backtrace."0.3.44"."default" or false); }
      { "0.3.44".default = (f.backtrace."0.3.44".default or true); }
    ];
    backtrace_sys = fold recursiveUpdate {} [
      { "${deps.backtrace."0.3.44".backtrace_sys}"."rustc-dep-of-std" =
        (f.backtrace_sys."${deps.backtrace."0.3.44".backtrace_sys}"."rustc-dep-of-std" or false) ||
        (backtrace."0.3.44"."rustc-dep-of-std" or false) ||
        (f."backtrace"."0.3.44"."rustc-dep-of-std" or false); }
      { "${deps.backtrace."0.3.44".backtrace_sys}".default = true; }
    ];
    cfg_if = fold recursiveUpdate {} [
      { "${deps.backtrace."0.3.44".cfg_if}"."rustc-dep-of-std" =
        (f.cfg_if."${deps.backtrace."0.3.44".cfg_if}"."rustc-dep-of-std" or false) ||
        (backtrace."0.3.44"."rustc-dep-of-std" or false) ||
        (f."backtrace"."0.3.44"."rustc-dep-of-std" or false); }
      { "${deps.backtrace."0.3.44".cfg_if}".default = true; }
    ];
    libc = fold recursiveUpdate {} [
      { "${deps.backtrace."0.3.44".libc}"."rustc-dep-of-std" =
        (f.libc."${deps.backtrace."0.3.44".libc}"."rustc-dep-of-std" or false) ||
        (backtrace."0.3.44"."rustc-dep-of-std" or false) ||
        (f."backtrace"."0.3.44"."rustc-dep-of-std" or false); }
      { "${deps.backtrace."0.3.44".libc}".default = (f.libc."${deps.backtrace."0.3.44".libc}".default or false); }
    ];
    rustc_demangle = fold recursiveUpdate {} [
      { "${deps.backtrace."0.3.44".rustc_demangle}"."rustc-dep-of-std" =
        (f.rustc_demangle."${deps.backtrace."0.3.44".rustc_demangle}"."rustc-dep-of-std" or false) ||
        (backtrace."0.3.44"."rustc-dep-of-std" or false) ||
        (f."backtrace"."0.3.44"."rustc-dep-of-std" or false); }
      { "${deps.backtrace."0.3.44".rustc_demangle}".default = true; }
    ];
  }) [
    (features_.backtrace_sys."${deps."backtrace"."0.3.44"."backtrace_sys"}" deps)
    (features_.cfg_if."${deps."backtrace"."0.3.44"."cfg_if"}" deps)
    (features_.libc."${deps."backtrace"."0.3.44"."libc"}" deps)
    (features_.rustc_demangle."${deps."backtrace"."0.3.44"."rustc_demangle"}" deps)
  ];


# end
# backtrace-sys-0.1.35

  crates.backtrace_sys."0.1.35" = deps: { features?(features_.backtrace_sys."0.1.35" deps {}) }: buildRustCrate {
    crateName = "backtrace-sys";
    version = "0.1.35";
    description = "Bindings to the libbacktrace gcc library\n";
    authors = [ "Alex Crichton <alex@alexcrichton.com>" ];
    sha256 = "0ipr3zppz25fyblzmb9xfg7g4zrqwnfbq1b10banfghydgg6qccj";
    build = "build.rs";
    dependencies = mapFeatures features ([
      (crates."libc"."${deps."backtrace_sys"."0.1.35"."libc"}" deps)
    ]);

    buildDependencies = mapFeatures features ([
      (crates."cc"."${deps."backtrace_sys"."0.1.35"."cc"}" deps)
    ]);
    features = mkFeatures (features."backtrace_sys"."0.1.35" or {});
  };
  features_.backtrace_sys."0.1.35" = deps: f: updateFeatures f (rec {
    backtrace_sys = fold recursiveUpdate {} [
      { "0.1.35"."backtrace-sys" =
        (f.backtrace_sys."0.1.35"."backtrace-sys" or false) ||
        (f.backtrace_sys."0.1.35".default or false) ||
        (backtrace_sys."0.1.35"."default" or false); }
      { "0.1.35"."compiler_builtins" =
        (f.backtrace_sys."0.1.35"."compiler_builtins" or false) ||
        (f.backtrace_sys."0.1.35".rustc-dep-of-std or false) ||
        (backtrace_sys."0.1.35"."rustc-dep-of-std" or false); }
      { "0.1.35"."core" =
        (f.backtrace_sys."0.1.35"."core" or false) ||
        (f.backtrace_sys."0.1.35".rustc-dep-of-std or false) ||
        (backtrace_sys."0.1.35"."rustc-dep-of-std" or false); }
      { "0.1.35".default = (f.backtrace_sys."0.1.35".default or true); }
    ];
    cc."${deps.backtrace_sys."0.1.35".cc}".default = true;
    libc."${deps.backtrace_sys."0.1.35".libc}".default = (f.libc."${deps.backtrace_sys."0.1.35".libc}".default or false);
  }) [
    (features_.libc."${deps."backtrace_sys"."0.1.35"."libc"}" deps)
    (features_.cc."${deps."backtrace_sys"."0.1.35"."cc"}" deps)
  ];


# end
# base64-0.11.0

  crates.base64."0.11.0" = deps: { features?(features_.base64."0.11.0" deps {}) }: buildRustCrate {
    crateName = "base64";
    version = "0.11.0";
    description = "encodes and decodes base64 as bytes or utf8";
    authors = [ "Alice Maz <alice@alicemaz.com>" "Marshall Pierce <marshall@mpierce.org>" ];
    edition = "2018";
    sha256 = "1idkqdfza39zyhl7ml4i2wq6dv2y331vj2nm9lshvmqbrc23j890";
    features = mkFeatures (features."base64"."0.11.0" or {});
  };
  features_.base64."0.11.0" = deps: f: updateFeatures f (rec {
    base64 = fold recursiveUpdate {} [
      { "0.11.0"."std" =
        (f.base64."0.11.0"."std" or false) ||
        (f.base64."0.11.0".default or false) ||
        (base64."0.11.0"."default" or false); }
      { "0.11.0".default = (f.base64."0.11.0".default or true); }
    ];
  }) [];


# end
# bincode-1.3.2

  crates.bincode."1.3.2" = deps: { features?(features_.bincode."1.3.2" deps {}) }: buildRustCrate {
    crateName = "bincode";
    version = "1.3.2";
    description = "A binary serialization / deserialization strategy that uses Serde for transforming structs into bytes and vice versa!";
    authors = [ "Ty Overby <ty@pre-alpha.com>" "Francesco Mazzoli <f@mazzo.li>" "David Tolnay <dtolnay@gmail.com>" "Zoey Riordan <zoey@dos.cafe>" ];
    sha256 = "0bzwhb2q9svas7xjh85091sbgv6mjs4w0963agmz02xq218fvc2b";
    dependencies = mapFeatures features ([
      (crates."byteorder"."${deps."bincode"."1.3.2"."byteorder"}" deps)
      (crates."serde"."${deps."bincode"."1.3.2"."serde"}" deps)
    ]);
    features = mkFeatures (features."bincode"."1.3.2" or {});
  };
  features_.bincode."1.3.2" = deps: f: updateFeatures f (rec {
    bincode."1.3.2".default = (f.bincode."1.3.2".default or true);
    byteorder."${deps.bincode."1.3.2".byteorder}".default = true;
    serde."${deps.bincode."1.3.2".serde}".default = true;
  }) [
    (features_.byteorder."${deps."bincode"."1.3.2"."byteorder"}" deps)
    (features_.serde."${deps."bincode"."1.3.2"."serde"}" deps)
  ];


# end
# bitflags-1.2.1

  crates.bitflags."1.2.1" = deps: { features?(features_.bitflags."1.2.1" deps {}) }: buildRustCrate {
    crateName = "bitflags";
    version = "1.2.1";
    description = "A macro to generate structures which behave like bitflags.\n";
    authors = [ "The Rust Project Developers" ];
    sha256 = "0b77awhpn7yaqjjibm69ginfn996azx5vkzfjj39g3wbsqs7mkxg";
    build = "build.rs";
    features = mkFeatures (features."bitflags"."1.2.1" or {});
  };
  features_.bitflags."1.2.1" = deps: f: updateFeatures f (rec {
    bitflags."1.2.1".default = (f.bitflags."1.2.1".default or true);
  }) [];


# end
# blake2b_simd-0.5.10

  crates.blake2b_simd."0.5.10" = deps: { features?(features_.blake2b_simd."0.5.10" deps {}) }: buildRustCrate {
    crateName = "blake2b_simd";
    version = "0.5.10";
    description = "a pure Rust BLAKE2b implementation with dynamic SIMD";
    authors = [ "Jack O'Connor" ];
    edition = "2018";
    sha256 = "1yf72mkvjw1gaw62cbijjvk8igvn1bzv9j4zrghg8awhlpygzw0n";
    dependencies = mapFeatures features ([
      (crates."arrayref"."${deps."blake2b_simd"."0.5.10"."arrayref"}" deps)
      (crates."arrayvec"."${deps."blake2b_simd"."0.5.10"."arrayvec"}" deps)
      (crates."constant_time_eq"."${deps."blake2b_simd"."0.5.10"."constant_time_eq"}" deps)
    ]);
    features = mkFeatures (features."blake2b_simd"."0.5.10" or {});
  };
  features_.blake2b_simd."0.5.10" = deps: f: updateFeatures f (rec {
    arrayref."${deps.blake2b_simd."0.5.10".arrayref}".default = true;
    arrayvec."${deps.blake2b_simd."0.5.10".arrayvec}".default = (f.arrayvec."${deps.blake2b_simd."0.5.10".arrayvec}".default or false);
    blake2b_simd = fold recursiveUpdate {} [
      { "0.5.10"."std" =
        (f.blake2b_simd."0.5.10"."std" or false) ||
        (f.blake2b_simd."0.5.10".default or false) ||
        (blake2b_simd."0.5.10"."default" or false); }
      { "0.5.10".default = (f.blake2b_simd."0.5.10".default or true); }
    ];
    constant_time_eq."${deps.blake2b_simd."0.5.10".constant_time_eq}".default = true;
  }) [
    (features_.arrayref."${deps."blake2b_simd"."0.5.10"."arrayref"}" deps)
    (features_.arrayvec."${deps."blake2b_simd"."0.5.10"."arrayvec"}" deps)
    (features_.constant_time_eq."${deps."blake2b_simd"."0.5.10"."constant_time_eq"}" deps)
  ];


# end
# byteorder-1.3.4

  crates.byteorder."1.3.4" = deps: { features?(features_.byteorder."1.3.4" deps {}) }: buildRustCrate {
    crateName = "byteorder";
    version = "1.3.4";
    description = "Library for reading/writing numbers in big-endian and little-endian.";
    authors = [ "Andrew Gallant <jamslam@gmail.com>" ];
    sha256 = "1hi7ixdls5qssw39wgp1gm8d20yjghgawc3m4xr2wkxmxsv08krz";
    build = "build.rs";
    features = mkFeatures (features."byteorder"."1.3.4" or {});
  };
  features_.byteorder."1.3.4" = deps: f: updateFeatures f (rec {
    byteorder = fold recursiveUpdate {} [
      { "1.3.4"."std" =
        (f.byteorder."1.3.4"."std" or false) ||
        (f.byteorder."1.3.4".default or false) ||
        (byteorder."1.3.4"."default" or false); }
      { "1.3.4".default = (f.byteorder."1.3.4".default or true); }
    ];
  }) [];


# end
# cc-1.0.54

  crates.cc."1.0.54" = deps: { features?(features_.cc."1.0.54" deps {}) }: buildRustCrate {
    crateName = "cc";
    version = "1.0.54";
    description = "A build-time dependency for Cargo build scripts to assist in invoking the native\nC compiler to compile native C code into a static archive to be linked into Rust\ncode.\n";
    authors = [ "Alex Crichton <alex@alexcrichton.com>" ];
    edition = "2018";
    sha256 = "159kfkhiinh3pbc4pi724s31zg2d1pr7fjbnvj9a1nx6gl47h8hv";
    dependencies = mapFeatures features ([
]);
    features = mkFeatures (features."cc"."1.0.54" or {});
  };
  features_.cc."1.0.54" = deps: f: updateFeatures f (rec {
    cc = fold recursiveUpdate {} [
      { "1.0.54"."jobserver" =
        (f.cc."1.0.54"."jobserver" or false) ||
        (f.cc."1.0.54".parallel or false) ||
        (cc."1.0.54"."parallel" or false); }
      { "1.0.54".default = (f.cc."1.0.54".default or true); }
    ];
  }) [];


# end
# cfg-if-0.1.10

  crates.cfg_if."0.1.10" = deps: { features?(features_.cfg_if."0.1.10" deps {}) }: buildRustCrate {
    crateName = "cfg-if";
    version = "0.1.10";
    description = "A macro to ergonomically define an item depending on a large number of #[cfg]\nparameters. Structured like an if-else chain, the first matching branch is the\nitem that gets emitted.\n";
    authors = [ "Alex Crichton <alex@alexcrichton.com>" ];
    edition = "2018";
    sha256 = "0x52qzpbyl2f2jqs7kkqzgfki2cpq99gpfjjigdp8pwwfqk01007";
    dependencies = mapFeatures features ([
]);
    features = mkFeatures (features."cfg_if"."0.1.10" or {});
  };
  features_.cfg_if."0.1.10" = deps: f: updateFeatures f (rec {
    cfg_if = fold recursiveUpdate {} [
      { "0.1.10"."compiler_builtins" =
        (f.cfg_if."0.1.10"."compiler_builtins" or false) ||
        (f.cfg_if."0.1.10".rustc-dep-of-std or false) ||
        (cfg_if."0.1.10"."rustc-dep-of-std" or false); }
      { "0.1.10"."core" =
        (f.cfg_if."0.1.10"."core" or false) ||
        (f.cfg_if."0.1.10".rustc-dep-of-std or false) ||
        (cfg_if."0.1.10"."rustc-dep-of-std" or false); }
      { "0.1.10".default = (f.cfg_if."0.1.10".default or true); }
    ];
  }) [];


# end
# cfg-if-1.0.0

  crates.cfg_if."1.0.0" = deps: { features?(features_.cfg_if."1.0.0" deps {}) }: buildRustCrate {
    crateName = "cfg-if";
    version = "1.0.0";
    description = "A macro to ergonomically define an item depending on a large number of #[cfg]\nparameters. Structured like an if-else chain, the first matching branch is the\nitem that gets emitted.\n";
    authors = [ "Alex Crichton <alex@alexcrichton.com>" ];
    edition = "2018";
    sha256 = "1fzidq152hnxhg4lj6r2gv4jpnn8yivp27z6q6xy7w6v0dp6bai9";
    dependencies = mapFeatures features ([
]);
    features = mkFeatures (features."cfg_if"."1.0.0" or {});
  };
  features_.cfg_if."1.0.0" = deps: f: updateFeatures f (rec {
    cfg_if = fold recursiveUpdate {} [
      { "1.0.0"."compiler_builtins" =
        (f.cfg_if."1.0.0"."compiler_builtins" or false) ||
        (f.cfg_if."1.0.0".rustc-dep-of-std or false) ||
        (cfg_if."1.0.0"."rustc-dep-of-std" or false); }
      { "1.0.0"."core" =
        (f.cfg_if."1.0.0"."core" or false) ||
        (f.cfg_if."1.0.0".rustc-dep-of-std or false) ||
        (cfg_if."1.0.0"."rustc-dep-of-std" or false); }
      { "1.0.0".default = (f.cfg_if."1.0.0".default or true); }
    ];
  }) [];


# end
# chashmap-2.2.2

  crates.chashmap."2.2.2" = deps: { features?(features_.chashmap."2.2.2" deps {}) }: buildRustCrate {
    crateName = "chashmap";
    version = "2.2.2";
    description = "Fast, concurrent hash maps with extensive API.";
    authors = [ "ticki <Ticki@users.noreply.github.com>" ];
    sha256 = "1kf09hv0i8nmqg36fmv255fv7l8grfk8xxk15h7gd5djg4kxjp0x";
    dependencies = mapFeatures features ([
      (crates."owning_ref"."${deps."chashmap"."2.2.2"."owning_ref"}" deps)
      (crates."parking_lot"."${deps."chashmap"."2.2.2"."parking_lot"}" deps)
    ]);
  };
  features_.chashmap."2.2.2" = deps: f: updateFeatures f (rec {
    chashmap."2.2.2".default = (f.chashmap."2.2.2".default or true);
    owning_ref."${deps.chashmap."2.2.2".owning_ref}".default = true;
    parking_lot."${deps.chashmap."2.2.2".parking_lot}".default = true;
  }) [
    (features_.owning_ref."${deps."chashmap"."2.2.2"."owning_ref"}" deps)
    (features_.parking_lot."${deps."chashmap"."2.2.2"."parking_lot"}" deps)
  ];


# end
# chrono-0.4.11

  crates.chrono."0.4.11" = deps: { features?(features_.chrono."0.4.11" deps {}) }: buildRustCrate {
    crateName = "chrono";
    version = "0.4.11";
    description = "Date and time library for Rust";
    authors = [ "Kang Seonghoon <public+rust@mearie.org>" "Brandon W Maister <quodlibetor@gmail.com>" ];
    sha256 = "0d4nsrphnqnggxlz5rblfr0q60qga4ylqggi44rh0kgziajzxrmp";
    dependencies = mapFeatures features ([
      (crates."num_integer"."${deps."chrono"."0.4.11"."num_integer"}" deps)
      (crates."num_traits"."${deps."chrono"."0.4.11"."num_traits"}" deps)
    ]
      ++ (if features.chrono."0.4.11".time or false then [ (crates.time."${deps."chrono"."0.4.11".time}" deps) ] else []))
      ++ (if cpu == "wasm32" && !(kernel == "emscripten" || kernel == "wasi") then mapFeatures features ([
]) else []);
    features = mkFeatures (features."chrono"."0.4.11" or {});
  };
  features_.chrono."0.4.11" = deps: f: updateFeatures f (rec {
    chrono = fold recursiveUpdate {} [
      { "0.4.11"."clock" =
        (f.chrono."0.4.11"."clock" or false) ||
        (f.chrono."0.4.11".default or false) ||
        (chrono."0.4.11"."default" or false); }
      { "0.4.11"."js-sys" =
        (f.chrono."0.4.11"."js-sys" or false) ||
        (f.chrono."0.4.11".wasmbind or false) ||
        (chrono."0.4.11"."wasmbind" or false); }
      { "0.4.11"."std" =
        (f.chrono."0.4.11"."std" or false) ||
        (f.chrono."0.4.11".clock or false) ||
        (chrono."0.4.11"."clock" or false) ||
        (f.chrono."0.4.11".default or false) ||
        (chrono."0.4.11"."default" or false); }
      { "0.4.11"."time" =
        (f.chrono."0.4.11"."time" or false) ||
        (f.chrono."0.4.11".clock or false) ||
        (chrono."0.4.11"."clock" or false); }
      { "0.4.11"."wasm-bindgen" =
        (f.chrono."0.4.11"."wasm-bindgen" or false) ||
        (f.chrono."0.4.11".wasmbind or false) ||
        (chrono."0.4.11"."wasmbind" or false); }
      { "0.4.11".default = (f.chrono."0.4.11".default or true); }
    ];
    num_integer."${deps.chrono."0.4.11".num_integer}".default = (f.num_integer."${deps.chrono."0.4.11".num_integer}".default or false);
    num_traits."${deps.chrono."0.4.11".num_traits}".default = (f.num_traits."${deps.chrono."0.4.11".num_traits}".default or false);
    time."${deps.chrono."0.4.11".time}".default = true;
  }) [
    (features_.num_integer."${deps."chrono"."0.4.11"."num_integer"}" deps)
    (features_.num_traits."${deps."chrono"."0.4.11"."num_traits"}" deps)
    (features_.time."${deps."chrono"."0.4.11"."time"}" deps)
  ];


# end
# clap-2.33.1

  crates.clap."2.33.1" = deps: { features?(features_.clap."2.33.1" deps {}) }: buildRustCrate {
    crateName = "clap";
    version = "2.33.1";
    description = "A simple to use, efficient, and full-featured Command Line Argument Parser\n";
    authors = [ "Kevin K. <kbknapp@gmail.com>" ];
    sha256 = "1ikmxr59nfrzghiywkp6qiy2dqw0jp2ljrmssvjniwg54lvcqdny";
    dependencies = mapFeatures features ([
      (crates."bitflags"."${deps."clap"."2.33.1"."bitflags"}" deps)
      (crates."textwrap"."${deps."clap"."2.33.1"."textwrap"}" deps)
      (crates."unicode_width"."${deps."clap"."2.33.1"."unicode_width"}" deps)
    ]
      ++ (if features.clap."2.33.1".atty or false then [ (crates.atty."${deps."clap"."2.33.1".atty}" deps) ] else [])
      ++ (if features.clap."2.33.1".strsim or false then [ (crates.strsim."${deps."clap"."2.33.1".strsim}" deps) ] else []))
      ++ (if !(kernel == "windows") then mapFeatures features ([
    ]
      ++ (if features.clap."2.33.1".ansi_term or false then [ (crates.ansi_term."${deps."clap"."2.33.1".ansi_term}" deps) ] else [])) else []);
    features = mkFeatures (features."clap"."2.33.1" or {});
  };
  features_.clap."2.33.1" = deps: f: updateFeatures f (rec {
    ansi_term."${deps.clap."2.33.1".ansi_term}".default = true;
    atty."${deps.clap."2.33.1".atty}".default = true;
    bitflags."${deps.clap."2.33.1".bitflags}".default = true;
    clap = fold recursiveUpdate {} [
      { "2.33.1"."ansi_term" =
        (f.clap."2.33.1"."ansi_term" or false) ||
        (f.clap."2.33.1".color or false) ||
        (clap."2.33.1"."color" or false); }
      { "2.33.1"."atty" =
        (f.clap."2.33.1"."atty" or false) ||
        (f.clap."2.33.1".color or false) ||
        (clap."2.33.1"."color" or false); }
      { "2.33.1"."clippy" =
        (f.clap."2.33.1"."clippy" or false) ||
        (f.clap."2.33.1".lints or false) ||
        (clap."2.33.1"."lints" or false); }
      { "2.33.1"."color" =
        (f.clap."2.33.1"."color" or false) ||
        (f.clap."2.33.1".default or false) ||
        (clap."2.33.1"."default" or false); }
      { "2.33.1"."strsim" =
        (f.clap."2.33.1"."strsim" or false) ||
        (f.clap."2.33.1".suggestions or false) ||
        (clap."2.33.1"."suggestions" or false); }
      { "2.33.1"."suggestions" =
        (f.clap."2.33.1"."suggestions" or false) ||
        (f.clap."2.33.1".default or false) ||
        (clap."2.33.1"."default" or false); }
      { "2.33.1"."term_size" =
        (f.clap."2.33.1"."term_size" or false) ||
        (f.clap."2.33.1".wrap_help or false) ||
        (clap."2.33.1"."wrap_help" or false); }
      { "2.33.1"."vec_map" =
        (f.clap."2.33.1"."vec_map" or false) ||
        (f.clap."2.33.1".default or false) ||
        (clap."2.33.1"."default" or false); }
      { "2.33.1"."yaml" =
        (f.clap."2.33.1"."yaml" or false) ||
        (f.clap."2.33.1".doc or false) ||
        (clap."2.33.1"."doc" or false); }
      { "2.33.1"."yaml-rust" =
        (f.clap."2.33.1"."yaml-rust" or false) ||
        (f.clap."2.33.1".yaml or false) ||
        (clap."2.33.1"."yaml" or false); }
      { "2.33.1".default = (f.clap."2.33.1".default or true); }
    ];
    strsim."${deps.clap."2.33.1".strsim}".default = true;
    textwrap = fold recursiveUpdate {} [
      { "${deps.clap."2.33.1".textwrap}"."term_size" =
        (f.textwrap."${deps.clap."2.33.1".textwrap}"."term_size" or false) ||
        (clap."2.33.1"."wrap_help" or false) ||
        (f."clap"."2.33.1"."wrap_help" or false); }
      { "${deps.clap."2.33.1".textwrap}".default = true; }
    ];
    unicode_width."${deps.clap."2.33.1".unicode_width}".default = true;
  }) [
    (features_.atty."${deps."clap"."2.33.1"."atty"}" deps)
    (features_.bitflags."${deps."clap"."2.33.1"."bitflags"}" deps)
    (features_.strsim."${deps."clap"."2.33.1"."strsim"}" deps)
    (features_.textwrap."${deps."clap"."2.33.1"."textwrap"}" deps)
    (features_.unicode_width."${deps."clap"."2.33.1"."unicode_width"}" deps)
    (features_.ansi_term."${deps."clap"."2.33.1"."ansi_term"}" deps)
  ];


# end
# constant_time_eq-0.1.5

  crates.constant_time_eq."0.1.5" = deps: { features?(features_.constant_time_eq."0.1.5" deps {}) }: buildRustCrate {
    crateName = "constant_time_eq";
    version = "0.1.5";
    description = "Compares two equal-sized byte strings in constant time.";
    authors = [ "Cesar Eduardo Barros <cesarb@cesarb.eti.br>" ];
    sha256 = "1dvk7vvfdbvg3k2r7m4n5scj82vv519cmm8695jqqnkh4wm670fv";
  };
  features_.constant_time_eq."0.1.5" = deps: f: updateFeatures f (rec {
    constant_time_eq."0.1.5".default = (f.constant_time_eq."0.1.5".default or true);
  }) [];


# end
# crossbeam-channel-0.3.9

  crates.crossbeam_channel."0.3.9" = deps: { features?(features_.crossbeam_channel."0.3.9" deps {}) }: buildRustCrate {
    crateName = "crossbeam-channel";
    version = "0.3.9";
    description = "Multi-producer multi-consumer channels for message passing";
    authors = [ "The Crossbeam Project Developers" ];
    sha256 = "0si8kg061qgadx56dfyil2jq0ffckg6sk3mf2vl8ha8fwi9kd34h";
    dependencies = mapFeatures features ([
      (crates."crossbeam_utils"."${deps."crossbeam_channel"."0.3.9"."crossbeam_utils"}" deps)
    ]);
  };
  features_.crossbeam_channel."0.3.9" = deps: f: updateFeatures f (rec {
    crossbeam_channel."0.3.9".default = (f.crossbeam_channel."0.3.9".default or true);
    crossbeam_utils."${deps.crossbeam_channel."0.3.9".crossbeam_utils}".default = true;
  }) [
    (features_.crossbeam_utils."${deps."crossbeam_channel"."0.3.9"."crossbeam_utils"}" deps)
  ];


# end
# crossbeam-utils-0.6.6

  crates.crossbeam_utils."0.6.6" = deps: { features?(features_.crossbeam_utils."0.6.6" deps {}) }: buildRustCrate {
    crateName = "crossbeam-utils";
    version = "0.6.6";
    description = "Utilities for concurrent programming";
    authors = [ "The Crossbeam Project Developers" ];
    sha256 = "01gxccmrjkkcavdh8fc01kj3b5fmk10f0lkx66jmnv69kcssry72";
    dependencies = mapFeatures features ([
      (crates."cfg_if"."${deps."crossbeam_utils"."0.6.6"."cfg_if"}" deps)
    ]
      ++ (if features.crossbeam_utils."0.6.6".lazy_static or false then [ (crates.lazy_static."${deps."crossbeam_utils"."0.6.6".lazy_static}" deps) ] else []));
    features = mkFeatures (features."crossbeam_utils"."0.6.6" or {});
  };
  features_.crossbeam_utils."0.6.6" = deps: f: updateFeatures f (rec {
    cfg_if."${deps.crossbeam_utils."0.6.6".cfg_if}".default = true;
    crossbeam_utils = fold recursiveUpdate {} [
      { "0.6.6"."lazy_static" =
        (f.crossbeam_utils."0.6.6"."lazy_static" or false) ||
        (f.crossbeam_utils."0.6.6".std or false) ||
        (crossbeam_utils."0.6.6"."std" or false); }
      { "0.6.6"."std" =
        (f.crossbeam_utils."0.6.6"."std" or false) ||
        (f.crossbeam_utils."0.6.6".default or false) ||
        (crossbeam_utils."0.6.6"."default" or false); }
      { "0.6.6".default = (f.crossbeam_utils."0.6.6".default or true); }
    ];
    lazy_static."${deps.crossbeam_utils."0.6.6".lazy_static}".default = true;
  }) [
    (features_.cfg_if."${deps."crossbeam_utils"."0.6.6"."cfg_if"}" deps)
    (features_.lazy_static."${deps."crossbeam_utils"."0.6.6"."lazy_static"}" deps)
  ];


# end
# crossbeam-utils-0.7.2

  crates.crossbeam_utils."0.7.2" = deps: { features?(features_.crossbeam_utils."0.7.2" deps {}) }: buildRustCrate {
    crateName = "crossbeam-utils";
    version = "0.7.2";
    description = "Utilities for concurrent programming";
    authors = [ "The Crossbeam Project Developers" ];
    sha256 = "17n0299c5y4d9pv4zr72shlx6klc0kl3mqmdgrvh70yg4bjr3837";
    dependencies = mapFeatures features ([
      (crates."cfg_if"."${deps."crossbeam_utils"."0.7.2"."cfg_if"}" deps)
    ]
      ++ (if features.crossbeam_utils."0.7.2".lazy_static or false then [ (crates.lazy_static."${deps."crossbeam_utils"."0.7.2".lazy_static}" deps) ] else []));

    buildDependencies = mapFeatures features ([
      (crates."autocfg"."${deps."crossbeam_utils"."0.7.2"."autocfg"}" deps)
    ]);
    features = mkFeatures (features."crossbeam_utils"."0.7.2" or {});
  };
  features_.crossbeam_utils."0.7.2" = deps: f: updateFeatures f (rec {
    autocfg."${deps.crossbeam_utils."0.7.2".autocfg}".default = true;
    cfg_if."${deps.crossbeam_utils."0.7.2".cfg_if}".default = true;
    crossbeam_utils = fold recursiveUpdate {} [
      { "0.7.2"."lazy_static" =
        (f.crossbeam_utils."0.7.2"."lazy_static" or false) ||
        (f.crossbeam_utils."0.7.2".std or false) ||
        (crossbeam_utils."0.7.2"."std" or false); }
      { "0.7.2"."std" =
        (f.crossbeam_utils."0.7.2"."std" or false) ||
        (f.crossbeam_utils."0.7.2".default or false) ||
        (crossbeam_utils."0.7.2"."default" or false); }
      { "0.7.2".default = (f.crossbeam_utils."0.7.2".default or true); }
    ];
    lazy_static."${deps.crossbeam_utils."0.7.2".lazy_static}".default = true;
  }) [
    (features_.cfg_if."${deps."crossbeam_utils"."0.7.2"."cfg_if"}" deps)
    (features_.lazy_static."${deps."crossbeam_utils"."0.7.2"."lazy_static"}" deps)
    (features_.autocfg."${deps."crossbeam_utils"."0.7.2"."autocfg"}" deps)
  ];


# end
# ctrlc-3.1.8

  crates.ctrlc."3.1.8" = deps: { features?(features_.ctrlc."3.1.8" deps {}) }: buildRustCrate {
    crateName = "ctrlc";
    version = "3.1.8";
    description = "Easy Ctrl-C handler for Rust projects";
    authors = [ "Antti Keränen <detegr@gmail.com>" ];
    edition = "2018";
    sha256 = "16gc3zgifh2myhda04b2lkc3ydmg1jwbgsxgb8vxz6i8yp1v4575";
    dependencies = (if (kernel == "linux" || kernel == "darwin") then mapFeatures features ([
      (crates."nix"."${deps."ctrlc"."3.1.8"."nix"}" deps)
    ]) else [])
      ++ (if kernel == "windows" then mapFeatures features ([
      (crates."winapi"."${deps."ctrlc"."3.1.8"."winapi"}" deps)
    ]) else []);
    features = mkFeatures (features."ctrlc"."3.1.8" or {});
  };
  features_.ctrlc."3.1.8" = deps: f: updateFeatures f (rec {
    ctrlc."3.1.8".default = (f.ctrlc."3.1.8".default or true);
    nix."${deps.ctrlc."3.1.8".nix}".default = true;
    winapi = fold recursiveUpdate {} [
      { "${deps.ctrlc."3.1.8".winapi}"."consoleapi" = true; }
      { "${deps.ctrlc."3.1.8".winapi}"."handleapi" = true; }
      { "${deps.ctrlc."3.1.8".winapi}"."synchapi" = true; }
      { "${deps.ctrlc."3.1.8".winapi}"."winbase" = true; }
      { "${deps.ctrlc."3.1.8".winapi}".default = true; }
    ];
  }) [
    (features_.nix."${deps."ctrlc"."3.1.8"."nix"}" deps)
    (features_.winapi."${deps."ctrlc"."3.1.8"."winapi"}" deps)
  ];


# end
# directories-3.0.1

  crates.directories."3.0.1" = deps: { features?(features_.directories."3.0.1" deps {}) }: buildRustCrate {
    crateName = "directories";
    version = "3.0.1";
    description = "A tiny mid-level library that provides platform-specific standard locations of directories for config, cache and other data on Linux, Windows and macOS by leveraging the mechanisms defined by the XDG base/user directory specifications on Linux, the Known Folder API on Windows, and the Standard Directory guidelines on macOS.";
    authors = [ "Simon Ochsenreither <simon@ochsenreither.de>" ];
    sha256 = "16gvghamz34a6nmp95ygycg9nax35l3r6mwavgd1lzxwlkhwdph4";
    dependencies = mapFeatures features ([
      (crates."dirs_sys"."${deps."directories"."3.0.1"."dirs_sys"}" deps)
    ]);
  };
  features_.directories."3.0.1" = deps: f: updateFeatures f (rec {
    directories."3.0.1".default = (f.directories."3.0.1".default or true);
    dirs_sys."${deps.directories."3.0.1".dirs_sys}".default = true;
  }) [
    (features_.dirs_sys."${deps."directories"."3.0.1"."dirs_sys"}" deps)
  ];


# end
# dirs-2.0.2

  crates.dirs."2.0.2" = deps: { features?(features_.dirs."2.0.2" deps {}) }: buildRustCrate {
    crateName = "dirs";
    version = "2.0.2";
    description = "A tiny low-level library that provides platform-specific standard locations of directories for config, cache and other data on Linux, Windows, macOS and Redox by leveraging the mechanisms defined by the XDG base/user directory specifications on Linux, the Known Folder API on Windows, and the Standard Directory guidelines on macOS.";
    authors = [ "Simon Ochsenreither <simon@ochsenreither.de>" ];
    sha256 = "0zk0kdnl2hd3qk76yq6yk7hc7s73gpnnzi1p208ygrh270y96fpx";
    dependencies = mapFeatures features ([
      (crates."cfg_if"."${deps."dirs"."2.0.2"."cfg_if"}" deps)
      (crates."dirs_sys"."${deps."dirs"."2.0.2"."dirs_sys"}" deps)
    ]);
  };
  features_.dirs."2.0.2" = deps: f: updateFeatures f (rec {
    cfg_if."${deps.dirs."2.0.2".cfg_if}".default = true;
    dirs."2.0.2".default = (f.dirs."2.0.2".default or true);
    dirs_sys."${deps.dirs."2.0.2".dirs_sys}".default = true;
  }) [
    (features_.cfg_if."${deps."dirs"."2.0.2"."cfg_if"}" deps)
    (features_.dirs_sys."${deps."dirs"."2.0.2"."dirs_sys"}" deps)
  ];


# end
# dirs-sys-0.3.5

  crates.dirs_sys."0.3.5" = deps: { features?(features_.dirs_sys."0.3.5" deps {}) }: buildRustCrate {
    crateName = "dirs-sys";
    version = "0.3.5";
    description = "System-level helper functions for the dirs and directories crates.";
    authors = [ "Simon Ochsenreither <simon@ochsenreither.de>" ];
    sha256 = "1qig65rnd8ygn4gwr49ddp4y4a58vmwn12ix1q1yib0w0ly58xss";
    dependencies = (if kernel == "redox" then mapFeatures features ([
      (crates."redox_users"."${deps."dirs_sys"."0.3.5"."redox_users"}" deps)
    ]) else [])
      ++ (if (kernel == "linux" || kernel == "darwin") then mapFeatures features ([
      (crates."libc"."${deps."dirs_sys"."0.3.5"."libc"}" deps)
    ]) else [])
      ++ (if kernel == "windows" then mapFeatures features ([
      (crates."winapi"."${deps."dirs_sys"."0.3.5"."winapi"}" deps)
    ]) else []);
  };
  features_.dirs_sys."0.3.5" = deps: f: updateFeatures f (rec {
    dirs_sys."0.3.5".default = (f.dirs_sys."0.3.5".default or true);
    libc."${deps.dirs_sys."0.3.5".libc}".default = true;
    redox_users."${deps.dirs_sys."0.3.5".redox_users}".default = true;
    winapi = fold recursiveUpdate {} [
      { "${deps.dirs_sys."0.3.5".winapi}"."knownfolders" = true; }
      { "${deps.dirs_sys."0.3.5".winapi}"."objbase" = true; }
      { "${deps.dirs_sys."0.3.5".winapi}"."shlobj" = true; }
      { "${deps.dirs_sys."0.3.5".winapi}"."winbase" = true; }
      { "${deps.dirs_sys."0.3.5".winapi}"."winerror" = true; }
      { "${deps.dirs_sys."0.3.5".winapi}".default = true; }
    ];
  }) [
    (features_.redox_users."${deps."dirs_sys"."0.3.5"."redox_users"}" deps)
    (features_.libc."${deps."dirs_sys"."0.3.5"."libc"}" deps)
    (features_.winapi."${deps."dirs_sys"."0.3.5"."winapi"}" deps)
  ];


# end
# fastrand-1.4.0

  crates.fastrand."1.4.0" = deps: { features?(features_.fastrand."1.4.0" deps {}) }: buildRustCrate {
    crateName = "fastrand";
    version = "1.4.0";
    description = "A simple and fast random number generator";
    authors = [ "Stjepan Glavina <stjepang@gmail.com>" ];
    edition = "2018";
    sha256 = "1gfmzp2nzfjka51h2ygdbw4fziiiqd7lg8spi4dvjlnsa3dakpiq";
    dependencies = (if cpu == "wasm32" then mapFeatures features ([
      (crates."instant"."${deps."fastrand"."1.4.0"."instant"}" deps)
    ]) else []);
  };
  features_.fastrand."1.4.0" = deps: f: updateFeatures f (rec {
    fastrand."1.4.0".default = (f.fastrand."1.4.0".default or true);
    instant."${deps.fastrand."1.4.0".instant}".default = true;
  }) [
    (features_.instant."${deps."fastrand"."1.4.0"."instant"}" deps)
  ];


# end
# filetime-0.2.10

  crates.filetime."0.2.10" = deps: { features?(features_.filetime."0.2.10" deps {}) }: buildRustCrate {
    crateName = "filetime";
    version = "0.2.10";
    description = "Platform-agnostic accessors of timestamps in File metadata\n";
    authors = [ "Alex Crichton <alex@alexcrichton.com>" ];
    edition = "2018";
    sha256 = "1sq7amygdsknj9qb9i990xqpn1m96p2zwapfq4wv7spx0rndrr9g";
    dependencies = mapFeatures features ([
      (crates."cfg_if"."${deps."filetime"."0.2.10"."cfg_if"}" deps)
    ])
      ++ (if kernel == "redox" then mapFeatures features ([
      (crates."redox_syscall"."${deps."filetime"."0.2.10"."redox_syscall"}" deps)
    ]) else [])
      ++ (if (kernel == "linux" || kernel == "darwin") then mapFeatures features ([
      (crates."libc"."${deps."filetime"."0.2.10"."libc"}" deps)
    ]) else [])
      ++ (if kernel == "windows" then mapFeatures features ([
      (crates."winapi"."${deps."filetime"."0.2.10"."winapi"}" deps)
    ]) else []);
  };
  features_.filetime."0.2.10" = deps: f: updateFeatures f (rec {
    cfg_if."${deps.filetime."0.2.10".cfg_if}".default = true;
    filetime."0.2.10".default = (f.filetime."0.2.10".default or true);
    libc."${deps.filetime."0.2.10".libc}".default = true;
    redox_syscall."${deps.filetime."0.2.10".redox_syscall}".default = true;
    winapi = fold recursiveUpdate {} [
      { "${deps.filetime."0.2.10".winapi}"."fileapi" = true; }
      { "${deps.filetime."0.2.10".winapi}"."minwindef" = true; }
      { "${deps.filetime."0.2.10".winapi}"."winbase" = true; }
      { "${deps.filetime."0.2.10".winapi}".default = true; }
    ];
  }) [
    (features_.cfg_if."${deps."filetime"."0.2.10"."cfg_if"}" deps)
    (features_.redox_syscall."${deps."filetime"."0.2.10"."redox_syscall"}" deps)
    (features_.libc."${deps."filetime"."0.2.10"."libc"}" deps)
    (features_.winapi."${deps."filetime"."0.2.10"."winapi"}" deps)
  ];


# end
# fsevent-0.4.0

  crates.fsevent."0.4.0" = deps: { features?(features_.fsevent."0.4.0" deps {}) }: buildRustCrate {
    crateName = "fsevent";
    version = "0.4.0";
    description = "Rust bindings to the fsevent-sys macOS API for file changes notifications";
    authors = [ "Pierre Baillet <pierre@baillet.name>" ];
    sha256 = "19wynmx2k8gmsxv6fa9kpjzb9v5k6qc2ykziw25bray645spg77v";
    dependencies = mapFeatures features ([
      (crates."bitflags"."${deps."fsevent"."0.4.0"."bitflags"}" deps)
      (crates."fsevent_sys"."${deps."fsevent"."0.4.0"."fsevent_sys"}" deps)
    ]);
  };
  features_.fsevent."0.4.0" = deps: f: updateFeatures f (rec {
    bitflags."${deps.fsevent."0.4.0".bitflags}".default = true;
    fsevent."0.4.0".default = (f.fsevent."0.4.0".default or true);
    fsevent_sys."${deps.fsevent."0.4.0".fsevent_sys}".default = true;
  }) [
    (features_.bitflags."${deps."fsevent"."0.4.0"."bitflags"}" deps)
    (features_.fsevent_sys."${deps."fsevent"."0.4.0"."fsevent_sys"}" deps)
  ];


# end
# fsevent-sys-2.0.1

  crates.fsevent_sys."2.0.1" = deps: { features?(features_.fsevent_sys."2.0.1" deps {}) }: buildRustCrate {
    crateName = "fsevent-sys";
    version = "2.0.1";
    description = "Rust bindings to the fsevent macOS API for file changes notifications";
    authors = [ "Pierre Baillet <pierre@baillet.name>" ];
    sha256 = "1jlnqp6iw4mmwd2f973j33k00mbfc1cv9wpdvxq1jk3bry558gbr";
    dependencies = mapFeatures features ([
      (crates."libc"."${deps."fsevent_sys"."2.0.1"."libc"}" deps)
    ]);
  };
  features_.fsevent_sys."2.0.1" = deps: f: updateFeatures f (rec {
    fsevent_sys."2.0.1".default = (f.fsevent_sys."2.0.1".default or true);
    libc."${deps.fsevent_sys."2.0.1".libc}".default = true;
  }) [
    (features_.libc."${deps."fsevent_sys"."2.0.1"."libc"}" deps)
  ];


# end
# fuchsia-cprng-0.1.1

  crates.fuchsia_cprng."0.1.1" = deps: { features?(features_.fuchsia_cprng."0.1.1" deps {}) }: buildRustCrate {
    crateName = "fuchsia-cprng";
    version = "0.1.1";
    description = "Rust crate for the Fuchsia cryptographically secure pseudorandom number generator";
    authors = [ "Erick Tryzelaar <etryzelaar@google.com>" ];
    edition = "2018";
    sha256 = "07apwv9dj716yjlcj29p94vkqn5zmfh7hlrqvrjx3wzshphc95h9";
  };
  features_.fuchsia_cprng."0.1.1" = deps: f: updateFeatures f (rec {
    fuchsia_cprng."0.1.1".default = (f.fuchsia_cprng."0.1.1".default or true);
  }) [];


# end
# fuchsia-zircon-0.3.3

  crates.fuchsia_zircon."0.3.3" = deps: { features?(features_.fuchsia_zircon."0.3.3" deps {}) }: buildRustCrate {
    crateName = "fuchsia-zircon";
    version = "0.3.3";
    description = "Rust bindings for the Zircon kernel";
    authors = [ "Raph Levien <raph@google.com>" ];
    sha256 = "0jrf4shb1699r4la8z358vri8318w4mdi6qzfqy30p2ymjlca4gk";
    dependencies = mapFeatures features ([
      (crates."bitflags"."${deps."fuchsia_zircon"."0.3.3"."bitflags"}" deps)
      (crates."fuchsia_zircon_sys"."${deps."fuchsia_zircon"."0.3.3"."fuchsia_zircon_sys"}" deps)
    ]);
  };
  features_.fuchsia_zircon."0.3.3" = deps: f: updateFeatures f (rec {
    bitflags."${deps.fuchsia_zircon."0.3.3".bitflags}".default = true;
    fuchsia_zircon."0.3.3".default = (f.fuchsia_zircon."0.3.3".default or true);
    fuchsia_zircon_sys."${deps.fuchsia_zircon."0.3.3".fuchsia_zircon_sys}".default = true;
  }) [
    (features_.bitflags."${deps."fuchsia_zircon"."0.3.3"."bitflags"}" deps)
    (features_.fuchsia_zircon_sys."${deps."fuchsia_zircon"."0.3.3"."fuchsia_zircon_sys"}" deps)
  ];


# end
# fuchsia-zircon-sys-0.3.3

  crates.fuchsia_zircon_sys."0.3.3" = deps: { features?(features_.fuchsia_zircon_sys."0.3.3" deps {}) }: buildRustCrate {
    crateName = "fuchsia-zircon-sys";
    version = "0.3.3";
    description = "Low-level Rust bindings for the Zircon kernel";
    authors = [ "Raph Levien <raph@google.com>" ];
    sha256 = "08jp1zxrm9jbrr6l26bjal4dbm8bxfy57ickdgibsqxr1n9j3hf5";
  };
  features_.fuchsia_zircon_sys."0.3.3" = deps: f: updateFeatures f (rec {
    fuchsia_zircon_sys."0.3.3".default = (f.fuchsia_zircon_sys."0.3.3".default or true);
  }) [];


# end
# getrandom-0.1.14

  crates.getrandom."0.1.14" = deps: { features?(features_.getrandom."0.1.14" deps {}) }: buildRustCrate {
    crateName = "getrandom";
    version = "0.1.14";
    description = "A small cross-platform library for retrieving random data from system source";
    authors = [ "The Rand Project Developers" ];
    edition = "2018";
    sha256 = "1i6r4q7i24zdy6v5h3l966a1cf8a1aip2fi1pmdsi71sk1m3w7wr";
    dependencies = mapFeatures features ([
      (crates."cfg_if"."${deps."getrandom"."0.1.14"."cfg_if"}" deps)
    ])
      ++ (if kernel == "wasi" then mapFeatures features ([
      (crates."wasi"."${deps."getrandom"."0.1.14"."wasi"}" deps)
    ]) else [])
      ++ (if (kernel == "linux" || kernel == "darwin") then mapFeatures features ([
      (crates."libc"."${deps."getrandom"."0.1.14"."libc"}" deps)
    ]) else [])
      ++ (if kernel == "wasm32-unknown-unknown" then mapFeatures features ([
]) else []);
    features = mkFeatures (features."getrandom"."0.1.14" or {});
  };
  features_.getrandom."0.1.14" = deps: f: updateFeatures f (rec {
    cfg_if."${deps.getrandom."0.1.14".cfg_if}".default = true;
    getrandom = fold recursiveUpdate {} [
      { "0.1.14"."compiler_builtins" =
        (f.getrandom."0.1.14"."compiler_builtins" or false) ||
        (f.getrandom."0.1.14".rustc-dep-of-std or false) ||
        (getrandom."0.1.14"."rustc-dep-of-std" or false); }
      { "0.1.14"."core" =
        (f.getrandom."0.1.14"."core" or false) ||
        (f.getrandom."0.1.14".rustc-dep-of-std or false) ||
        (getrandom."0.1.14"."rustc-dep-of-std" or false); }
      { "0.1.14"."wasm-bindgen" =
        (f.getrandom."0.1.14"."wasm-bindgen" or false) ||
        (f.getrandom."0.1.14".test-in-browser or false) ||
        (getrandom."0.1.14"."test-in-browser" or false); }
      { "0.1.14".default = (f.getrandom."0.1.14".default or true); }
    ];
    libc."${deps.getrandom."0.1.14".libc}".default = (f.libc."${deps.getrandom."0.1.14".libc}".default or false);
    wasi."${deps.getrandom."0.1.14".wasi}".default = true;
  }) [
    (features_.cfg_if."${deps."getrandom"."0.1.14"."cfg_if"}" deps)
    (features_.wasi."${deps."getrandom"."0.1.14"."wasi"}" deps)
    (features_.libc."${deps."getrandom"."0.1.14"."libc"}" deps)
  ];


# end
# heck-0.3.1

  crates.heck."0.3.1" = deps: { features?(features_.heck."0.3.1" deps {}) }: buildRustCrate {
    crateName = "heck";
    version = "0.3.1";
    description = "heck is a case conversion library.";
    authors = [ "Without Boats <woboats@gmail.com>" ];
    sha256 = "1q7vmnlh62kls6cvkfhbcacxkawaznaqa5wwm9dg1xkcza846c3d";
    dependencies = mapFeatures features ([
      (crates."unicode_segmentation"."${deps."heck"."0.3.1"."unicode_segmentation"}" deps)
    ]);
  };
  features_.heck."0.3.1" = deps: f: updateFeatures f (rec {
    heck."0.3.1".default = (f.heck."0.3.1".default or true);
    unicode_segmentation."${deps.heck."0.3.1".unicode_segmentation}".default = true;
  }) [
    (features_.unicode_segmentation."${deps."heck"."0.3.1"."unicode_segmentation"}" deps)
  ];


# end
# hermit-abi-0.1.14

  crates.hermit_abi."0.1.14" = deps: { features?(features_.hermit_abi."0.1.14" deps {}) }: buildRustCrate {
    crateName = "hermit-abi";
    version = "0.1.14";
    description = "hermit-abi is small interface to call functions from the unikernel RustyHermit.\nIt is used to build the target `x86_64-unknown-hermit`.\n";
    authors = [ "Stefan Lankes" ];
    edition = "2018";
    sha256 = "0daamdm4shifwf3sbagagwrkq157r3xxrg8nkfspabnjnj79n6rf";
    dependencies = mapFeatures features ([
      (crates."libc"."${deps."hermit_abi"."0.1.14"."libc"}" deps)
    ]);
    features = mkFeatures (features."hermit_abi"."0.1.14" or {});
  };
  features_.hermit_abi."0.1.14" = deps: f: updateFeatures f (rec {
    hermit_abi = fold recursiveUpdate {} [
      { "0.1.14"."core" =
        (f.hermit_abi."0.1.14"."core" or false) ||
        (f.hermit_abi."0.1.14".rustc-dep-of-std or false) ||
        (hermit_abi."0.1.14"."rustc-dep-of-std" or false); }
      { "0.1.14".default = (f.hermit_abi."0.1.14".default or true); }
    ];
    libc = fold recursiveUpdate {} [
      { "${deps.hermit_abi."0.1.14".libc}"."rustc-dep-of-std" =
        (f.libc."${deps.hermit_abi."0.1.14".libc}"."rustc-dep-of-std" or false) ||
        (hermit_abi."0.1.14"."rustc-dep-of-std" or false) ||
        (f."hermit_abi"."0.1.14"."rustc-dep-of-std" or false); }
      { "${deps.hermit_abi."0.1.14".libc}".default = (f.libc."${deps.hermit_abi."0.1.14".libc}".default or false); }
    ];
  }) [
    (features_.libc."${deps."hermit_abi"."0.1.14"."libc"}" deps)
  ];


# end
# inotify-0.7.1

  crates.inotify."0.7.1" = deps: { features?(features_.inotify."0.7.1" deps {}) }: buildRustCrate {
    crateName = "inotify";
    version = "0.7.1";
    description = "Idiomatic wrapper for inotify";
    authors = [ "Hanno Braun <mail@hannobraun.de>" "Félix Saparelli <me@passcod.name>" "Cristian Kubis <cristian.kubis@tsunix.de>" "Frank Denis <github@pureftpd.org>" ];
    sha256 = "1rl3df4viy2smw84ba60c013m1mnmakjmj3mzk956qgzranmkzgf";
    dependencies = mapFeatures features ([
      (crates."bitflags"."${deps."inotify"."0.7.1"."bitflags"}" deps)
      (crates."inotify_sys"."${deps."inotify"."0.7.1"."inotify_sys"}" deps)
      (crates."libc"."${deps."inotify"."0.7.1"."libc"}" deps)
    ]);
    features = mkFeatures (features."inotify"."0.7.1" or {});
  };
  features_.inotify."0.7.1" = deps: f: updateFeatures f (rec {
    bitflags."${deps.inotify."0.7.1".bitflags}".default = true;
    inotify = fold recursiveUpdate {} [
      { "0.7.1"."futures" =
        (f.inotify."0.7.1"."futures" or false) ||
        (f.inotify."0.7.1".stream or false) ||
        (inotify."0.7.1"."stream" or false); }
      { "0.7.1"."mio" =
        (f.inotify."0.7.1"."mio" or false) ||
        (f.inotify."0.7.1".stream or false) ||
        (inotify."0.7.1"."stream" or false); }
      { "0.7.1"."stream" =
        (f.inotify."0.7.1"."stream" or false) ||
        (f.inotify."0.7.1".default or false) ||
        (inotify."0.7.1"."default" or false); }
      { "0.7.1"."tokio" =
        (f.inotify."0.7.1"."tokio" or false) ||
        (f.inotify."0.7.1".stream or false) ||
        (inotify."0.7.1"."stream" or false); }
      { "0.7.1"."tokio-io" =
        (f.inotify."0.7.1"."tokio-io" or false) ||
        (f.inotify."0.7.1".stream or false) ||
        (inotify."0.7.1"."stream" or false); }
      { "0.7.1"."tokio-reactor" =
        (f.inotify."0.7.1"."tokio-reactor" or false) ||
        (f.inotify."0.7.1".stream or false) ||
        (inotify."0.7.1"."stream" or false); }
      { "0.7.1".default = (f.inotify."0.7.1".default or true); }
    ];
    inotify_sys."${deps.inotify."0.7.1".inotify_sys}".default = true;
    libc."${deps.inotify."0.7.1".libc}".default = true;
  }) [
    (features_.bitflags."${deps."inotify"."0.7.1"."bitflags"}" deps)
    (features_.inotify_sys."${deps."inotify"."0.7.1"."inotify_sys"}" deps)
    (features_.libc."${deps."inotify"."0.7.1"."libc"}" deps)
  ];


# end
# inotify-sys-0.1.3

  crates.inotify_sys."0.1.3" = deps: { features?(features_.inotify_sys."0.1.3" deps {}) }: buildRustCrate {
    crateName = "inotify-sys";
    version = "0.1.3";
    description = "inotify bindings for the Rust programming language";
    authors = [ "Hanno Braun <hb@hannobraun.de>" ];
    sha256 = "110bbc9vprrj3cmp5g5v1adfh3wlnlbxqllwfksrlcdv1k3dnv8n";
    dependencies = mapFeatures features ([
      (crates."libc"."${deps."inotify_sys"."0.1.3"."libc"}" deps)
    ]);
  };
  features_.inotify_sys."0.1.3" = deps: f: updateFeatures f (rec {
    inotify_sys."0.1.3".default = (f.inotify_sys."0.1.3".default or true);
    libc."${deps.inotify_sys."0.1.3".libc}".default = true;
  }) [
    (features_.libc."${deps."inotify_sys"."0.1.3"."libc"}" deps)
  ];


# end
# instant-0.1.9

  crates.instant."0.1.9" = deps: { features?(features_.instant."0.1.9" deps {}) }: buildRustCrate {
    crateName = "instant";
    version = "0.1.9";
    description = "A partial replacement for std::time::Instant that works on WASM too.";
    authors = [ "sebcrozet <developer@crozet.re>" ];
    edition = "2018";
    sha256 = "1j60nmsaixd9r28rylc9wzsj6f282azynhmp0ykamgxzn9v8zbvg";
    dependencies = mapFeatures features ([
      (crates."cfg_if"."${deps."instant"."0.1.9"."cfg_if"}" deps)
    ])
      ++ (if kernel == "asmjs-unknown-emscripten" then mapFeatures features ([
]) else [])
      ++ (if !(true || true) then mapFeatures features ([
]) else [])
      ++ (if kernel == "wasm32-unknown-emscripten" then mapFeatures features ([
]) else [])
      ++ (if kernel == "wasm32-unknown-unknown" then mapFeatures features ([
]) else []);
    features = mkFeatures (features."instant"."0.1.9" or {});
  };
  features_.instant."0.1.9" = deps: f: updateFeatures f (rec {
    cfg_if."${deps.instant."0.1.9".cfg_if}".default = true;
    instant = fold recursiveUpdate {} [
      { "0.1.9"."js-sys" =
        (f.instant."0.1.9"."js-sys" or false) ||
        (f.instant."0.1.9".wasm-bindgen or false) ||
        (instant."0.1.9"."wasm-bindgen" or false); }
      { "0.1.9"."time" =
        (f.instant."0.1.9"."time" or false) ||
        (f.instant."0.1.9".now or false) ||
        (instant."0.1.9"."now" or false); }
      { "0.1.9"."wasm-bindgen_rs" =
        (f.instant."0.1.9"."wasm-bindgen_rs" or false) ||
        (f.instant."0.1.9".wasm-bindgen or false) ||
        (instant."0.1.9"."wasm-bindgen" or false); }
      { "0.1.9"."web-sys" =
        (f.instant."0.1.9"."web-sys" or false) ||
        (f.instant."0.1.9".wasm-bindgen or false) ||
        (instant."0.1.9"."wasm-bindgen" or false); }
      { "0.1.9".default = (f.instant."0.1.9".default or true); }
    ];
  }) [
    (features_.cfg_if."${deps."instant"."0.1.9"."cfg_if"}" deps)
  ];


# end
# iovec-0.1.4

  crates.iovec."0.1.4" = deps: { features?(features_.iovec."0.1.4" deps {}) }: buildRustCrate {
    crateName = "iovec";
    version = "0.1.4";
    description = "Portable buffer type for scatter/gather I/O operations\n";
    authors = [ "Carl Lerche <me@carllerche.com>" ];
    sha256 = "1wy7rsm8rx6y4rjy98jws1aqxdy0v5wbz9whz73p45cwpsg4prfa";
    dependencies = (if (kernel == "linux" || kernel == "darwin") then mapFeatures features ([
      (crates."libc"."${deps."iovec"."0.1.4"."libc"}" deps)
    ]) else []);
  };
  features_.iovec."0.1.4" = deps: f: updateFeatures f (rec {
    iovec."0.1.4".default = (f.iovec."0.1.4".default or true);
    libc."${deps.iovec."0.1.4".libc}".default = true;
  }) [
    (features_.libc."${deps."iovec"."0.1.4"."libc"}" deps)
  ];


# end
# itoa-0.4.6

  crates.itoa."0.4.6" = deps: { features?(features_.itoa."0.4.6" deps {}) }: buildRustCrate {
    crateName = "itoa";
    version = "0.4.6";
    description = "Fast functions for printing integer primitives to an io::Write";
    authors = [ "David Tolnay <dtolnay@gmail.com>" ];
    sha256 = "1pl959kjafa0riia6s3mvxw8jv4af66620z1rggjiqj7bcx9b5kk";
    features = mkFeatures (features."itoa"."0.4.6" or {});
  };
  features_.itoa."0.4.6" = deps: f: updateFeatures f (rec {
    itoa = fold recursiveUpdate {} [
      { "0.4.6"."std" =
        (f.itoa."0.4.6"."std" or false) ||
        (f.itoa."0.4.6".default or false) ||
        (itoa."0.4.6"."default" or false); }
      { "0.4.6".default = (f.itoa."0.4.6".default or true); }
    ];
  }) [];


# end
# kernel32-sys-0.2.2

  crates.kernel32_sys."0.2.2" = deps: { features?(features_.kernel32_sys."0.2.2" deps {}) }: buildRustCrate {
    crateName = "kernel32-sys";
    version = "0.2.2";
    description = "Contains function definitions for the Windows API library kernel32. See winapi for types and constants.";
    authors = [ "Peter Atashian <retep998@gmail.com>" ];
    sha256 = "1lrw1hbinyvr6cp28g60z97w32w8vsk6pahk64pmrv2fmby8srfj";
    libName = "kernel32";
    build = "build.rs";
    dependencies = mapFeatures features ([
      (crates."winapi"."${deps."kernel32_sys"."0.2.2"."winapi"}" deps)
    ]);

    buildDependencies = mapFeatures features ([
      (crates."winapi_build"."${deps."kernel32_sys"."0.2.2"."winapi_build"}" deps)
    ]);
  };
  features_.kernel32_sys."0.2.2" = deps: f: updateFeatures f (rec {
    kernel32_sys."0.2.2".default = (f.kernel32_sys."0.2.2".default or true);
    winapi."${deps.kernel32_sys."0.2.2".winapi}".default = true;
    winapi_build."${deps.kernel32_sys."0.2.2".winapi_build}".default = true;
  }) [
    (features_.winapi."${deps."kernel32_sys"."0.2.2"."winapi"}" deps)
    (features_.winapi_build."${deps."kernel32_sys"."0.2.2"."winapi_build"}" deps)
  ];


# end
# lazy_static-1.4.0

  crates.lazy_static."1.4.0" = deps: { features?(features_.lazy_static."1.4.0" deps {}) }: buildRustCrate {
    crateName = "lazy_static";
    version = "1.4.0";
    description = "A macro for declaring lazily evaluated statics in Rust.";
    authors = [ "Marvin Löbel <loebel.marvin@gmail.com>" ];
    sha256 = "13h6sdghdcy7vcqsm2gasfw3qg7ssa0fl3sw7lq6pdkbk52wbyfr";
    dependencies = mapFeatures features ([
]);
    features = mkFeatures (features."lazy_static"."1.4.0" or {});
  };
  features_.lazy_static."1.4.0" = deps: f: updateFeatures f (rec {
    lazy_static = fold recursiveUpdate {} [
      { "1.4.0"."spin" =
        (f.lazy_static."1.4.0"."spin" or false) ||
        (f.lazy_static."1.4.0".spin_no_std or false) ||
        (lazy_static."1.4.0"."spin_no_std" or false); }
      { "1.4.0".default = (f.lazy_static."1.4.0".default or true); }
    ];
  }) [];


# end
# lazycell-1.2.1

  crates.lazycell."1.2.1" = deps: { features?(features_.lazycell."1.2.1" deps {}) }: buildRustCrate {
    crateName = "lazycell";
    version = "1.2.1";
    description = "A library providing a lazily filled Cell struct";
    authors = [ "Alex Crichton <alex@alexcrichton.com>" "Nikita Pekin <contact@nikitapek.in>" ];
    sha256 = "1m4h2q9rgxrgc7xjnws1x81lrb68jll8w3pykx1a9bhr29q2mcwm";
    dependencies = mapFeatures features ([
]);
    features = mkFeatures (features."lazycell"."1.2.1" or {});
  };
  features_.lazycell."1.2.1" = deps: f: updateFeatures f (rec {
    lazycell = fold recursiveUpdate {} [
      { "1.2.1"."clippy" =
        (f.lazycell."1.2.1"."clippy" or false) ||
        (f.lazycell."1.2.1".nightly-testing or false) ||
        (lazycell."1.2.1"."nightly-testing" or false); }
      { "1.2.1"."nightly" =
        (f.lazycell."1.2.1"."nightly" or false) ||
        (f.lazycell."1.2.1".nightly-testing or false) ||
        (lazycell."1.2.1"."nightly-testing" or false); }
      { "1.2.1".default = (f.lazycell."1.2.1".default or true); }
    ];
  }) [];


# end
# libc-0.2.86

  crates.libc."0.2.86" = deps: { features?(features_.libc."0.2.86" deps {}) }: buildRustCrate {
    crateName = "libc";
    version = "0.2.86";
    description = "Raw FFI bindings to platform libraries like libc.\n";
    authors = [ "The Rust Project Developers" ];
    sha256 = "07w3fdl40mdizrcm8v3ysy9ar7x0rkp1wyihl4vvayanyhhzcqyk";
    build = "build.rs";
    dependencies = mapFeatures features ([
]);
    features = mkFeatures (features."libc"."0.2.86" or {});
  };
  features_.libc."0.2.86" = deps: f: updateFeatures f (rec {
    libc = fold recursiveUpdate {} [
      { "0.2.86"."align" =
        (f.libc."0.2.86"."align" or false) ||
        (f.libc."0.2.86".rustc-dep-of-std or false) ||
        (libc."0.2.86"."rustc-dep-of-std" or false); }
      { "0.2.86"."rustc-std-workspace-core" =
        (f.libc."0.2.86"."rustc-std-workspace-core" or false) ||
        (f.libc."0.2.86".rustc-dep-of-std or false) ||
        (libc."0.2.86"."rustc-dep-of-std" or false); }
      { "0.2.86"."std" =
        (f.libc."0.2.86"."std" or false) ||
        (f.libc."0.2.86".default or false) ||
        (libc."0.2.86"."default" or false) ||
        (f.libc."0.2.86".use_std or false) ||
        (libc."0.2.86"."use_std" or false); }
      { "0.2.86".default = (f.libc."0.2.86".default or true); }
    ];
  }) [];


# end
# log-0.4.8

  crates.log."0.4.8" = deps: { features?(features_.log."0.4.8" deps {}) }: buildRustCrate {
    crateName = "log";
    version = "0.4.8";
    description = "A lightweight logging facade for Rust\n";
    authors = [ "The Rust Project Developers" ];
    sha256 = "0wvzzzcn89dai172rrqcyz06pzldyyy0lf0w71csmn206rdpnb15";
    build = "build.rs";
    dependencies = mapFeatures features ([
      (crates."cfg_if"."${deps."log"."0.4.8"."cfg_if"}" deps)
    ]);
    features = mkFeatures (features."log"."0.4.8" or {});
  };
  features_.log."0.4.8" = deps: f: updateFeatures f (rec {
    cfg_if."${deps.log."0.4.8".cfg_if}".default = true;
    log = fold recursiveUpdate {} [
      { "0.4.8"."kv_unstable" =
        (f.log."0.4.8"."kv_unstable" or false) ||
        (f.log."0.4.8".kv_unstable_sval or false) ||
        (log."0.4.8"."kv_unstable_sval" or false); }
      { "0.4.8".default = (f.log."0.4.8".default or true); }
    ];
  }) [
    (features_.cfg_if."${deps."log"."0.4.8"."cfg_if"}" deps)
  ];


# end
# maybe-uninit-2.0.0

  crates.maybe_uninit."2.0.0" = deps: { features?(features_.maybe_uninit."2.0.0" deps {}) }: buildRustCrate {
    crateName = "maybe-uninit";
    version = "2.0.0";
    description = "MaybeUninit for friends of backwards compatibility";
    authors = [ "est31 <MTest31@outlook.com>" "The Rust Project Developers" ];
    sha256 = "0crrwlngxjswhcnw8dvsccx8qnm2cbp4fvq6xhz3akmznvnv77gk";
  };
  features_.maybe_uninit."2.0.0" = deps: f: updateFeatures f (rec {
    maybe_uninit."2.0.0".default = (f.maybe_uninit."2.0.0".default or true);
  }) [];


# end
# md5-0.7.0

  crates.md5."0.7.0" = deps: { features?(features_.md5."0.7.0" deps {}) }: buildRustCrate {
    crateName = "md5";
    version = "0.7.0";
    description = "The package provides the MD5 hash function.";
    authors = [ "Ivan Ukhov <ivan.ukhov@gmail.com>" "Kamal Ahmad <shibe@openmailbox.org>" "Konstantin Stepanov <milezv@gmail.com>" "Lukas Kalbertodt <lukas.kalbertodt@gmail.com>" "Nathan Musoke <nathan.musoke@gmail.com>" "Scott Mabin <scott@mabez.dev>" "Tony Arcieri <bascule@gmail.com>" "Wim de With <register@dewith.io>" "Yosef Dinerstein <yosefdi@gmail.com>" ];
    sha256 = "0wm8p4xr40sgl0mdvqvs7w42s5v9jpaianmadkszsclmkmy8a5zc";
    features = mkFeatures (features."md5"."0.7.0" or {});
  };
  features_.md5."0.7.0" = deps: f: updateFeatures f (rec {
    md5 = fold recursiveUpdate {} [
      { "0.7.0"."std" =
        (f.md5."0.7.0"."std" or false) ||
        (f.md5."0.7.0".default or false) ||
        (md5."0.7.0"."default" or false); }
      { "0.7.0".default = (f.md5."0.7.0".default or true); }
    ];
  }) [];


# end
# memchr-2.3.3

  crates.memchr."2.3.3" = deps: { features?(features_.memchr."2.3.3" deps {}) }: buildRustCrate {
    crateName = "memchr";
    version = "2.3.3";
    description = "Safe interface to memchr.";
    authors = [ "Andrew Gallant <jamslam@gmail.com>" "bluss" ];
    sha256 = "1ivxvlswglk6wd46gadkbbsknr94gwryk6y21v64ja7x4icrpihw";
    dependencies = mapFeatures features ([
]);
    features = mkFeatures (features."memchr"."2.3.3" or {});
  };
  features_.memchr."2.3.3" = deps: f: updateFeatures f (rec {
    memchr = fold recursiveUpdate {} [
      { "2.3.3"."std" =
        (f.memchr."2.3.3"."std" or false) ||
        (f.memchr."2.3.3".default or false) ||
        (memchr."2.3.3"."default" or false) ||
        (f.memchr."2.3.3".use_std or false) ||
        (memchr."2.3.3"."use_std" or false); }
      { "2.3.3".default = (f.memchr."2.3.3".default or true); }
    ];
  }) [];


# end
# mio-0.6.22

  crates.mio."0.6.22" = deps: { features?(features_.mio."0.6.22" deps {}) }: buildRustCrate {
    crateName = "mio";
    version = "0.6.22";
    description = "Lightweight non-blocking IO";
    authors = [ "Carl Lerche <me@carllerche.com>" ];
    sha256 = "1lf8mwxq5lblz3496zfh5qqmnsl7hrjzycqhkjhpsn3mlmg6ms9m";
    dependencies = mapFeatures features ([
      (crates."cfg_if"."${deps."mio"."0.6.22"."cfg_if"}" deps)
      (crates."iovec"."${deps."mio"."0.6.22"."iovec"}" deps)
      (crates."log"."${deps."mio"."0.6.22"."log"}" deps)
      (crates."net2"."${deps."mio"."0.6.22"."net2"}" deps)
      (crates."slab"."${deps."mio"."0.6.22"."slab"}" deps)
    ])
      ++ (if kernel == "fuchsia" then mapFeatures features ([
      (crates."fuchsia_zircon"."${deps."mio"."0.6.22"."fuchsia_zircon"}" deps)
      (crates."fuchsia_zircon_sys"."${deps."mio"."0.6.22"."fuchsia_zircon_sys"}" deps)
    ]) else [])
      ++ (if (kernel == "linux" || kernel == "darwin") then mapFeatures features ([
      (crates."libc"."${deps."mio"."0.6.22"."libc"}" deps)
    ]) else [])
      ++ (if kernel == "windows" then mapFeatures features ([
      (crates."kernel32_sys"."${deps."mio"."0.6.22"."kernel32_sys"}" deps)
      (crates."miow"."${deps."mio"."0.6.22"."miow"}" deps)
      (crates."winapi"."${deps."mio"."0.6.22"."winapi"}" deps)
    ]) else []);
    features = mkFeatures (features."mio"."0.6.22" or {});
  };
  features_.mio."0.6.22" = deps: f: updateFeatures f (rec {
    cfg_if."${deps.mio."0.6.22".cfg_if}".default = true;
    fuchsia_zircon."${deps.mio."0.6.22".fuchsia_zircon}".default = true;
    fuchsia_zircon_sys."${deps.mio."0.6.22".fuchsia_zircon_sys}".default = true;
    iovec."${deps.mio."0.6.22".iovec}".default = true;
    kernel32_sys."${deps.mio."0.6.22".kernel32_sys}".default = true;
    libc."${deps.mio."0.6.22".libc}".default = true;
    log."${deps.mio."0.6.22".log}".default = true;
    mio = fold recursiveUpdate {} [
      { "0.6.22"."with-deprecated" =
        (f.mio."0.6.22"."with-deprecated" or false) ||
        (f.mio."0.6.22".default or false) ||
        (mio."0.6.22"."default" or false); }
      { "0.6.22".default = (f.mio."0.6.22".default or true); }
    ];
    miow."${deps.mio."0.6.22".miow}".default = true;
    net2."${deps.mio."0.6.22".net2}".default = true;
    slab."${deps.mio."0.6.22".slab}".default = true;
    winapi."${deps.mio."0.6.22".winapi}".default = true;
  }) [
    (features_.cfg_if."${deps."mio"."0.6.22"."cfg_if"}" deps)
    (features_.iovec."${deps."mio"."0.6.22"."iovec"}" deps)
    (features_.log."${deps."mio"."0.6.22"."log"}" deps)
    (features_.net2."${deps."mio"."0.6.22"."net2"}" deps)
    (features_.slab."${deps."mio"."0.6.22"."slab"}" deps)
    (features_.fuchsia_zircon."${deps."mio"."0.6.22"."fuchsia_zircon"}" deps)
    (features_.fuchsia_zircon_sys."${deps."mio"."0.6.22"."fuchsia_zircon_sys"}" deps)
    (features_.libc."${deps."mio"."0.6.22"."libc"}" deps)
    (features_.kernel32_sys."${deps."mio"."0.6.22"."kernel32_sys"}" deps)
    (features_.miow."${deps."mio"."0.6.22"."miow"}" deps)
    (features_.winapi."${deps."mio"."0.6.22"."winapi"}" deps)
  ];


# end
# mio-extras-2.0.6

  crates.mio_extras."2.0.6" = deps: { features?(features_.mio_extras."2.0.6" deps {}) }: buildRustCrate {
    crateName = "mio-extras";
    version = "2.0.6";
    description = "Extra components for use with Mio";
    authors = [ "Carl Lerche <me@carllerche.com>" "David Hotham" ];
    edition = "2018";
    sha256 = "04nypmgcbj5wm11sw4j6300piip6c4ihw8qy07dyii97fc3dlim0";
    dependencies = mapFeatures features ([
      (crates."lazycell"."${deps."mio_extras"."2.0.6"."lazycell"}" deps)
      (crates."log"."${deps."mio_extras"."2.0.6"."log"}" deps)
      (crates."mio"."${deps."mio_extras"."2.0.6"."mio"}" deps)
      (crates."slab"."${deps."mio_extras"."2.0.6"."slab"}" deps)
    ]);
  };
  features_.mio_extras."2.0.6" = deps: f: updateFeatures f (rec {
    lazycell."${deps.mio_extras."2.0.6".lazycell}".default = true;
    log."${deps.mio_extras."2.0.6".log}".default = true;
    mio."${deps.mio_extras."2.0.6".mio}".default = true;
    mio_extras."2.0.6".default = (f.mio_extras."2.0.6".default or true);
    slab."${deps.mio_extras."2.0.6".slab}".default = true;
  }) [
    (features_.lazycell."${deps."mio_extras"."2.0.6"."lazycell"}" deps)
    (features_.log."${deps."mio_extras"."2.0.6"."log"}" deps)
    (features_.mio."${deps."mio_extras"."2.0.6"."mio"}" deps)
    (features_.slab."${deps."mio_extras"."2.0.6"."slab"}" deps)
  ];


# end
# miow-0.2.1

  crates.miow."0.2.1" = deps: { features?(features_.miow."0.2.1" deps {}) }: buildRustCrate {
    crateName = "miow";
    version = "0.2.1";
    description = "A zero overhead I/O library for Windows, focusing on IOCP and Async I/O\nabstractions.\n";
    authors = [ "Alex Crichton <alex@alexcrichton.com>" ];
    sha256 = "14f8zkc6ix7mkyis1vsqnim8m29b6l55abkba3p2yz7j1ibcvrl0";
    dependencies = mapFeatures features ([
      (crates."kernel32_sys"."${deps."miow"."0.2.1"."kernel32_sys"}" deps)
      (crates."net2"."${deps."miow"."0.2.1"."net2"}" deps)
      (crates."winapi"."${deps."miow"."0.2.1"."winapi"}" deps)
      (crates."ws2_32_sys"."${deps."miow"."0.2.1"."ws2_32_sys"}" deps)
    ]);
  };
  features_.miow."0.2.1" = deps: f: updateFeatures f (rec {
    kernel32_sys."${deps.miow."0.2.1".kernel32_sys}".default = true;
    miow."0.2.1".default = (f.miow."0.2.1".default or true);
    net2."${deps.miow."0.2.1".net2}".default = (f.net2."${deps.miow."0.2.1".net2}".default or false);
    winapi."${deps.miow."0.2.1".winapi}".default = true;
    ws2_32_sys."${deps.miow."0.2.1".ws2_32_sys}".default = true;
  }) [
    (features_.kernel32_sys."${deps."miow"."0.2.1"."kernel32_sys"}" deps)
    (features_.net2."${deps."miow"."0.2.1"."net2"}" deps)
    (features_.winapi."${deps."miow"."0.2.1"."winapi"}" deps)
    (features_.ws2_32_sys."${deps."miow"."0.2.1"."ws2_32_sys"}" deps)
  ];


# end
# net2-0.2.34

  crates.net2."0.2.34" = deps: { features?(features_.net2."0.2.34" deps {}) }: buildRustCrate {
    crateName = "net2";
    version = "0.2.34";
    description = "Extensions to the standard library's networking types as proposed in RFC 1158.\n";
    authors = [ "Alex Crichton <alex@alexcrichton.com>" ];
    sha256 = "1rmlndwwj31hy7zgr6maqh8dsp830zwpzwjkp409jhi0xc888d5d";
    dependencies = mapFeatures features ([
      (crates."cfg_if"."${deps."net2"."0.2.34"."cfg_if"}" deps)
    ])
      ++ (if kernel == "redox" || (kernel == "linux" || kernel == "darwin") || kernel == "wasi" then mapFeatures features ([
      (crates."libc"."${deps."net2"."0.2.34"."libc"}" deps)
    ]) else [])
      ++ (if kernel == "windows" then mapFeatures features ([
      (crates."winapi"."${deps."net2"."0.2.34"."winapi"}" deps)
    ]) else []);
    features = mkFeatures (features."net2"."0.2.34" or {});
  };
  features_.net2."0.2.34" = deps: f: updateFeatures f (rec {
    cfg_if."${deps.net2."0.2.34".cfg_if}".default = true;
    libc."${deps.net2."0.2.34".libc}".default = true;
    net2 = fold recursiveUpdate {} [
      { "0.2.34"."duration" =
        (f.net2."0.2.34"."duration" or false) ||
        (f.net2."0.2.34".default or false) ||
        (net2."0.2.34"."default" or false); }
      { "0.2.34".default = (f.net2."0.2.34".default or true); }
    ];
    winapi = fold recursiveUpdate {} [
      { "${deps.net2."0.2.34".winapi}"."handleapi" = true; }
      { "${deps.net2."0.2.34".winapi}"."winsock2" = true; }
      { "${deps.net2."0.2.34".winapi}"."ws2def" = true; }
      { "${deps.net2."0.2.34".winapi}"."ws2ipdef" = true; }
      { "${deps.net2."0.2.34".winapi}"."ws2tcpip" = true; }
      { "${deps.net2."0.2.34".winapi}".default = true; }
    ];
  }) [
    (features_.cfg_if."${deps."net2"."0.2.34"."cfg_if"}" deps)
    (features_.libc."${deps."net2"."0.2.34"."libc"}" deps)
    (features_.winapi."${deps."net2"."0.2.34"."winapi"}" deps)
  ];


# end
# nix-0.14.1

  crates.nix."0.14.1" = deps: { features?(features_.nix."0.14.1" deps {}) }: buildRustCrate {
    crateName = "nix";
    version = "0.14.1";
    description = "Rust friendly bindings to *nix APIs";
    authors = [ "The nix-rust Project Developers" ];
    sha256 = "1hikdrihw975fcf3m2nmqjd7a00gxdzsbwjzlnjf6bgamr7ygipz";
    dependencies = mapFeatures features ([
      (crates."bitflags"."${deps."nix"."0.14.1"."bitflags"}" deps)
      (crates."cfg_if"."${deps."nix"."0.14.1"."cfg_if"}" deps)
      (crates."libc"."${deps."nix"."0.14.1"."libc"}" deps)
      (crates."void"."${deps."nix"."0.14.1"."void"}" deps)
    ])
      ++ (if kernel == "android" || kernel == "linux" then mapFeatures features ([
]) else [])
      ++ (if kernel == "dragonfly" then mapFeatures features ([
]) else [])
      ++ (if kernel == "freebsd" then mapFeatures features ([
]) else []);
  };
  features_.nix."0.14.1" = deps: f: updateFeatures f (rec {
    bitflags."${deps.nix."0.14.1".bitflags}".default = true;
    cfg_if."${deps.nix."0.14.1".cfg_if}".default = true;
    libc."${deps.nix."0.14.1".libc}".default = true;
    nix."0.14.1".default = (f.nix."0.14.1".default or true);
    void."${deps.nix."0.14.1".void}".default = true;
  }) [
    (features_.bitflags."${deps."nix"."0.14.1"."bitflags"}" deps)
    (features_.cfg_if."${deps."nix"."0.14.1"."cfg_if"}" deps)
    (features_.libc."${deps."nix"."0.14.1"."libc"}" deps)
    (features_.void."${deps."nix"."0.14.1"."void"}" deps)
  ];


# end
# nix-0.20.0

  crates.nix."0.20.0" = deps: { features?(features_.nix."0.20.0" deps {}) }: buildRustCrate {
    crateName = "nix";
    version = "0.20.0";
    description = "Rust friendly bindings to *nix APIs";
    authors = [ "The nix-rust Project Developers" ];
    edition = "2018";
    sha256 = "175h2nlkq2lpwyb8gqfivlkia53p9iqjgwpl5wsrf79fclh9p4mx";
    dependencies = mapFeatures features ([
      (crates."bitflags"."${deps."nix"."0.20.0"."bitflags"}" deps)
      (crates."cfg_if"."${deps."nix"."0.20.0"."cfg_if"}" deps)
      (crates."libc"."${deps."nix"."0.20.0"."libc"}" deps)
    ])
      ++ (if kernel == "android" || kernel == "linux" then mapFeatures features ([
]) else [])
      ++ (if kernel == "dragonfly" then mapFeatures features ([
]) else [])
      ++ (if kernel == "freebsd" then mapFeatures features ([
]) else []);
  };
  features_.nix."0.20.0" = deps: f: updateFeatures f (rec {
    bitflags."${deps.nix."0.20.0".bitflags}".default = true;
    cfg_if."${deps.nix."0.20.0".cfg_if}".default = true;
    libc = fold recursiveUpdate {} [
      { "${deps.nix."0.20.0".libc}"."extra_traits" = true; }
      { "${deps.nix."0.20.0".libc}".default = true; }
    ];
    nix."0.20.0".default = (f.nix."0.20.0".default or true);
  }) [
    (features_.bitflags."${deps."nix"."0.20.0"."bitflags"}" deps)
    (features_.cfg_if."${deps."nix"."0.20.0"."cfg_if"}" deps)
    (features_.libc."${deps."nix"."0.20.0"."libc"}" deps)
  ];


# end
# notify-5.0.0-pre.1

  crates.notify."5.0.0-pre.1" = deps: { features?(features_.notify."5.0.0-pre.1" deps {}) }: buildRustCrate {
    crateName = "notify";
    version = "5.0.0-pre.1";
    description = "Cross-platform filesystem notification library";
    authors = [ "Félix Saparelli <me@passcod.name>" "Daniel Faust <hessijames@gmail.com>" ];
    edition = "2018";
    sha256 = "0hpawy8igcb1ga6p5xbwdk2c638hyg0148yfhdmqv6j42j0z4n9b";
    dependencies = mapFeatures features ([
      (crates."anymap"."${deps."notify"."5.0.0-pre.1"."anymap"}" deps)
      (crates."bitflags"."${deps."notify"."5.0.0-pre.1"."bitflags"}" deps)
      (crates."chashmap"."${deps."notify"."5.0.0-pre.1"."chashmap"}" deps)
      (crates."crossbeam_channel"."${deps."notify"."5.0.0-pre.1"."crossbeam_channel"}" deps)
      (crates."filetime"."${deps."notify"."5.0.0-pre.1"."filetime"}" deps)
      (crates."libc"."${deps."notify"."5.0.0-pre.1"."libc"}" deps)
      (crates."walkdir"."${deps."notify"."5.0.0-pre.1"."walkdir"}" deps)
    ])
      ++ (if kernel == "linux" then mapFeatures features ([
      (crates."inotify"."${deps."notify"."5.0.0-pre.1"."inotify"}" deps)
      (crates."mio"."${deps."notify"."5.0.0-pre.1"."mio"}" deps)
      (crates."mio_extras"."${deps."notify"."5.0.0-pre.1"."mio_extras"}" deps)
    ]) else [])
      ++ (if kernel == "darwin" then mapFeatures features ([
      (crates."fsevent"."${deps."notify"."5.0.0-pre.1"."fsevent"}" deps)
      (crates."fsevent_sys"."${deps."notify"."5.0.0-pre.1"."fsevent_sys"}" deps)
    ]) else [])
      ++ (if kernel == "windows" then mapFeatures features ([
      (crates."kernel32_sys"."${deps."notify"."5.0.0-pre.1"."kernel32_sys"}" deps)
      (crates."winapi"."${deps."notify"."5.0.0-pre.1"."winapi"}" deps)
    ]) else []);
    features = mkFeatures (features."notify"."5.0.0-pre.1" or {});
  };
  features_.notify."5.0.0-pre.1" = deps: f: updateFeatures f (rec {
    anymap."${deps.notify."5.0.0-pre.1".anymap}".default = true;
    bitflags."${deps.notify."5.0.0-pre.1".bitflags}".default = true;
    chashmap."${deps.notify."5.0.0-pre.1".chashmap}".default = true;
    crossbeam_channel."${deps.notify."5.0.0-pre.1".crossbeam_channel}".default = true;
    filetime."${deps.notify."5.0.0-pre.1".filetime}".default = true;
    fsevent."${deps.notify."5.0.0-pre.1".fsevent}".default = true;
    fsevent_sys."${deps.notify."5.0.0-pre.1".fsevent_sys}".default = true;
    inotify."${deps.notify."5.0.0-pre.1".inotify}".default = (f.inotify."${deps.notify."5.0.0-pre.1".inotify}".default or false);
    kernel32_sys."${deps.notify."5.0.0-pre.1".kernel32_sys}".default = true;
    libc."${deps.notify."5.0.0-pre.1".libc}".default = true;
    mio."${deps.notify."5.0.0-pre.1".mio}".default = true;
    mio_extras."${deps.notify."5.0.0-pre.1".mio_extras}".default = true;
    notify."5.0.0-pre.1".default = (f.notify."5.0.0-pre.1".default or true);
    walkdir."${deps.notify."5.0.0-pre.1".walkdir}".default = true;
    winapi."${deps.notify."5.0.0-pre.1".winapi}".default = true;
  }) [
    (features_.anymap."${deps."notify"."5.0.0-pre.1"."anymap"}" deps)
    (features_.bitflags."${deps."notify"."5.0.0-pre.1"."bitflags"}" deps)
    (features_.chashmap."${deps."notify"."5.0.0-pre.1"."chashmap"}" deps)
    (features_.crossbeam_channel."${deps."notify"."5.0.0-pre.1"."crossbeam_channel"}" deps)
    (features_.filetime."${deps."notify"."5.0.0-pre.1"."filetime"}" deps)
    (features_.libc."${deps."notify"."5.0.0-pre.1"."libc"}" deps)
    (features_.walkdir."${deps."notify"."5.0.0-pre.1"."walkdir"}" deps)
    (features_.inotify."${deps."notify"."5.0.0-pre.1"."inotify"}" deps)
    (features_.mio."${deps."notify"."5.0.0-pre.1"."mio"}" deps)
    (features_.mio_extras."${deps."notify"."5.0.0-pre.1"."mio_extras"}" deps)
    (features_.fsevent."${deps."notify"."5.0.0-pre.1"."fsevent"}" deps)
    (features_.fsevent_sys."${deps."notify"."5.0.0-pre.1"."fsevent_sys"}" deps)
    (features_.kernel32_sys."${deps."notify"."5.0.0-pre.1"."kernel32_sys"}" deps)
    (features_.winapi."${deps."notify"."5.0.0-pre.1"."winapi"}" deps)
  ];


# end
# num-integer-0.1.43

  crates.num_integer."0.1.43" = deps: { features?(features_.num_integer."0.1.43" deps {}) }: buildRustCrate {
    crateName = "num-integer";
    version = "0.1.43";
    description = "Integer traits and functions";
    authors = [ "The Rust Project Developers" ];
    sha256 = "1mfnc141sw133nkc4124sxwzvzs9lnv8h54gap08qgcn7pj2xxpw";
    build = "build.rs";
    dependencies = mapFeatures features ([
      (crates."num_traits"."${deps."num_integer"."0.1.43"."num_traits"}" deps)
    ]);

    buildDependencies = mapFeatures features ([
      (crates."autocfg"."${deps."num_integer"."0.1.43"."autocfg"}" deps)
    ]);
    features = mkFeatures (features."num_integer"."0.1.43" or {});
  };
  features_.num_integer."0.1.43" = deps: f: updateFeatures f (rec {
    autocfg."${deps.num_integer."0.1.43".autocfg}".default = true;
    num_integer = fold recursiveUpdate {} [
      { "0.1.43"."std" =
        (f.num_integer."0.1.43"."std" or false) ||
        (f.num_integer."0.1.43".default or false) ||
        (num_integer."0.1.43"."default" or false); }
      { "0.1.43".default = (f.num_integer."0.1.43".default or true); }
    ];
    num_traits = fold recursiveUpdate {} [
      { "${deps.num_integer."0.1.43".num_traits}"."i128" =
        (f.num_traits."${deps.num_integer."0.1.43".num_traits}"."i128" or false) ||
        (num_integer."0.1.43"."i128" or false) ||
        (f."num_integer"."0.1.43"."i128" or false); }
      { "${deps.num_integer."0.1.43".num_traits}"."std" =
        (f.num_traits."${deps.num_integer."0.1.43".num_traits}"."std" or false) ||
        (num_integer."0.1.43"."std" or false) ||
        (f."num_integer"."0.1.43"."std" or false); }
      { "${deps.num_integer."0.1.43".num_traits}".default = (f.num_traits."${deps.num_integer."0.1.43".num_traits}".default or false); }
    ];
  }) [
    (features_.num_traits."${deps."num_integer"."0.1.43"."num_traits"}" deps)
    (features_.autocfg."${deps."num_integer"."0.1.43"."autocfg"}" deps)
  ];


# end
# num-traits-0.2.12

  crates.num_traits."0.2.12" = deps: { features?(features_.num_traits."0.2.12" deps {}) }: buildRustCrate {
    crateName = "num-traits";
    version = "0.2.12";
    description = "Numeric traits for generic mathematics";
    authors = [ "The Rust Project Developers" ];
    sha256 = "1i3643iyqd3m24k0sp8bvyrvk4r6qnqp1qm304c9zv9khvjxa8nm";
    build = "build.rs";
    dependencies = mapFeatures features ([
]);

    buildDependencies = mapFeatures features ([
      (crates."autocfg"."${deps."num_traits"."0.2.12"."autocfg"}" deps)
    ]);
    features = mkFeatures (features."num_traits"."0.2.12" or {});
  };
  features_.num_traits."0.2.12" = deps: f: updateFeatures f (rec {
    autocfg."${deps.num_traits."0.2.12".autocfg}".default = true;
    num_traits = fold recursiveUpdate {} [
      { "0.2.12"."std" =
        (f.num_traits."0.2.12"."std" or false) ||
        (f.num_traits."0.2.12".default or false) ||
        (num_traits."0.2.12"."default" or false); }
      { "0.2.12".default = (f.num_traits."0.2.12".default or true); }
    ];
  }) [
    (features_.autocfg."${deps."num_traits"."0.2.12"."autocfg"}" deps)
  ];


# end
# os_type-2.2.0

  crates.os_type."2.2.0" = deps: { features?(features_.os_type."2.2.0" deps {}) }: buildRustCrate {
    crateName = "os_type";
    version = "2.2.0";
    description = "Detect the operating system type";
    authors = [ "Jan Schulte <hello@unexpected-co.de>" ];
    sha256 = "100ldg1vv0pxrb9s83vb4awvczbg8iy1by6vx6zl8vpdnr8n2ghg";
    dependencies = mapFeatures features ([
      (crates."regex"."${deps."os_type"."2.2.0"."regex"}" deps)
    ]);
  };
  features_.os_type."2.2.0" = deps: f: updateFeatures f (rec {
    os_type."2.2.0".default = (f.os_type."2.2.0".default or true);
    regex."${deps.os_type."2.2.0".regex}".default = true;
  }) [
    (features_.regex."${deps."os_type"."2.2.0"."regex"}" deps)
  ];


# end
# owning_ref-0.3.3

  crates.owning_ref."0.3.3" = deps: { features?(features_.owning_ref."0.3.3" deps {}) }: buildRustCrate {
    crateName = "owning_ref";
    version = "0.3.3";
    description = "A library for creating references that carry their owner with them.";
    authors = [ "Marvin Löbel <loebel.marvin@gmail.com>" ];
    sha256 = "13ivn0ydc0hf957ix0f5si9nnplzzykbr70hni1qz9m19i9kvmrh";
    dependencies = mapFeatures features ([
      (crates."stable_deref_trait"."${deps."owning_ref"."0.3.3"."stable_deref_trait"}" deps)
    ]);
  };
  features_.owning_ref."0.3.3" = deps: f: updateFeatures f (rec {
    owning_ref."0.3.3".default = (f.owning_ref."0.3.3".default or true);
    stable_deref_trait."${deps.owning_ref."0.3.3".stable_deref_trait}".default = true;
  }) [
    (features_.stable_deref_trait."${deps."owning_ref"."0.3.3"."stable_deref_trait"}" deps)
  ];


# end
# parking_lot-0.4.8

  crates.parking_lot."0.4.8" = deps: { features?(features_.parking_lot."0.4.8" deps {}) }: buildRustCrate {
    crateName = "parking_lot";
    version = "0.4.8";
    description = "More compact and efficient implementations of the standard synchronization primitives.";
    authors = [ "Amanieu d'Antras <amanieu@gmail.com>" ];
    sha256 = "0qrb2f0azglbsx7k3skgnc7mmv9z9spnqgk1m450g91r94nlklqi";
    dependencies = mapFeatures features ([
      (crates."parking_lot_core"."${deps."parking_lot"."0.4.8"."parking_lot_core"}" deps)
    ]
      ++ (if features.parking_lot."0.4.8".owning_ref or false then [ (crates.owning_ref."${deps."parking_lot"."0.4.8".owning_ref}" deps) ] else []));
    features = mkFeatures (features."parking_lot"."0.4.8" or {});
  };
  features_.parking_lot."0.4.8" = deps: f: updateFeatures f (rec {
    owning_ref."${deps.parking_lot."0.4.8".owning_ref}".default = true;
    parking_lot = fold recursiveUpdate {} [
      { "0.4.8"."owning_ref" =
        (f.parking_lot."0.4.8"."owning_ref" or false) ||
        (f.parking_lot."0.4.8".default or false) ||
        (parking_lot."0.4.8"."default" or false); }
      { "0.4.8".default = (f.parking_lot."0.4.8".default or true); }
    ];
    parking_lot_core = fold recursiveUpdate {} [
      { "${deps.parking_lot."0.4.8".parking_lot_core}"."deadlock_detection" =
        (f.parking_lot_core."${deps.parking_lot."0.4.8".parking_lot_core}"."deadlock_detection" or false) ||
        (parking_lot."0.4.8"."deadlock_detection" or false) ||
        (f."parking_lot"."0.4.8"."deadlock_detection" or false); }
      { "${deps.parking_lot."0.4.8".parking_lot_core}"."nightly" =
        (f.parking_lot_core."${deps.parking_lot."0.4.8".parking_lot_core}"."nightly" or false) ||
        (parking_lot."0.4.8"."nightly" or false) ||
        (f."parking_lot"."0.4.8"."nightly" or false); }
      { "${deps.parking_lot."0.4.8".parking_lot_core}".default = true; }
    ];
  }) [
    (features_.owning_ref."${deps."parking_lot"."0.4.8"."owning_ref"}" deps)
    (features_.parking_lot_core."${deps."parking_lot"."0.4.8"."parking_lot_core"}" deps)
  ];


# end
# parking_lot_core-0.2.14

  crates.parking_lot_core."0.2.14" = deps: { features?(features_.parking_lot_core."0.2.14" deps {}) }: buildRustCrate {
    crateName = "parking_lot_core";
    version = "0.2.14";
    description = "An advanced API for creating custom synchronization primitives.";
    authors = [ "Amanieu d'Antras <amanieu@gmail.com>" ];
    sha256 = "0giypb8ckkpi34p14nfk4b19c7przj4jxs95gs7x2v5ncmi0y286";
    dependencies = mapFeatures features ([
      (crates."rand"."${deps."parking_lot_core"."0.2.14"."rand"}" deps)
      (crates."smallvec"."${deps."parking_lot_core"."0.2.14"."smallvec"}" deps)
    ])
      ++ (if (kernel == "linux" || kernel == "darwin") then mapFeatures features ([
      (crates."libc"."${deps."parking_lot_core"."0.2.14"."libc"}" deps)
    ]) else [])
      ++ (if kernel == "windows" then mapFeatures features ([
      (crates."winapi"."${deps."parking_lot_core"."0.2.14"."winapi"}" deps)
    ]) else []);
    features = mkFeatures (features."parking_lot_core"."0.2.14" or {});
  };
  features_.parking_lot_core."0.2.14" = deps: f: updateFeatures f (rec {
    libc."${deps.parking_lot_core."0.2.14".libc}".default = true;
    parking_lot_core = fold recursiveUpdate {} [
      { "0.2.14"."backtrace" =
        (f.parking_lot_core."0.2.14"."backtrace" or false) ||
        (f.parking_lot_core."0.2.14".deadlock_detection or false) ||
        (parking_lot_core."0.2.14"."deadlock_detection" or false); }
      { "0.2.14"."petgraph" =
        (f.parking_lot_core."0.2.14"."petgraph" or false) ||
        (f.parking_lot_core."0.2.14".deadlock_detection or false) ||
        (parking_lot_core."0.2.14"."deadlock_detection" or false); }
      { "0.2.14"."thread-id" =
        (f.parking_lot_core."0.2.14"."thread-id" or false) ||
        (f.parking_lot_core."0.2.14".deadlock_detection or false) ||
        (parking_lot_core."0.2.14"."deadlock_detection" or false); }
      { "0.2.14".default = (f.parking_lot_core."0.2.14".default or true); }
    ];
    rand."${deps.parking_lot_core."0.2.14".rand}".default = true;
    smallvec."${deps.parking_lot_core."0.2.14".smallvec}".default = true;
    winapi = fold recursiveUpdate {} [
      { "${deps.parking_lot_core."0.2.14".winapi}"."errhandlingapi" = true; }
      { "${deps.parking_lot_core."0.2.14".winapi}"."handleapi" = true; }
      { "${deps.parking_lot_core."0.2.14".winapi}"."minwindef" = true; }
      { "${deps.parking_lot_core."0.2.14".winapi}"."ntstatus" = true; }
      { "${deps.parking_lot_core."0.2.14".winapi}"."winbase" = true; }
      { "${deps.parking_lot_core."0.2.14".winapi}"."winerror" = true; }
      { "${deps.parking_lot_core."0.2.14".winapi}"."winnt" = true; }
      { "${deps.parking_lot_core."0.2.14".winapi}".default = true; }
    ];
  }) [
    (features_.rand."${deps."parking_lot_core"."0.2.14"."rand"}" deps)
    (features_.smallvec."${deps."parking_lot_core"."0.2.14"."smallvec"}" deps)
    (features_.libc."${deps."parking_lot_core"."0.2.14"."libc"}" deps)
    (features_.winapi."${deps."parking_lot_core"."0.2.14"."winapi"}" deps)
  ];


# end
# ppv-lite86-0.2.8

  crates.ppv_lite86."0.2.8" = deps: { features?(features_.ppv_lite86."0.2.8" deps {}) }: buildRustCrate {
    crateName = "ppv-lite86";
    version = "0.2.8";
    description = "Implementation of the crypto-simd API for x86";
    authors = [ "The CryptoCorrosion Contributors" ];
    edition = "2018";
    sha256 = "1kc3bpc9rrqk1yd0d6k4jqabwycjdqgl88d3jfz3hks5rjln19ig";
    features = mkFeatures (features."ppv_lite86"."0.2.8" or {});
  };
  features_.ppv_lite86."0.2.8" = deps: f: updateFeatures f (rec {
    ppv_lite86 = fold recursiveUpdate {} [
      { "0.2.8"."std" =
        (f.ppv_lite86."0.2.8"."std" or false) ||
        (f.ppv_lite86."0.2.8".default or false) ||
        (ppv_lite86."0.2.8"."default" or false); }
      { "0.2.8".default = (f.ppv_lite86."0.2.8".default or true); }
    ];
  }) [];


# end
# proc-macro2-0.4.30

  crates.proc_macro2."0.4.30" = deps: { features?(features_.proc_macro2."0.4.30" deps {}) }: buildRustCrate {
    crateName = "proc-macro2";
    version = "0.4.30";
    description = "A stable implementation of the upcoming new `proc_macro` API. Comes with an\noption, off by default, to also reimplement itself in terms of the upstream\nunstable API.\n";
    authors = [ "Alex Crichton <alex@alexcrichton.com>" ];
    sha256 = "0iifv51wrm6r4r2gghw6rray3nv53zcap355bbz1nsmbhj5s09b9";
    build = "build.rs";
    dependencies = mapFeatures features ([
      (crates."unicode_xid"."${deps."proc_macro2"."0.4.30"."unicode_xid"}" deps)
    ]);
    features = mkFeatures (features."proc_macro2"."0.4.30" or {});
  };
  features_.proc_macro2."0.4.30" = deps: f: updateFeatures f (rec {
    proc_macro2 = fold recursiveUpdate {} [
      { "0.4.30"."proc-macro" =
        (f.proc_macro2."0.4.30"."proc-macro" or false) ||
        (f.proc_macro2."0.4.30".default or false) ||
        (proc_macro2."0.4.30"."default" or false); }
      { "0.4.30".default = (f.proc_macro2."0.4.30".default or true); }
    ];
    unicode_xid."${deps.proc_macro2."0.4.30".unicode_xid}".default = true;
  }) [
    (features_.unicode_xid."${deps."proc_macro2"."0.4.30"."unicode_xid"}" deps)
  ];


# end
# proc-macro2-1.0.24

  crates.proc_macro2."1.0.24" = deps: { features?(features_.proc_macro2."1.0.24" deps {}) }: buildRustCrate {
    crateName = "proc-macro2";
    version = "1.0.24";
    description = "A substitute implementation of the compiler's `proc_macro` API to decouple\ntoken-based libraries from the procedural macro use case.\n";
    authors = [ "Alex Crichton <alex@alexcrichton.com>" "David Tolnay <dtolnay@gmail.com>" ];
    edition = "2018";
    sha256 = "0fds8lvic0qy9dknwnr6zbqr3ri18jx66cy7sdkdm4yw2kipy0yd";
    dependencies = mapFeatures features ([
      (crates."unicode_xid"."${deps."proc_macro2"."1.0.24"."unicode_xid"}" deps)
    ]);
    features = mkFeatures (features."proc_macro2"."1.0.24" or {});
  };
  features_.proc_macro2."1.0.24" = deps: f: updateFeatures f (rec {
    proc_macro2 = fold recursiveUpdate {} [
      { "1.0.24"."proc-macro" =
        (f.proc_macro2."1.0.24"."proc-macro" or false) ||
        (f.proc_macro2."1.0.24".default or false) ||
        (proc_macro2."1.0.24"."default" or false); }
      { "1.0.24".default = (f.proc_macro2."1.0.24".default or true); }
    ];
    unicode_xid."${deps.proc_macro2."1.0.24".unicode_xid}".default = true;
  }) [
    (features_.unicode_xid."${deps."proc_macro2"."1.0.24"."unicode_xid"}" deps)
  ];


# end
# proptest-0.10.1

  crates.proptest."0.10.1" = deps: { features?(features_.proptest."0.10.1" deps {}) }: buildRustCrate {
    crateName = "proptest";
    version = "0.10.1";
    description = "Hypothesis-like property-based testing and shrinking.\n";
    authors = [ "Jason Lingle" ];
    edition = "2018";
    sha256 = "05ssy0pkc17h7q210db6d9dxy3a2y3jk6qnnhpqlc30xvx3kdncz";
    dependencies = mapFeatures features ([
      (crates."bitflags"."${deps."proptest"."0.10.1"."bitflags"}" deps)
      (crates."byteorder"."${deps."proptest"."0.10.1"."byteorder"}" deps)
      (crates."num_traits"."${deps."proptest"."0.10.1"."num_traits"}" deps)
      (crates."rand"."${deps."proptest"."0.10.1"."rand"}" deps)
      (crates."rand_chacha"."${deps."proptest"."0.10.1"."rand_chacha"}" deps)
      (crates."rand_xorshift"."${deps."proptest"."0.10.1"."rand_xorshift"}" deps)
    ]
      ++ (if features.proptest."0.10.1".lazy_static or false then [ (crates.lazy_static."${deps."proptest"."0.10.1".lazy_static}" deps) ] else [])
      ++ (if features.proptest."0.10.1".quick-error or false then [ (crates.quick_error."${deps."proptest"."0.10.1".quick_error}" deps) ] else [])
      ++ (if features.proptest."0.10.1".regex-syntax or false then [ (crates.regex_syntax."${deps."proptest"."0.10.1".regex_syntax}" deps) ] else []));
    features = mkFeatures (features."proptest"."0.10.1" or {});
  };
  features_.proptest."0.10.1" = deps: f: updateFeatures f (rec {
    bitflags."${deps.proptest."0.10.1".bitflags}".default = true;
    byteorder = fold recursiveUpdate {} [
      { "${deps.proptest."0.10.1".byteorder}"."std" =
        (f.byteorder."${deps.proptest."0.10.1".byteorder}"."std" or false) ||
        (proptest."0.10.1"."std" or false) ||
        (f."proptest"."0.10.1"."std" or false); }
      { "${deps.proptest."0.10.1".byteorder}".default = (f.byteorder."${deps.proptest."0.10.1".byteorder}".default or false); }
    ];
    lazy_static."${deps.proptest."0.10.1".lazy_static}".default = true;
    num_traits = fold recursiveUpdate {} [
      { "${deps.proptest."0.10.1".num_traits}"."std" =
        (f.num_traits."${deps.proptest."0.10.1".num_traits}"."std" or false) ||
        (proptest."0.10.1"."std" or false) ||
        (f."proptest"."0.10.1"."std" or false); }
      { "${deps.proptest."0.10.1".num_traits}".default = (f.num_traits."${deps.proptest."0.10.1".num_traits}".default or false); }
    ];
    proptest = fold recursiveUpdate {} [
      { "0.10.1"."bit-set" =
        (f.proptest."0.10.1"."bit-set" or false) ||
        (f.proptest."0.10.1".default or false) ||
        (proptest."0.10.1"."default" or false) ||
        (f.proptest."0.10.1".default-code-coverage or false) ||
        (proptest."0.10.1"."default-code-coverage" or false); }
      { "0.10.1"."break-dead-code" =
        (f.proptest."0.10.1"."break-dead-code" or false) ||
        (f.proptest."0.10.1".default or false) ||
        (proptest."0.10.1"."default" or false); }
      { "0.10.1"."fork" =
        (f.proptest."0.10.1"."fork" or false) ||
        (f.proptest."0.10.1".default or false) ||
        (proptest."0.10.1"."default" or false) ||
        (f.proptest."0.10.1".default-code-coverage or false) ||
        (proptest."0.10.1"."default-code-coverage" or false) ||
        (f.proptest."0.10.1".timeout or false) ||
        (proptest."0.10.1"."timeout" or false); }
      { "0.10.1"."lazy_static" =
        (f.proptest."0.10.1"."lazy_static" or false) ||
        (f.proptest."0.10.1".std or false) ||
        (proptest."0.10.1"."std" or false); }
      { "0.10.1"."quick-error" =
        (f.proptest."0.10.1"."quick-error" or false) ||
        (f.proptest."0.10.1".std or false) ||
        (proptest."0.10.1"."std" or false); }
      { "0.10.1"."regex-syntax" =
        (f.proptest."0.10.1"."regex-syntax" or false) ||
        (f.proptest."0.10.1".std or false) ||
        (proptest."0.10.1"."std" or false); }
      { "0.10.1"."rusty-fork" =
        (f.proptest."0.10.1"."rusty-fork" or false) ||
        (f.proptest."0.10.1".fork or false) ||
        (proptest."0.10.1"."fork" or false); }
      { "0.10.1"."std" =
        (f.proptest."0.10.1"."std" or false) ||
        (f.proptest."0.10.1".default or false) ||
        (proptest."0.10.1"."default" or false) ||
        (f.proptest."0.10.1".default-code-coverage or false) ||
        (proptest."0.10.1"."default-code-coverage" or false) ||
        (f.proptest."0.10.1".fork or false) ||
        (proptest."0.10.1"."fork" or false); }
      { "0.10.1"."tempfile" =
        (f.proptest."0.10.1"."tempfile" or false) ||
        (f.proptest."0.10.1".fork or false) ||
        (proptest."0.10.1"."fork" or false); }
      { "0.10.1"."timeout" =
        (f.proptest."0.10.1"."timeout" or false) ||
        (f.proptest."0.10.1".default or false) ||
        (proptest."0.10.1"."default" or false) ||
        (f.proptest."0.10.1".default-code-coverage or false) ||
        (proptest."0.10.1"."default-code-coverage" or false); }
      { "0.10.1"."x86" =
        (f.proptest."0.10.1"."x86" or false) ||
        (f.proptest."0.10.1".hardware-rng or false) ||
        (proptest."0.10.1"."hardware-rng" or false); }
      { "0.10.1".default = (f.proptest."0.10.1".default or true); }
    ];
    quick_error."${deps.proptest."0.10.1".quick_error}".default = true;
    rand = fold recursiveUpdate {} [
      { "${deps.proptest."0.10.1".rand}"."alloc" = true; }
      { "${deps.proptest."0.10.1".rand}"."std" =
        (f.rand."${deps.proptest."0.10.1".rand}"."std" or false) ||
        (proptest."0.10.1"."std" or false) ||
        (f."proptest"."0.10.1"."std" or false); }
      { "${deps.proptest."0.10.1".rand}".default = (f.rand."${deps.proptest."0.10.1".rand}".default or false); }
    ];
    rand_chacha."${deps.proptest."0.10.1".rand_chacha}".default = (f.rand_chacha."${deps.proptest."0.10.1".rand_chacha}".default or false);
    rand_xorshift."${deps.proptest."0.10.1".rand_xorshift}".default = true;
    regex_syntax."${deps.proptest."0.10.1".regex_syntax}".default = true;
  }) [
    (features_.bitflags."${deps."proptest"."0.10.1"."bitflags"}" deps)
    (features_.byteorder."${deps."proptest"."0.10.1"."byteorder"}" deps)
    (features_.lazy_static."${deps."proptest"."0.10.1"."lazy_static"}" deps)
    (features_.num_traits."${deps."proptest"."0.10.1"."num_traits"}" deps)
    (features_.quick_error."${deps."proptest"."0.10.1"."quick_error"}" deps)
    (features_.rand."${deps."proptest"."0.10.1"."rand"}" deps)
    (features_.rand_chacha."${deps."proptest"."0.10.1"."rand_chacha"}" deps)
    (features_.rand_xorshift."${deps."proptest"."0.10.1"."rand_xorshift"}" deps)
    (features_.regex_syntax."${deps."proptest"."0.10.1"."regex_syntax"}" deps)
  ];


# end
# quick-error-1.2.3

  crates.quick_error."1.2.3" = deps: { features?(features_.quick_error."1.2.3" deps {}) }: buildRustCrate {
    crateName = "quick-error";
    version = "1.2.3";
    description = "    A macro which makes error types pleasant to write.\n";
    authors = [ "Paul Colomiets <paul@colomiets.name>" "Colin Kiegel <kiegel@gmx.de>" ];
    sha256 = "17gqp7ifp6j5pcnk450f964a5jkqmy71848x69ahmsa9gyzhkh7x";
  };
  features_.quick_error."1.2.3" = deps: f: updateFeatures f (rec {
    quick_error."1.2.3".default = (f.quick_error."1.2.3".default or true);
  }) [];


# end
# quote-0.6.13

  crates.quote."0.6.13" = deps: { features?(features_.quote."0.6.13" deps {}) }: buildRustCrate {
    crateName = "quote";
    version = "0.6.13";
    description = "Quasi-quoting macro quote!(...)";
    authors = [ "David Tolnay <dtolnay@gmail.com>" ];
    sha256 = "1hrvsin40i4q8swrhlj9057g7nsp0lg02h8zbzmgz14av9mzv8g8";
    dependencies = mapFeatures features ([
      (crates."proc_macro2"."${deps."quote"."0.6.13"."proc_macro2"}" deps)
    ]);
    features = mkFeatures (features."quote"."0.6.13" or {});
  };
  features_.quote."0.6.13" = deps: f: updateFeatures f (rec {
    proc_macro2 = fold recursiveUpdate {} [
      { "${deps.quote."0.6.13".proc_macro2}"."proc-macro" =
        (f.proc_macro2."${deps.quote."0.6.13".proc_macro2}"."proc-macro" or false) ||
        (quote."0.6.13"."proc-macro" or false) ||
        (f."quote"."0.6.13"."proc-macro" or false); }
      { "${deps.quote."0.6.13".proc_macro2}".default = (f.proc_macro2."${deps.quote."0.6.13".proc_macro2}".default or false); }
    ];
    quote = fold recursiveUpdate {} [
      { "0.6.13"."proc-macro" =
        (f.quote."0.6.13"."proc-macro" or false) ||
        (f.quote."0.6.13".default or false) ||
        (quote."0.6.13"."default" or false); }
      { "0.6.13".default = (f.quote."0.6.13".default or true); }
    ];
  }) [
    (features_.proc_macro2."${deps."quote"."0.6.13"."proc_macro2"}" deps)
  ];


# end
# quote-1.0.7

  crates.quote."1.0.7" = deps: { features?(features_.quote."1.0.7" deps {}) }: buildRustCrate {
    crateName = "quote";
    version = "1.0.7";
    description = "Quasi-quoting macro quote!(...)";
    authors = [ "David Tolnay <dtolnay@gmail.com>" ];
    edition = "2018";
    sha256 = "0n4qkwj9zwbbgraxc5wnly466dzwyzxlpw396h5m4yazp0sai6ha";
    dependencies = mapFeatures features ([
      (crates."proc_macro2"."${deps."quote"."1.0.7"."proc_macro2"}" deps)
    ]);
    features = mkFeatures (features."quote"."1.0.7" or {});
  };
  features_.quote."1.0.7" = deps: f: updateFeatures f (rec {
    proc_macro2 = fold recursiveUpdate {} [
      { "${deps.quote."1.0.7".proc_macro2}"."proc-macro" =
        (f.proc_macro2."${deps.quote."1.0.7".proc_macro2}"."proc-macro" or false) ||
        (quote."1.0.7"."proc-macro" or false) ||
        (f."quote"."1.0.7"."proc-macro" or false); }
      { "${deps.quote."1.0.7".proc_macro2}".default = (f.proc_macro2."${deps.quote."1.0.7".proc_macro2}".default or false); }
    ];
    quote = fold recursiveUpdate {} [
      { "1.0.7"."proc-macro" =
        (f.quote."1.0.7"."proc-macro" or false) ||
        (f.quote."1.0.7".default or false) ||
        (quote."1.0.7"."default" or false); }
      { "1.0.7".default = (f.quote."1.0.7".default or true); }
    ];
  }) [
    (features_.proc_macro2."${deps."quote"."1.0.7"."proc_macro2"}" deps)
  ];


# end
# rand-0.4.6

  crates.rand."0.4.6" = deps: { features?(features_.rand."0.4.6" deps {}) }: buildRustCrate {
    crateName = "rand";
    version = "0.4.6";
    description = "Random number generators and other randomness functionality.\n";
    authors = [ "The Rust Project Developers" ];
    sha256 = "0c3rmg5q7d6qdi7cbmg5py9alm70wd3xsg0mmcawrnl35qv37zfs";
    dependencies = (if abi == "sgx" then mapFeatures features ([
      (crates."rand_core"."${deps."rand"."0.4.6"."rand_core"}" deps)
      (crates."rdrand"."${deps."rand"."0.4.6"."rdrand"}" deps)
    ]) else [])
      ++ (if kernel == "fuchsia" then mapFeatures features ([
      (crates."fuchsia_cprng"."${deps."rand"."0.4.6"."fuchsia_cprng"}" deps)
    ]) else [])
      ++ (if (kernel == "linux" || kernel == "darwin") then mapFeatures features ([
    ]
      ++ (if features.rand."0.4.6".libc or false then [ (crates.libc."${deps."rand"."0.4.6".libc}" deps) ] else [])) else [])
      ++ (if kernel == "windows" then mapFeatures features ([
      (crates."winapi"."${deps."rand"."0.4.6"."winapi"}" deps)
    ]) else []);
    features = mkFeatures (features."rand"."0.4.6" or {});
  };
  features_.rand."0.4.6" = deps: f: updateFeatures f (rec {
    fuchsia_cprng."${deps.rand."0.4.6".fuchsia_cprng}".default = true;
    libc."${deps.rand."0.4.6".libc}".default = true;
    rand = fold recursiveUpdate {} [
      { "0.4.6"."i128_support" =
        (f.rand."0.4.6"."i128_support" or false) ||
        (f.rand."0.4.6".nightly or false) ||
        (rand."0.4.6"."nightly" or false); }
      { "0.4.6"."libc" =
        (f.rand."0.4.6"."libc" or false) ||
        (f.rand."0.4.6".std or false) ||
        (rand."0.4.6"."std" or false); }
      { "0.4.6"."std" =
        (f.rand."0.4.6"."std" or false) ||
        (f.rand."0.4.6".default or false) ||
        (rand."0.4.6"."default" or false); }
      { "0.4.6".default = (f.rand."0.4.6".default or true); }
    ];
    rand_core."${deps.rand."0.4.6".rand_core}".default = (f.rand_core."${deps.rand."0.4.6".rand_core}".default or false);
    rdrand."${deps.rand."0.4.6".rdrand}".default = true;
    winapi = fold recursiveUpdate {} [
      { "${deps.rand."0.4.6".winapi}"."minwindef" = true; }
      { "${deps.rand."0.4.6".winapi}"."ntsecapi" = true; }
      { "${deps.rand."0.4.6".winapi}"."profileapi" = true; }
      { "${deps.rand."0.4.6".winapi}"."winnt" = true; }
      { "${deps.rand."0.4.6".winapi}".default = true; }
    ];
  }) [
    (features_.rand_core."${deps."rand"."0.4.6"."rand_core"}" deps)
    (features_.rdrand."${deps."rand"."0.4.6"."rdrand"}" deps)
    (features_.fuchsia_cprng."${deps."rand"."0.4.6"."fuchsia_cprng"}" deps)
    (features_.libc."${deps."rand"."0.4.6"."libc"}" deps)
    (features_.winapi."${deps."rand"."0.4.6"."winapi"}" deps)
  ];


# end
# rand-0.7.3

  crates.rand."0.7.3" = deps: { features?(features_.rand."0.7.3" deps {}) }: buildRustCrate {
    crateName = "rand";
    version = "0.7.3";
    description = "Random number generators and other randomness functionality.\n";
    authors = [ "The Rand Project Developers" "The Rust Project Developers" ];
    edition = "2018";
    sha256 = "1amg6qj53ylq3ix22n27kmj1gyj6i15my36mkayr30ndymny0b8r";
    dependencies = mapFeatures features ([
      (crates."rand_core"."${deps."rand"."0.7.3"."rand_core"}" deps)
    ])
      ++ (if !(kernel == "emscripten") then mapFeatures features ([
      (crates."rand_chacha"."${deps."rand"."0.7.3"."rand_chacha"}" deps)
    ]) else [])
      ++ (if kernel == "emscripten" then mapFeatures features ([
      (crates."rand_hc"."${deps."rand"."0.7.3"."rand_hc"}" deps)
    ]) else [])
      ++ (if (kernel == "linux" || kernel == "darwin") then mapFeatures features ([
    ]
      ++ (if features.rand."0.7.3".libc or false then [ (crates.libc."${deps."rand"."0.7.3".libc}" deps) ] else [])) else []);
    features = mkFeatures (features."rand"."0.7.3" or {});
  };
  features_.rand."0.7.3" = deps: f: updateFeatures f (rec {
    libc."${deps.rand."0.7.3".libc}".default = (f.libc."${deps.rand."0.7.3".libc}".default or false);
    rand = fold recursiveUpdate {} [
      { "0.7.3"."alloc" =
        (f.rand."0.7.3"."alloc" or false) ||
        (f.rand."0.7.3".std or false) ||
        (rand."0.7.3"."std" or false); }
      { "0.7.3"."getrandom" =
        (f.rand."0.7.3"."getrandom" or false) ||
        (f.rand."0.7.3".std or false) ||
        (rand."0.7.3"."std" or false); }
      { "0.7.3"."getrandom_package" =
        (f.rand."0.7.3"."getrandom_package" or false) ||
        (f.rand."0.7.3".getrandom or false) ||
        (rand."0.7.3"."getrandom" or false); }
      { "0.7.3"."libc" =
        (f.rand."0.7.3"."libc" or false) ||
        (f.rand."0.7.3".std or false) ||
        (rand."0.7.3"."std" or false); }
      { "0.7.3"."packed_simd" =
        (f.rand."0.7.3"."packed_simd" or false) ||
        (f.rand."0.7.3".simd_support or false) ||
        (rand."0.7.3"."simd_support" or false); }
      { "0.7.3"."rand_pcg" =
        (f.rand."0.7.3"."rand_pcg" or false) ||
        (f.rand."0.7.3".small_rng or false) ||
        (rand."0.7.3"."small_rng" or false); }
      { "0.7.3"."simd_support" =
        (f.rand."0.7.3"."simd_support" or false) ||
        (f.rand."0.7.3".nightly or false) ||
        (rand."0.7.3"."nightly" or false); }
      { "0.7.3"."std" =
        (f.rand."0.7.3"."std" or false) ||
        (f.rand."0.7.3".default or false) ||
        (rand."0.7.3"."default" or false); }
      { "0.7.3".default = (f.rand."0.7.3".default or true); }
    ];
    rand_chacha."${deps.rand."0.7.3".rand_chacha}".default = (f.rand_chacha."${deps.rand."0.7.3".rand_chacha}".default or false);
    rand_core = fold recursiveUpdate {} [
      { "${deps.rand."0.7.3".rand_core}"."alloc" =
        (f.rand_core."${deps.rand."0.7.3".rand_core}"."alloc" or false) ||
        (rand."0.7.3"."alloc" or false) ||
        (f."rand"."0.7.3"."alloc" or false); }
      { "${deps.rand."0.7.3".rand_core}"."getrandom" =
        (f.rand_core."${deps.rand."0.7.3".rand_core}"."getrandom" or false) ||
        (rand."0.7.3"."getrandom" or false) ||
        (f."rand"."0.7.3"."getrandom" or false); }
      { "${deps.rand."0.7.3".rand_core}"."std" =
        (f.rand_core."${deps.rand."0.7.3".rand_core}"."std" or false) ||
        (rand."0.7.3"."std" or false) ||
        (f."rand"."0.7.3"."std" or false); }
      { "${deps.rand."0.7.3".rand_core}".default = true; }
    ];
    rand_hc."${deps.rand."0.7.3".rand_hc}".default = true;
  }) [
    (features_.rand_core."${deps."rand"."0.7.3"."rand_core"}" deps)
    (features_.rand_chacha."${deps."rand"."0.7.3"."rand_chacha"}" deps)
    (features_.rand_hc."${deps."rand"."0.7.3"."rand_hc"}" deps)
    (features_.libc."${deps."rand"."0.7.3"."libc"}" deps)
  ];


# end
# rand_chacha-0.2.2

  crates.rand_chacha."0.2.2" = deps: { features?(features_.rand_chacha."0.2.2" deps {}) }: buildRustCrate {
    crateName = "rand_chacha";
    version = "0.2.2";
    description = "ChaCha random number generator\n";
    authors = [ "The Rand Project Developers" "The Rust Project Developers" "The CryptoCorrosion Contributors" ];
    edition = "2018";
    sha256 = "1adx0h0h17y6sxlx1zpwg0hnyccnwlp5ad7dxn2jib9caw1s7ghk";
    dependencies = mapFeatures features ([
      (crates."ppv_lite86"."${deps."rand_chacha"."0.2.2"."ppv_lite86"}" deps)
      (crates."rand_core"."${deps."rand_chacha"."0.2.2"."rand_core"}" deps)
    ]);
    features = mkFeatures (features."rand_chacha"."0.2.2" or {});
  };
  features_.rand_chacha."0.2.2" = deps: f: updateFeatures f (rec {
    ppv_lite86 = fold recursiveUpdate {} [
      { "${deps.rand_chacha."0.2.2".ppv_lite86}"."simd" = true; }
      { "${deps.rand_chacha."0.2.2".ppv_lite86}"."std" =
        (f.ppv_lite86."${deps.rand_chacha."0.2.2".ppv_lite86}"."std" or false) ||
        (rand_chacha."0.2.2"."std" or false) ||
        (f."rand_chacha"."0.2.2"."std" or false); }
      { "${deps.rand_chacha."0.2.2".ppv_lite86}".default = (f.ppv_lite86."${deps.rand_chacha."0.2.2".ppv_lite86}".default or false); }
    ];
    rand_chacha = fold recursiveUpdate {} [
      { "0.2.2"."simd" =
        (f.rand_chacha."0.2.2"."simd" or false) ||
        (f.rand_chacha."0.2.2".default or false) ||
        (rand_chacha."0.2.2"."default" or false); }
      { "0.2.2"."std" =
        (f.rand_chacha."0.2.2"."std" or false) ||
        (f.rand_chacha."0.2.2".default or false) ||
        (rand_chacha."0.2.2"."default" or false); }
      { "0.2.2".default = (f.rand_chacha."0.2.2".default or true); }
    ];
    rand_core."${deps.rand_chacha."0.2.2".rand_core}".default = true;
  }) [
    (features_.ppv_lite86."${deps."rand_chacha"."0.2.2"."ppv_lite86"}" deps)
    (features_.rand_core."${deps."rand_chacha"."0.2.2"."rand_core"}" deps)
  ];


# end
# rand_core-0.3.1

  crates.rand_core."0.3.1" = deps: { features?(features_.rand_core."0.3.1" deps {}) }: buildRustCrate {
    crateName = "rand_core";
    version = "0.3.1";
    description = "Core random number generator traits and tools for implementation.\n";
    authors = [ "The Rand Project Developers" "The Rust Project Developers" ];
    sha256 = "0q0ssgpj9x5a6fda83nhmfydy7a6c0wvxm0jhncsmjx8qp8gw91m";
    dependencies = mapFeatures features ([
      (crates."rand_core"."${deps."rand_core"."0.3.1"."rand_core"}" deps)
    ]);
    features = mkFeatures (features."rand_core"."0.3.1" or {});
  };
  features_.rand_core."0.3.1" = deps: f: updateFeatures f (rec {
    rand_core = fold recursiveUpdate {} [
      { "${deps.rand_core."0.3.1".rand_core}"."alloc" =
        (f.rand_core."${deps.rand_core."0.3.1".rand_core}"."alloc" or false) ||
        (rand_core."0.3.1"."alloc" or false) ||
        (f."rand_core"."0.3.1"."alloc" or false); }
      { "${deps.rand_core."0.3.1".rand_core}"."serde1" =
        (f.rand_core."${deps.rand_core."0.3.1".rand_core}"."serde1" or false) ||
        (rand_core."0.3.1"."serde1" or false) ||
        (f."rand_core"."0.3.1"."serde1" or false); }
      { "${deps.rand_core."0.3.1".rand_core}"."std" =
        (f.rand_core."${deps.rand_core."0.3.1".rand_core}"."std" or false) ||
        (rand_core."0.3.1"."std" or false) ||
        (f."rand_core"."0.3.1"."std" or false); }
      { "${deps.rand_core."0.3.1".rand_core}".default = true; }
      { "0.3.1"."std" =
        (f.rand_core."0.3.1"."std" or false) ||
        (f.rand_core."0.3.1".default or false) ||
        (rand_core."0.3.1"."default" or false); }
      { "0.3.1".default = (f.rand_core."0.3.1".default or true); }
    ];
  }) [
    (features_.rand_core."${deps."rand_core"."0.3.1"."rand_core"}" deps)
  ];


# end
# rand_core-0.4.2

  crates.rand_core."0.4.2" = deps: { features?(features_.rand_core."0.4.2" deps {}) }: buildRustCrate {
    crateName = "rand_core";
    version = "0.4.2";
    description = "Core random number generator traits and tools for implementation.\n";
    authors = [ "The Rand Project Developers" "The Rust Project Developers" ];
    sha256 = "18zpzwn4bl7lp9f36iacy8mvdnfrhfmzsl35gmln98dcindff2ly";
    dependencies = mapFeatures features ([
]);
    features = mkFeatures (features."rand_core"."0.4.2" or {});
  };
  features_.rand_core."0.4.2" = deps: f: updateFeatures f (rec {
    rand_core = fold recursiveUpdate {} [
      { "0.4.2"."alloc" =
        (f.rand_core."0.4.2"."alloc" or false) ||
        (f.rand_core."0.4.2".std or false) ||
        (rand_core."0.4.2"."std" or false); }
      { "0.4.2"."serde" =
        (f.rand_core."0.4.2"."serde" or false) ||
        (f.rand_core."0.4.2".serde1 or false) ||
        (rand_core."0.4.2"."serde1" or false); }
      { "0.4.2"."serde_derive" =
        (f.rand_core."0.4.2"."serde_derive" or false) ||
        (f.rand_core."0.4.2".serde1 or false) ||
        (rand_core."0.4.2"."serde1" or false); }
      { "0.4.2".default = (f.rand_core."0.4.2".default or true); }
    ];
  }) [];


# end
# rand_core-0.5.1

  crates.rand_core."0.5.1" = deps: { features?(features_.rand_core."0.5.1" deps {}) }: buildRustCrate {
    crateName = "rand_core";
    version = "0.5.1";
    description = "Core random number generator traits and tools for implementation.\n";
    authors = [ "The Rand Project Developers" "The Rust Project Developers" ];
    edition = "2018";
    sha256 = "19qfnh77bzz0x2gfsk91h0gygy0z1s5l3yyc2j91gmprq60d6s3r";
    dependencies = mapFeatures features ([
    ]
      ++ (if features.rand_core."0.5.1".getrandom or false then [ (crates.getrandom."${deps."rand_core"."0.5.1".getrandom}" deps) ] else []));
    features = mkFeatures (features."rand_core"."0.5.1" or {});
  };
  features_.rand_core."0.5.1" = deps: f: updateFeatures f (rec {
    getrandom = fold recursiveUpdate {} [
      { "${deps.rand_core."0.5.1".getrandom}"."std" =
        (f.getrandom."${deps.rand_core."0.5.1".getrandom}"."std" or false) ||
        (rand_core."0.5.1"."std" or false) ||
        (f."rand_core"."0.5.1"."std" or false); }
      { "${deps.rand_core."0.5.1".getrandom}".default = true; }
    ];
    rand_core = fold recursiveUpdate {} [
      { "0.5.1"."alloc" =
        (f.rand_core."0.5.1"."alloc" or false) ||
        (f.rand_core."0.5.1".std or false) ||
        (rand_core."0.5.1"."std" or false); }
      { "0.5.1"."getrandom" =
        (f.rand_core."0.5.1"."getrandom" or false) ||
        (f.rand_core."0.5.1".std or false) ||
        (rand_core."0.5.1"."std" or false); }
      { "0.5.1"."serde" =
        (f.rand_core."0.5.1"."serde" or false) ||
        (f.rand_core."0.5.1".serde1 or false) ||
        (rand_core."0.5.1"."serde1" or false); }
      { "0.5.1".default = (f.rand_core."0.5.1".default or true); }
    ];
  }) [
    (features_.getrandom."${deps."rand_core"."0.5.1"."getrandom"}" deps)
  ];


# end
# rand_hc-0.2.0

  crates.rand_hc."0.2.0" = deps: { features?(features_.rand_hc."0.2.0" deps {}) }: buildRustCrate {
    crateName = "rand_hc";
    version = "0.2.0";
    description = "HC128 random number generator\n";
    authors = [ "The Rand Project Developers" ];
    edition = "2018";
    sha256 = "0592q9kqcna9aiyzy6vp3fadxkkbpfkmi2cnkv48zhybr0v2yf01";
    dependencies = mapFeatures features ([
      (crates."rand_core"."${deps."rand_hc"."0.2.0"."rand_core"}" deps)
    ]);
  };
  features_.rand_hc."0.2.0" = deps: f: updateFeatures f (rec {
    rand_core."${deps.rand_hc."0.2.0".rand_core}".default = true;
    rand_hc."0.2.0".default = (f.rand_hc."0.2.0".default or true);
  }) [
    (features_.rand_core."${deps."rand_hc"."0.2.0"."rand_core"}" deps)
  ];


# end
# rand_xorshift-0.2.0

  crates.rand_xorshift."0.2.0" = deps: { features?(features_.rand_xorshift."0.2.0" deps {}) }: buildRustCrate {
    crateName = "rand_xorshift";
    version = "0.2.0";
    description = "Xorshift random number generator\n";
    authors = [ "The Rand Project Developers" "The Rust Project Developers" ];
    edition = "2018";
    sha256 = "14lj3xzbaxc5sh7kn0jlcbik1dp2jw8dyp6xwjdi1y9jgia07ww3";
    dependencies = mapFeatures features ([
      (crates."rand_core"."${deps."rand_xorshift"."0.2.0"."rand_core"}" deps)
    ]);
    features = mkFeatures (features."rand_xorshift"."0.2.0" or {});
  };
  features_.rand_xorshift."0.2.0" = deps: f: updateFeatures f (rec {
    rand_core."${deps.rand_xorshift."0.2.0".rand_core}".default = true;
    rand_xorshift = fold recursiveUpdate {} [
      { "0.2.0"."serde" =
        (f.rand_xorshift."0.2.0"."serde" or false) ||
        (f.rand_xorshift."0.2.0".serde1 or false) ||
        (rand_xorshift."0.2.0"."serde1" or false); }
      { "0.2.0".default = (f.rand_xorshift."0.2.0".default or true); }
    ];
  }) [
    (features_.rand_core."${deps."rand_xorshift"."0.2.0"."rand_core"}" deps)
  ];


# end
# rdrand-0.4.0

  crates.rdrand."0.4.0" = deps: { features?(features_.rdrand."0.4.0" deps {}) }: buildRustCrate {
    crateName = "rdrand";
    version = "0.4.0";
    description = "An implementation of random number generator based on rdrand and rdseed instructions";
    authors = [ "Simonas Kazlauskas <rdrand@kazlauskas.me>" ];
    sha256 = "15hrcasn0v876wpkwab1dwbk9kvqwrb3iv4y4dibb6yxnfvzwajk";
    dependencies = mapFeatures features ([
      (crates."rand_core"."${deps."rdrand"."0.4.0"."rand_core"}" deps)
    ]);
    features = mkFeatures (features."rdrand"."0.4.0" or {});
  };
  features_.rdrand."0.4.0" = deps: f: updateFeatures f (rec {
    rand_core."${deps.rdrand."0.4.0".rand_core}".default = (f.rand_core."${deps.rdrand."0.4.0".rand_core}".default or false);
    rdrand = fold recursiveUpdate {} [
      { "0.4.0"."std" =
        (f.rdrand."0.4.0"."std" or false) ||
        (f.rdrand."0.4.0".default or false) ||
        (rdrand."0.4.0"."default" or false); }
      { "0.4.0".default = (f.rdrand."0.4.0".default or true); }
    ];
  }) [
    (features_.rand_core."${deps."rdrand"."0.4.0"."rand_core"}" deps)
  ];


# end
# redox_syscall-0.1.56

  crates.redox_syscall."0.1.56" = deps: { features?(features_.redox_syscall."0.1.56" deps {}) }: buildRustCrate {
    crateName = "redox_syscall";
    version = "0.1.56";
    description = "A Rust library to access raw Redox system calls";
    authors = [ "Jeremy Soller <jackpot51@gmail.com>" ];
    sha256 = "0jcp8nd947zcy938bz09pzlmi3vyxfdzg92pjxdvvk0699vwcc26";
    libName = "syscall";
  };
  features_.redox_syscall."0.1.56" = deps: f: updateFeatures f (rec {
    redox_syscall."0.1.56".default = (f.redox_syscall."0.1.56".default or true);
  }) [];


# end
# redox_users-0.3.4

  crates.redox_users."0.3.4" = deps: { features?(features_.redox_users."0.3.4" deps {}) }: buildRustCrate {
    crateName = "redox_users";
    version = "0.3.4";
    description = "A Rust library to access Redox users and groups functionality";
    authors = [ "Jose Narvaez <goyox86@gmail.com>" "Wesley Hershberger <mggmugginsmc@gmail.com>" ];
    sha256 = "1x6mhzplqyn1glkhxxmm84v795p8sa1mnrcd3nhz8j2jfrq8c1qs";
    dependencies = mapFeatures features ([
      (crates."getrandom"."${deps."redox_users"."0.3.4"."getrandom"}" deps)
      (crates."redox_syscall"."${deps."redox_users"."0.3.4"."redox_syscall"}" deps)
      (crates."rust_argon2"."${deps."redox_users"."0.3.4"."rust_argon2"}" deps)
    ]);
  };
  features_.redox_users."0.3.4" = deps: f: updateFeatures f (rec {
    getrandom."${deps.redox_users."0.3.4".getrandom}".default = true;
    redox_syscall."${deps.redox_users."0.3.4".redox_syscall}".default = true;
    redox_users."0.3.4".default = (f.redox_users."0.3.4".default or true);
    rust_argon2."${deps.redox_users."0.3.4".rust_argon2}".default = true;
  }) [
    (features_.getrandom."${deps."redox_users"."0.3.4"."getrandom"}" deps)
    (features_.redox_syscall."${deps."redox_users"."0.3.4"."redox_syscall"}" deps)
    (features_.rust_argon2."${deps."redox_users"."0.3.4"."rust_argon2"}" deps)
  ];


# end
# regex-1.4.3

  crates.regex."1.4.3" = deps: { features?(features_.regex."1.4.3" deps {}) }: buildRustCrate {
    crateName = "regex";
    version = "1.4.3";
    description = "An implementation of regular expressions for Rust. This implementation uses\nfinite automata and guarantees linear time matching on all inputs.\n";
    authors = [ "The Rust Project Developers" ];
    sha256 = "0w0b4bh0ng20lf5y8raaxmxj46ikjqpgwy1iggzpby9lhv9vydkp";
    dependencies = mapFeatures features ([
      (crates."regex_syntax"."${deps."regex"."1.4.3"."regex_syntax"}" deps)
    ]
      ++ (if features.regex."1.4.3".aho-corasick or false then [ (crates.aho_corasick."${deps."regex"."1.4.3".aho_corasick}" deps) ] else [])
      ++ (if features.regex."1.4.3".memchr or false then [ (crates.memchr."${deps."regex"."1.4.3".memchr}" deps) ] else [])
      ++ (if features.regex."1.4.3".thread_local or false then [ (crates.thread_local."${deps."regex"."1.4.3".thread_local}" deps) ] else []));
    features = mkFeatures (features."regex"."1.4.3" or {});
  };
  features_.regex."1.4.3" = deps: f: updateFeatures f (rec {
    aho_corasick."${deps.regex."1.4.3".aho_corasick}".default = true;
    memchr."${deps.regex."1.4.3".memchr}".default = true;
    regex = fold recursiveUpdate {} [
      { "1.4.3"."aho-corasick" =
        (f.regex."1.4.3"."aho-corasick" or false) ||
        (f.regex."1.4.3".perf-literal or false) ||
        (regex."1.4.3"."perf-literal" or false); }
      { "1.4.3"."memchr" =
        (f.regex."1.4.3"."memchr" or false) ||
        (f.regex."1.4.3".perf-literal or false) ||
        (regex."1.4.3"."perf-literal" or false); }
      { "1.4.3"."pattern" =
        (f.regex."1.4.3"."pattern" or false) ||
        (f.regex."1.4.3".unstable or false) ||
        (regex."1.4.3"."unstable" or false); }
      { "1.4.3"."perf" =
        (f.regex."1.4.3"."perf" or false) ||
        (f.regex."1.4.3".default or false) ||
        (regex."1.4.3"."default" or false); }
      { "1.4.3"."perf-cache" =
        (f.regex."1.4.3"."perf-cache" or false) ||
        (f.regex."1.4.3".perf or false) ||
        (regex."1.4.3"."perf" or false); }
      { "1.4.3"."perf-dfa" =
        (f.regex."1.4.3"."perf-dfa" or false) ||
        (f.regex."1.4.3".perf or false) ||
        (regex."1.4.3"."perf" or false); }
      { "1.4.3"."perf-inline" =
        (f.regex."1.4.3"."perf-inline" or false) ||
        (f.regex."1.4.3".perf or false) ||
        (regex."1.4.3"."perf" or false); }
      { "1.4.3"."perf-literal" =
        (f.regex."1.4.3"."perf-literal" or false) ||
        (f.regex."1.4.3".perf or false) ||
        (regex."1.4.3"."perf" or false); }
      { "1.4.3"."std" =
        (f.regex."1.4.3"."std" or false) ||
        (f.regex."1.4.3".default or false) ||
        (regex."1.4.3"."default" or false) ||
        (f.regex."1.4.3".use_std or false) ||
        (regex."1.4.3"."use_std" or false); }
      { "1.4.3"."thread_local" =
        (f.regex."1.4.3"."thread_local" or false) ||
        (f.regex."1.4.3".perf-cache or false) ||
        (regex."1.4.3"."perf-cache" or false); }
      { "1.4.3"."unicode" =
        (f.regex."1.4.3"."unicode" or false) ||
        (f.regex."1.4.3".default or false) ||
        (regex."1.4.3"."default" or false); }
      { "1.4.3"."unicode-age" =
        (f.regex."1.4.3"."unicode-age" or false) ||
        (f.regex."1.4.3".unicode or false) ||
        (regex."1.4.3"."unicode" or false); }
      { "1.4.3"."unicode-bool" =
        (f.regex."1.4.3"."unicode-bool" or false) ||
        (f.regex."1.4.3".unicode or false) ||
        (regex."1.4.3"."unicode" or false); }
      { "1.4.3"."unicode-case" =
        (f.regex."1.4.3"."unicode-case" or false) ||
        (f.regex."1.4.3".unicode or false) ||
        (regex."1.4.3"."unicode" or false); }
      { "1.4.3"."unicode-gencat" =
        (f.regex."1.4.3"."unicode-gencat" or false) ||
        (f.regex."1.4.3".unicode or false) ||
        (regex."1.4.3"."unicode" or false); }
      { "1.4.3"."unicode-perl" =
        (f.regex."1.4.3"."unicode-perl" or false) ||
        (f.regex."1.4.3".unicode or false) ||
        (regex."1.4.3"."unicode" or false); }
      { "1.4.3"."unicode-script" =
        (f.regex."1.4.3"."unicode-script" or false) ||
        (f.regex."1.4.3".unicode or false) ||
        (regex."1.4.3"."unicode" or false); }
      { "1.4.3"."unicode-segment" =
        (f.regex."1.4.3"."unicode-segment" or false) ||
        (f.regex."1.4.3".unicode or false) ||
        (regex."1.4.3"."unicode" or false); }
      { "1.4.3".default = (f.regex."1.4.3".default or true); }
    ];
    regex_syntax = fold recursiveUpdate {} [
      { "${deps.regex."1.4.3".regex_syntax}"."default" =
        (f.regex_syntax."${deps.regex."1.4.3".regex_syntax}"."default" or false) ||
        (regex."1.4.3"."default" or false) ||
        (f."regex"."1.4.3"."default" or false); }
      { "${deps.regex."1.4.3".regex_syntax}"."unicode" =
        (f.regex_syntax."${deps.regex."1.4.3".regex_syntax}"."unicode" or false) ||
        (regex."1.4.3"."unicode" or false) ||
        (f."regex"."1.4.3"."unicode" or false); }
      { "${deps.regex."1.4.3".regex_syntax}"."unicode-age" =
        (f.regex_syntax."${deps.regex."1.4.3".regex_syntax}"."unicode-age" or false) ||
        (regex."1.4.3"."unicode-age" or false) ||
        (f."regex"."1.4.3"."unicode-age" or false); }
      { "${deps.regex."1.4.3".regex_syntax}"."unicode-bool" =
        (f.regex_syntax."${deps.regex."1.4.3".regex_syntax}"."unicode-bool" or false) ||
        (regex."1.4.3"."unicode-bool" or false) ||
        (f."regex"."1.4.3"."unicode-bool" or false); }
      { "${deps.regex."1.4.3".regex_syntax}"."unicode-case" =
        (f.regex_syntax."${deps.regex."1.4.3".regex_syntax}"."unicode-case" or false) ||
        (regex."1.4.3"."unicode-case" or false) ||
        (f."regex"."1.4.3"."unicode-case" or false); }
      { "${deps.regex."1.4.3".regex_syntax}"."unicode-gencat" =
        (f.regex_syntax."${deps.regex."1.4.3".regex_syntax}"."unicode-gencat" or false) ||
        (regex."1.4.3"."unicode-gencat" or false) ||
        (f."regex"."1.4.3"."unicode-gencat" or false); }
      { "${deps.regex."1.4.3".regex_syntax}"."unicode-perl" =
        (f.regex_syntax."${deps.regex."1.4.3".regex_syntax}"."unicode-perl" or false) ||
        (regex."1.4.3"."unicode-perl" or false) ||
        (f."regex"."1.4.3"."unicode-perl" or false); }
      { "${deps.regex."1.4.3".regex_syntax}"."unicode-script" =
        (f.regex_syntax."${deps.regex."1.4.3".regex_syntax}"."unicode-script" or false) ||
        (regex."1.4.3"."unicode-script" or false) ||
        (f."regex"."1.4.3"."unicode-script" or false); }
      { "${deps.regex."1.4.3".regex_syntax}"."unicode-segment" =
        (f.regex_syntax."${deps.regex."1.4.3".regex_syntax}"."unicode-segment" or false) ||
        (regex."1.4.3"."unicode-segment" or false) ||
        (f."regex"."1.4.3"."unicode-segment" or false); }
      { "${deps.regex."1.4.3".regex_syntax}".default = (f.regex_syntax."${deps.regex."1.4.3".regex_syntax}".default or false); }
    ];
    thread_local."${deps.regex."1.4.3".thread_local}".default = true;
  }) [
    (features_.aho_corasick."${deps."regex"."1.4.3"."aho_corasick"}" deps)
    (features_.memchr."${deps."regex"."1.4.3"."memchr"}" deps)
    (features_.regex_syntax."${deps."regex"."1.4.3"."regex_syntax"}" deps)
    (features_.thread_local."${deps."regex"."1.4.3"."thread_local"}" deps)
  ];


# end
# regex-syntax-0.6.22

  crates.regex_syntax."0.6.22" = deps: { features?(features_.regex_syntax."0.6.22" deps {}) }: buildRustCrate {
    crateName = "regex-syntax";
    version = "0.6.22";
    description = "A regular expression parser.";
    authors = [ "The Rust Project Developers" ];
    sha256 = "0r00n2dgyixacl1sczqp18gxf0xh7x272hcdp62412lypba2gqyg";
    features = mkFeatures (features."regex_syntax"."0.6.22" or {});
  };
  features_.regex_syntax."0.6.22" = deps: f: updateFeatures f (rec {
    regex_syntax = fold recursiveUpdate {} [
      { "0.6.22"."unicode" =
        (f.regex_syntax."0.6.22"."unicode" or false) ||
        (f.regex_syntax."0.6.22".default or false) ||
        (regex_syntax."0.6.22"."default" or false); }
      { "0.6.22"."unicode-age" =
        (f.regex_syntax."0.6.22"."unicode-age" or false) ||
        (f.regex_syntax."0.6.22".unicode or false) ||
        (regex_syntax."0.6.22"."unicode" or false); }
      { "0.6.22"."unicode-bool" =
        (f.regex_syntax."0.6.22"."unicode-bool" or false) ||
        (f.regex_syntax."0.6.22".unicode or false) ||
        (regex_syntax."0.6.22"."unicode" or false); }
      { "0.6.22"."unicode-case" =
        (f.regex_syntax."0.6.22"."unicode-case" or false) ||
        (f.regex_syntax."0.6.22".unicode or false) ||
        (regex_syntax."0.6.22"."unicode" or false); }
      { "0.6.22"."unicode-gencat" =
        (f.regex_syntax."0.6.22"."unicode-gencat" or false) ||
        (f.regex_syntax."0.6.22".unicode or false) ||
        (regex_syntax."0.6.22"."unicode" or false); }
      { "0.6.22"."unicode-perl" =
        (f.regex_syntax."0.6.22"."unicode-perl" or false) ||
        (f.regex_syntax."0.6.22".unicode or false) ||
        (regex_syntax."0.6.22"."unicode" or false); }
      { "0.6.22"."unicode-script" =
        (f.regex_syntax."0.6.22"."unicode-script" or false) ||
        (f.regex_syntax."0.6.22".unicode or false) ||
        (regex_syntax."0.6.22"."unicode" or false); }
      { "0.6.22"."unicode-segment" =
        (f.regex_syntax."0.6.22"."unicode-segment" or false) ||
        (f.regex_syntax."0.6.22".unicode or false) ||
        (regex_syntax."0.6.22"."unicode" or false); }
      { "0.6.22".default = (f.regex_syntax."0.6.22".default or true); }
    ];
  }) [];


# end
# remove_dir_all-0.5.3

  crates.remove_dir_all."0.5.3" = deps: { features?(features_.remove_dir_all."0.5.3" deps {}) }: buildRustCrate {
    crateName = "remove_dir_all";
    version = "0.5.3";
    description = "A safe, reliable implementation of remove_dir_all for Windows";
    authors = [ "Aaronepower <theaaronepower@gmail.com>" ];
    sha256 = "0djicj9b4sighqykdd9sfysbzp97fwc0m6nwbzq4qdbbpf97klll";
    dependencies = (if kernel == "windows" then mapFeatures features ([
      (crates."winapi"."${deps."remove_dir_all"."0.5.3"."winapi"}" deps)
    ]) else []);
  };
  features_.remove_dir_all."0.5.3" = deps: f: updateFeatures f (rec {
    remove_dir_all."0.5.3".default = (f.remove_dir_all."0.5.3".default or true);
    winapi = fold recursiveUpdate {} [
      { "${deps.remove_dir_all."0.5.3".winapi}"."errhandlingapi" = true; }
      { "${deps.remove_dir_all."0.5.3".winapi}"."fileapi" = true; }
      { "${deps.remove_dir_all."0.5.3".winapi}"."std" = true; }
      { "${deps.remove_dir_all."0.5.3".winapi}"."winbase" = true; }
      { "${deps.remove_dir_all."0.5.3".winapi}"."winerror" = true; }
      { "${deps.remove_dir_all."0.5.3".winapi}".default = true; }
    ];
  }) [
    (features_.winapi."${deps."remove_dir_all"."0.5.3"."winapi"}" deps)
  ];


# end
# rust-argon2-0.7.0

  crates.rust_argon2."0.7.0" = deps: { features?(features_.rust_argon2."0.7.0" deps {}) }: buildRustCrate {
    crateName = "rust-argon2";
    version = "0.7.0";
    description = "Rust implementation of the Argon2 password hashing function.";
    authors = [ "Martijn Rijkeboer <mrr@sru-systems.com>" ];
    edition = "2018";
    sha256 = "0xsg3i35nmbj36jdpwn7gwg1xck42a4z4p2c9j178f8p3jlkayb9";
    libName = "argon2";
    dependencies = mapFeatures features ([
      (crates."base64"."${deps."rust_argon2"."0.7.0"."base64"}" deps)
      (crates."blake2b_simd"."${deps."rust_argon2"."0.7.0"."blake2b_simd"}" deps)
      (crates."constant_time_eq"."${deps."rust_argon2"."0.7.0"."constant_time_eq"}" deps)
      (crates."crossbeam_utils"."${deps."rust_argon2"."0.7.0"."crossbeam_utils"}" deps)
    ]);
  };
  features_.rust_argon2."0.7.0" = deps: f: updateFeatures f (rec {
    base64."${deps.rust_argon2."0.7.0".base64}".default = true;
    blake2b_simd."${deps.rust_argon2."0.7.0".blake2b_simd}".default = true;
    constant_time_eq."${deps.rust_argon2."0.7.0".constant_time_eq}".default = true;
    crossbeam_utils."${deps.rust_argon2."0.7.0".crossbeam_utils}".default = true;
    rust_argon2."0.7.0".default = (f.rust_argon2."0.7.0".default or true);
  }) [
    (features_.base64."${deps."rust_argon2"."0.7.0"."base64"}" deps)
    (features_.blake2b_simd."${deps."rust_argon2"."0.7.0"."blake2b_simd"}" deps)
    (features_.constant_time_eq."${deps."rust_argon2"."0.7.0"."constant_time_eq"}" deps)
    (features_.crossbeam_utils."${deps."rust_argon2"."0.7.0"."crossbeam_utils"}" deps)
  ];


# end
# rustc-demangle-0.1.16

  crates.rustc_demangle."0.1.16" = deps: { features?(features_.rustc_demangle."0.1.16" deps {}) }: buildRustCrate {
    crateName = "rustc-demangle";
    version = "0.1.16";
    description = "Rust compiler symbol demangling.\n";
    authors = [ "Alex Crichton <alex@alexcrichton.com>" ];
    sha256 = "0zmn448d0f898ahfkz7cir0fi0vk84dabjpw84mk6a1r6nf9vzmi";
    dependencies = mapFeatures features ([
]);
    features = mkFeatures (features."rustc_demangle"."0.1.16" or {});
  };
  features_.rustc_demangle."0.1.16" = deps: f: updateFeatures f (rec {
    rustc_demangle = fold recursiveUpdate {} [
      { "0.1.16"."compiler_builtins" =
        (f.rustc_demangle."0.1.16"."compiler_builtins" or false) ||
        (f.rustc_demangle."0.1.16".rustc-dep-of-std or false) ||
        (rustc_demangle."0.1.16"."rustc-dep-of-std" or false); }
      { "0.1.16"."core" =
        (f.rustc_demangle."0.1.16"."core" or false) ||
        (f.rustc_demangle."0.1.16".rustc-dep-of-std or false) ||
        (rustc_demangle."0.1.16"."rustc-dep-of-std" or false); }
      { "0.1.16".default = (f.rustc_demangle."0.1.16".default or true); }
    ];
  }) [];


# end
# ryu-1.0.5

  crates.ryu."1.0.5" = deps: { features?(features_.ryu."1.0.5" deps {}) }: buildRustCrate {
    crateName = "ryu";
    version = "1.0.5";
    description = "Fast floating point to string conversion";
    authors = [ "David Tolnay <dtolnay@gmail.com>" ];
    edition = "2018";
    sha256 = "060y2ln1csix593ingwxr2y3wl236ls0ly1ffkv39h5im7xydhrc";
    dependencies = mapFeatures features ([
]);
    features = mkFeatures (features."ryu"."1.0.5" or {});
  };
  features_.ryu."1.0.5" = deps: f: updateFeatures f (rec {
    ryu."1.0.5".default = (f.ryu."1.0.5".default or true);
  }) [];


# end
# same-file-1.0.6

  crates.same_file."1.0.6" = deps: { features?(features_.same_file."1.0.6" deps {}) }: buildRustCrate {
    crateName = "same-file";
    version = "1.0.6";
    description = "A simple crate for determining whether two file paths point to the same file.\n";
    authors = [ "Andrew Gallant <jamslam@gmail.com>" ];
    edition = "2018";
    sha256 = "1cxk0l015nkr3n0hs8wkkc0mpni0yn6a06r0jxqv4r61sgl227mz";
    dependencies = (if kernel == "windows" then mapFeatures features ([
      (crates."winapi_util"."${deps."same_file"."1.0.6"."winapi_util"}" deps)
    ]) else []);
  };
  features_.same_file."1.0.6" = deps: f: updateFeatures f (rec {
    same_file."1.0.6".default = (f.same_file."1.0.6".default or true);
    winapi_util."${deps.same_file."1.0.6".winapi_util}".default = true;
  }) [
    (features_.winapi_util."${deps."same_file"."1.0.6"."winapi_util"}" deps)
  ];


# end
# serde-1.0.114

  crates.serde."1.0.114" = deps: { features?(features_.serde."1.0.114" deps {}) }: buildRustCrate {
    crateName = "serde";
    version = "1.0.114";
    description = "A generic serialization/deserialization framework";
    authors = [ "Erick Tryzelaar <erick.tryzelaar@gmail.com>" "David Tolnay <dtolnay@gmail.com>" ];
    sha256 = "108wasih2s7d77qhfw2wjda54r309jvhr83ifvvzdp3vjahrfk8i";
    build = "build.rs";
    dependencies = mapFeatures features ([
]);
    features = mkFeatures (features."serde"."1.0.114" or {});
  };
  features_.serde."1.0.114" = deps: f: updateFeatures f (rec {
    serde = fold recursiveUpdate {} [
      { "1.0.114"."serde_derive" =
        (f.serde."1.0.114"."serde_derive" or false) ||
        (f.serde."1.0.114".derive or false) ||
        (serde."1.0.114"."derive" or false); }
      { "1.0.114"."std" =
        (f.serde."1.0.114"."std" or false) ||
        (f.serde."1.0.114".default or false) ||
        (serde."1.0.114"."default" or false); }
      { "1.0.114".default = (f.serde."1.0.114".default or true); }
    ];
  }) [];


# end
# serde_derive-1.0.114

  crates.serde_derive."1.0.114" = deps: { features?(features_.serde_derive."1.0.114" deps {}) }: buildRustCrate {
    crateName = "serde_derive";
    version = "1.0.114";
    description = "Macros 1.1 implementation of #[derive(Serialize, Deserialize)]";
    authors = [ "Erick Tryzelaar <erick.tryzelaar@gmail.com>" "David Tolnay <dtolnay@gmail.com>" ];
    sha256 = "1inmayqc8z5siy2pwa7c4gclz7y70618zl3q9byvgy5mnzpbcjfv";
    procMacro = true;
    dependencies = mapFeatures features ([
      (crates."proc_macro2"."${deps."serde_derive"."1.0.114"."proc_macro2"}" deps)
      (crates."quote"."${deps."serde_derive"."1.0.114"."quote"}" deps)
      (crates."syn"."${deps."serde_derive"."1.0.114"."syn"}" deps)
    ]);
    features = mkFeatures (features."serde_derive"."1.0.114" or {});
  };
  features_.serde_derive."1.0.114" = deps: f: updateFeatures f (rec {
    proc_macro2."${deps.serde_derive."1.0.114".proc_macro2}".default = true;
    quote."${deps.serde_derive."1.0.114".quote}".default = true;
    serde_derive."1.0.114".default = (f.serde_derive."1.0.114".default or true);
    syn = fold recursiveUpdate {} [
      { "${deps.serde_derive."1.0.114".syn}"."visit" = true; }
      { "${deps.serde_derive."1.0.114".syn}".default = true; }
    ];
  }) [
    (features_.proc_macro2."${deps."serde_derive"."1.0.114"."proc_macro2"}" deps)
    (features_.quote."${deps."serde_derive"."1.0.114"."quote"}" deps)
    (features_.syn."${deps."serde_derive"."1.0.114"."syn"}" deps)
  ];


# end
# serde_json-1.0.55

  crates.serde_json."1.0.55" = deps: { features?(features_.serde_json."1.0.55" deps {}) }: buildRustCrate {
    crateName = "serde_json";
    version = "1.0.55";
    description = "A JSON serialization file format";
    authors = [ "Erick Tryzelaar <erick.tryzelaar@gmail.com>" "David Tolnay <dtolnay@gmail.com>" ];
    edition = "2018";
    sha256 = "1aj2nr1nska7dyqgbpzm5zvmjsy4xb1yip970vp94af8i3dx58q5";
    dependencies = mapFeatures features ([
      (crates."itoa"."${deps."serde_json"."1.0.55"."itoa"}" deps)
      (crates."ryu"."${deps."serde_json"."1.0.55"."ryu"}" deps)
      (crates."serde"."${deps."serde_json"."1.0.55"."serde"}" deps)
    ]);
    features = mkFeatures (features."serde_json"."1.0.55" or {});
  };
  features_.serde_json."1.0.55" = deps: f: updateFeatures f (rec {
    itoa."${deps.serde_json."1.0.55".itoa}".default = (f.itoa."${deps.serde_json."1.0.55".itoa}".default or false);
    ryu."${deps.serde_json."1.0.55".ryu}".default = true;
    serde = fold recursiveUpdate {} [
      { "${deps.serde_json."1.0.55".serde}"."alloc" =
        (f.serde."${deps.serde_json."1.0.55".serde}"."alloc" or false) ||
        (serde_json."1.0.55"."alloc" or false) ||
        (f."serde_json"."1.0.55"."alloc" or false); }
      { "${deps.serde_json."1.0.55".serde}"."std" =
        (f.serde."${deps.serde_json."1.0.55".serde}"."std" or false) ||
        (serde_json."1.0.55"."std" or false) ||
        (f."serde_json"."1.0.55"."std" or false); }
      { "${deps.serde_json."1.0.55".serde}".default = (f.serde."${deps.serde_json."1.0.55".serde}".default or false); }
    ];
    serde_json = fold recursiveUpdate {} [
      { "1.0.55"."indexmap" =
        (f.serde_json."1.0.55"."indexmap" or false) ||
        (f.serde_json."1.0.55".preserve_order or false) ||
        (serde_json."1.0.55"."preserve_order" or false); }
      { "1.0.55"."std" =
        (f.serde_json."1.0.55"."std" or false) ||
        (f.serde_json."1.0.55".default or false) ||
        (serde_json."1.0.55"."default" or false); }
      { "1.0.55".default = (f.serde_json."1.0.55".default or true); }
    ];
  }) [
    (features_.itoa."${deps."serde_json"."1.0.55"."itoa"}" deps)
    (features_.ryu."${deps."serde_json"."1.0.55"."ryu"}" deps)
    (features_.serde."${deps."serde_json"."1.0.55"."serde"}" deps)
  ];


# end
# slab-0.4.2

  crates.slab."0.4.2" = deps: { features?(features_.slab."0.4.2" deps {}) }: buildRustCrate {
    crateName = "slab";
    version = "0.4.2";
    description = "Pre-allocated storage for a uniform data type";
    authors = [ "Carl Lerche <me@carllerche.com>" ];
    sha256 = "0h1l2z7qy6207kv0v3iigdf2xfk9yrhbwj1svlxk6wxjmdxvgdl7";
  };
  features_.slab."0.4.2" = deps: f: updateFeatures f (rec {
    slab."0.4.2".default = (f.slab."0.4.2".default or true);
  }) [];


# end
# slog-2.7.0

  crates.slog."2.7.0" = deps: { features?(features_.slog."2.7.0" deps {}) }: buildRustCrate {
    crateName = "slog";
    version = "2.7.0";
    description = "Structured, extensible, composable logging for Rust";
    authors = [ "Dawid Ciężarkiewicz <dpc@dpc.pw>" ];
    sha256 = "0jgfignj5x3ynv6ikmbpncp8xgvq02224fpcg8acjy5n20gk8gii";
    build = "build.rs";
    dependencies = mapFeatures features ([
]);
    features = mkFeatures (features."slog"."2.7.0" or {});
  };
  features_.slog."2.7.0" = deps: f: updateFeatures f (rec {
    slog = fold recursiveUpdate {} [
      { "2.7.0"."erased-serde" =
        (f.slog."2.7.0"."erased-serde" or false) ||
        (f.slog."2.7.0".nested-values or false) ||
        (slog."2.7.0"."nested-values" or false); }
      { "2.7.0"."std" =
        (f.slog."2.7.0"."std" or false) ||
        (f.slog."2.7.0".default or false) ||
        (slog."2.7.0"."default" or false); }
      { "2.7.0".default = (f.slog."2.7.0".default or true); }
    ];
  }) [];


# end
# slog-scope-4.3.0

  crates.slog_scope."4.3.0" = deps: { features?(features_.slog_scope."4.3.0" deps {}) }: buildRustCrate {
    crateName = "slog-scope";
    version = "4.3.0";
    description = "Logging scopes for slog-rs";
    authors = [ "Dawid Ciężarkiewicz <dpc@dpc.pw>" ];
    sha256 = "1hyy13j664r5cn3s2w79737mpv3p2i48lk78gmb4i9jsnzvzbk24";
    libPath = "lib.rs";
    dependencies = mapFeatures features ([
      (crates."arc_swap"."${deps."slog_scope"."4.3.0"."arc_swap"}" deps)
      (crates."lazy_static"."${deps."slog_scope"."4.3.0"."lazy_static"}" deps)
      (crates."slog"."${deps."slog_scope"."4.3.0"."slog"}" deps)
    ]);
  };
  features_.slog_scope."4.3.0" = deps: f: updateFeatures f (rec {
    arc_swap."${deps.slog_scope."4.3.0".arc_swap}".default = true;
    lazy_static."${deps.slog_scope."4.3.0".lazy_static}".default = true;
    slog."${deps.slog_scope."4.3.0".slog}".default = true;
    slog_scope."4.3.0".default = (f.slog_scope."4.3.0".default or true);
  }) [
    (features_.arc_swap."${deps."slog_scope"."4.3.0"."arc_swap"}" deps)
    (features_.lazy_static."${deps."slog_scope"."4.3.0"."lazy_static"}" deps)
    (features_.slog."${deps."slog_scope"."4.3.0"."slog"}" deps)
  ];


# end
# slog-term-2.6.0

  crates.slog_term."2.6.0" = deps: { features?(features_.slog_term."2.6.0" deps {}) }: buildRustCrate {
    crateName = "slog-term";
    version = "2.6.0";
    description = "Unix terminal drain and formatter for slog-rs";
    authors = [ "Dawid Ciężarkiewicz <dpc@dpc.pw>" ];
    edition = "2018";
    sha256 = "1hz9z96lhlkipvjc1whg6ggjpjg2phgd926fyas054bj2ncfydkr";
    dependencies = mapFeatures features ([
      (crates."atty"."${deps."slog_term"."2.6.0"."atty"}" deps)
      (crates."chrono"."${deps."slog_term"."2.6.0"."chrono"}" deps)
      (crates."slog"."${deps."slog_term"."2.6.0"."slog"}" deps)
      (crates."term"."${deps."slog_term"."2.6.0"."term"}" deps)
      (crates."thread_local"."${deps."slog_term"."2.6.0"."thread_local"}" deps)
    ]);
    features = mkFeatures (features."slog_term"."2.6.0" or {});
  };
  features_.slog_term."2.6.0" = deps: f: updateFeatures f (rec {
    atty."${deps.slog_term."2.6.0".atty}".default = true;
    chrono."${deps.slog_term."2.6.0".chrono}".default = true;
    slog = fold recursiveUpdate {} [
      { "${deps.slog_term."2.6.0".slog}"."nested-values" =
        (f.slog."${deps.slog_term."2.6.0".slog}"."nested-values" or false) ||
        (slog_term."2.6.0"."nested-values" or false) ||
        (f."slog_term"."2.6.0"."nested-values" or false); }
      { "${deps.slog_term."2.6.0".slog}".default = true; }
    ];
    slog_term = fold recursiveUpdate {} [
      { "2.6.0"."erased-serde" =
        (f.slog_term."2.6.0"."erased-serde" or false) ||
        (f.slog_term."2.6.0".nested-values or false) ||
        (slog_term."2.6.0"."nested-values" or false); }
      { "2.6.0"."serde" =
        (f.slog_term."2.6.0"."serde" or false) ||
        (f.slog_term."2.6.0".nested-values or false) ||
        (slog_term."2.6.0"."nested-values" or false); }
      { "2.6.0"."serde_json" =
        (f.slog_term."2.6.0"."serde_json" or false) ||
        (f.slog_term."2.6.0".nested-values or false) ||
        (slog_term."2.6.0"."nested-values" or false); }
      { "2.6.0".default = (f.slog_term."2.6.0".default or true); }
    ];
    term."${deps.slog_term."2.6.0".term}".default = true;
    thread_local."${deps.slog_term."2.6.0".thread_local}".default = true;
  }) [
    (features_.atty."${deps."slog_term"."2.6.0"."atty"}" deps)
    (features_.chrono."${deps."slog_term"."2.6.0"."chrono"}" deps)
    (features_.slog."${deps."slog_term"."2.6.0"."slog"}" deps)
    (features_.term."${deps."slog_term"."2.6.0"."term"}" deps)
    (features_.thread_local."${deps."slog_term"."2.6.0"."thread_local"}" deps)
  ];


# end
# smallvec-0.6.13

  crates.smallvec."0.6.13" = deps: { features?(features_.smallvec."0.6.13" deps {}) }: buildRustCrate {
    crateName = "smallvec";
    version = "0.6.13";
    description = "'Small vector' optimization: store up to a small number of items on the stack";
    authors = [ "Simon Sapin <simon.sapin@exyr.org>" ];
    sha256 = "15784fxgp1bvld5pbhb3171rv4kwvvy2p83jlyr0smp5hqg2b68w";
    libPath = "lib.rs";
    dependencies = mapFeatures features ([
      (crates."maybe_uninit"."${deps."smallvec"."0.6.13"."maybe_uninit"}" deps)
    ]);
    features = mkFeatures (features."smallvec"."0.6.13" or {});
  };
  features_.smallvec."0.6.13" = deps: f: updateFeatures f (rec {
    maybe_uninit."${deps.smallvec."0.6.13".maybe_uninit}".default = true;
    smallvec = fold recursiveUpdate {} [
      { "0.6.13"."std" =
        (f.smallvec."0.6.13"."std" or false) ||
        (f.smallvec."0.6.13".default or false) ||
        (smallvec."0.6.13"."default" or false); }
      { "0.6.13".default = (f.smallvec."0.6.13".default or true); }
    ];
  }) [
    (features_.maybe_uninit."${deps."smallvec"."0.6.13"."maybe_uninit"}" deps)
  ];


# end
# stable_deref_trait-1.1.1

  crates.stable_deref_trait."1.1.1" = deps: { features?(features_.stable_deref_trait."1.1.1" deps {}) }: buildRustCrate {
    crateName = "stable_deref_trait";
    version = "1.1.1";
    description = "An unsafe marker trait for types like Box and Rc that dereference to a stable address even when moved, and hence can be used with libraries such as owning_ref and rental.\n";
    authors = [ "Robert Grosse <n210241048576@gmail.com>" ];
    sha256 = "1xy9slzslrzr31nlnw52sl1d820b09y61b7f13lqgsn8n7y0l4g8";
    features = mkFeatures (features."stable_deref_trait"."1.1.1" or {});
  };
  features_.stable_deref_trait."1.1.1" = deps: f: updateFeatures f (rec {
    stable_deref_trait = fold recursiveUpdate {} [
      { "1.1.1"."std" =
        (f.stable_deref_trait."1.1.1"."std" or false) ||
        (f.stable_deref_trait."1.1.1".default or false) ||
        (stable_deref_trait."1.1.1"."default" or false); }
      { "1.1.1".default = (f.stable_deref_trait."1.1.1".default or true); }
    ];
  }) [];


# end
# strsim-0.8.0

  crates.strsim."0.8.0" = deps: { features?(features_.strsim."0.8.0" deps {}) }: buildRustCrate {
    crateName = "strsim";
    version = "0.8.0";
    description = "Implementations of string similarity metrics.\nIncludes Hamming, Levenshtein, OSA, Damerau-Levenshtein, Jaro, and Jaro-Winkler.\n";
    authors = [ "Danny Guo <dannyguo91@gmail.com>" ];
    sha256 = "0d3jsdz22wgjyxdakqnvdgmwjdvkximz50d9zfk4qlalw635qcvy";
  };
  features_.strsim."0.8.0" = deps: f: updateFeatures f (rec {
    strsim."0.8.0".default = (f.strsim."0.8.0".default or true);
  }) [];


# end
# structopt-0.2.18

  crates.structopt."0.2.18" = deps: { features?(features_.structopt."0.2.18" deps {}) }: buildRustCrate {
    crateName = "structopt";
    version = "0.2.18";
    description = "Parse command line argument by defining a struct.";
    authors = [ "Guillaume Pinot <texitoi@texitoi.eu>" "others" ];
    sha256 = "096mzwn2d5qsa0k5kxvd1ag38fm5rfrr262fnacfrq5k13ldl9j2";
    dependencies = mapFeatures features ([
      (crates."clap"."${deps."structopt"."0.2.18"."clap"}" deps)
      (crates."structopt_derive"."${deps."structopt"."0.2.18"."structopt_derive"}" deps)
    ]);
    features = mkFeatures (features."structopt"."0.2.18" or {});
  };
  features_.structopt."0.2.18" = deps: f: updateFeatures f (rec {
    clap = fold recursiveUpdate {} [
      { "${deps.structopt."0.2.18".clap}"."color" =
        (f.clap."${deps.structopt."0.2.18".clap}"."color" or false) ||
        (structopt."0.2.18"."color" or false) ||
        (f."structopt"."0.2.18"."color" or false); }
      { "${deps.structopt."0.2.18".clap}"."debug" =
        (f.clap."${deps.structopt."0.2.18".clap}"."debug" or false) ||
        (structopt."0.2.18"."debug" or false) ||
        (f."structopt"."0.2.18"."debug" or false); }
      { "${deps.structopt."0.2.18".clap}"."default" =
        (f.clap."${deps.structopt."0.2.18".clap}"."default" or false) ||
        (structopt."0.2.18"."default" or false) ||
        (f."structopt"."0.2.18"."default" or false); }
      { "${deps.structopt."0.2.18".clap}"."doc" =
        (f.clap."${deps.structopt."0.2.18".clap}"."doc" or false) ||
        (structopt."0.2.18"."doc" or false) ||
        (f."structopt"."0.2.18"."doc" or false); }
      { "${deps.structopt."0.2.18".clap}"."lints" =
        (f.clap."${deps.structopt."0.2.18".clap}"."lints" or false) ||
        (structopt."0.2.18"."lints" or false) ||
        (f."structopt"."0.2.18"."lints" or false); }
      { "${deps.structopt."0.2.18".clap}"."no_cargo" =
        (f.clap."${deps.structopt."0.2.18".clap}"."no_cargo" or false) ||
        (structopt."0.2.18"."no_cargo" or false) ||
        (f."structopt"."0.2.18"."no_cargo" or false); }
      { "${deps.structopt."0.2.18".clap}"."suggestions" =
        (f.clap."${deps.structopt."0.2.18".clap}"."suggestions" or false) ||
        (structopt."0.2.18"."suggestions" or false) ||
        (f."structopt"."0.2.18"."suggestions" or false); }
      { "${deps.structopt."0.2.18".clap}"."wrap_help" =
        (f.clap."${deps.structopt."0.2.18".clap}"."wrap_help" or false) ||
        (structopt."0.2.18"."wrap_help" or false) ||
        (f."structopt"."0.2.18"."wrap_help" or false); }
      { "${deps.structopt."0.2.18".clap}"."yaml" =
        (f.clap."${deps.structopt."0.2.18".clap}"."yaml" or false) ||
        (structopt."0.2.18"."yaml" or false) ||
        (f."structopt"."0.2.18"."yaml" or false); }
      { "${deps.structopt."0.2.18".clap}".default = (f.clap."${deps.structopt."0.2.18".clap}".default or false); }
    ];
    structopt."0.2.18".default = (f.structopt."0.2.18".default or true);
    structopt_derive = fold recursiveUpdate {} [
      { "${deps.structopt."0.2.18".structopt_derive}"."nightly" =
        (f.structopt_derive."${deps.structopt."0.2.18".structopt_derive}"."nightly" or false) ||
        (structopt."0.2.18"."nightly" or false) ||
        (f."structopt"."0.2.18"."nightly" or false); }
      { "${deps.structopt."0.2.18".structopt_derive}"."paw" =
        (f.structopt_derive."${deps.structopt."0.2.18".structopt_derive}"."paw" or false) ||
        (structopt."0.2.18"."paw" or false) ||
        (f."structopt"."0.2.18"."paw" or false); }
      { "${deps.structopt."0.2.18".structopt_derive}".default = true; }
    ];
  }) [
    (features_.clap."${deps."structopt"."0.2.18"."clap"}" deps)
    (features_.structopt_derive."${deps."structopt"."0.2.18"."structopt_derive"}" deps)
  ];


# end
# structopt-derive-0.2.18

  crates.structopt_derive."0.2.18" = deps: { features?(features_.structopt_derive."0.2.18" deps {}) }: buildRustCrate {
    crateName = "structopt-derive";
    version = "0.2.18";
    description = "Parse command line argument by defining a struct, derive crate.";
    authors = [ "Guillaume Pinot <texitoi@texitoi.eu>" ];
    sha256 = "0wrhvq92psxa62jx6ypyhld7d5l3l7va0s0qwy1mq7c863wnhp7p";
    procMacro = true;
    dependencies = mapFeatures features ([
      (crates."heck"."${deps."structopt_derive"."0.2.18"."heck"}" deps)
      (crates."proc_macro2"."${deps."structopt_derive"."0.2.18"."proc_macro2"}" deps)
      (crates."quote"."${deps."structopt_derive"."0.2.18"."quote"}" deps)
      (crates."syn"."${deps."structopt_derive"."0.2.18"."syn"}" deps)
    ]);
    features = mkFeatures (features."structopt_derive"."0.2.18" or {});
  };
  features_.structopt_derive."0.2.18" = deps: f: updateFeatures f (rec {
    heck."${deps.structopt_derive."0.2.18".heck}".default = true;
    proc_macro2 = fold recursiveUpdate {} [
      { "${deps.structopt_derive."0.2.18".proc_macro2}"."nightly" =
        (f.proc_macro2."${deps.structopt_derive."0.2.18".proc_macro2}"."nightly" or false) ||
        (structopt_derive."0.2.18"."nightly" or false) ||
        (f."structopt_derive"."0.2.18"."nightly" or false); }
      { "${deps.structopt_derive."0.2.18".proc_macro2}".default = true; }
    ];
    quote."${deps.structopt_derive."0.2.18".quote}".default = true;
    structopt_derive."0.2.18".default = (f.structopt_derive."0.2.18".default or true);
    syn."${deps.structopt_derive."0.2.18".syn}".default = true;
  }) [
    (features_.heck."${deps."structopt_derive"."0.2.18"."heck"}" deps)
    (features_.proc_macro2."${deps."structopt_derive"."0.2.18"."proc_macro2"}" deps)
    (features_.quote."${deps."structopt_derive"."0.2.18"."quote"}" deps)
    (features_.syn."${deps."structopt_derive"."0.2.18"."syn"}" deps)
  ];


# end
# syn-0.15.44

  crates.syn."0.15.44" = deps: { features?(features_.syn."0.15.44" deps {}) }: buildRustCrate {
    crateName = "syn";
    version = "0.15.44";
    description = "Parser for Rust source code";
    authors = [ "David Tolnay <dtolnay@gmail.com>" ];
    sha256 = "09v11h141grmsnamd5j14mn8vpnfng6p60kdmsm8akz9m0qn7s1n";
    dependencies = mapFeatures features ([
      (crates."proc_macro2"."${deps."syn"."0.15.44"."proc_macro2"}" deps)
      (crates."unicode_xid"."${deps."syn"."0.15.44"."unicode_xid"}" deps)
    ]
      ++ (if features.syn."0.15.44".quote or false then [ (crates.quote."${deps."syn"."0.15.44".quote}" deps) ] else []));
    features = mkFeatures (features."syn"."0.15.44" or {});
  };
  features_.syn."0.15.44" = deps: f: updateFeatures f (rec {
    proc_macro2 = fold recursiveUpdate {} [
      { "${deps.syn."0.15.44".proc_macro2}"."proc-macro" =
        (f.proc_macro2."${deps.syn."0.15.44".proc_macro2}"."proc-macro" or false) ||
        (syn."0.15.44"."proc-macro" or false) ||
        (f."syn"."0.15.44"."proc-macro" or false); }
      { "${deps.syn."0.15.44".proc_macro2}".default = (f.proc_macro2."${deps.syn."0.15.44".proc_macro2}".default or false); }
    ];
    quote = fold recursiveUpdate {} [
      { "${deps.syn."0.15.44".quote}"."proc-macro" =
        (f.quote."${deps.syn."0.15.44".quote}"."proc-macro" or false) ||
        (syn."0.15.44"."proc-macro" or false) ||
        (f."syn"."0.15.44"."proc-macro" or false); }
      { "${deps.syn."0.15.44".quote}".default = (f.quote."${deps.syn."0.15.44".quote}".default or false); }
    ];
    syn = fold recursiveUpdate {} [
      { "0.15.44"."clone-impls" =
        (f.syn."0.15.44"."clone-impls" or false) ||
        (f.syn."0.15.44".default or false) ||
        (syn."0.15.44"."default" or false); }
      { "0.15.44"."derive" =
        (f.syn."0.15.44"."derive" or false) ||
        (f.syn."0.15.44".default or false) ||
        (syn."0.15.44"."default" or false); }
      { "0.15.44"."parsing" =
        (f.syn."0.15.44"."parsing" or false) ||
        (f.syn."0.15.44".default or false) ||
        (syn."0.15.44"."default" or false); }
      { "0.15.44"."printing" =
        (f.syn."0.15.44"."printing" or false) ||
        (f.syn."0.15.44".default or false) ||
        (syn."0.15.44"."default" or false); }
      { "0.15.44"."proc-macro" =
        (f.syn."0.15.44"."proc-macro" or false) ||
        (f.syn."0.15.44".default or false) ||
        (syn."0.15.44"."default" or false); }
      { "0.15.44"."quote" =
        (f.syn."0.15.44"."quote" or false) ||
        (f.syn."0.15.44".printing or false) ||
        (syn."0.15.44"."printing" or false); }
      { "0.15.44".default = (f.syn."0.15.44".default or true); }
    ];
    unicode_xid."${deps.syn."0.15.44".unicode_xid}".default = true;
  }) [
    (features_.proc_macro2."${deps."syn"."0.15.44"."proc_macro2"}" deps)
    (features_.quote."${deps."syn"."0.15.44"."quote"}" deps)
    (features_.unicode_xid."${deps."syn"."0.15.44"."unicode_xid"}" deps)
  ];


# end
# syn-1.0.64

  crates.syn."1.0.64" = deps: { features?(features_.syn."1.0.64" deps {}) }: buildRustCrate {
    crateName = "syn";
    version = "1.0.64";
    description = "Parser for Rust source code";
    authors = [ "David Tolnay <dtolnay@gmail.com>" ];
    edition = "2018";
    sha256 = "0m1k4jnwa4cb1f8ryy35fbs7knisah7s1bn7zva2s35r8bnhh7rf";
    dependencies = mapFeatures features ([
      (crates."proc_macro2"."${deps."syn"."1.0.64"."proc_macro2"}" deps)
      (crates."unicode_xid"."${deps."syn"."1.0.64"."unicode_xid"}" deps)
    ]
      ++ (if features.syn."1.0.64".quote or false then [ (crates.quote."${deps."syn"."1.0.64".quote}" deps) ] else []));
    features = mkFeatures (features."syn"."1.0.64" or {});
  };
  features_.syn."1.0.64" = deps: f: updateFeatures f (rec {
    proc_macro2 = fold recursiveUpdate {} [
      { "${deps.syn."1.0.64".proc_macro2}"."proc-macro" =
        (f.proc_macro2."${deps.syn."1.0.64".proc_macro2}"."proc-macro" or false) ||
        (syn."1.0.64"."proc-macro" or false) ||
        (f."syn"."1.0.64"."proc-macro" or false); }
      { "${deps.syn."1.0.64".proc_macro2}".default = (f.proc_macro2."${deps.syn."1.0.64".proc_macro2}".default or false); }
    ];
    quote = fold recursiveUpdate {} [
      { "${deps.syn."1.0.64".quote}"."proc-macro" =
        (f.quote."${deps.syn."1.0.64".quote}"."proc-macro" or false) ||
        (syn."1.0.64"."proc-macro" or false) ||
        (f."syn"."1.0.64"."proc-macro" or false); }
      { "${deps.syn."1.0.64".quote}".default = (f.quote."${deps.syn."1.0.64".quote}".default or false); }
    ];
    syn = fold recursiveUpdate {} [
      { "1.0.64"."clone-impls" =
        (f.syn."1.0.64"."clone-impls" or false) ||
        (f.syn."1.0.64".default or false) ||
        (syn."1.0.64"."default" or false); }
      { "1.0.64"."derive" =
        (f.syn."1.0.64"."derive" or false) ||
        (f.syn."1.0.64".default or false) ||
        (syn."1.0.64"."default" or false); }
      { "1.0.64"."parsing" =
        (f.syn."1.0.64"."parsing" or false) ||
        (f.syn."1.0.64".default or false) ||
        (syn."1.0.64"."default" or false); }
      { "1.0.64"."printing" =
        (f.syn."1.0.64"."printing" or false) ||
        (f.syn."1.0.64".default or false) ||
        (syn."1.0.64"."default" or false); }
      { "1.0.64"."proc-macro" =
        (f.syn."1.0.64"."proc-macro" or false) ||
        (f.syn."1.0.64".default or false) ||
        (syn."1.0.64"."default" or false); }
      { "1.0.64"."quote" =
        (f.syn."1.0.64"."quote" or false) ||
        (f.syn."1.0.64".printing or false) ||
        (syn."1.0.64"."printing" or false); }
      { "1.0.64".default = (f.syn."1.0.64".default or true); }
    ];
    unicode_xid."${deps.syn."1.0.64".unicode_xid}".default = true;
  }) [
    (features_.proc_macro2."${deps."syn"."1.0.64"."proc_macro2"}" deps)
    (features_.quote."${deps."syn"."1.0.64"."quote"}" deps)
    (features_.unicode_xid."${deps."syn"."1.0.64"."unicode_xid"}" deps)
  ];


# end
# tempdir-0.3.7

  crates.tempdir."0.3.7" = deps: { features?(features_.tempdir."0.3.7" deps {}) }: buildRustCrate {
    crateName = "tempdir";
    version = "0.3.7";
    description = "A library for managing a temporary directory and deleting all contents when it's\ndropped.\n";
    authors = [ "The Rust Project Developers" ];
    sha256 = "0y53sxybyljrr7lh0x0ysrsa7p7cljmwv9v80acy3rc6n97g67vy";
    dependencies = mapFeatures features ([
      (crates."rand"."${deps."tempdir"."0.3.7"."rand"}" deps)
      (crates."remove_dir_all"."${deps."tempdir"."0.3.7"."remove_dir_all"}" deps)
    ]);
  };
  features_.tempdir."0.3.7" = deps: f: updateFeatures f (rec {
    rand."${deps.tempdir."0.3.7".rand}".default = true;
    remove_dir_all."${deps.tempdir."0.3.7".remove_dir_all}".default = true;
    tempdir."0.3.7".default = (f.tempdir."0.3.7".default or true);
  }) [
    (features_.rand."${deps."tempdir"."0.3.7"."rand"}" deps)
    (features_.remove_dir_all."${deps."tempdir"."0.3.7"."remove_dir_all"}" deps)
  ];


# end
# tempfile-3.1.0

  crates.tempfile."3.1.0" = deps: { features?(features_.tempfile."3.1.0" deps {}) }: buildRustCrate {
    crateName = "tempfile";
    version = "3.1.0";
    description = "A library for managing temporary files and directories.";
    authors = [ "Steven Allen <steven@stebalien.com>" "The Rust Project Developers" "Ashley Mannix <ashleymannix@live.com.au>" "Jason White <jasonaw0@gmail.com>" ];
    edition = "2018";
    sha256 = "1r7ykxw90p5hm1g46i8ia33j5iwl3q252kbb6b074qhdav3sqndk";
    dependencies = mapFeatures features ([
      (crates."cfg_if"."${deps."tempfile"."3.1.0"."cfg_if"}" deps)
      (crates."rand"."${deps."tempfile"."3.1.0"."rand"}" deps)
      (crates."remove_dir_all"."${deps."tempfile"."3.1.0"."remove_dir_all"}" deps)
    ])
      ++ (if kernel == "redox" then mapFeatures features ([
      (crates."redox_syscall"."${deps."tempfile"."3.1.0"."redox_syscall"}" deps)
    ]) else [])
      ++ (if (kernel == "linux" || kernel == "darwin") then mapFeatures features ([
      (crates."libc"."${deps."tempfile"."3.1.0"."libc"}" deps)
    ]) else [])
      ++ (if kernel == "windows" then mapFeatures features ([
      (crates."winapi"."${deps."tempfile"."3.1.0"."winapi"}" deps)
    ]) else []);
  };
  features_.tempfile."3.1.0" = deps: f: updateFeatures f (rec {
    cfg_if."${deps.tempfile."3.1.0".cfg_if}".default = true;
    libc."${deps.tempfile."3.1.0".libc}".default = true;
    rand."${deps.tempfile."3.1.0".rand}".default = true;
    redox_syscall."${deps.tempfile."3.1.0".redox_syscall}".default = true;
    remove_dir_all."${deps.tempfile."3.1.0".remove_dir_all}".default = true;
    tempfile."3.1.0".default = (f.tempfile."3.1.0".default or true);
    winapi = fold recursiveUpdate {} [
      { "${deps.tempfile."3.1.0".winapi}"."fileapi" = true; }
      { "${deps.tempfile."3.1.0".winapi}"."handleapi" = true; }
      { "${deps.tempfile."3.1.0".winapi}"."winbase" = true; }
      { "${deps.tempfile."3.1.0".winapi}".default = true; }
    ];
  }) [
    (features_.cfg_if."${deps."tempfile"."3.1.0"."cfg_if"}" deps)
    (features_.rand."${deps."tempfile"."3.1.0"."rand"}" deps)
    (features_.remove_dir_all."${deps."tempfile"."3.1.0"."remove_dir_all"}" deps)
    (features_.redox_syscall."${deps."tempfile"."3.1.0"."redox_syscall"}" deps)
    (features_.libc."${deps."tempfile"."3.1.0"."libc"}" deps)
    (features_.winapi."${deps."tempfile"."3.1.0"."winapi"}" deps)
  ];


# end
# term-0.6.1

  crates.term."0.6.1" = deps: { features?(features_.term."0.6.1" deps {}) }: buildRustCrate {
    crateName = "term";
    version = "0.6.1";
    description = "A terminal formatting library\n";
    authors = [ "The Rust Project Developers" "Steven Allen" ];
    edition = "2018";
    sha256 = "1wdij7b4an6bdr3md7qfy2v70niphl638cw7p34ly5db5r8acl61";
    dependencies = mapFeatures features ([
      (crates."dirs"."${deps."term"."0.6.1"."dirs"}" deps)
    ])
      ++ (if kernel == "windows" then mapFeatures features ([
      (crates."winapi"."${deps."term"."0.6.1"."winapi"}" deps)
    ]) else []);
  };
  features_.term."0.6.1" = deps: f: updateFeatures f (rec {
    dirs."${deps.term."0.6.1".dirs}".default = true;
    term."0.6.1".default = (f.term."0.6.1".default or true);
    winapi = fold recursiveUpdate {} [
      { "${deps.term."0.6.1".winapi}"."consoleapi" = true; }
      { "${deps.term."0.6.1".winapi}"."fileapi" = true; }
      { "${deps.term."0.6.1".winapi}"."handleapi" = true; }
      { "${deps.term."0.6.1".winapi}"."wincon" = true; }
      { "${deps.term."0.6.1".winapi}".default = true; }
    ];
  }) [
    (features_.dirs."${deps."term"."0.6.1"."dirs"}" deps)
    (features_.winapi."${deps."term"."0.6.1"."winapi"}" deps)
  ];


# end
# textwrap-0.11.0

  crates.textwrap."0.11.0" = deps: { features?(features_.textwrap."0.11.0" deps {}) }: buildRustCrate {
    crateName = "textwrap";
    version = "0.11.0";
    description = "Textwrap is a small library for word wrapping, indenting, and\ndedenting strings.\n\nYou can use it to format strings (such as help and error messages) for\ndisplay in commandline applications. It is designed to be efficient\nand handle Unicode characters correctly.\n";
    authors = [ "Martin Geisler <martin@geisler.net>" ];
    sha256 = "0s25qh49n7kjayrdj4q3v0jk0jc6vy88rdw0bvgfxqlscpqpxi7d";
    dependencies = mapFeatures features ([
      (crates."unicode_width"."${deps."textwrap"."0.11.0"."unicode_width"}" deps)
    ]);
  };
  features_.textwrap."0.11.0" = deps: f: updateFeatures f (rec {
    textwrap."0.11.0".default = (f.textwrap."0.11.0".default or true);
    unicode_width."${deps.textwrap."0.11.0".unicode_width}".default = true;
  }) [
    (features_.unicode_width."${deps."textwrap"."0.11.0"."unicode_width"}" deps)
  ];


# end
# thiserror-1.0.24

  crates.thiserror."1.0.24" = deps: { features?(features_.thiserror."1.0.24" deps {}) }: buildRustCrate {
    crateName = "thiserror";
    version = "1.0.24";
    description = "derive(Error)";
    authors = [ "David Tolnay <dtolnay@gmail.com>" ];
    edition = "2018";
    sha256 = "06r6wml3y1n8vsh1i5v1mq25rn9ii2lj0fkhgay82czshrjl35w1";
    dependencies = mapFeatures features ([
      (crates."thiserror_impl"."${deps."thiserror"."1.0.24"."thiserror_impl"}" deps)
    ]);
  };
  features_.thiserror."1.0.24" = deps: f: updateFeatures f (rec {
    thiserror."1.0.24".default = (f.thiserror."1.0.24".default or true);
    thiserror_impl."${deps.thiserror."1.0.24".thiserror_impl}".default = true;
  }) [
    (features_.thiserror_impl."${deps."thiserror"."1.0.24"."thiserror_impl"}" deps)
  ];


# end
# thiserror-impl-1.0.24

  crates.thiserror_impl."1.0.24" = deps: { features?(features_.thiserror_impl."1.0.24" deps {}) }: buildRustCrate {
    crateName = "thiserror-impl";
    version = "1.0.24";
    description = "Implementation detail of the `thiserror` crate";
    authors = [ "David Tolnay <dtolnay@gmail.com>" ];
    edition = "2018";
    sha256 = "1cn96jkmmy7bb76777r3yb0zqa7b1k3glsyvk78y2x8r92h71vwn";
    procMacro = true;
    dependencies = mapFeatures features ([
      (crates."proc_macro2"."${deps."thiserror_impl"."1.0.24"."proc_macro2"}" deps)
      (crates."quote"."${deps."thiserror_impl"."1.0.24"."quote"}" deps)
      (crates."syn"."${deps."thiserror_impl"."1.0.24"."syn"}" deps)
    ]);
  };
  features_.thiserror_impl."1.0.24" = deps: f: updateFeatures f (rec {
    proc_macro2."${deps.thiserror_impl."1.0.24".proc_macro2}".default = true;
    quote."${deps.thiserror_impl."1.0.24".quote}".default = true;
    syn."${deps.thiserror_impl."1.0.24".syn}".default = true;
    thiserror_impl."1.0.24".default = (f.thiserror_impl."1.0.24".default or true);
  }) [
    (features_.proc_macro2."${deps."thiserror_impl"."1.0.24"."proc_macro2"}" deps)
    (features_.quote."${deps."thiserror_impl"."1.0.24"."quote"}" deps)
    (features_.syn."${deps."thiserror_impl"."1.0.24"."syn"}" deps)
  ];


# end
# thread_local-1.0.1

  crates.thread_local."1.0.1" = deps: { features?(features_.thread_local."1.0.1" deps {}) }: buildRustCrate {
    crateName = "thread_local";
    version = "1.0.1";
    description = "Per-object thread-local storage";
    authors = [ "Amanieu d'Antras <amanieu@gmail.com>" ];
    sha256 = "0vs440x0nwpsw30ks6b8f70178y0gl7zhrqydhjykrhn56bj57h7";
    dependencies = mapFeatures features ([
      (crates."lazy_static"."${deps."thread_local"."1.0.1"."lazy_static"}" deps)
    ]);
  };
  features_.thread_local."1.0.1" = deps: f: updateFeatures f (rec {
    lazy_static."${deps.thread_local."1.0.1".lazy_static}".default = true;
    thread_local."1.0.1".default = (f.thread_local."1.0.1".default or true);
  }) [
    (features_.lazy_static."${deps."thread_local"."1.0.1"."lazy_static"}" deps)
  ];


# end
# time-0.1.43

  crates.time."0.1.43" = deps: { features?(features_.time."0.1.43" deps {}) }: buildRustCrate {
    crateName = "time";
    version = "0.1.43";
    description = "Utilities for working with time-related functions in Rust.\n";
    authors = [ "The Rust Project Developers" ];
    sha256 = "1hv2cwyyqrcycy3fapqf094q4qv1vzh9yp95l5k8m2mznjz7r6m0";
    dependencies = mapFeatures features ([
      (crates."libc"."${deps."time"."0.1.43"."libc"}" deps)
    ])
      ++ (if kernel == "windows" then mapFeatures features ([
      (crates."winapi"."${deps."time"."0.1.43"."winapi"}" deps)
    ]) else []);
  };
  features_.time."0.1.43" = deps: f: updateFeatures f (rec {
    libc."${deps.time."0.1.43".libc}".default = true;
    time."0.1.43".default = (f.time."0.1.43".default or true);
    winapi = fold recursiveUpdate {} [
      { "${deps.time."0.1.43".winapi}"."minwinbase" = true; }
      { "${deps.time."0.1.43".winapi}"."minwindef" = true; }
      { "${deps.time."0.1.43".winapi}"."ntdef" = true; }
      { "${deps.time."0.1.43".winapi}"."profileapi" = true; }
      { "${deps.time."0.1.43".winapi}"."std" = true; }
      { "${deps.time."0.1.43".winapi}"."sysinfoapi" = true; }
      { "${deps.time."0.1.43".winapi}"."timezoneapi" = true; }
      { "${deps.time."0.1.43".winapi}".default = true; }
    ];
  }) [
    (features_.libc."${deps."time"."0.1.43"."libc"}" deps)
    (features_.winapi."${deps."time"."0.1.43"."winapi"}" deps)
  ];


# end
# toml-0.5.6

  crates.toml."0.5.6" = deps: { features?(features_.toml."0.5.6" deps {}) }: buildRustCrate {
    crateName = "toml";
    version = "0.5.6";
    description = "A native Rust encoder and decoder of TOML-formatted files and streams. Provides\nimplementations of the standard Serialize/Deserialize traits for TOML data to\nfacilitate deserializing and serializing Rust structures.\n";
    authors = [ "Alex Crichton <alex@alexcrichton.com>" ];
    edition = "2018";
    sha256 = "1c34474di15700wgwa4ns2g3qh2kjbq4az50m5f8bnn4bv2zny49";
    dependencies = mapFeatures features ([
      (crates."serde"."${deps."toml"."0.5.6"."serde"}" deps)
    ]);
    features = mkFeatures (features."toml"."0.5.6" or {});
  };
  features_.toml."0.5.6" = deps: f: updateFeatures f (rec {
    serde."${deps.toml."0.5.6".serde}".default = true;
    toml = fold recursiveUpdate {} [
      { "0.5.6"."indexmap" =
        (f.toml."0.5.6"."indexmap" or false) ||
        (f.toml."0.5.6".preserve_order or false) ||
        (toml."0.5.6"."preserve_order" or false); }
      { "0.5.6".default = (f.toml."0.5.6".default or true); }
    ];
  }) [
    (features_.serde."${deps."toml"."0.5.6"."serde"}" deps)
  ];


# end
# unicode-segmentation-1.6.0

  crates.unicode_segmentation."1.6.0" = deps: { features?(features_.unicode_segmentation."1.6.0" deps {}) }: buildRustCrate {
    crateName = "unicode-segmentation";
    version = "1.6.0";
    description = "This crate provides Grapheme Cluster, Word and Sentence boundaries\naccording to Unicode Standard Annex #29 rules.\n";
    authors = [ "kwantam <kwantam@gmail.com>" "Manish Goregaokar <manishsmail@gmail.com>" ];
    sha256 = "1i9a9gzj4i7iqwrgfs3dagf3h2b9qxdy7bviykhnsjrxm3azgsyc";
    features = mkFeatures (features."unicode_segmentation"."1.6.0" or {});
  };
  features_.unicode_segmentation."1.6.0" = deps: f: updateFeatures f (rec {
    unicode_segmentation."1.6.0".default = (f.unicode_segmentation."1.6.0".default or true);
  }) [];


# end
# unicode-width-0.1.7

  crates.unicode_width."0.1.7" = deps: { features?(features_.unicode_width."0.1.7" deps {}) }: buildRustCrate {
    crateName = "unicode-width";
    version = "0.1.7";
    description = "Determine displayed width of `char` and `str` types\naccording to Unicode Standard Annex #11 rules.\n";
    authors = [ "kwantam <kwantam@gmail.com>" "Manish Goregaokar <manishsmail@gmail.com>" ];
    sha256 = "052w5vx2k332h7ycsxsc61rr7hj0szmfsky94f61228z3znsnq9h";
    dependencies = mapFeatures features ([
]);
    features = mkFeatures (features."unicode_width"."0.1.7" or {});
  };
  features_.unicode_width."0.1.7" = deps: f: updateFeatures f (rec {
    unicode_width = fold recursiveUpdate {} [
      { "0.1.7"."compiler_builtins" =
        (f.unicode_width."0.1.7"."compiler_builtins" or false) ||
        (f.unicode_width."0.1.7".rustc-dep-of-std or false) ||
        (unicode_width."0.1.7"."rustc-dep-of-std" or false); }
      { "0.1.7"."core" =
        (f.unicode_width."0.1.7"."core" or false) ||
        (f.unicode_width."0.1.7".rustc-dep-of-std or false) ||
        (unicode_width."0.1.7"."rustc-dep-of-std" or false); }
      { "0.1.7"."std" =
        (f.unicode_width."0.1.7"."std" or false) ||
        (f.unicode_width."0.1.7".rustc-dep-of-std or false) ||
        (unicode_width."0.1.7"."rustc-dep-of-std" or false); }
      { "0.1.7".default = (f.unicode_width."0.1.7".default or true); }
    ];
  }) [];


# end
# unicode-xid-0.1.0

  crates.unicode_xid."0.1.0" = deps: { features?(features_.unicode_xid."0.1.0" deps {}) }: buildRustCrate {
    crateName = "unicode-xid";
    version = "0.1.0";
    description = "Determine whether characters have the XID_Start\nor XID_Continue properties according to\nUnicode Standard Annex #31.\n";
    authors = [ "erick.tryzelaar <erick.tryzelaar@gmail.com>" "kwantam <kwantam@gmail.com>" ];
    sha256 = "05wdmwlfzxhq3nhsxn6wx4q8dhxzzfb9szsz6wiw092m1rjj01zj";
    features = mkFeatures (features."unicode_xid"."0.1.0" or {});
  };
  features_.unicode_xid."0.1.0" = deps: f: updateFeatures f (rec {
    unicode_xid."0.1.0".default = (f.unicode_xid."0.1.0".default or true);
  }) [];


# end
# unicode-xid-0.2.0

  crates.unicode_xid."0.2.0" = deps: { features?(features_.unicode_xid."0.2.0" deps {}) }: buildRustCrate {
    crateName = "unicode-xid";
    version = "0.2.0";
    description = "Determine whether characters have the XID_Start\nor XID_Continue properties according to\nUnicode Standard Annex #31.\n";
    authors = [ "erick.tryzelaar <erick.tryzelaar@gmail.com>" "kwantam <kwantam@gmail.com>" ];
    sha256 = "1c85gb3p3qhbjvfyjb31m06la4f024jx319k10ig7n47dz2fk8v7";
    features = mkFeatures (features."unicode_xid"."0.2.0" or {});
  };
  features_.unicode_xid."0.2.0" = deps: f: updateFeatures f (rec {
    unicode_xid."0.2.0".default = (f.unicode_xid."0.2.0".default or true);
  }) [];


# end
# uuid-0.8.1

  crates.uuid."0.8.1" = deps: { features?(features_.uuid."0.8.1" deps {}) }: buildRustCrate {
    crateName = "uuid";
    version = "0.8.1";
    description = "A library to generate and parse UUIDs.";
    authors = [ "Ashley Mannix<ashleymannix@live.com.au>" "Christopher Armstrong" "Dylan DPC<dylan.dpc@gmail.com>" "Hunar Roop Kahlon<hunar.roop@gmail.com>" ];
    edition = "2018";
    sha256 = "1xkaidb1cpmvbhjdkjzpbk13rkkb6nbpqbz4kxlzx55xcp1ix1bd";
    dependencies = mapFeatures features ([
    ]
      ++ (if features.uuid."0.8.1".rand or false then [ (crates.rand."${deps."uuid"."0.8.1".rand}" deps) ] else []))
      ++ (if kernel == "windows" then mapFeatures features ([
]) else []);
    features = mkFeatures (features."uuid"."0.8.1" or {});
  };
  features_.uuid."0.8.1" = deps: f: updateFeatures f (rec {
    rand = fold recursiveUpdate {} [
      { "${deps.uuid."0.8.1".rand}"."stdweb" =
        (f.rand."${deps.uuid."0.8.1".rand}"."stdweb" or false) ||
        (uuid."0.8.1"."stdweb" or false) ||
        (f."uuid"."0.8.1"."stdweb" or false); }
      { "${deps.uuid."0.8.1".rand}"."wasm-bindgen" =
        (f.rand."${deps.uuid."0.8.1".rand}"."wasm-bindgen" or false) ||
        (uuid."0.8.1"."wasm-bindgen" or false) ||
        (f."uuid"."0.8.1"."wasm-bindgen" or false); }
      { "${deps.uuid."0.8.1".rand}".default = true; }
    ];
    uuid = fold recursiveUpdate {} [
      { "0.8.1"."md5" =
        (f.uuid."0.8.1"."md5" or false) ||
        (f.uuid."0.8.1".v3 or false) ||
        (uuid."0.8.1"."v3" or false); }
      { "0.8.1"."rand" =
        (f.uuid."0.8.1"."rand" or false) ||
        (f.uuid."0.8.1".v4 or false) ||
        (uuid."0.8.1"."v4" or false); }
      { "0.8.1"."sha1" =
        (f.uuid."0.8.1"."sha1" or false) ||
        (f.uuid."0.8.1".v5 or false) ||
        (uuid."0.8.1"."v5" or false); }
      { "0.8.1"."std" =
        (f.uuid."0.8.1"."std" or false) ||
        (f.uuid."0.8.1".default or false) ||
        (uuid."0.8.1"."default" or false); }
      { "0.8.1"."winapi" =
        (f.uuid."0.8.1"."winapi" or false) ||
        (f.uuid."0.8.1".guid or false) ||
        (uuid."0.8.1"."guid" or false); }
      { "0.8.1".default = (f.uuid."0.8.1".default or true); }
    ];
  }) [
    (features_.rand."${deps."uuid"."0.8.1"."rand"}" deps)
  ];


# end
# vec1-1.5.0

  crates.vec1."1.5.0" = deps: { features?(features_.vec1."1.5.0" deps {}) }: buildRustCrate {
    crateName = "vec1";
    version = "1.5.0";
    description = "a std Vec wrapper assuring that it has at least 1 element";
    authors = [ "Philipp Korber <philippkorber@gmail.com>" ];
    sha256 = "15v8llpri0mamcg76yn9xyk9rvjmymg3lihh9qwh1bxk6dgglvnr";
    dependencies = mapFeatures features ([
]);
    features = mkFeatures (features."vec1"."1.5.0" or {});
  };
  features_.vec1."1.5.0" = deps: f: updateFeatures f (rec {
    vec1."1.5.0".default = (f.vec1."1.5.0".default or true);
  }) [];


# end
# void-1.0.2

  crates.void."1.0.2" = deps: { features?(features_.void."1.0.2" deps {}) }: buildRustCrate {
    crateName = "void";
    version = "1.0.2";
    description = "The uninhabited void type for use in statically impossible cases.";
    authors = [ "Jonathan Reem <jonathan.reem@gmail.com>" ];
    sha256 = "0h1dm0dx8dhf56a83k68mijyxigqhizpskwxfdrs1drwv2cdclv3";
    features = mkFeatures (features."void"."1.0.2" or {});
  };
  features_.void."1.0.2" = deps: f: updateFeatures f (rec {
    void = fold recursiveUpdate {} [
      { "1.0.2"."std" =
        (f.void."1.0.2"."std" or false) ||
        (f.void."1.0.2".default or false) ||
        (void."1.0.2"."default" or false); }
      { "1.0.2".default = (f.void."1.0.2".default or true); }
    ];
  }) [];


# end
# walkdir-2.3.1

  crates.walkdir."2.3.1" = deps: { features?(features_.walkdir."2.3.1" deps {}) }: buildRustCrate {
    crateName = "walkdir";
    version = "2.3.1";
    description = "Recursively walk a directory.";
    authors = [ "Andrew Gallant <jamslam@gmail.com>" ];
    edition = "2018";
    sha256 = "1a6gbhzaqf7hmlhdn7fcxvac83sbc6bkxvrz5d24ldx40hr00nyn";
    dependencies = mapFeatures features ([
      (crates."same_file"."${deps."walkdir"."2.3.1"."same_file"}" deps)
    ])
      ++ (if kernel == "windows" then mapFeatures features ([
      (crates."winapi"."${deps."walkdir"."2.3.1"."winapi"}" deps)
      (crates."winapi_util"."${deps."walkdir"."2.3.1"."winapi_util"}" deps)
    ]) else []);
  };
  features_.walkdir."2.3.1" = deps: f: updateFeatures f (rec {
    same_file."${deps.walkdir."2.3.1".same_file}".default = true;
    walkdir."2.3.1".default = (f.walkdir."2.3.1".default or true);
    winapi = fold recursiveUpdate {} [
      { "${deps.walkdir."2.3.1".winapi}"."std" = true; }
      { "${deps.walkdir."2.3.1".winapi}"."winnt" = true; }
      { "${deps.walkdir."2.3.1".winapi}".default = true; }
    ];
    winapi_util."${deps.walkdir."2.3.1".winapi_util}".default = true;
  }) [
    (features_.same_file."${deps."walkdir"."2.3.1"."same_file"}" deps)
    (features_.winapi."${deps."walkdir"."2.3.1"."winapi"}" deps)
    (features_.winapi_util."${deps."walkdir"."2.3.1"."winapi_util"}" deps)
  ];


# end
# wasi-0.9.0+wasi-snapshot-preview1

  crates.wasi."0.9.0+wasi-snapshot-preview1" = deps: { features?(features_.wasi."0.9.0+wasi-snapshot-preview1" deps {}) }: buildRustCrate {
    crateName = "wasi";
    version = "0.9.0+wasi-snapshot-preview1";
    description = "Experimental WASI API bindings for Rust";
    authors = [ "The Cranelift Project Developers" ];
    edition = "2018";
    sha256 = "0xa6b3rnsmhi13nvs9q51wmavx51yzs5qdbc7bvs0pvs6iar3hsd";
    dependencies = mapFeatures features ([
]);
    features = mkFeatures (features."wasi"."0.9.0+wasi-snapshot-preview1" or {});
  };
  features_.wasi."0.9.0+wasi-snapshot-preview1" = deps: f: updateFeatures f (rec {
    wasi = fold recursiveUpdate {} [
      { "0.9.0+wasi-snapshot-preview1"."compiler_builtins" =
        (f.wasi."0.9.0+wasi-snapshot-preview1"."compiler_builtins" or false) ||
        (f.wasi."0.9.0+wasi-snapshot-preview1".rustc-dep-of-std or false) ||
        (wasi."0.9.0+wasi-snapshot-preview1"."rustc-dep-of-std" or false); }
      { "0.9.0+wasi-snapshot-preview1"."core" =
        (f.wasi."0.9.0+wasi-snapshot-preview1"."core" or false) ||
        (f.wasi."0.9.0+wasi-snapshot-preview1".rustc-dep-of-std or false) ||
        (wasi."0.9.0+wasi-snapshot-preview1"."rustc-dep-of-std" or false); }
      { "0.9.0+wasi-snapshot-preview1"."rustc-std-workspace-alloc" =
        (f.wasi."0.9.0+wasi-snapshot-preview1"."rustc-std-workspace-alloc" or false) ||
        (f.wasi."0.9.0+wasi-snapshot-preview1".rustc-dep-of-std or false) ||
        (wasi."0.9.0+wasi-snapshot-preview1"."rustc-dep-of-std" or false); }
      { "0.9.0+wasi-snapshot-preview1"."std" =
        (f.wasi."0.9.0+wasi-snapshot-preview1"."std" or false) ||
        (f.wasi."0.9.0+wasi-snapshot-preview1".default or false) ||
        (wasi."0.9.0+wasi-snapshot-preview1"."default" or false); }
      { "0.9.0+wasi-snapshot-preview1".default = (f.wasi."0.9.0+wasi-snapshot-preview1".default or true); }
    ];
  }) [];


# end
# winapi-0.2.8

  crates.winapi."0.2.8" = deps: { features?(features_.winapi."0.2.8" deps {}) }: buildRustCrate {
    crateName = "winapi";
    version = "0.2.8";
    description = "Types and constants for WinAPI bindings. See README for list of crates providing function bindings.";
    authors = [ "Peter Atashian <retep998@gmail.com>" ];
    sha256 = "0a45b58ywf12vb7gvj6h3j264nydynmzyqz8d8rqxsj6icqv82as";
  };
  features_.winapi."0.2.8" = deps: f: updateFeatures f (rec {
    winapi."0.2.8".default = (f.winapi."0.2.8".default or true);
  }) [];


# end
# winapi-0.3.9

  crates.winapi."0.3.9" = deps: { features?(features_.winapi."0.3.9" deps {}) }: buildRustCrate {
    crateName = "winapi";
    version = "0.3.9";
    description = "Raw FFI bindings for all of Windows API.";
    authors = [ "Peter Atashian <retep998@gmail.com>" ];
    sha256 = "1r53g3rwnb8pwv8qa0hdxxn3s3iiix0n2anan33n0r2gdck70qsn";
    build = "build.rs";
    dependencies = (if kernel == "i686-pc-windows-gnu" then mapFeatures features ([
      (crates."winapi_i686_pc_windows_gnu"."${deps."winapi"."0.3.9"."winapi_i686_pc_windows_gnu"}" deps)
    ]) else [])
      ++ (if kernel == "x86_64-pc-windows-gnu" then mapFeatures features ([
      (crates."winapi_x86_64_pc_windows_gnu"."${deps."winapi"."0.3.9"."winapi_x86_64_pc_windows_gnu"}" deps)
    ]) else []);
    features = mkFeatures (features."winapi"."0.3.9" or {});
  };
  features_.winapi."0.3.9" = deps: f: updateFeatures f (rec {
    winapi = fold recursiveUpdate {} [
      { "0.3.9"."impl-debug" =
        (f.winapi."0.3.9"."impl-debug" or false) ||
        (f.winapi."0.3.9".debug or false) ||
        (winapi."0.3.9"."debug" or false); }
      { "0.3.9".default = (f.winapi."0.3.9".default or true); }
    ];
    winapi_i686_pc_windows_gnu."${deps.winapi."0.3.9".winapi_i686_pc_windows_gnu}".default = true;
    winapi_x86_64_pc_windows_gnu."${deps.winapi."0.3.9".winapi_x86_64_pc_windows_gnu}".default = true;
  }) [
    (features_.winapi_i686_pc_windows_gnu."${deps."winapi"."0.3.9"."winapi_i686_pc_windows_gnu"}" deps)
    (features_.winapi_x86_64_pc_windows_gnu."${deps."winapi"."0.3.9"."winapi_x86_64_pc_windows_gnu"}" deps)
  ];


# end
# winapi-build-0.1.1

  crates.winapi_build."0.1.1" = deps: { features?(features_.winapi_build."0.1.1" deps {}) }: buildRustCrate {
    crateName = "winapi-build";
    version = "0.1.1";
    description = "Common code for build.rs in WinAPI -sys crates.";
    authors = [ "Peter Atashian <retep998@gmail.com>" ];
    sha256 = "1lxlpi87rkhxcwp2ykf1ldw3p108hwm24nywf3jfrvmff4rjhqga";
    libName = "build";
  };
  features_.winapi_build."0.1.1" = deps: f: updateFeatures f (rec {
    winapi_build."0.1.1".default = (f.winapi_build."0.1.1".default or true);
  }) [];


# end
# winapi-i686-pc-windows-gnu-0.4.0

  crates.winapi_i686_pc_windows_gnu."0.4.0" = deps: { features?(features_.winapi_i686_pc_windows_gnu."0.4.0" deps {}) }: buildRustCrate {
    crateName = "winapi-i686-pc-windows-gnu";
    version = "0.4.0";
    description = "Import libraries for the i686-pc-windows-gnu target. Please don't use this crate directly, depend on winapi instead.";
    authors = [ "Peter Atashian <retep998@gmail.com>" ];
    sha256 = "05ihkij18r4gamjpxj4gra24514can762imjzlmak5wlzidplzrp";
    build = "build.rs";
  };
  features_.winapi_i686_pc_windows_gnu."0.4.0" = deps: f: updateFeatures f (rec {
    winapi_i686_pc_windows_gnu."0.4.0".default = (f.winapi_i686_pc_windows_gnu."0.4.0".default or true);
  }) [];


# end
# winapi-util-0.1.5

  crates.winapi_util."0.1.5" = deps: { features?(features_.winapi_util."0.1.5" deps {}) }: buildRustCrate {
    crateName = "winapi-util";
    version = "0.1.5";
    description = "A dumping ground for high level safe wrappers over winapi.";
    authors = [ "Andrew Gallant <jamslam@gmail.com>" ];
    edition = "2018";
    sha256 = "0h8l3gjhdsa0s6ibiv277jgg6q7vwplwxir44hcjizws9avpcphj";
    dependencies = (if kernel == "windows" then mapFeatures features ([
      (crates."winapi"."${deps."winapi_util"."0.1.5"."winapi"}" deps)
    ]) else []);
  };
  features_.winapi_util."0.1.5" = deps: f: updateFeatures f (rec {
    winapi = fold recursiveUpdate {} [
      { "${deps.winapi_util."0.1.5".winapi}"."consoleapi" = true; }
      { "${deps.winapi_util."0.1.5".winapi}"."errhandlingapi" = true; }
      { "${deps.winapi_util."0.1.5".winapi}"."fileapi" = true; }
      { "${deps.winapi_util."0.1.5".winapi}"."minwindef" = true; }
      { "${deps.winapi_util."0.1.5".winapi}"."processenv" = true; }
      { "${deps.winapi_util."0.1.5".winapi}"."std" = true; }
      { "${deps.winapi_util."0.1.5".winapi}"."winbase" = true; }
      { "${deps.winapi_util."0.1.5".winapi}"."wincon" = true; }
      { "${deps.winapi_util."0.1.5".winapi}"."winerror" = true; }
      { "${deps.winapi_util."0.1.5".winapi}"."winnt" = true; }
      { "${deps.winapi_util."0.1.5".winapi}".default = true; }
    ];
    winapi_util."0.1.5".default = (f.winapi_util."0.1.5".default or true);
  }) [
    (features_.winapi."${deps."winapi_util"."0.1.5"."winapi"}" deps)
  ];


# end
# winapi-x86_64-pc-windows-gnu-0.4.0

  crates.winapi_x86_64_pc_windows_gnu."0.4.0" = deps: { features?(features_.winapi_x86_64_pc_windows_gnu."0.4.0" deps {}) }: buildRustCrate {
    crateName = "winapi-x86_64-pc-windows-gnu";
    version = "0.4.0";
    description = "Import libraries for the x86_64-pc-windows-gnu target. Please don't use this crate directly, depend on winapi instead.";
    authors = [ "Peter Atashian <retep998@gmail.com>" ];
    sha256 = "0n1ylmlsb8yg1v583i4xy0qmqg42275flvbc51hdqjjfjcl9vlbj";
    build = "build.rs";
  };
  features_.winapi_x86_64_pc_windows_gnu."0.4.0" = deps: f: updateFeatures f (rec {
    winapi_x86_64_pc_windows_gnu."0.4.0".default = (f.winapi_x86_64_pc_windows_gnu."0.4.0".default or true);
  }) [];


# end
# ws2_32-sys-0.2.1

  crates.ws2_32_sys."0.2.1" = deps: { features?(features_.ws2_32_sys."0.2.1" deps {}) }: buildRustCrate {
    crateName = "ws2_32-sys";
    version = "0.2.1";
    description = "Contains function definitions for the Windows API library ws2_32. See winapi for types and constants.";
    authors = [ "Peter Atashian <retep998@gmail.com>" ];
    sha256 = "1zpy9d9wk11sj17fczfngcj28w4xxjs3b4n036yzpy38dxp4f7kc";
    libName = "ws2_32";
    build = "build.rs";
    dependencies = mapFeatures features ([
      (crates."winapi"."${deps."ws2_32_sys"."0.2.1"."winapi"}" deps)
    ]);

    buildDependencies = mapFeatures features ([
      (crates."winapi_build"."${deps."ws2_32_sys"."0.2.1"."winapi_build"}" deps)
    ]);
  };
  features_.ws2_32_sys."0.2.1" = deps: f: updateFeatures f (rec {
    winapi."${deps.ws2_32_sys."0.2.1".winapi}".default = true;
    winapi_build."${deps.ws2_32_sys."0.2.1".winapi_build}".default = true;
    ws2_32_sys."0.2.1".default = (f.ws2_32_sys."0.2.1".default or true);
  }) [
    (features_.winapi."${deps."ws2_32_sys"."0.2.1"."winapi"}" deps)
    (features_.winapi_build."${deps."ws2_32_sys"."0.2.1"."winapi_build"}" deps)
  ];


# end
}
