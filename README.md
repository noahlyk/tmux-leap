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
tmux-leap edit    # edit projects file directly in $EDITOR
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

tmux-leap maintains a list of your projects in `~/.projects` and intelligently combines them with existing tmux sessions. When you select a project:

1. If a tmux session already exists for that directory, it switches to it
2. If no session exists, it creates a new one and attaches to it
3. Your most frequently used sessions are cached for faster access

## 🛠️ Configuration

Your projects are stored in `~/.projects` with a simple format:

```
~/projects/awesome-project
~/personal/blog
~/Downloads
     etc...
```

The `--depth` option allows you to include subdirectories up to the specified depth.

```
~/work/client-project --depth 2
~ --depth 1
~/.config --depth 1
~/projects --depth 1
     etc...
```

## 📚 Command Reference

| Command | Description |
|---------|-------------|
| `tmux-leap` | Main command - shows fuzzy finder |
| `tmux-leap add [dir] [--depth N]` | Add current or specified directory with optional depth |
| `tmux-leap delete` | Remove a project (interactive) |
| `tmux-leap list` | List all projects |
| `tmux-leap status` | Show raw projects file content |
| `tmux-leap set-depth` | Set recursive depth for a project (interactive) |
| `tmux-leap edit` | Edit projects file in your default editor $EDITOR |
| `tmux-leap completion <shell>` | Generate shell completions |

## 🤝 Contributing

Contributions are welcome! Feel free to open issues or submit pull requests.

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
