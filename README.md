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
- `~/.codeium/windsurf/skills/`

Managing the same skills across all these locations is tedious and error-prone. Skills get duplicated, go out of sync, or are forgotten entirely.

## The Solution

AI Skill Manager introduces a **central store** at `~/.config/skillmanager/skills/` and manages symlinks to each agent's skill directory. You get:

- **One source of truth** — skills live in one place, symlinked everywhere
- **Activate/deactivate** — toggle skills on or off per agent with a single keypress
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

On first launch, Skill Manager scans all known agent skill directories. If it finds skills that aren't in the central store, it presents an import dialog — skills are copied to the central store and replaced with symlinks.

### Keyboard Shortcuts

| Key | Action |
|---|---|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `Space` / `Enter` | Toggle skill active/inactive |
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
    "~/.codeium/windsurf/skills",
]

[skills]
# Each skill's active state is tracked here
# e.g. frontend-design = { active = true }
```

Add or remove directories from `targets.dirs` to control which agents are managed.

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
