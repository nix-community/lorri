package main

// NixOptions holds extra CLI options passed to nix-instantiate and nix-build.
// Mirrors src/nix/options.rs NixOptions.

import "strings"

// NixOptions carries optional overrides for Nix builder/substituter settings.
//
// nil slice means "use whatever is in nix.conf" (the default).
// Empty non-nil slice means "use none".
type NixOptions struct {
	// List of remote builder specifications (--builders).
	// nil → use nix.conf default; []string{} → disable builders.
	Builders []string

	// List of binary cache substituter URLs (--substituters).
	// nil → use nix.conf default; []string{} → disable substituters.
	Substituters []string
}

// Append merges other into o.
// If both slices are non-nil they are concatenated; otherwise the existing
// non-nil one is kept (nil is the identity element).
// Mirrors NixOptions::append() in Rust.
func (o *NixOptions) Append(other NixOptions) {
	o.Builders = extendOptionSlice(o.Builders, other.Builders)
	o.Substituters = extendOptionSlice(o.Substituters, other.Substituters)
}

func extendOptionSlice(base, extra []string) []string {
	switch {
	case base != nil && extra != nil:
		return append(base, extra...)
	case base != nil:
		return base
	case extra != nil:
		return extra
	default:
		return nil
	}
}

// ToNixArglist converts NixOptions to CLI flags for nix-instantiate / nix-build.
// Mirrors NixOptions::to_nix_arglist() in Rust.
//
//	--builders "spec1\nspec2"   (builders joined by newline, matching /etc/nix/machines format)
//	--substituters "s1 s2"      (substituters joined by space, matching nix.conf format)
func (o NixOptions) ToNixArglist() []string {
	var args []string

	if o.Builders != nil {
		args = append(args, "--builders", strings.Join(o.Builders, "\n"))
	}
	if o.Substituters != nil {
		args = append(args, "--substituters", strings.Join(o.Substituters, " "))
	}

	return args
}
