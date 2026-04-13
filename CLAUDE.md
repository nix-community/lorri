# Git Commit Rules

## Commit Message Format

### Structure
```
<type>(<scope>): ✨ <description under 72 chars>

Body of the commit message with an empty line between subject and
body. This text should explain what the change does and why it has
been made, especially if it introduces a new feature.

Relevant issues should be mentioned if they exist.

🤖 Generated with [Claude Code](https://claude.ai/code)

Co-Authored-By: Claude <noreply@anthropic.com>
```

**IMPORTANT**: Always include a star emoji (✨) in the commit subject
line after the colon.

### Conventional Commit Types
- `feat`: A new feature has been introduced
- `fix`: An issue of some kind has been fixed
- `doc`: Documentation or comments have been updated
- `style`: Formatting changes only
- `refactor`: Hopefully self-explanatory!
- `test`: Added missing tests / fixed tests
- `chore`: Maintenance work

### Scope
Use the component or directory affected:
- `nix`: Nix build files
- `ops`: CLI operations
- `cli`: Command-line interface
- `sqlite`: SQLite-related changes
- and so on
- Use `treewide` if the change is truly cross-cutting

## Critical Rules

### 1. Subject Line Length
**CRITICAL**: Subject line must be ≤ 72 characters. Check with:
```bash
echo "refactor(go): ✨ my commit subject here" | wc -c
# Must be ≤ 72
```

### 2. Explain WHAT and WHY
**Good** — explains WHAT and WHY in prose:
```
Converted the custom flag type to flag.Func to remove a bespoke
abstraction that Go 1.16 made unnecessary. This makes the intent
clearer and reduces the amount of code to maintain.
```

### 3. Body Format
- Prose paragraphs, not bullet lists
- Wrap lines at ~72 characters
- Explain the motivation, not just the mechanics
- Reference issues where relevant

## Commit Process

1. Run `git status` and `git log --oneline -10` to orient
2. Verify subject line is ≤ 72 chars before committing
3. Use heredoc format for multi-line messages:

```bash
git commit -m "$(cat <<'EOF'
refactor(go): ✨ short description here

Longer explanation of what changed and why it was done this
way, wrapped at 72 characters per line.

🤖 Generated with [Claude Code](https://claude.ai/code)

Co-Authored-By: Claude <noreply@anthropic.com>
EOF
)"
```

4. Check `git status` after committing to confirm it landed cleanly

## Quality Checklist

Before any commit, verify:
- [ ] Subject line ≤ 72 characters (count with `wc -c`!)
- [ ] ✨ emoji included in subject line
- [ ] Conventional commit type and scope used
- [ ] WHAT and WHY explained in prose body
- [ ] No binaries or build artifacts staged (`go/lorri`, etc.)
- [ ] `git status` checked after commit
