# Linux sync runbook: purrgotchi v4.3.1 + v4.4.0

Audience: a Claude session running on the Linux machine. Goal: bring this machine up to date with purrgotchi v4.4.0, step by step, verifying each step.

## Ground rules

- Work one step at a time. Report the result of each step's verification before moving on.
- Never edit `~/.claude/settings.json`, any VS Code settings file, or any other config without showing the exact diff and getting an explicit "yes" from the user first. Read-only inspection is fine.
- Back up before overwriting anything (see Rollback). Keep all backups in one directory, `~/.claude/backups/linux-sync-<date>`.
- If a verification fails, stop and report. Do not improvise around it.
- Do not merge, tag or push anything in the repo.

Placeholders: `<repo path>` is the purrgotchi checkout on this machine (find it with `git -C <candidate> remote -v`, or ask the user).

## 0. Context: what changed

Since this machine was last updated:

- **v4.3.1.** The statusline now writes the budget cache itself: `~/.claude/hooks/budget/cache/<session>.json`. It is opt-in: it only writes if `~/.claude/hooks/budget/` exists. The `statusline-wrap.sh` wrapper is no longer needed. The budget hook got a deadline fix. The repo ships a local plugin marketplace so the mod loads without `--plugin-dir`.
- **v4.4.0.** Neon Grid now paints a true-black backdrop (was `#0d0a14`), a new "Neon Grid Violet" Claude theme paints `#1a1633`, and there is a VS Code theme extension in `vscode-theme/`.

## 1. Update the repo and the binary

```bash
git -C <repo path> status --short       # expect clean; if not, stop and ask
git -C <repo path> pull
sed -n 1,5p <repo path>/rust/Cargo.toml   # expect version = "4.4.0"
```

Find how the statusline points at the binary (read-only):

```bash
grep -n -A3 '"statusLine"' ~/.claude/settings.json
which -a purrgotchi; ls -l ~/.local/bin/purrgotchi* 2>/dev/null
```

Check whether the binary is in use:

```bash
pgrep -af purrgotchi
```

Statusline invocations are short-lived, so a hit is usually harmless. On Linux a running binary can be replaced safely by copying to a temp name and renaming over the old one; if `cp` reports "Text file busy", use `cp <new> <dest>.new && mv <dest>.new <dest>`.

Build:

```bash
cd <repo path>/rust && cargo build --release
./target/release/purrgotchi --version    # expect 4.4.0
```

If the statusLine command points at a different path than `target/release/purrgotchi` (for example a copy in `~/.local/bin`), back up that copy and replace it with the new build. Verify with `<that path> --version`.

## 2. Budget pacing scripts

```bash
B=~/.claude/backups/linux-sync-$(date +%F); mkdir -p "$B"
cp -a ~/.claude/hooks/budget/budget.py ~/.claude/hooks/budget/test_budget.py "$B"/ 2>/dev/null
S=<repo path>/docs/budget-pacing/hooks/budget
cp "$S"/budget.py "$S"/test_budget.py ~/.claude/hooks/budget/
cd ~/.claude/hooks/budget && python3 test_budget.py
cmp "$S"/budget.py ~/.claude/hooks/budget/budget.py && cmp "$S"/test_budget.py ~/.claude/hooks/budget/test_budget.py && echo IDENTICAL
```

Expect: all 25 tests run and pass on Linux (2 tests are skipped on Windows only), and `IDENTICAL`.

## 3. Statusline wrapper (needs approval)

Inspect `statusLine` in `~/.claude/settings.json`.

- If the command does not mention `statusline-wrap.sh`, go straight to the verification below.
- If it runs `statusline-wrap.sh`: propose changing the command to run the binary directly, i.e. `<binary path> statusline` (the same binary the wrapper called). Show the exact before/after diff and **ask the user before editing**. Back up `~/.claude/settings.json` into the backup directory first.

Verification (in a new Claude session, or after the next statusline refresh):

```bash
ls -l --time-style=full-iso ~/.claude/hooks/budget/cache/ | tail -3
# wait ~10 seconds while a session is active; the current session's file mtime should advance
BUDGET_NO_NET=1 python3 ~/.claude/hooks/budget/budget.py line   # reads CLAUDE_CODE_SESSION_ID
```

Expect a `<session>.json` for the current session that refreshes, and a budget line printed. If `CLAUDE_CODE_SESSION_ID` is unset in your shell, tell the user and skip the `line` check rather than guessing an id. The old `statusline-wrap.sh` can stay on disk; it is just unused.

## 4. Mod via the local marketplace

First check for a conflicting load of the same plugin (a silent name collision otherwise):

```bash
grep -n "plugin-dir\|CLAUDE_CODE_PLUGIN_DIRS" ~/.bashrc ~/.zshrc ~/.profile ~/.bash_aliases ~/.config/fish/config.fish 2>/dev/null
env | grep CLAUDE_CODE_PLUGIN_DIRS
claude plugin list
```

If a `--plugin-dir .../mod` alias or a `CLAUDE_CODE_PLUGIN_DIRS` entry points at the purrgotchi mod, propose removing it (show the diff, **ask first**).

Install:

```bash
claude plugin marketplace add <repo path>
claude plugin install purrgotchi@purrgotchi-local
claude plugin list        # expect purrgotchi 4.4.0 enabled
```

If it is already installed from the marketplace, run `claude plugin marketplace update purrgotchi-local` and `claude plugin update purrgotchi@purrgotchi-local` instead.

Then ask the user to restart `claude` and, in a real terminal tab, check:

1. `/theme` shows both "Neon Grid" and "Neon Grid Violet". The user picks one.
2. The terminal backdrop paints: black for Neon Grid, `#1a1633` for Neon Grid Violet. It follows the theme chosen in `/theme`.
3. Switching to a non-Neon theme restores the terminal's own background.

You cannot verify the backdrop visually yourself; ask the user to confirm. If it does not change, follow "If the backdrop does not change" in the README.

## 5. VS Code theme

Needs `node` and `code` on PATH (`node --version`, `code --version`).

```bash
cd <repo path>/vscode-theme
npx @vscode/vsce package --skip-license     # the .vsix is gitignored, so build it here
code --install-extension neon-grid-4.4.0.vsix
code --list-extensions | grep -i neon
```

The user then selects "Neon Grid" (Ctrl+K Ctrl+T). "Neon Grid Violet" is the alternative.

Check for leftovers from earlier colour experiments (read-only):

```bash
grep -n "workbench.colorCustomizations\|editor.tokenColorCustomizations\|editor.semanticTokenColorCustomizations" ~/.config/Code/User/settings.json
```

If any are present they override the theme. Show them to the user and **ask before removing** (back up `settings.json` first).

## 6. Final verification checklist

- [ ] `<binary> --version` prints 4.4.0 and the statusLine command points at that binary.
- [ ] `python3 ~/.claude/hooks/budget/test_budget.py` passes (25 tests) and `cmp` shows the scripts identical to the repo.
- [ ] `~/.claude/hooks/budget/cache/<current session>.json` exists and its mtime advances.
- [ ] `BUDGET_NO_NET=1 python3 ~/.claude/hooks/budget/budget.py line` prints a budget line.
- [ ] `claude plugin list` shows purrgotchi 4.4.0 from `purrgotchi-local`, with no `--plugin-dir` or `CLAUDE_CODE_PLUGIN_DIRS` duplicate.
- [ ] `/theme` lists Neon Grid and Neon Grid Violet; backdrop is black / `#1a1633` (user confirmed).
- [ ] VS Code lists the Neon Grid extension and the theme applies; no stale colour customizations.
- [ ] Every config change was shown to and approved by the user.

## 7. Rollback

- Binary: restore the backed-up copy, or check out the previous release tag in the repo and rebuild with `cargo build --release`.
- Budget scripts: copy `budget.py` and `test_budget.py` back from `~/.claude/backups/linux-sync-<date>/`.
- `settings.json`: restore the backup copy taken before the statusLine edit.
- Mod: `claude plugin uninstall purrgotchi@purrgotchi-local` and `claude plugin marketplace remove purrgotchi-local`; restore any removed alias or env entry from its backup.
- VS Code: `code --uninstall-extension <id shown by --list-extensions>`; restore the backed-up `settings.json` if customizations were removed.
