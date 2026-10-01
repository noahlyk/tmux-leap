use std::env;
use std::path::Path;
use std::process::Command;

#[must_use]
pub fn is_inside_tmux() -> bool {
    env::var("TMUX").is_ok()
}

/// Gets a list of all tmux sessions.
///
/// # Panics
/// Panics if the `tmux list-sessions` command fails to execute.
#[must_use]
pub fn get_sessions() -> Vec<String> {
    let output = Command::new("tmux")
        .arg("list-sessions")
        .arg("-F")
        .arg("#{session_name}")
        .output()
        .expect("Failed to list tmux sessions");

    if output.status.success() {
        let sessions = String::from_utf8_lossy(&output.stdout);
        sessions
            .lines()
            .map(std::string::ToString::to_string)
            .collect()
    } else {
        Vec::new()
    }
}

/// Checks if a tmux session with the given name exists.
///
/// # Panics
/// Panics if the `tmux list-sessions` command fails to execute.
#[must_use]
pub fn session_exists(session_name: &str) -> bool {
    let output = Command::new("tmux")
        .arg("list-sessions")
        .output()
        .expect("Failed to list tmux sessions");

    let sessions = String::from_utf8_lossy(&output.stdout);
    sessions
        .lines()
        .any(|line| line.starts_with(&format!("{session_name}:")))
}

/// Creates a new tmux session with the given name in the specified directory.
///
/// # Panics
/// Panics if:
/// - The directory change fails
/// - The `tmux new-session` command fails to execute
#[must_use]
pub fn create_session(session_name: &str, dir: &str) -> bool {
    env::set_current_dir(Path::new(dir))
        .unwrap_or_else(|_| panic!("Failed to change directory to {dir}"));
    Command::new("tmux")
        .arg("new-session")
        .arg("-d")
        .arg("-s")
        .arg(session_name)
        .status()
        .expect("Failed to create new tmux session")
        .success()
}

/// Kills the tmux session with exactly this name.
///
/// # Panics
/// Panics if the `tmux kill-session` command fails to execute.
#[must_use]
pub fn kill_session(session_name: &str) -> bool {
    Command::new("tmux")
        .arg("kill-session")
        .arg("-t")
        .arg(format!("={session_name}"))
        .status()
        .expect("Failed to kill tmux session")
        .success()
}

/// Switches the current tmux client to the specified session.
///
/// # Panics
/// Panics if the `tmux switch-client` command fails to execute.
#[must_use]
pub fn switch_client(session_name: &str) -> bool {
    Command::new("tmux")
        .arg("switch-client")
        .arg("-t")
        .arg(session_name)
        .status()
        .expect("Failed to switch tmux client")
        .success()
}

/// Attaches to the specified tmux session.
///
/// # Panics
/// Panics if the `tmux attach-session` command fails to execute.
#[must_use]
pub fn attach_session(session_name: &str) -> bool {
    Command::new("tmux")
        .arg("attach-session")
        .arg("-t")
        .arg(session_name)
        .env_remove("TMUX")
        .status()
        .expect("Failed to attach to tmux session")
        .success()
}

/// Gets the name of the current tmux session.
///
/// # Panics
/// Panics if the `tmux display-message` command fails to execute.
#[must_use]
pub fn get_current_session() -> Option<String> {
    let output = Command::new("tmux")
        .arg("display-message")
        .arg("-p")
        .arg("#S")
        .output()
        .expect("Failed to execute tmux command");
    if output.status.success() {
        let session_name = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Some(session_name)
    } else {
        eprintln!("Failed to get current tmux session name");
        None
    }
}

/// Attaches to the specified tmux session, replacing the current process.
/// This is used when running from outside tmux and you want to process to persist.
///
/// # Panics
/// Panics if the `exec` call fails or the tmux command is not found.
pub fn attach_session_exec(session_name: &str, dir: &str) -> ! {
    use std::os::unix::process::CommandExt;
    use std::path::Path;

    let escaped_name = if session_name == "~" {
        r"\~".to_string()
    } else {
        session_name.to_string()
    };

    // Set working directory before attaching
    env::set_current_dir(Path::new(dir))
        .unwrap_or_else(|_| panic!("Failed to change directory to {}", dir));

    let mut command = std::process::Command::new("tmux");
    command
        .arg("attach-session")
        .arg("-t")
        .arg(&escaped_name)
        .env_remove("TMUX");

    // Replace the current process with tmux
    let result = command.exec();
    // exec replaces the process, so this never runs unless exec fails
    eprintln!("Failed to attach to tmux session: {}", result);
    std::process::exit(1);
}
