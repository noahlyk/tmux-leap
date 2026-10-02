pub mod tmux;

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use dirs::home_dir;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::{env, thread};
use tempfile::NamedTempFile;
extern crate whoami;

const PROJECTS_FILE: &str = ".projects";
const CACHE_FILE: &str = ".projects_cache";
const FZF_LAYOUT: &str = "--layout=reverse --no-border --cycle --extended";
const FZF_PICKER_FIELDS: &str = "--ansi --delimiter='\\t' --with-nth=2..";
const ANNOTATE_COMMAND_ENV: &str = "TMUX_LEAP_ANNOTATE";
const LAUNCH_COMMAND_ENV: &str = "TMUX_LEAP_LAUNCH";
const LAUNCH_KEY: &str = "ctrl-o";
const OPEN_ONLY_PROMPT_MARK: &str = "● ";
const LONGEST_PATH_WIDTH_ENV: &str = "TMUX_LEAP_PATH_WIDTH";
const DEFAULT_LONGEST_PATH_WIDTH: usize = 80;
const MAX_CACHE_ENTRIES: usize = 100;

#[derive(Debug, Parser)]
#[command(name = "tmux-leap", about = "fzf through a list of directories", version = env!("CARGO_PKG_VERSION"))]
struct Opt {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Add a project to the .projects file
    #[command(name = "add", aliases = &["a"])]
    Add {
        /// The project directory to add. If not provided, the current directory will be added.
        dir: Option<String>,

        /// Set a recursive depth for subdirectories
        #[arg(long)]
        depth: Option<u32>,
    },
    /// Delete a project from the .projects file
    #[command(name = "delete", aliases = &["del", "d"])]
    Delete,
    /// List all projects in the .projects file
    #[command(name = "list", aliases = &["ls", "l"])]
    List,
    /// Display the contents of the .projects file
    #[command(name = "status", aliases = &["stat", "s"])]
    Status,
    /// Set or remove depth for a project
    #[command(name = "set-depth", aliases = &["depth", "sd"])]
    SetDepth,
    /// Edit the .projects file in your default editor
    #[command(name = "edit", aliases = &["e"])]
    Edit,
    /// Kill the tmux sessions of one or more projects, after a y/N prompt
    #[command(name = "kill-session", aliases = &["kill", "k"])]
    KillSession {
        /// The project directories whose sessions should be killed
        #[arg(required = true)]
        dirs: Vec<String>,
    },
    /// Print the picker lines; only projects with a session when the fzf prompt starts with "● "
    #[command(name = "picker-lines", hide = true)]
    PickerLines,
    /// Print the fzf actions that toggle the picker between all projects and open sessions only
    #[command(name = "picker-toggle-open-only", hide = true)]
    PickerToggleOpenOnly,
    /// Go directly to a project path
    #[command(name = "goto", aliases = &["g"])]
    Goto {
        /// The project directory to go to
        dir: String,
    },
    /// Generate shell completion scripts
    #[command(name = "completion", aliases = &["comp", "c"])]
    Completion {
        #[arg(value_enum)]
        shell: Shell,
    },
}

#[derive(Debug, Clone)]
struct Project {
    shortened_path: String,
    expanded_path: String,
    tmux_display_path: String,
}

impl Project {
    fn new(path: &str) -> Self {
        let expanded_path = shellexpand::tilde(&path)
            .to_string()
            .trim_end_matches('/')
            .to_string();
        let shortened_path = Self::shorten_path(&expanded_path)
            .trim_end_matches('/')
            .to_string();
        let tmux_display_path = Self::format_for_tmux(&shortened_path)
            .trim_end_matches('/')
            .to_string();

        Self {
            shortened_path,
            expanded_path,
            tmux_display_path,
        }
    }

    fn shorten_path(path_str: &str) -> String {
        let path = PathBuf::from(path_str);
        let home = home_dir().expect("Unable to find home directory");

        if let Ok(relative) = path.strip_prefix(&home) {
            format!("~/{}", relative.display())
        } else {
            path.display().to_string()
        }
    }

    fn format_for_tmux(path: &str) -> String {
        path.replace('.', "_")
    }

    fn to_fzf_display(&self) -> &str {
        &self.shortened_path
    }


    fn to_fzf_line(&self, annotations: Option<&SessionAnnotations>) -> String {
        let Some(annotations) = annotations else {
            return format!("{path}\t{path}", path = self.shortened_path);
        };
        let bold = "\x1b[1m";
        let dim = "\x1b[2m";
        let green = "\x1b[32m";
        let reset = "\x1b[0m";
        let path = &self.shortened_path;
        let shown_path = truncate_from_left(path, longest_path_width());
        match annotations.get(&self.tmux_display_path) {
            Some(annotation) => {
                let (parent, folder) = shown_path
                    .rfind('/')
                    .map_or(("", shown_path.as_str()), |slash| shown_path.split_at(slash + 1));
                format!("{path}\t{green}●{reset} {parent}{bold}{folder}{reset}  {annotation}")
            }
            None => format!("{path}\t  {dim}{shown_path}{reset}"),
        }
    }

    fn exists(&self) -> bool {
        let path = PathBuf::from(&self.expanded_path);
        path.exists() && path.is_dir()
    }

    fn attach(&self) {
        self.attach_running(None);
    }

    /// Attaches like `attach`, and when `launch_command` is given types it into a
    /// fresh pane: the first pane of a new session, or a new window of an open one.
    fn attach_running(&self, launch_command: Option<&str>) {
        let tmux_session_name = &self.tmux_display_path;

        let session_exists = tmux::session_exists(tmux_session_name);

        if !session_exists {
            if !tmux::create_session(tmux_session_name, &self.expanded_path) {
                eprintln!("Failed to create new tmux session");
                return;
            }
        }

        if let Some(command) = launch_command {
            let launch_pane = if session_exists {
                tmux::create_window(tmux_session_name, &self.expanded_path)
            } else {
                Some(format!("={tmux_session_name}:"))
            };
            match launch_pane {
                Some(pane) if tmux::run_in_pane(&pane, command) => {}
                _ => eprintln!("Failed to launch \"{command}\""),
            }
        }

        if tmux::is_inside_tmux() {
            let escaped_name = if tmux_session_name == "~" {
                "\\~".to_string()
            } else {
                tmux_session_name.to_string()
            };

            if !tmux::switch_client(&escaped_name) {
                eprintln!("Failed to switch tmux client");
            }
        } else if !tmux::attach_session(tmux_session_name) {
            eprintln!("Failed to attach to tmux session");
        }
    }
}

trait FilterExists {
    fn filter_exists(&self) -> Vec<Project>;
}

impl FilterExists for Vec<Project> {
    fn filter_exists(&self) -> Vec<Project> {
        self.iter()
            .filter(|project| project.exists())
            .cloned()
            .collect()
    }
}

fn main() {
    let opt = Opt::parse();
    match opt.command {
        Some(Commands::Add { dir, depth }) => add_project(dir.as_deref(), depth),
        Some(Commands::Delete) => delete_project(),
        Some(Commands::List) => list_projects(),
        Some(Commands::Status) => status_projects(),
        Some(Commands::SetDepth) => set_depth(),
        Some(Commands::Edit) => edit_projects_file(),
        Some(Commands::KillSession { dirs }) => kill_project_sessions(&dirs),
        Some(Commands::PickerLines) => print_picker_lines(),
        Some(Commands::PickerToggleOpenOnly) => print_open_only_toggle_actions(),
        Some(Commands::Goto { dir }) => goto_project(&dir),
        Some(Commands::Completion { shell }) => generate_completion(shell),
        None => execution(),
    }
}

fn longest_path_width() -> usize {
    env::var(LONGEST_PATH_WIDTH_ENV)
        .ok()
        .and_then(|width| width.parse().ok())
        .filter(|width| *width > 1)
        .unwrap_or(DEFAULT_LONGEST_PATH_WIDTH)
}

/// Keeps the end of the text (the folder name) and marks the cut with `…`.
fn truncate_from_left(text: &str, max_width: usize) -> String {
    let width = text.chars().count();
    if width <= max_width {
        return text.to_string();
    }
    let kept_tail: String = text.chars().skip(width - (max_width - 1)).collect();
    format!("…{kept_tail}")
}

fn generate_completion(shell: Shell) {
    let mut cmd = Opt::command();
    let bin_name = env!("CARGO_PKG_NAME");
    generate(shell, &mut cmd, bin_name, &mut std::io::stdout());
}

fn get_home_path(file: &str) -> PathBuf {
    let mut file = file;
    if file.starts_with('~') {
        file = file.strip_prefix("~").unwrap();
    }
    home_dir()
        .expect("Unable to find home directory")
        .join(file)
}

fn touch_file(path: &PathBuf) {
    OpenOptions::new()
        .create(true)
        .write(true)
        .open(path)
        .unwrap();
}

fn read_lines<P>(filename: P) -> std::io::Result<Vec<String>>
where
    P: AsRef<std::path::Path>,
{
    let file = File::open(filename)?;
    let buf = BufReader::new(file);
    buf.lines().collect()
}

fn write_lines<P>(filename: P, lines: &[String]) -> std::io::Result<()>
where
    P: AsRef<std::path::Path>,
{
    let mut file = File::create(filename)?;
    for line in lines {
        writeln!(file, "{line}")?;
    }
    Ok(())
}

fn add_project(dir: Option<&str>, depth: Option<u32>) {
    let projects_file = get_home_path(PROJECTS_FILE);
    touch_file(&projects_file);
    let current_dir = env::current_dir().unwrap().to_str().unwrap().to_string();
    let dir = dir.unwrap_or(&current_dir).to_string();
    let project = Project::new(&dir);
    let mut lines = read_lines(&projects_file).unwrap_or_else(|_| vec![]);

    // Remove any existing entry for this path
    lines.retain(|line| {
        let re = Regex::new(r"(.*) --depth \d+").unwrap();
        if let Some(captures) = re.captures(line) {
            let path = captures.get(1).unwrap().as_str();
            path != project.shortened_path
        } else {
            line != &project.shortened_path
        }
    });

    // Add the path with depth if specified
    if let Some(depth_value) = depth {
        lines.push(format!(
            "{} --depth {}",
            project.shortened_path, depth_value
        ));
        println!(
            "Added \"{}\" to .projects with depth {}",
            project.shortened_path, depth_value
        );
    } else {
        lines.push(project.shortened_path.clone());
        println!("Added \"{}\" to .projects", project.shortened_path);
    }

    write_lines(&projects_file, &lines).unwrap();
}

fn delete_project() {
    let projects_file = get_home_path(PROJECTS_FILE);
    let lines = read_lines(&projects_file).unwrap_or_else(|_| vec![]);
    let mut selected = Command::new("fzf")
        .args(FZF_LAYOUT.split_whitespace())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to execute fzf");
    {
        let fzf_stdin = selected.stdin.as_mut().expect("Failed to open fzf stdin");
        fzf_stdin
            .write_all(lines.join("\n").as_bytes())
            .expect("Failed to write to fzf stdin");
    }
    let output = selected
        .wait_with_output()
        .expect("Failed to read fzf output");
    if !output.stdout.is_empty() {
        let selected_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let new_lines: Vec<String> = lines
            .into_iter()
            .filter(|line| line != &selected_str)
            .collect();
        write_lines(&projects_file, &new_lines).unwrap();
        println!("Deleted \"{selected_str}\" from .projects");
    }
}

type SessionAnnotations = HashMap<String, String>;

/// Runs the command in `TMUX_LEAP_ANNOTATE`, which prints `session_name<TAB>annotation`
/// lines. Returns `None` when the variable is unset, keeping the plain path list.
fn load_session_annotations() -> Option<SessionAnnotations> {
    let command = env::var(ANNOTATE_COMMAND_ENV).ok()?;
    let annotations = Command::new("sh")
        .arg("-c")
        .arg(&command)
        .output()
        .map(|output| {
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter_map(|line| line.split_once('\t'))
                .map(|(session_name, annotation)| {
                    (session_name.to_string(), annotation.to_string())
                })
                .collect()
        })
        .unwrap_or_default();
    Some(annotations)
}

fn get_tmux_sessions() -> Vec<Project> {
    tmux::get_sessions()
        .iter()
        .map(|i| Project::new(i))
        .collect()
}

fn get_projects() -> Vec<Project> {
    let projects_file = get_home_path(PROJECTS_FILE);
    let mut projects = Vec::new();
    let mut unique_projects = HashSet::new();
    if let Ok(lines) = read_lines(&projects_file) {
        let re = Regex::new(r"(.*) --depth (\d+)").unwrap();
        for line in lines {
            if let Some(captures) = re.captures(&line) {
                let dir = captures.get(1).unwrap().as_str();
                let depth = captures.get(2).unwrap().as_str().parse::<u32>().unwrap();
                projects.push(Project::new(dir));
                let project = Project::new(dir);
                let sub_dirs = Command::new("find")
                    .arg("-L")
                    .arg(&project.expanded_path)
                    .arg("-maxdepth")
                    .arg(depth.to_string())
                    .arg("-type")
                    .arg("d")
                    .output()
                    .expect("Failed to execute find");
                let sub_dirs = String::from_utf8_lossy(&sub_dirs.stdout);
                for sub_dir in sub_dirs.lines() {
                    projects.push(Project::new(sub_dir));
                }
            } else {
                projects.push(Project::new(&line));
            }
        }
    }
    projects.extend(get_tmux_sessions());
    projects
        .into_iter()
        .filter(|project| unique_projects.insert(project.expanded_path.clone()))
        .collect()
}

fn prepare_fzf_content_from_cache(
    cache_file: &PathBuf,
    output: &mut impl Write,
    annotations: Option<&SessionAnnotations>,
) -> Vec<String> {
    let current_session = tmux::get_current_session();
    read_lines(cache_file)
        .unwrap_or_else(|_| vec![])
        .into_iter()
        .map(|line| Project::new(&line))
        .filter(|project| {
            current_session
                .as_ref()
                .is_none_or(|session| project.tmux_display_path != *session)
        })
        .filter(Project::exists)
        .scan(HashSet::new(), |seen, project| {
            if !seen.insert(project.expanded_path.clone()) {
                return Some(None);
            }
            writeln!(output, "{}", project.to_fzf_line(annotations)).ok()?;
            Some(Some(project.shortened_path.clone()))
        })
        .flatten()
        .collect()
}

fn write_remaining_projects(
    output: &mut impl Write,
    seen_items: &mut HashSet<String>,
    annotations: Option<&SessionAnnotations>,
) {
    let projects = load_and_filter_projects();
    for (path, line) in prepare_fzf_content(&projects, annotations) {
        if seen_items.insert(path) && writeln!(output, "{line}").is_err() {
            return;
        }
    }
}

fn picker_shows_open_only() -> bool {
    env::var("FZF_PROMPT").is_ok_and(|prompt| prompt.starts_with(OPEN_ONLY_PROMPT_MARK))
}

fn print_picker_lines() {
    let cache_file = get_home_path(CACHE_FILE);
    let annotations = load_session_annotations();
    let mut picker_lines = Vec::new();
    let cache_lines =
        prepare_fzf_content_from_cache(&cache_file, &mut picker_lines, annotations.as_ref());
    let mut seen_items: HashSet<String> = cache_lines.into_iter().collect();
    write_remaining_projects(&mut picker_lines, &mut seen_items, annotations.as_ref());

    let open_sessions: Option<HashSet<String>> =
        picker_shows_open_only().then(|| tmux::get_sessions().into_iter().collect());
    let mut stdout = std::io::stdout().lock();
    for line in String::from_utf8_lossy(&picker_lines).lines() {
        let path = line.split('\t').next().unwrap_or_default();
        let is_shown = open_sessions
            .as_ref()
            .is_none_or(|sessions| sessions.contains(&Project::new(path).tmux_display_path));
        if is_shown && writeln!(stdout, "{line}").is_err() {
            return;
        }
    }
}

/// Picks an fzf action delimiter that does not occur in `argument`.
fn fzf_action(name: &str, argument: &str) -> String {
    let delimiters = [('(', ')'), ('[', ']'), ('{', '}'), ('<', '>'), ('~', '~'), ('!', '!')];
    let (open, close) = delimiters
        .into_iter()
        .find(|(open, close)| !argument.contains(*open) && !argument.contains(*close))
        .unwrap_or(('(', ')'));
    format!("{name}{open}{argument}{close}")
}

fn print_open_only_toggle_actions() {
    let prompt = env::var("FZF_PROMPT").unwrap_or_default();
    let toggled_prompt = match prompt.strip_prefix(OPEN_ONLY_PROMPT_MARK) {
        Some(all_projects_prompt) => all_projects_prompt.to_string(),
        None => format!("{OPEN_ONLY_PROMPT_MARK}{prompt}"),
    };
    println!(
        "{}+{}",
        fzf_action("change-prompt", &toggled_prompt),
        fzf_action("reload", &format!("{} picker-lines", own_executable_quoted()))
    );
}

fn kill_project_sessions(dirs: &[String]) {
    let session_names: Vec<String> = dirs
        .iter()
        .map(|dir| Project::new(dir).tmux_display_path)
        .filter(|session_name| tmux::session_exists(session_name))
        .collect();
    match session_names.as_slice() {
        [] => return,
        [session_name] => print!("\n  Kill session {session_name}? [y/N] "),
        _ => print!(
            "\n  Kill {} sessions?\n    {}\n  [y/N] ",
            session_names.len(),
            session_names.join("\n    ")
        ),
    }
    std::io::stdout().flush().ok();
    if !matches!(read_single_key(), Some(b'y' | b'Y')) {
        return;
    }
    for session_name in &session_names {
        if !tmux::kill_session(session_name) {
            eprintln!("Failed to kill tmux session {session_name}");
        }
    }
}

/// Reads one key from the terminal without waiting for Enter.
fn read_single_key() -> Option<u8> {
    let run_stty = |args: &[&str]| -> Option<String> {
        let tty = File::open("/dev/tty").ok()?;
        let output = Command::new("stty").args(args).stdin(tty).output().ok()?;
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    };
    let saved_terminal_state = run_stty(&["-g"])?;
    run_stty(&["-icanon", "-echo", "min", "1"])?;
    let mut key = [0u8; 1];
    let read_result = File::open("/dev/tty").and_then(|mut tty| std::io::Read::read_exact(&mut tty, &mut key));
    run_stty(&[&saved_terminal_state]);
    read_result.ok().map(|()| key[0])
}

fn execution() {
    let cache_file = get_home_path(CACHE_FILE);
    touch_file(&cache_file);
    let temp_file = NamedTempFile::new().expect("Failed to create temporary file");
    let temp_path = temp_file.path().to_path_buf();
    let annotations = load_session_annotations();
    let mut temp_output = OpenOptions::new()
        .append(true)
        .open(&temp_path)
        .expect("Failed to open temp file for appending");
    let cache_lines =
        prepare_fzf_content_from_cache(&cache_file, &mut temp_output, annotations.as_ref());
    let fzf_process = start_fzf(&temp_path);
    let mut seen_items: HashSet<String> = cache_lines.into_iter().collect();
    thread::spawn(move || {
        write_remaining_projects(&mut temp_output, &mut seen_items, annotations.as_ref());
    });
    let PickerResult { selected_str, pressed_key } = wait_for_fzf_selection(fzf_process);
    {
        let cleanup_result = cleanup(&cache_file, &selected_str);
        if let Err(e) = cleanup_result {
            eprintln!("Cleanup failed: {e}");
        }
    }
    if selected_str.is_empty() {
        println!("No selection made");
        return;
    }
    let selected_project = Project::new(&selected_str);
    load_and_filter_projects()
        .iter()
        .find(|p| p.expanded_path == selected_project.expanded_path)
        .map(|p| p.attach_running(launch_command_for(&pressed_key).as_deref()))
        .expect("Selected project not found");
}

fn launch_command_for(pressed_key: &str) -> Option<String> {
    (pressed_key == LAUNCH_KEY)
        .then(|| env::var(LAUNCH_COMMAND_ENV).ok())
        .flatten()
        .filter(|command| !command.trim().is_empty())
}

fn cleanup(cache_file: &PathBuf, selected_str: &str) -> std::io::Result<()> {
    if !selected_str.is_empty() {
        let mut cache_lines = read_lines(cache_file).unwrap_or_else(|_| vec![]);
        cache_lines.retain(|line| line != selected_str);
        cache_lines.insert(0, selected_str.to_string());
        if cache_lines.len() > MAX_CACHE_ENTRIES {
            cache_lines.truncate(MAX_CACHE_ENTRIES);
        }
        write_lines(cache_file, &cache_lines)?;
    }
    Ok(())
}

fn shell_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', r"'\''"))
}

fn own_executable_quoted() -> String {
    let own_executable = env::current_exe()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|_| env!("CARGO_PKG_NAME").to_string());
    shell_quote(&own_executable)
}

/// fzf only reloads once its input ends, so the growing list is streamed by a
/// `start:reload` command (which fzf can stop) instead of a `tail -f` on stdin.
fn picker_bindings(temp_file: &PathBuf) -> String {
    let executable = own_executable_quoted();
    let stream_temp_file = format!("tail -f -n +0 {}", shell_quote(&temp_file.display().to_string()));
    let bindings = [
        format!("start:reload({stream_temp_file})"),
        format!(
            "ctrl-x:execute({executable} kill-session {{+1}})+clear-multi+reload({executable} picker-lines)"
        ),
        format!("ctrl-t:transform({executable} picker-toggle-open-only)"),
    ];
    let mut options = vec!["--multi".to_string()];
    if launch_command_for(LAUNCH_KEY).is_some() {
        options.push(format!("--expect={LAUNCH_KEY}"));
    }
    options.extend(bindings.iter().map(|binding| format!("--bind={}", shell_quote(binding))));
    options.join(" ")
}

fn start_fzf(temp_file: &PathBuf) -> std::process::Child {
    Command::new("sh")
        .arg("-c")
        .arg(format!(
            "fzf {} {} {}",
            FZF_LAYOUT,
            FZF_PICKER_FIELDS,
            picker_bindings(temp_file)
        ))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to execute fzf")
}

fn load_and_filter_projects() -> Vec<Project> {
    get_projects().filter_exists()
}

fn prepare_fzf_content(
    projects: &[Project],
    annotations: Option<&SessionAnnotations>,
) -> Vec<(String, String)> {
    let mut fzf_through = Vec::new();
    let mut seen = HashSet::new();
    for project in projects {
        if seen.insert(project.expanded_path.clone()) {
            fzf_through.push((
                project.to_fzf_display().to_string(),
                project.to_fzf_line(annotations),
            ));
        }
    }
    fzf_through
}

struct PickerResult {
    selected_str: String,
    pressed_key: String,
}

/// With `--expect`, fzf prints the pressed key on the first line (empty for Enter).
fn wait_for_fzf_selection(fzf_process: std::process::Child) -> PickerResult {
    let output = fzf_process
        .wait_with_output()
        .expect("Failed to read fzf output");
    let output = String::from_utf8_lossy(&output.stdout);
    let mut lines = output.lines();
    let pressed_key = if launch_command_for(LAUNCH_KEY).is_some() {
        lines.next().unwrap_or_default().to_string()
    } else {
        String::new()
    };
    let selected_str = lines
        .next()
        .unwrap_or_default()
        .split('\t')
        .next()
        .unwrap_or_default()
        .trim()
        .to_string();
    PickerResult {
        selected_str,
        pressed_key,
    }
}

fn list_projects() {
    let projects = get_projects();
    for project in projects {
        println!("{}", project.shortened_path);
    }
}

fn status_projects() {
    let projects_file = get_home_path(PROJECTS_FILE);
    let lines = read_lines(&projects_file).unwrap_or_else(|_| vec![]);
    for line in lines {
        println!("{line}");
    }
}

fn set_depth() {
    let projects_file = get_home_path(PROJECTS_FILE);
    let lines = read_lines(&projects_file).unwrap_or_else(|_| vec![]);
    let mut selected = Command::new("fzf")
        .args(FZF_LAYOUT.split_whitespace())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to execute fzf");
    {
        let fzf_stdin = selected.stdin.as_mut().expect("Failed to open fzf stdin");
        fzf_stdin
            .write_all(lines.join("\n").as_bytes())
            .expect("Failed to write to fzf stdin");
    }
    let output = selected
        .wait_with_output()
        .expect("Failed to read fzf output");
    if !output.stdout.is_empty() {
        let selected_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        println!("Set depth for {selected_str}: (Press Enter to remove depth, Ctrl+C to cancel)");
        let mut depth_input = String::new();
        std::io::stdin()
            .read_line(&mut depth_input)
            .expect("Failed to read depth input");
        let depth_input = depth_input.trim();
        let re = Regex::new(r"(.*) --depth \d+").unwrap();
        let mut new_lines: Vec<String> = lines
            .into_iter()
            .filter(|line| !re.is_match(line) || !line.starts_with(&selected_str))
            .collect();
        if depth_input.is_empty() {
            new_lines.push(selected_str.clone());
        } else {
            new_lines.push(format!("{selected_str} --depth {depth_input}"));
        }
        new_lines.sort();
        write_lines(&projects_file, &new_lines).unwrap();
        println!("Set depth for \"{selected_str}\" to {depth_input}");
    }
}

fn edit_projects_file() {
    let projects_file = get_home_path(PROJECTS_FILE);
    touch_file(&projects_file);
    let editor = env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
    let status = Command::new(editor)
        .arg(projects_file)
        .status()
        .expect("Failed to launch editor");
    if !status.success() {
        eprintln!("Editor exited with non-zero status: {}", status);
    }
}

fn goto_project(dir: &str) {
    let project = Project::new(dir);

    // Check if the directory exists
    if !project.exists() {
        eprintln!(
            "Directory \"{}\" does not exist or is not a directory",
            project.expanded_path
        );
        return;
    }

    if tmux::is_inside_tmux() {
        // Inside tmux: just switch/attach normally
        project.attach();
    } else {
        // Outside tmux: ensure session exists, then attach using exec to persist
        let tmux_session_name = &project.tmux_display_path;

        if !tmux::session_exists(tmux_session_name) {
            if !tmux::create_session(tmux_session_name, &project.expanded_path) {
                eprintln!("Failed to create new tmux session");
                return;
            }
        }

        // Replace current process with tmux attach (this persists)
        tmux::attach_session_exec(tmux_session_name, &project.expanded_path);
    }
}
