package main

import "os"

// runtimeClosure is baked in at link time by go/default.nix via:
//
//	x_defs."main.runtimeClosure" = "${rtc}"
//
// This mirrors how build.rs bakes RUN_TIME_CLOSURE into the Rust binary.
// In development (lorri nix-shell), runtimeClosure is "" and the env var
// RUN_TIME_CLOSURE is used as a fallback instead.
var runtimeClosure = ""

// requireRTC returns the lorri runtime closure store path.
// Prefers the link-time constant; falls back to $RUN_TIME_CLOSURE.
func requireRTC() string {
	if runtimeClosure != "" {
		return runtimeClosure
	}
	return os.Getenv("RUN_TIME_CLOSURE")
}
