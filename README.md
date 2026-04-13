# AI Skill Manager

A terminal UI (TUI) for managing agent skills across multiple coding agents — Claude, Amp, Cursor, Windsurf, and more.

![Rust](https://img.shields.io/badge/Rust-1.70%2B-orange) ![License](https://img.shields.io/badge/license-MIT-blue)

## The Problem

Modern AI coding agents use "skills" — markdown files (typically `SKILL.md`) that provide domain-specific instructions and workflows. Each agent stores skills in its own directory:

- `~/.claude/skills/`
- `~/.config/agents/skills/`
- `~/.agents/skills/`
- `~/.config/amp/skills/`
- `~/.cursor/skills/`
- `~/.codex/skills/`
- `~/.codeium/windsurf/skills/`

Every installed skill is loaded into the system context as frontmatter on **every request**, consuming tokens whether the skill is relevant or not. With dozens of skills installed, this adds up fast — burning through context windows and increasing cost per interaction.

On top of that, managing the same skills across all these locations is tedious and error-prone. Skills get duplicated, go out of sync, or are forgotten entirely.

## The Solution

AI Skill Manager introduces a **central store** at `~/.config/skillmanager/skills/` and manages symlinks to each agent's skill directory. You get:

- **One source of truth** — skills live in one place, symlinked everywhere
- **Activate/deactivate** — toggle skills on or off per agent with a single keypress
- **Group toggles** — define bundles of skills and enable or disable them together
- **Auto-discovery** — on launch, detects unmanaged skills scattered across agent directories and offers to import them
- **Search & filter** — quickly find skills by name or description

## Installation

### Build from source

```bash
git clone https://github.com/daveashworth/ai-skill-manager.git
cd ai-skill-manager
cargo build --release
```

The binary is at `target/release/skill-manager` (~1.4MB). Add it to your PATH:

```bash
# Symlink to a directory already in your PATH
ln -sf "$(pwd)/target/release/skill-manager" ~/.local/bin/skill-manager
```

## Usage

```bash
skill-manager
```

### First Run

On first launch, Skill Manager scans all known agent skill directories. If it finds skills that aren't in the central store, it presents an import dialog.

If you choose to import them:

- The skill directories are copied into `~/.config/skillmanager/skills/`
- The original copies in agent-specific folders are replaced with symlinks
- Imported skills start out active so your agents keep working immediately

After that, the main screen shows all managed skills in one list. You can toggle a single skill, search, or switch over to groups.

### Quick Start

If you've never used the tool before, this is the fastest way to get value from it:

1. Run `skill-manager`.
2. Import any unmanaged skills the app finds.
3. Move through the skill list with `j`/`k` or the arrow keys.
4. Press `Space` to turn a single skill on or off.
5. Press `Tab` to focus the Groups panel on the right.
6. Press `n` to create your first group.
7. Type a group name and press `Enter`.
8. In the member editor, press `Space` to include or remove highlighted skills, then press `Enter` to save.
9. Back in the Groups panel, press `Space` or `Enter` to turn the entire group on or off at once.

When a skill's friendly name and real managed key differ, the UI shows both so you can tell exactly what will be grouped or toggled.

### Keyboard Shortcuts

| Key | Action |
|---|---|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `Tab` | Switch focus between skills and groups |
| `Space` / `Enter` | Toggle the selected skill or group |
| `n` | Create a group when the Groups panel is focused |
| `e` | Edit the selected group's members |
| `r` | Rename the selected group |
| `x` | Delete the focused skill or group |
| `/` | Search / filter skills |
| `a` | Activate all skills |
| `d` | Deactivate all skills |
| `Esc` | Clear search filter |
| `q` | Quit |

## How It Works

```
~/.config/skillmanager/
├── config.toml          # Tracks which skills are active
└── skills/              # Central store (source of truth)
    ├── frontend-design/
    │   └── SKILL.md
    ├── code-review/
    │   └── SKILL.md
    └── ...

~/.claude/skills/
├── frontend-design -> ~/.config/skillmanager/skills/frontend-design  (symlink)
└── code-review -> ~/.config/skillmanager/skills/code-review          (symlink)

~/.config/amp/skills/
├── frontend-design -> ~/.config/skillmanager/skills/frontend-design  (symlink)
└── ...
```

1. **Central store** — all skill files live under `~/.config/skillmanager/skills/`
2. **Symlinks** — when a skill is active, symlinks are created in each agent's skill directory pointing back to the central store
3. **Deactivation** — when toggled off, the symlinks are removed (the skill is preserved in the central store)
4. **Config** — `~/.config/skillmanager/config.toml` tracks active/inactive state and target directories
5. **Groups** — optional bundles can toggle multiple canonical skill keys together

### Configuration

The config file at `~/.config/skillmanager/config.toml` is created automatically:

```toml
[targets]
dirs = [
    "~/.claude/skills",
    "~/.config/agents/skills",
    "~/.agents/skills",
    "~/.config/amp/skills",
    "~/.cursor/skills",
    "~/.codex/skills",
    "~/.codeium/windsurf/skills",
]

[skills]
# Each skill's active state is tracked here
# e.g. frontend-design = { active = true }

[groups]
# Group members must use canonical managed keys, not display names
core = ["frontend-design", "code-review"]
shipping = ["ship", "qa", "gstack-review"]
```

Add or remove directories from `targets.dirs` to control which agents are managed.

You do not need to edit this file by hand to use groups. The TUI can create, rename, edit, and delete groups for you. Manual config editing is still useful if you want to seed groups ahead of time or version them in dotfiles.

The TUI includes a Groups panel. Press `Tab` to focus it, then use `j`/`k` or the arrow keys to select a group and `Space` or `Enter` to toggle the whole bundle.

You can also manage groups directly in the TUI:

- Press `n` to create a group
- Press `e` to edit the selected group's members
- Press `r` to rename the selected group
- Press `x` to delete the selected group

Group membership is stored with canonical keys so it stays aligned with symlink syncing and the startup repair logic. If two skills have similar display names, use the `key:` line shown in the skill list and details pane to disambiguate them.

## Skill Format

Skills are directories containing a `SKILL.md` file with optional YAML frontmatter:

```markdown
---
name: my-skill
description: What this skill does
metadata:
  version: '1.0.0'
  author: Your Name
---

# Skill instructions here...
```

## Tech Stack

- **Rust** — fast, single binary, no runtime dependencies
- [ratatui](https://github.com/ratatui/ratatui) — terminal UI framework
- [crossterm](https://github.com/crossterm-rs/crossterm) — cross-platform terminal manipulation

## License

MIT
