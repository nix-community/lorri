Output assets/lorri-demo.webm

Require lorri
Require direnv

Set Shell "bash"
Set FontSize 18
Set Width 120
Set Height 32
Set Theme "Catppuccin Mocha"

# ── setup ────────────────────────────────────────────────────────────────────

Hide
Type "export PS1='$ '" Enter
Type "cd $(mktemp -d)" Enter
Show

# ── scene 1: lorri init ───────────────────────────────────────────────────────

Sleep 500ms
Type "lorri init"
Sleep 500ms
Enter
Wait+Screen /lorri: done/
Sleep 1.5s

Type "cat shell.nix"
Enter
Sleep 2s

# ── scene 2: start daemon in background ──────────────────────────────────────

Type "lorri daemon &"
Enter
Wait+Screen /daemon ready/
Sleep 1s

# ── scene 3: direnv allow ─────────────────────────────────────────────────────

Type "direnv allow"
Enter
Wait+Screen /direnv: export/
Sleep 1s

Type "hello"
Enter
Wait+Screen /Hello, world/
Sleep 2s

# ── scene 4: live rebuild — add cowsay ───────────────────────────────────────

Type "printf '{ pkgs ? import <nixpkgs> {} }:\npkgs.mkShell {\n  buildInputs = [ pkgs.hello pkgs.cowsay pkgs.bashInteractive ];\n}\n' > shell.nix"
Enter
Sleep 500ms

Wait+Screen /build completed/
Sleep 500ms

Type "cowsay 'lorri rebuilt my shell!'"
Enter
Wait+Screen /lorri rebuilt/
Sleep 2.5s

# ── scene 5: lorri prompt ─────────────────────────────────────────────────────

Type "PS1='\$(lorri prompt default) \$ '"
Enter
Sleep 500ms
Type ""
Enter
Sleep 2s

