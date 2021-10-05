# Generated by carnix 0.10.0: carnix generate-nix --src ../..
{ lib, buildPlatform, buildRustCrate, buildRustCrateHelpers, cratesIO, fetchgit }:
with buildRustCrateHelpers;
let inherit (lib.lists) fold;
    inherit (lib.attrsets) recursiveUpdate;
in
rec {
  crates = cratesIO // rec {
# human-panic-1.0.4-alpha.0

    crates.human_panic."1.0.4-alpha.0" = deps: { features?(features_.human_panic."1.0.4-alpha.0" deps {}) }: buildRustCrate {
      crateName = "human-panic";
      version = "1.0.4-alpha.0";
      description = "Panic messages for humans";
      authors = [ "Yoshua Wuyts <yoshuawuyts@gmail.com>" "Pascal Hertleif <killercup@gmail.com>" "Katharina Fey <kookie@spacekookie.de>" ];
      edition = "2018";
      src = exclude [ ".git" "target" ] vendor/human-panic;
      dependencies = mapFeatures features ([
        (cratesIO.crates."backtrace"."${deps."human_panic"."1.0.4-alpha.0"."backtrace"}" deps)
        (cratesIO.crates."os_type"."${deps."human_panic"."1.0.4-alpha.0"."os_type"}" deps)
        (cratesIO.crates."serde"."${deps."human_panic"."1.0.4-alpha.0"."serde"}" deps)
        (cratesIO.crates."serde_derive"."${deps."human_panic"."1.0.4-alpha.0"."serde_derive"}" deps)
        (cratesIO.crates."toml"."${deps."human_panic"."1.0.4-alpha.0"."toml"}" deps)
        (cratesIO.crates."uuid"."${deps."human_panic"."1.0.4-alpha.0"."uuid"}" deps)
      ]);
      features = mkFeatures (features."human_panic"."1.0.4-alpha.0" or {});
    };
    features_.human_panic."1.0.4-alpha.0" = deps: f: updateFeatures f (rec {
      backtrace."${deps.human_panic."1.0.4-alpha.0".backtrace}".default = true;
      human_panic."1.0.4-alpha.0".default = (f.human_panic."1.0.4-alpha.0".default or true);
      os_type."${deps.human_panic."1.0.4-alpha.0".os_type}".default = true;
      serde."${deps.human_panic."1.0.4-alpha.0".serde}".default = true;
      serde_derive."${deps.human_panic."1.0.4-alpha.0".serde_derive}".default = true;
      toml."${deps.human_panic."1.0.4-alpha.0".toml}".default = true;
      uuid = fold recursiveUpdate {} [
        { "${deps.human_panic."1.0.4-alpha.0".uuid}"."v4" = true; }
        { "${deps.human_panic."1.0.4-alpha.0".uuid}".default = (f.uuid."${deps.human_panic."1.0.4-alpha.0".uuid}".default or false); }
      ];
    }) [
      (cratesIO.features_.backtrace."${deps."human_panic"."1.0.4-alpha.0"."backtrace"}" deps)
      (cratesIO.features_.os_type."${deps."human_panic"."1.0.4-alpha.0"."os_type"}" deps)
      (cratesIO.features_.serde."${deps."human_panic"."1.0.4-alpha.0"."serde"}" deps)
      (cratesIO.features_.serde_derive."${deps."human_panic"."1.0.4-alpha.0"."serde_derive"}" deps)
      (cratesIO.features_.toml."${deps."human_panic"."1.0.4-alpha.0"."toml"}" deps)
      (cratesIO.features_.uuid."${deps."human_panic"."1.0.4-alpha.0"."uuid"}" deps)
    ];


# end
# lorri-1.5.0

    crates.lorri."1.5.0" = deps: { features?(features_.lorri."1.5.0" deps {}) }: buildRustCrate {
      crateName = "lorri";
      version = "1.5.0";
      authors = [ "Graham Christensen <graham.christensen@target.com>" "Profpatsch <mail@profpatsch.de>" ];
      edition = "2018";
      src = exclude [ ".git" "target" ] ./.;
      dependencies = mapFeatures features ([
        (cratesIO.crates."anyhow"."${deps."lorri"."1.5.0"."anyhow"}" deps)
        (cratesIO.crates."atomicwrites"."${deps."lorri"."1.5.0"."atomicwrites"}" deps)
        (cratesIO.crates."bincode"."${deps."lorri"."1.5.0"."bincode"}" deps)
        (cratesIO.crates."crossbeam_channel"."${deps."lorri"."1.5.0"."crossbeam_channel"}" deps)
        (cratesIO.crates."ctrlc"."${deps."lorri"."1.5.0"."ctrlc"}" deps)
        (cratesIO.crates."directories"."${deps."lorri"."1.5.0"."directories"}" deps)
        (cratesIO.crates."fastrand"."${deps."lorri"."1.5.0"."fastrand"}" deps)
        (crates."human_panic"."${deps."lorri"."1.5.0"."human_panic"}" deps)
        (cratesIO.crates."lazy_static"."${deps."lorri"."1.5.0"."lazy_static"}" deps)
        (cratesIO.crates."md5"."${deps."lorri"."1.5.0"."md5"}" deps)
        (cratesIO.crates."nix"."${deps."lorri"."1.5.0"."nix"}" deps)
        (cratesIO.crates."notify"."${deps."lorri"."1.5.0"."notify"}" deps)
        (cratesIO.crates."regex"."${deps."lorri"."1.5.0"."regex"}" deps)
        (cratesIO.crates."serde"."${deps."lorri"."1.5.0"."serde"}" deps)
        (cratesIO.crates."serde_derive"."${deps."lorri"."1.5.0"."serde_derive"}" deps)
        (cratesIO.crates."serde_json"."${deps."lorri"."1.5.0"."serde_json"}" deps)
        (cratesIO.crates."slog"."${deps."lorri"."1.5.0"."slog"}" deps)
        (cratesIO.crates."slog_scope"."${deps."lorri"."1.5.0"."slog_scope"}" deps)
        (cratesIO.crates."slog_term"."${deps."lorri"."1.5.0"."slog_term"}" deps)
        (cratesIO.crates."structopt"."${deps."lorri"."1.5.0"."structopt"}" deps)
        (cratesIO.crates."tempfile"."${deps."lorri"."1.5.0"."tempfile"}" deps)
        (cratesIO.crates."thiserror"."${deps."lorri"."1.5.0"."thiserror"}" deps)
        (cratesIO.crates."vec1"."${deps."lorri"."1.5.0"."vec1"}" deps)
      ]);
    };
    features_.lorri."1.5.0" = deps: f: updateFeatures f (rec {
      anyhow."${deps.lorri."1.5.0".anyhow}".default = true;
      atomicwrites."${deps.lorri."1.5.0".atomicwrites}".default = true;
      bincode."${deps.lorri."1.5.0".bincode}".default = true;
      crossbeam_channel."${deps.lorri."1.5.0".crossbeam_channel}".default = true;
      ctrlc = fold recursiveUpdate {} [
        { "${deps.lorri."1.5.0".ctrlc}"."termination" = true; }
        { "${deps.lorri."1.5.0".ctrlc}".default = true; }
      ];
      directories."${deps.lorri."1.5.0".directories}".default = true;
      fastrand."${deps.lorri."1.5.0".fastrand}".default = true;
      human_panic."${deps.lorri."1.5.0".human_panic}".default = true;
      lazy_static."${deps.lorri."1.5.0".lazy_static}".default = true;
      lorri."1.5.0".default = (f.lorri."1.5.0".default or true);
      md5."${deps.lorri."1.5.0".md5}".default = true;
      nix."${deps.lorri."1.5.0".nix}".default = true;
      notify."${deps.lorri."1.5.0".notify}".default = true;
      regex."${deps.lorri."1.5.0".regex}".default = true;
      serde."${deps.lorri."1.5.0".serde}".default = true;
      serde_derive."${deps.lorri."1.5.0".serde_derive}".default = true;
      serde_json."${deps.lorri."1.5.0".serde_json}".default = true;
      slog = fold recursiveUpdate {} [
        { "${deps.lorri."1.5.0".slog}"."release_max_level_debug" = true; }
        { "${deps.lorri."1.5.0".slog}".default = true; }
      ];
      slog_scope."${deps.lorri."1.5.0".slog_scope}".default = true;
      slog_term."${deps.lorri."1.5.0".slog_term}".default = true;
      structopt = fold recursiveUpdate {} [
        { "${deps.lorri."1.5.0".structopt}"."color" = true; }
        { "${deps.lorri."1.5.0".structopt}"."no_cargo" = true; }
        { "${deps.lorri."1.5.0".structopt}"."suggestions" = true; }
        { "${deps.lorri."1.5.0".structopt}".default = (f.structopt."${deps.lorri."1.5.0".structopt}".default or false); }
      ];
      tempfile."${deps.lorri."1.5.0".tempfile}".default = true;
      thiserror."${deps.lorri."1.5.0".thiserror}".default = true;
      vec1."${deps.lorri."1.5.0".vec1}".default = true;
    }) [
      (cratesIO.features_.anyhow."${deps."lorri"."1.5.0"."anyhow"}" deps)
      (cratesIO.features_.atomicwrites."${deps."lorri"."1.5.0"."atomicwrites"}" deps)
      (cratesIO.features_.bincode."${deps."lorri"."1.5.0"."bincode"}" deps)
      (cratesIO.features_.crossbeam_channel."${deps."lorri"."1.5.0"."crossbeam_channel"}" deps)
      (cratesIO.features_.ctrlc."${deps."lorri"."1.5.0"."ctrlc"}" deps)
      (cratesIO.features_.directories."${deps."lorri"."1.5.0"."directories"}" deps)
      (cratesIO.features_.fastrand."${deps."lorri"."1.5.0"."fastrand"}" deps)
      (features_.human_panic."${deps."lorri"."1.5.0"."human_panic"}" deps)
      (cratesIO.features_.lazy_static."${deps."lorri"."1.5.0"."lazy_static"}" deps)
      (cratesIO.features_.md5."${deps."lorri"."1.5.0"."md5"}" deps)
      (cratesIO.features_.nix."${deps."lorri"."1.5.0"."nix"}" deps)
      (cratesIO.features_.notify."${deps."lorri"."1.5.0"."notify"}" deps)
      (cratesIO.features_.regex."${deps."lorri"."1.5.0"."regex"}" deps)
      (cratesIO.features_.serde."${deps."lorri"."1.5.0"."serde"}" deps)
      (cratesIO.features_.serde_derive."${deps."lorri"."1.5.0"."serde_derive"}" deps)
      (cratesIO.features_.serde_json."${deps."lorri"."1.5.0"."serde_json"}" deps)
      (cratesIO.features_.slog."${deps."lorri"."1.5.0"."slog"}" deps)
      (cratesIO.features_.slog_scope."${deps."lorri"."1.5.0"."slog_scope"}" deps)
      (cratesIO.features_.slog_term."${deps."lorri"."1.5.0"."slog_term"}" deps)
      (cratesIO.features_.structopt."${deps."lorri"."1.5.0"."structopt"}" deps)
      (cratesIO.features_.tempfile."${deps."lorri"."1.5.0"."tempfile"}" deps)
      (cratesIO.features_.thiserror."${deps."lorri"."1.5.0"."thiserror"}" deps)
      (cratesIO.features_.vec1."${deps."lorri"."1.5.0"."vec1"}" deps)
    ];


# end

  };

  lorri = crates.crates.lorri."1.5.0" deps;
  __all = [ (lorri {}) ];
  deps.aho_corasick."0.7.12" = {
    memchr = "2.3.3";
  };
  deps.ansi_term."0.11.0" = {
    winapi = "0.3.9";
  };
  deps.anyhow."1.0.38" = {};
  deps.anymap."0.12.1" = {};
  deps.arc_swap."0.4.7" = {};
  deps.arrayref."0.3.6" = {};
  deps.arrayvec."0.5.1" = {};
  deps.atomicwrites."0.2.5" = {
    tempdir = "0.3.7";
    nix = "0.14.1";
    winapi = "0.3.9";
  };
  deps.atty."0.2.14" = {
    hermit_abi = "0.1.14";
    libc = "0.2.86";
    winapi = "0.3.9";
  };
  deps.autocfg."1.0.0" = {};
  deps.backtrace."0.3.44" = {
    backtrace_sys = "0.1.35";
    cfg_if = "0.1.10";
    libc = "0.2.86";
    rustc_demangle = "0.1.16";
  };
  deps.backtrace_sys."0.1.35" = {
    libc = "0.2.86";
    cc = "1.0.54";
  };
  deps.base64."0.11.0" = {};
  deps.bincode."1.3.2" = {
    byteorder = "1.3.4";
    serde = "1.0.114";
  };
  deps.bitflags."1.2.1" = {};
  deps.blake2b_simd."0.5.10" = {
    arrayref = "0.3.6";
    arrayvec = "0.5.1";
    constant_time_eq = "0.1.5";
  };
  deps.byteorder."1.3.4" = {};
  deps.cc."1.0.54" = {};
  deps.cfg_if."0.1.10" = {};
  deps.cfg_if."1.0.0" = {};
  deps.chashmap."2.2.2" = {
    owning_ref = "0.3.3";
    parking_lot = "0.4.8";
  };
  deps.chrono."0.4.11" = {
    num_integer = "0.1.43";
    num_traits = "0.2.12";
    time = "0.1.43";
  };
  deps.clap."2.33.1" = {
    atty = "0.2.14";
    bitflags = "1.2.1";
    strsim = "0.8.0";
    textwrap = "0.11.0";
    unicode_width = "0.1.7";
    ansi_term = "0.11.0";
  };
  deps.constant_time_eq."0.1.5" = {};
  deps.crossbeam_channel."0.3.9" = {
    crossbeam_utils = "0.6.6";
  };
  deps.crossbeam_utils."0.6.6" = {
    cfg_if = "0.1.10";
    lazy_static = "1.4.0";
  };
  deps.crossbeam_utils."0.7.2" = {
    cfg_if = "0.1.10";
    lazy_static = "1.4.0";
    autocfg = "1.0.0";
  };
  deps.ctrlc."3.1.8" = {
    nix = "0.20.0";
    winapi = "0.3.9";
  };
  deps.directories."3.0.1" = {
    dirs_sys = "0.3.5";
  };
  deps.dirs."2.0.2" = {
    cfg_if = "0.1.10";
    dirs_sys = "0.3.5";
  };
  deps.dirs_sys."0.3.5" = {
    redox_users = "0.3.4";
    libc = "0.2.86";
    winapi = "0.3.9";
  };
  deps.fastrand."1.4.0" = {
    instant = "0.1.9";
  };
  deps.filetime."0.2.10" = {
    cfg_if = "0.1.10";
    redox_syscall = "0.1.56";
    libc = "0.2.86";
    winapi = "0.3.9";
  };
  deps.fsevent."0.4.0" = {
    bitflags = "1.2.1";
    fsevent_sys = "2.0.1";
  };
  deps.fsevent_sys."2.0.1" = {
    libc = "0.2.86";
  };
  deps.fuchsia_cprng."0.1.1" = {};
  deps.fuchsia_zircon."0.3.3" = {
    bitflags = "1.2.1";
    fuchsia_zircon_sys = "0.3.3";
  };
  deps.fuchsia_zircon_sys."0.3.3" = {};
  deps.getrandom."0.1.14" = {
    cfg_if = "0.1.10";
    wasi = "0.9.0+wasi-snapshot-preview1";
    libc = "0.2.86";
  };
  deps.heck."0.3.1" = {
    unicode_segmentation = "1.6.0";
  };
  deps.hermit_abi."0.1.14" = {
    libc = "0.2.86";
  };
  deps.human_panic."1.0.4-alpha.0" = {
    backtrace = "0.3.44";
    os_type = "2.2.0";
    serde = "1.0.114";
    serde_derive = "1.0.114";
    toml = "0.5.6";
    uuid = "0.8.1";
  };
  deps.inotify."0.7.1" = {
    bitflags = "1.2.1";
    inotify_sys = "0.1.3";
    libc = "0.2.86";
  };
  deps.inotify_sys."0.1.3" = {
    libc = "0.2.86";
  };
  deps.instant."0.1.9" = {
    cfg_if = "1.0.0";
  };
  deps.iovec."0.1.4" = {
    libc = "0.2.86";
  };
  deps.itoa."0.4.6" = {};
  deps.kernel32_sys."0.2.2" = {
    winapi = "0.2.8";
    winapi_build = "0.1.1";
  };
  deps.lazy_static."1.4.0" = {};
  deps.lazycell."1.2.1" = {};
  deps.libc."0.2.86" = {};
  deps.log."0.4.8" = {
    cfg_if = "0.1.10";
  };
  deps.lorri."1.5.0" = {
    anyhow = "1.0.38";
    atomicwrites = "0.2.5";
    bincode = "1.3.2";
    crossbeam_channel = "0.3.9";
    ctrlc = "3.1.8";
    directories = "3.0.1";
    fastrand = "1.4.0";
    human_panic = "1.0.4-alpha.0";
    lazy_static = "1.4.0";
    md5 = "0.7.0";
    nix = "0.20.0";
    notify = "5.0.0-pre.1";
    regex = "1.4.3";
    serde = "1.0.114";
    serde_derive = "1.0.114";
    serde_json = "1.0.55";
    slog = "2.7.0";
    slog_scope = "4.3.0";
    slog_term = "2.6.0";
    structopt = "0.2.18";
    tempfile = "3.1.0";
    thiserror = "1.0.24";
    vec1 = "1.5.0";
  };
  deps.maybe_uninit."2.0.0" = {};
  deps.md5."0.7.0" = {};
  deps.memchr."2.3.3" = {};
  deps.mio."0.6.22" = {
    cfg_if = "0.1.10";
    iovec = "0.1.4";
    log = "0.4.8";
    net2 = "0.2.34";
    slab = "0.4.2";
    fuchsia_zircon = "0.3.3";
    fuchsia_zircon_sys = "0.3.3";
    libc = "0.2.86";
    kernel32_sys = "0.2.2";
    miow = "0.2.1";
    winapi = "0.2.8";
  };
  deps.mio_extras."2.0.6" = {
    lazycell = "1.2.1";
    log = "0.4.8";
    mio = "0.6.22";
    slab = "0.4.2";
  };
  deps.miow."0.2.1" = {
    kernel32_sys = "0.2.2";
    net2 = "0.2.34";
    winapi = "0.2.8";
    ws2_32_sys = "0.2.1";
  };
  deps.net2."0.2.34" = {
    cfg_if = "0.1.10";
    libc = "0.2.86";
    winapi = "0.3.9";
  };
  deps.nix."0.14.1" = {
    bitflags = "1.2.1";
    cfg_if = "0.1.10";
    libc = "0.2.86";
    void = "1.0.2";
  };
  deps.nix."0.20.0" = {
    bitflags = "1.2.1";
    cfg_if = "1.0.0";
    libc = "0.2.86";
  };
  deps.notify."5.0.0-pre.1" = {
    anymap = "0.12.1";
    bitflags = "1.2.1";
    chashmap = "2.2.2";
    crossbeam_channel = "0.3.9";
    filetime = "0.2.10";
    libc = "0.2.86";
    walkdir = "2.3.1";
    inotify = "0.7.1";
    mio = "0.6.22";
    mio_extras = "2.0.6";
    fsevent = "0.4.0";
    fsevent_sys = "2.0.1";
    kernel32_sys = "0.2.2";
    winapi = "0.3.9";
  };
  deps.num_integer."0.1.43" = {
    num_traits = "0.2.12";
    autocfg = "1.0.0";
  };
  deps.num_traits."0.2.12" = {
    autocfg = "1.0.0";
  };
  deps.os_type."2.2.0" = {
    regex = "1.4.3";
  };
  deps.owning_ref."0.3.3" = {
    stable_deref_trait = "1.1.1";
  };
  deps.parking_lot."0.4.8" = {
    owning_ref = "0.3.3";
    parking_lot_core = "0.2.14";
  };
  deps.parking_lot_core."0.2.14" = {
    rand = "0.4.6";
    smallvec = "0.6.13";
    libc = "0.2.86";
    winapi = "0.3.9";
  };
  deps.ppv_lite86."0.2.8" = {};
  deps.proc_macro2."0.4.30" = {
    unicode_xid = "0.1.0";
  };
  deps.proc_macro2."1.0.24" = {
    unicode_xid = "0.2.0";
  };
  deps.proptest."0.10.1" = {
    bitflags = "1.2.1";
    byteorder = "1.3.4";
    lazy_static = "1.4.0";
    num_traits = "0.2.12";
    quick_error = "1.2.3";
    rand = "0.7.3";
    rand_chacha = "0.2.2";
    rand_xorshift = "0.2.0";
    regex_syntax = "0.6.22";
  };
  deps.quick_error."1.2.3" = {};
  deps.quote."0.6.13" = {
    proc_macro2 = "0.4.30";
  };
  deps.quote."1.0.7" = {
    proc_macro2 = "1.0.24";
  };
  deps.rand."0.4.6" = {
    rand_core = "0.3.1";
    rdrand = "0.4.0";
    fuchsia_cprng = "0.1.1";
    libc = "0.2.86";
    winapi = "0.3.9";
  };
  deps.rand."0.7.3" = {
    rand_core = "0.5.1";
    rand_chacha = "0.2.2";
    rand_hc = "0.2.0";
    libc = "0.2.86";
  };
  deps.rand_chacha."0.2.2" = {
    ppv_lite86 = "0.2.8";
    rand_core = "0.5.1";
  };
  deps.rand_core."0.3.1" = {
    rand_core = "0.4.2";
  };
  deps.rand_core."0.4.2" = {};
  deps.rand_core."0.5.1" = {
    getrandom = "0.1.14";
  };
  deps.rand_hc."0.2.0" = {
    rand_core = "0.5.1";
  };
  deps.rand_xorshift."0.2.0" = {
    rand_core = "0.5.1";
  };
  deps.rdrand."0.4.0" = {
    rand_core = "0.3.1";
  };
  deps.redox_syscall."0.1.56" = {};
  deps.redox_users."0.3.4" = {
    getrandom = "0.1.14";
    redox_syscall = "0.1.56";
    rust_argon2 = "0.7.0";
  };
  deps.regex."1.4.3" = {
    aho_corasick = "0.7.12";
    memchr = "2.3.3";
    regex_syntax = "0.6.22";
    thread_local = "1.0.1";
  };
  deps.regex_syntax."0.6.22" = {};
  deps.remove_dir_all."0.5.3" = {
    winapi = "0.3.9";
  };
  deps.rust_argon2."0.7.0" = {
    base64 = "0.11.0";
    blake2b_simd = "0.5.10";
    constant_time_eq = "0.1.5";
    crossbeam_utils = "0.7.2";
  };
  deps.rustc_demangle."0.1.16" = {};
  deps.ryu."1.0.5" = {};
  deps.same_file."1.0.6" = {
    winapi_util = "0.1.5";
  };
  deps.serde."1.0.114" = {};
  deps.serde_derive."1.0.114" = {
    proc_macro2 = "1.0.24";
    quote = "1.0.7";
    syn = "1.0.64";
  };
  deps.serde_json."1.0.55" = {
    itoa = "0.4.6";
    ryu = "1.0.5";
    serde = "1.0.114";
  };
  deps.slab."0.4.2" = {};
  deps.slog."2.7.0" = {};
  deps.slog_scope."4.3.0" = {
    arc_swap = "0.4.7";
    lazy_static = "1.4.0";
    slog = "2.7.0";
  };
  deps.slog_term."2.6.0" = {
    atty = "0.2.14";
    chrono = "0.4.11";
    slog = "2.7.0";
    term = "0.6.1";
    thread_local = "1.0.1";
  };
  deps.smallvec."0.6.13" = {
    maybe_uninit = "2.0.0";
  };
  deps.stable_deref_trait."1.1.1" = {};
  deps.strsim."0.8.0" = {};
  deps.structopt."0.2.18" = {
    clap = "2.33.1";
    structopt_derive = "0.2.18";
  };
  deps.structopt_derive."0.2.18" = {
    heck = "0.3.1";
    proc_macro2 = "0.4.30";
    quote = "0.6.13";
    syn = "0.15.44";
  };
  deps.syn."0.15.44" = {
    proc_macro2 = "0.4.30";
    quote = "0.6.13";
    unicode_xid = "0.1.0";
  };
  deps.syn."1.0.64" = {
    proc_macro2 = "1.0.24";
    quote = "1.0.7";
    unicode_xid = "0.2.0";
  };
  deps.tempdir."0.3.7" = {
    rand = "0.4.6";
    remove_dir_all = "0.5.3";
  };
  deps.tempfile."3.1.0" = {
    cfg_if = "0.1.10";
    rand = "0.7.3";
    remove_dir_all = "0.5.3";
    redox_syscall = "0.1.56";
    libc = "0.2.86";
    winapi = "0.3.9";
  };
  deps.term."0.6.1" = {
    dirs = "2.0.2";
    winapi = "0.3.9";
  };
  deps.textwrap."0.11.0" = {
    unicode_width = "0.1.7";
  };
  deps.thiserror."1.0.24" = {
    thiserror_impl = "1.0.24";
  };
  deps.thiserror_impl."1.0.24" = {
    proc_macro2 = "1.0.24";
    quote = "1.0.7";
    syn = "1.0.64";
  };
  deps.thread_local."1.0.1" = {
    lazy_static = "1.4.0";
  };
  deps.time."0.1.43" = {
    libc = "0.2.86";
    winapi = "0.3.9";
  };
  deps.toml."0.5.6" = {
    serde = "1.0.114";
  };
  deps.unicode_segmentation."1.6.0" = {};
  deps.unicode_width."0.1.7" = {};
  deps.unicode_xid."0.1.0" = {};
  deps.unicode_xid."0.2.0" = {};
  deps.uuid."0.8.1" = {
    rand = "0.7.3";
  };
  deps.vec1."1.5.0" = {};
  deps.void."1.0.2" = {};
  deps.walkdir."2.3.1" = {
    same_file = "1.0.6";
    winapi = "0.3.9";
    winapi_util = "0.1.5";
  };
  deps.wasi."0.9.0+wasi-snapshot-preview1" = {};
  deps.winapi."0.2.8" = {};
  deps.winapi."0.3.9" = {
    winapi_i686_pc_windows_gnu = "0.4.0";
    winapi_x86_64_pc_windows_gnu = "0.4.0";
  };
  deps.winapi_build."0.1.1" = {};
  deps.winapi_i686_pc_windows_gnu."0.4.0" = {};
  deps.winapi_util."0.1.5" = {
    winapi = "0.3.9";
  };
  deps.winapi_x86_64_pc_windows_gnu."0.4.0" = {};
  deps.ws2_32_sys."0.2.1" = {
    winapi = "0.2.8";
    winapi_build = "0.1.1";
  };
}
