# tmux-leap 🚀

**Supercharge your tmux workflow with lightning-fast session management**

tmux-leap is a powerful CLI tool that lets you instantly navigate between tmux sessions using fuzzy search. Stop wasting time typing long session names or scrolling through lists - just leap to where you need to be!



## ✨ Features

- **Blazing Fast Navigation**: Instantly jump between tmux sessions with fuzzy find
- **Smart Session Management**: Automatically creates sessions
- **Recursive Directory Support**: Scan subdirectories at configurable depths
- **Intelligent Caching**: Remembers your last leaps

## 🔧 Installation

### One-line Install

```bash
curl -fsSL https://raw.githubusercontent.com/noahlyk/tmux-leap/main/install.sh | bash
```
**Note:** For security, inspect the install script before running it. View it [here](https://github.com/noahlyk/tmux-leap/blob/main/install.sh).

### Manual Installation (from source)

```bash
# Clone the repository
git clone https://github.com/noahlyk/tmux-leap.git
cd tmux-leap

# Build and install
cargo build --release
sudo cp target/release/tmux-leap /usr/local/bin/
```

## 🚀 Getting Started

### Basic Usage

Simply run `tmux-leap` to see a fuzzy finder with all your available sessions and projects:
```bash
tmux-leap
```

### Managing Projects

Add directories to your projects list:
```bash
tmux-leap add                                    # adds current directory
tmux-leap add ~/projects/awesome-project         # adds specific directory
tmux-leap add ~/projects/awesome-project --depth 2  # includes subdirectories
```

View and manage your projects:
```bash
tmux-leap list    # view all projects
tmux-leap delete  # remove a project (interactive)
tmux-leap edit    # edit config.ron directly in $EDITOR
```

Interactively chooses a project to set the depth to recursively include subdirectories:
```bash
tmux-leap set-depth
```

## ⌨️ Recommended tmux Keybindings

Add one of these to your `~/.tmux.conf` for quick access:

### Option 1: No tmux prefix (Ctrl+F)

```bash
bind-key -n C-f popup -E -d '#{pane_current_path}' 'tmux-leap'
```

### Option 2: With tmux prefix (Prefix+Ctrl+F)

```bash
bind-key C-f popup -E -d '#{pane_current_path}' 'tmux-leap'
```

## 🔍 How It Works

tmux-leap maintains a list of your projects in `~/.config/tmux-leap/config.ron` and intelligently combines them with existing tmux sessions. When you select a project:

1. If a tmux session already exists for that directory, it switches to it
2. If no session exists, it creates a new one and attaches to it
3. Your most frequently used sessions are cached for faster access

### Picker keys

| Key | Action |
|-----|--------|
| `enter` | Open the project (switch to its session, or create one) |
| `ctrl-o` | Open the project and run the `launch` command in a fresh pane: the first pane of a new session, or a new window of an open one. Only bound when `launch` (or `TMUX_LEAP_LAUNCH`) is set |
| `tab` | Mark several projects |
| `ctrl-x` | Kill the tmux sessions of the marked projects (or the highlighted one) after one `[y/N]` prompt. The list reloads; projects stay in it |
| `ctrl-t` | Toggle between all projects and only projects with an open session. The prompt gets a `● ` prefix while filtered |

```ron
launch: "claude",   // in config.ron: ctrl-o opens the project with Claude Code
```

The command is typed into the pane's shell, so it runs with your interactive shell setup.

## 🛠️ Configuration

Everything lives in `~/.config/tmux-leap/config.ron`. It is created on first run, and `tmux-leap edit` opens it. It's plain [RON](https://github.com/ron-rs/ron), commented, and safe to hand-edit; only `projects` is required. A reference copy ships at `/usr/share/tmux-leap/config.ron.template` when installed from a package.

```ron
(
    annotate_status: true,
    preview_screen: true,

    // annotate: "tmux list-sessions -F \"#{session_name}\\t#{session_windows} windows\"",
    // preview_command: "ls -la {4}",
    // launch: "claude",
    // path_width: 80,

    projects: [
        "~/projects/awesome-project",
        "~/personal/blog",
        "~/Downloads",
        "~/work/client-project --depth 2",
        "~ --depth 1",
    ],
)
```

`tmux-leap add`, `delete` and `set-depth` edit the `projects` list in place and leave your comments and settings alone. The `--depth` suffix includes subdirectories up to that depth.

> **Upgrading from v1.x?** Your existing `~/.projects` and `~/.projects_cache` are copied into the new location the first time you run tmux-leap. The originals are left untouched.

| Setting | Environment variable (wins over the file) | Default |
|---------|-------------------------------------------|---------|
| `annotate_status` | | `true` in a new config |
| `annotate` | `TMUX_LEAP_ANNOTATE` | unset |
| `preview_screen` | | `true` in a new config |
| `preview_command` | | unset |
| `launch` | `TMUX_LEAP_LAUNCH` | unset: `ctrl-o` not bound |
| `path_width` | `TMUX_LEAP_PATH_WIDTH` | 80 |

With neither `annotate_status` nor `annotate` the list is the plain path list.

### Session annotations

Projects with an open tmux session get a green dot and a bold folder name; the rest are dimmed. Paths longer than `path_width` characters are cut from the left with `…`, so the folder name stays visible.

- `annotate_status: true` adds a small Claude Code status after the path: one `cod-claude` glyph (U+EC82, from a [Nerd Font](https://www.nerdfonts.com/)) per distinct agent state among that session's Claude panes, with a count when several panes share it. Three colours: needs input (yellow, rendered first), just finished (green), and everything else (grey). Colours follow your tmux theme's `@thm_yellow`, `@thm_green` and `@thm_overlay_1` when set. State is read from the per-pane tmux option `@claude_state`, which tmux-leap does not set itself. A Claude Code hook should (`UserPromptSubmit`/`PostToolUse` to `working`, a permission `Notification` to `waiting`, `Stop` to `done`), for example `tmux set -p -t "$TMUX_PANE" @claude_state waiting`. Without it, Claude panes show as one neutral icon.
- `annotate: "..."` (or `TMUX_LEAP_ANNOTATE`) replaces that with your own shell command, which prints one `session_name<TAB>annotation` line per session. ANSI colours are kept. It takes priority over `annotate_status`.

### Preview pane

- `preview_screen: true` shows the live screen of the highlighted project's tmux session on the right. The pane hides itself for projects with no session.
- `preview_command: "..."` replaces it with your own command. In it, `{4}` is the project's absolute path and `{3}` its tmux session name, e.g. `preview_command: "ls -la {4}"`. It takes priority over `preview_screen`.

## 📚 Command Reference

| Command | Description |
|---------|-------------|
| `tmux-leap` | Main command - shows fuzzy finder |
| `tmux-leap add [dir] [--depth N]` | Add current or specified directory with optional depth |
| `tmux-leap delete` | Remove a project (interactive) |
| `tmux-leap list` | List all projects |
| `tmux-leap status` | Show the config's project list |
| `tmux-leap set-depth` | Set recursive depth for a project (interactive) |
| `tmux-leap kill-session <dir>...` | Kill the tmux sessions of projects (asks y/N once) |
| `tmux-leap edit` | Edit config.ron in your default editor $EDITOR |
| `tmux-leap completion <shell>` | Generate shell completions |

## 🤝 Contributing

Contributions are welcome! Feel free to open issues or submit pull requests.

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
