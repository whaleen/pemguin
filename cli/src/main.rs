use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

use arboard::Clipboard;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};
use regex::Regex;

// ── Theme (Vesper) ────────────────────────────────────────────────────────────

const ACCENT: Color = Color::Rgb(255, 199, 153); // #FFC799 — orange highlight
const SEL_FG: Color = Color::Rgb(16, 16, 16); // #101010 — selected text fg
const FG_DIM: Color = Color::Rgb(160, 160, 160); // #A0A0A0
const FG_XDIM: Color = Color::Rgb(80, 80, 80); // very dim
const C_GREEN: Color = Color::Rgb(153, 255, 228); // #99FFE4 — mint
const C_RED: Color = Color::Rgb(245, 161, 145); // #f5a191
const C_YELLOW: Color = Color::Rgb(230, 185, 157); // #e6b99d
const C_PURPLE: Color = Color::Rgb(172, 161, 207); // #aca1cf
const C_PINK: Color = Color::Rgb(226, 158, 202); // #e29eca

// ── Icons (Nerd Fonts) ────────────────────────────────────────────────────────

const I_BRANCH: &str = "\u{e0a0}"; // git branch
const I_CHECK: &str = "\u{f00c}"; // ✓ check
const I_CROSS: &str = "\u{f12a}"; // ! exclamation (missing)
const I_WARN: &str = "\u{f071}"; // ⚠ triangle (stale)
const I_AHEAD: &str = "\u{f062}"; // ↑ arrow-up
const I_BULLET: &str = "\u{f111}"; // ● circle (active marker)
const I_COMMIT: &str = "\u{f1d3}"; // git commit
const I_HOME: &str = "\u{f015}"; // home
const I_ISSUES: &str = "\u{f41b}"; // github mark
const I_SETUP: &str = "\u{f013}"; // cog
const I_PROMPTS: &str = "\u{f0ae}"; // list
const I_PROJECTS: &str = "\u{f07b}"; // folder
const I_MEMORY: &str = "\u{f0eb}"; // lightbulb (memories)
const I_SKILLS: &str = "\u{f0ad}"; // wrench
const I_MCP: &str = "\u{f0c1}"; // link/chain
const I_PANE: &str = "\u{f120}"; // >_ terminal prompt

// ── Config ────────────────────────────────────────────────────────────────────

#[derive(Clone, serde::Deserialize, Default)]
struct Config {
    #[serde(default)]
    projects: ProjectsConfig,
}

#[derive(Clone, serde::Deserialize, Default)]
struct ProjectsConfig {
    root: Option<String>,
}

fn load_config() -> Config {
    let home = dirs_home().unwrap_or_default();
    let path = home.join(".pemguin.toml");
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| toml::from_str(&s).ok())
        .unwrap_or_default()
}

// ── Data ──────────────────────────────────────────────────────────────────────

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct RepoMeta {
    language: Option<String>,
    topics: Vec<String>,
    pushed_at: Option<String>, // ISO date string from GitHub
}

#[derive(Clone)]
struct Prompt {
    name: String,
    body: String,
    preview: String,
    placeholders: Vec<String>,
}

struct MemoryFile {
    name: String,
    path: PathBuf,
    content: String,
}

struct Issue {
    number: u64,
    title: String,
    body: String,
    labels: Vec<String>,
}

struct Project {
    path: PathBuf,
    group: String, // parent dir name relative to base; "" for top-level repos
    repo: String,  // "owner/repo" or dir name
    branch: String,
    is_dirty: bool,
    commits_ahead: u32,
    setup_ok: usize,
    setup_total: usize,
}

#[derive(Clone)]
enum ProjectEntry {
    Group(String), // section header
    Item(usize),   // index into app.projects
}

// ── Screens ───────────────────────────────────────────────────────────────────

#[derive(PartialEq, Clone)]
enum ProjectTab {
    Home,
    Issues,
    Setup,
    Prompts,
    Memories,
    Skills,
    Mcp,
    Pane,
}

#[derive(PartialEq, Clone)]
enum MemoriesView {
    Project,
    Global,
    Claude,
}

#[derive(PartialEq, Clone)]
enum PromptsView {
    Global,
    Project,
}

#[derive(PartialEq, Clone)]
enum HomeEditField {
    Description,
    Homepage,
}

#[derive(PartialEq)]
enum Screen {
    Projects,              // root / launcher
    InProject(ProjectTab), // drilled into a project
}

// ── Home screen ───────────────────────────────────────────────────────────────

struct Skill {
    name: String,
    source: String,
    description: String,
}

struct McpServer {
    name: String,
    command: String,
    args: Vec<String>,
}

struct HomeData {
    gh_description: Option<String>, // GitHub repo description
    homepage: Option<String>,       // GitHub homepage URL (custom)
    url: String,                    // https://github.com/owner/repo
    recent_commits: Vec<String>,    // git log --oneline -6
    setup_ok: usize,
    setup_total: usize,
    stack: Option<String>, // detected stack label
}

fn detect_stack(path: &Path) -> Option<String> {
    if let Ok(s) = fs::read_to_string(path.join("Cargo.toml")) {
        let name = s.lines().find_map(|l| {
            l.strip_prefix("name = ")
                .map(|v| v.trim_matches('"').to_string())
        });
        return Some(format!("Rust ({})", name.as_deref().unwrap_or("?")));
    }
    if let Ok(s) = fs::read_to_string(path.join("package.json")) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&s) {
            let name = v["name"].as_str().unwrap_or("?");
            let stack = if path.join("next.config.ts").exists()
                || path.join("next.config.js").exists()
            {
                "Next.js"
            } else if path.join("vite.config.ts").exists() || path.join("vite.config.js").exists() {
                "Vite"
            } else {
                "Node"
            };
            return Some(format!("{stack} ({name})"));
        }
    }
    if path.join("go.mod").exists() {
        return Some("Go".to_string());
    }
    None
}

fn load_home_data(path: &Path, repo: &str) -> HomeData {
    let (gh_description, homepage) = if !repo.is_empty() {
        let out = Command::new("gh")
            .args(["repo", "view", repo, "--json", "description,homepageUrl"])
            .output()
            .ok()
            .filter(|o| o.status.success());
        if let Some(out) = out {
            if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&out.stdout) {
                let desc = v["description"]
                    .as_str()
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string());
                let home = v["homepageUrl"]
                    .as_str()
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string());
                (desc, home)
            } else {
                (None, None)
            }
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };

    let url = if repo.contains('/') {
        format!("https://github.com/{repo}")
    } else {
        String::new()
    };

    // Recent commits
    let recent_commits = git_in(path, &["log", "--oneline", "-6"])
        .unwrap_or_default()
        .lines()
        .map(|l| l.to_string())
        .collect();

    // Setup health
    let items = scan_setup(path);
    let setup_total = items.len();
    let setup_ok = items.iter().filter(|i| i.status == SetupStatus::Ok).count();

    let stack = detect_stack(path);

    HomeData {
        gh_description,
        homepage,
        url,
        recent_commits,
        setup_ok,
        setup_total,
        stack,
    }
}

fn load_home_data_local(path: &Path, repo: &str) -> HomeData {
    let url = if repo.contains('/') {
        format!("https://github.com/{repo}")
    } else {
        String::new()
    };

    let recent_commits = git_in(path, &["log", "--oneline", "-6"])
        .unwrap_or_default()
        .lines()
        .map(|l| l.to_string())
        .collect();

    let items = scan_setup(path);
    let setup_total = items.len();
    let setup_ok = items.iter().filter(|i| i.status == SetupStatus::Ok).count();

    let stack = detect_stack(path);

    HomeData {
        gh_description: None,
        homepage: None,
        url,
        recent_commits,
        setup_ok,
        setup_total,
        stack,
    }
}

// ── Setup screen ──────────────────────────────────────────────────────────────

#[derive(PartialEq, Clone)]
enum SetupStatus {
    Ok,
    Missing,
    Stale,
}

#[derive(Clone)]
struct SetupItem {
    label: &'static str,
    detail: &'static str,
    status: SetupStatus,
}

fn scan_setup(path: &Path) -> Vec<SetupItem> {
    let agent_ok = path.join("AGENT.md").exists();
    let spec_ok = path.join("SPEC.md").exists();
    let claude_ok = {
        let p = path.join("CLAUDE.md");
        p.is_symlink() || p.exists()
    };
    let gemini_ok = {
        let p = path.join("GEMINI.md");
        p.is_symlink() || p.exists()
    };
    let docs_ok = path.join("docs").is_dir();
    let gitignore_ok = fs::read_to_string(path.join(".gitignore"))
        .map(|s| s.contains("# Agent dirs"))
        .unwrap_or(false);
    let agents_stale = path.join("AGENTS.md").exists();
    let prompts_ok = path.join(".prompts").is_dir();
    let memory_ok = path.join(".memory").join("MEMORY.md").exists();

    let mut items = vec![
        SetupItem {
            label: "AGENT.md",
            detail: "agent context file",
            status: if agent_ok {
                SetupStatus::Ok
            } else {
                SetupStatus::Missing
            },
        },
        SetupItem {
            label: "SPEC.md",
            detail: "feature spec",
            status: if spec_ok {
                SetupStatus::Ok
            } else {
                SetupStatus::Missing
            },
        },
        SetupItem {
            label: "CLAUDE.md → AGENT.md",
            detail: "symlink for Claude Code",
            status: if claude_ok {
                SetupStatus::Ok
            } else {
                SetupStatus::Missing
            },
        },
        SetupItem {
            label: "GEMINI.md → AGENT.md",
            detail: "symlink for Gemini",
            status: if gemini_ok {
                SetupStatus::Ok
            } else {
                SetupStatus::Missing
            },
        },
        SetupItem {
            label: "docs/",
            detail: "architecture/features skeleton",
            status: if docs_ok {
                SetupStatus::Ok
            } else {
                SetupStatus::Missing
            },
        },
        SetupItem {
            label: ".gitignore",
            detail: "agent dirs excluded",
            status: if gitignore_ok {
                SetupStatus::Ok
            } else {
                SetupStatus::Missing
            },
        },
        SetupItem {
            label: ".prompts/",
            detail: "project-local prompts",
            status: if prompts_ok {
                SetupStatus::Ok
            } else {
                SetupStatus::Missing
            },
        },
        SetupItem {
            label: ".memory/",
            detail: "agent memory index",
            status: if memory_ok {
                SetupStatus::Ok
            } else {
                SetupStatus::Missing
            },
        },
    ];
    if agents_stale {
        items.push(SetupItem {
            label: "AGENTS.md",
            detail: "stale file — delete it",
            status: SetupStatus::Stale,
        });
    }
    items
}

fn apply_setup_item(project_path: &Path, item: &SetupItem) -> Result<String, String> {
    let pemguin_dir = std::env::var("PEMGUIN_DIR")
        .or_else(|_| std::env::var("SCAFFOLD_DIR"))
        .map(PathBuf::from)
        .map_err(|_| "$PEMGUIN_DIR not set".to_string())?;

    match item.label {
        "AGENT.md" => fs::copy(
            template_file(&pemguin_dir, "AGENT.md"),
            project_path.join("AGENT.md"),
        )
        .map(|_| "Created AGENT.md".to_string())
        .map_err(|e| e.to_string()),
        "SPEC.md" => fs::copy(
            template_file(&pemguin_dir, "SPEC.md"),
            project_path.join("SPEC.md"),
        )
        .map(|_| "Created SPEC.md".to_string())
        .map_err(|e| e.to_string()),
        "CLAUDE.md → AGENT.md" => {
            std::os::unix::fs::symlink("AGENT.md", project_path.join("CLAUDE.md"))
                .map(|_| "Symlinked CLAUDE.md → AGENT.md".to_string())
                .map_err(|e| e.to_string())
        }
        "GEMINI.md → AGENT.md" => {
            std::os::unix::fs::symlink("AGENT.md", project_path.join("GEMINI.md"))
                .map(|_| "Symlinked GEMINI.md → AGENT.md".to_string())
                .map_err(|e| e.to_string())
        }
        "docs/" => {
            let src = pemguin_dir.join("docs");
            let dst = project_path.join("docs");
            copy_dir_recursive(&src, &dst)
                .map(|_| "Created docs/".to_string())
                .map_err(|e| e.to_string())
        }
        ".memory/" => {
            let dir = project_path.join(".memory");
            fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let index = dir.join("MEMORY.md");
            if !index.exists() {
                fs::write(&index, MEMORY_INDEX_TEMPLATE).map_err(|e| e.to_string())?;
            }
            Ok("Created .memory/MEMORY.md".to_string())
        }
        ".gitignore" => {
            let gitignore = project_path.join(".gitignore");
            let block = "\n# Agent dirs\n.agents/\n.claude/\n.kiro/\n.vite-hooks/\nskills-lock.json\nCLAUDE.md\nGEMINI.md\n.memory/\n";
            let mut f = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&gitignore)
                .map_err(|e| e.to_string())?;
            use std::io::Write;
            f.write_all(block.as_bytes()).map_err(|e| e.to_string())?;
            Ok("Patched .gitignore".to_string())
        }
        ".prompts/" => {
            let dir = project_path.join(".prompts");
            fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let sample = dir.join("work-on-task.md");
            if !sample.exists() {
                fs::copy(
                    template_file(&pemguin_dir, "prompts/work-on-task.md"),
                    &sample,
                )
                .map_err(|e| e.to_string())?;
            }
            Ok("Created .prompts/ with sample prompt".to_string())
        }
        "AGENTS.md" => fs::remove_file(project_path.join("AGENTS.md"))
            .map(|_| "Removed stale AGENTS.md".to_string())
            .map_err(|e| e.to_string()),
        _ => Err("unknown item".to_string()),
    }
}

fn template_file(pemguin_dir: &Path, relative: &str) -> PathBuf {
    pemguin_dir.join("templates").join(relative)
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &dst_path)?;
        } else {
            fs::copy(entry.path(), dst_path)?;
        }
    }
    Ok(())
}

fn apply_all_setup(project_path: &Path) -> Result<String, String> {
    let pemguin_dir = std::env::var("PEMGUIN_DIR")
        .or_else(|_| std::env::var("SCAFFOLD_DIR"))
        .map_err(|_| "$PEMGUIN_DIR not set".to_string())?;
    let init_sh = PathBuf::from(&pemguin_dir).join("init.sh");
    let out = Command::new("bash")
        .arg(&init_sh)
        .current_dir(project_path)
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

enum PromptState {
    Browse {
        list_state: ListState,
    },
    Fill {
        prompt_idx: usize,
        field_idx: usize,
        values: HashMap<String, String>,
        input: String,
    },
    Done(String),
}

struct App {
    config: Config,
    screen: Screen,
    // Prompts
    global_prompts: Vec<Prompt>, // always loaded from $PEMGUIN_DIR/prompts
    project_prompts: Vec<Prompt>, // loaded from <project>/.prompts/ on drill-in
    prompts_view: PromptsView,
    prompts: Vec<Prompt>, // current display list (points at global or project)
    prompt_state: PromptState,
    // Issues
    issues: Vec<Issue>,
    issue_list_state: ListState,
    issues_error: Option<String>,
    issues_loaded: bool,
    issues_loading: bool,
    // Projects (root screen)
    projects: Vec<Project>,
    project_entries: Vec<ProjectEntry>, // flat render list (Group headers + Item refs)
    project_list_state: ListState,
    active_project_idx: Option<usize>, // index into projects; set on drill-in
    projects_msg: Option<String>,      // transient status shown in footer
    projects_loading: bool,
    scan_generation: u64,
    // Home (project sub-screen)
    home_data: Option<HomeData>,
    home_remote_loaded: bool,
    home_loading: bool,
    home_edit: Option<HomeEditField>,
    home_edit_input: String,
    home_save_msg: Option<String>,
    // Setup (project sub-screen)
    setup_items: Vec<SetupItem>,
    setup_list_state: ListState,
    setup_message: Option<String>,
    // GitHub metadata cache (keyed by "owner/repo")
    meta_cache: HashMap<String, RepoMeta>,
    // Avatar cache (keyed by "owner" -> raw chafa ANSI output)
    avatar_cache: HashMap<String, String>,
    avatar_loading_owner: Option<String>,
    // Memories tab
    memories_view: MemoriesView,
    memory_files: Vec<MemoryFile>,
    memory_list_state: ListState,
    memory_message: Option<String>,
    memories_loaded: bool,
    memory_input: String,
    memory_inputting: bool,
    pending_editor: Option<PathBuf>,
    // Skills
    skills: Vec<Skill>,
    skills_list_state: ListState,
    skills_loaded: bool,
    // MCP
    mcp_servers: Vec<McpServer>,
    mcp_list_state: ListState,
    mcp_loaded: bool,
    // Active context
    context: String,
    repo: String,
    async_tx: Sender<AsyncResult>,
    async_rx: Receiver<AsyncResult>,
}

enum AsyncResult {
    Home {
        repo: String,
        data: HomeData,
    },
    Issues {
        repo: String,
        result: Result<Vec<Issue>, String>,
    },
    Avatar {
        owner: String,
        ansi: Option<String>,
    },
    Projects {
        generation: u64,
        projects: Vec<Project>,
    },
}

// ── Prompt loading ────────────────────────────────────────────────────────────

fn expand_tilde(s: &str) -> String {
    if let Some(rest) = s.strip_prefix("~/") {
        if let Some(home) = dirs_home() {
            return home.join(rest).to_string_lossy().into_owned();
        }
    }
    s.to_string()
}

fn load_prompts_from(dir: &Path) -> Vec<Prompt> {
    if !dir.is_dir() {
        return vec![];
    }

    let re = Regex::new(r"\{([A-Z][A-Z0-9_]*)\}").unwrap();
    let mut entries: Vec<_> = fs::read_dir(dir)
        .map(|r| r.filter_map(|e| e.ok()).collect())
        .unwrap_or_default();
    entries.sort_by_key(|e: &std::fs::DirEntry| e.file_name());
    entries.retain(|e| {
        e.path()
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s == "md")
            .unwrap_or(false)
    });

    entries
        .iter()
        .filter_map(|entry| {
            let path = entry.path();
            let name = path.file_stem()?.to_str()?.to_string();
            let content = fs::read_to_string(&path).ok()?;
            let body = extract_body(&content);
            let mut placeholders: Vec<String> = Vec::new();
            for cap in re.captures_iter(&body) {
                let p = cap[1].to_string();
                if !placeholders.contains(&p) {
                    placeholders.push(p);
                }
            }
            Some(Prompt {
                name,
                body,
                preview: content,
                placeholders,
            })
        })
        .collect()
}

fn global_prompts_dir() -> PathBuf {
    dirs_home()
        .unwrap_or_default()
        .join(".pemguin")
        .join("prompts")
}

fn extract_body(content: &str) -> String {
    let mut in_block = false;
    let mut block: Vec<&str> = Vec::new();
    for line in content.lines() {
        if line.starts_with("```") && !in_block {
            in_block = true;
            continue;
        }
        if line.starts_with("```") && in_block {
            if !block.is_empty() {
                return block.join("\n");
            }
            in_block = false;
            block.clear();
            continue;
        }
        if in_block {
            block.push(line);
        }
    }
    content.to_string()
}

// ── Issue loading ─────────────────────────────────────────────────────────────

fn load_issues(repo: &str) -> Result<Vec<Issue>, String> {
    if repo.is_empty() {
        return Err("No repo context".to_string());
    }
    let output = Command::new("gh")
        .args([
            "issue",
            "list",
            "--repo",
            repo,
            "--json",
            "number,title,body,labels,state",
            "--limit",
            "50",
        ])
        .output()
        .map_err(|_| "gh CLI not found".to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).map_err(|e| format!("JSON: {e}"))?;
    Ok(json
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|item| {
            let number = item["number"].as_u64()?;
            let title = item["title"].as_str().unwrap_or("").to_string();
            let body = item["body"].as_str().unwrap_or("").to_string();
            let labels = item["labels"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|l| l["name"].as_str().map(|s| s.to_string()))
                .collect();
            Some(Issue {
                number,
                title,
                body,
                labels,
            })
        })
        .collect())
}

// ── Project scanning ──────────────────────────────────────────────────────────

fn scan_projects(config: &Config) -> Vec<Project> {
    let base = std::env::var("PEMGUIN_PROJECTS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            config
                .projects
                .root
                .as_ref()
                .map(|r| PathBuf::from(expand_tilde(r)))
                .unwrap_or_else(|| {
                    dirs_home()
                        .map(|h| h.join("Projects"))
                        .unwrap_or_else(|| PathBuf::from("."))
                })
        });

    // Walk up to 2 levels for .git dirs
    let Ok(level1) = fs::read_dir(&base) else {
        return vec![];
    };
    let mut level1_dirs: Vec<_> = level1
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();
    level1_dirs.sort_by_key(|e| e.file_name());

    let mut candidates: Vec<(PathBuf, String)> = Vec::new();
    for entry in level1_dirs {
        let path = entry.path();
        let dir_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        // Skip hidden dirs
        if dir_name.starts_with('.') {
            continue;
        }
        if path.join(".git").is_dir() {
            candidates.push((path, String::new()));
        } else if let Ok(level2) = fs::read_dir(&path) {
            let mut level2_dirs: Vec<_> = level2
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .collect();
            level2_dirs.sort_by_key(|e| e.file_name());
            for sub in level2_dirs {
                let sub_path = sub.path();
                if sub_path.join(".git").is_dir() {
                    candidates.push((sub_path, dir_name.clone()));
                }
            }
        }
    }

    let mut projects: Vec<Project> = Vec::new();
    let worker_count = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        .min(candidates.len().max(1));
    let mut buckets = vec![Vec::new(); worker_count];
    for (i, candidate) in candidates.into_iter().enumerate() {
        buckets[i % worker_count].push(candidate);
    }

    let mut threads = Vec::new();
    for bucket in buckets {
        threads.push(std::thread::spawn(move || {
            bucket
                .into_iter()
                .filter_map(|(path, group)| project_info(&path, group))
                .collect::<Vec<_>>()
        }));
    }
    for thread in threads {
        if let Ok(mut batch) = thread.join() {
            projects.append(&mut batch);
        }
    }

    // Sort: group first (empty last), then repo name
    projects.sort_by(|a, b| {
        let ga = if a.group.is_empty() { "\x7f" } else { &a.group }; // empty → sort last
        let gb = if b.group.is_empty() { "\x7f" } else { &b.group };
        ga.cmp(gb).then(a.repo.cmp(&b.repo))
    });
    projects
}

fn project_info(path: &Path, group: String) -> Option<Project> {
    let repo = git_in(path, &["remote", "get-url", "origin"])
        .map(|u| parse_repo(&u))
        .unwrap_or_else(|| {
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("?")
                .to_string()
        });
    let (branch, is_dirty, ahead) = git_status_summary(path);
    let setup_items = scan_setup(path);
    let setup_total = setup_items.len();
    let setup_ok = setup_items
        .iter()
        .filter(|i| i.status == SetupStatus::Ok)
        .count();
    Some(Project {
        path: path.to_path_buf(),
        group,
        repo,
        branch,
        is_dirty,
        commits_ahead: ahead,
        setup_ok,
        setup_total,
    })
}

fn git_status_summary(path: &Path) -> (String, bool, u32) {
    let out = match git_in(path, &["status", "--porcelain=2", "--branch"]) {
        Some(s) => s,
        None => return ("?".to_string(), false, 0),
    };

    let mut branch = "?".to_string();
    let mut ahead = 0;
    let mut is_dirty = false;

    for line in out.lines() {
        if let Some(head) = line.strip_prefix("# branch.head ") {
            if head != "(detached)" {
                branch = head.to_string();
            }
            continue;
        }
        if let Some(ab) = line.strip_prefix("# branch.ab ") {
            for part in ab.split_whitespace() {
                if let Some(n) = part.strip_prefix('+') {
                    ahead = n.parse().unwrap_or(0);
                }
            }
            continue;
        }
        if !line.starts_with('#') && !line.is_empty() {
            is_dirty = true;
        }
    }

    (branch, is_dirty, ahead)
}

fn build_project_entries(projects: &[Project]) -> Vec<ProjectEntry> {
    let mut entries: Vec<ProjectEntry> = Vec::new();
    let mut last_group: Option<&str> = None;
    for (i, p) in projects.iter().enumerate() {
        let group_str = if p.group.is_empty() {
            None
        } else {
            Some(p.group.as_str())
        };
        if group_str != last_group {
            if let Some(g) = group_str {
                entries.push(ProjectEntry::Group(g.to_string()));
            }
            last_group = group_str;
        }
        entries.push(ProjectEntry::Item(i));
    }
    entries
}

// ── GitHub metadata cache ─────────────────────────────────────────────────────

fn meta_cache_path() -> PathBuf {
    dirs_home()
        .unwrap_or_default()
        .join(".pemguin")
        .join("cache.json")
}

fn load_meta_cache() -> HashMap<String, RepoMeta> {
    let path = meta_cache_path();
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_meta_cache(cache: &HashMap<String, RepoMeta>) {
    let path = meta_cache_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(cache) {
        let _ = fs::write(&path, json);
    }
}

fn refresh_project_meta(repo: &str) -> Option<RepoMeta> {
    if !repo.contains('/') {
        return None;
    }
    let out = Command::new("gh")
        .args([
            "repo",
            "view",
            repo,
            "--json",
            "primaryLanguage,repositoryTopics,pushedAt",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
    let language = v["primaryLanguage"]["name"].as_str().map(|s| s.to_string());
    let topics = v["repositoryTopics"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|t| t["name"].as_str().map(|s| s.to_string()))
        .collect();
    let pushed_at = v["pushedAt"].as_str().map(|s| s.to_string());
    Some(RepoMeta {
        language,
        topics,
        pushed_at,
    })
}

fn sync_meta(projects: &[Project]) -> HashMap<String, RepoMeta> {
    // Batch by org: one `gh repo list` call per org instead of per repo
    let mut orgs: Vec<String> = projects
        .iter()
        .filter(|p| p.repo.contains('/'))
        .map(|p| p.repo.split('/').next().unwrap_or("").to_string())
        .collect();
    orgs.sort();
    orgs.dedup();

    let mut cache: HashMap<String, RepoMeta> = HashMap::new();
    for org in &orgs {
        if org.is_empty() {
            continue;
        }
        let Ok(out) = Command::new("gh")
            .args([
                "repo",
                "list",
                org,
                "--limit",
                "100",
                "--json",
                "name,primaryLanguage,repositoryTopics,pushedAt",
            ])
            .output()
        else {
            continue;
        };
        if !out.status.success() {
            continue;
        }
        let Ok(arr) = serde_json::from_slice::<serde_json::Value>(&out.stdout) else {
            continue;
        };
        let Some(items) = arr.as_array() else {
            continue;
        };
        for item in items {
            let name = item["name"].as_str().unwrap_or("").to_string();
            if name.is_empty() {
                continue;
            }
            let language = item["primaryLanguage"]["name"]
                .as_str()
                .map(|s| s.to_string());
            let topics = item["repositoryTopics"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|t| t["name"].as_str().map(|s| s.to_string()))
                .collect();
            let pushed_at = item["pushedAt"].as_str().map(|s| s.to_string());
            cache.insert(
                format!("{org}/{name}"),
                RepoMeta {
                    language,
                    topics,
                    pushed_at,
                },
            );
        }
    }
    cache
}

fn lang_short(lang: &str) -> &str {
    match lang {
        "TypeScript" => "TS",
        "JavaScript" => "JS",
        "Rust" => "RS",
        "Go" => "Go",
        "Python" => "Py",
        "Ruby" => "Rb",
        "CSS" => "CS",
        "HTML" => "HT",
        "Shell" => "SH",
        "Svelte" => "SV",
        "Solidity" => "So",
        "Nix" => "Nx",
        other => {
            if other.len() >= 2 {
                &other[..2]
            } else {
                other
            }
        }
    }
}

fn relative_date(iso: &str) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let parts: Vec<u64> = iso
        .splitn(2, 'T')
        .next()
        .unwrap_or("")
        .split('-')
        .filter_map(|s| s.parse().ok())
        .collect();
    if parts.len() < 3 {
        return String::new();
    }
    let (y, m, d) = (parts[0], parts[1], parts[2]);
    let month_days = [0u64, 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let month_sum: u64 = (1..m as usize).map(|i| month_days[i]).sum();
    let days_epoch = (y - 1970) * 365 + (y - 1970) / 4 + month_sum + d - 1;
    let diff = now.saturating_sub(days_epoch * 86400) / 86400;
    match diff {
        0 => "today".to_string(),
        1..=6 => format!("{}d", diff),
        7..=29 => format!("{}w", diff / 7),
        30..=364 => format!("{}mo", diff / 30),
        _ => format!("{}y", diff / 365),
    }
}

// ── Git / system helpers ──────────────────────────────────────────────────────

fn git(args: &[&str]) -> Option<String> {
    Command::new("git")
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
}

fn git_in(dir: &Path, args: &[&str]) -> Option<String> {
    Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
}

fn parse_repo(url: &str) -> String {
    let url = url.trim().trim_end_matches(".git");
    // HTTPS: https://host/owner/repo or http://host/owner/repo
    if url.starts_with("https://") || url.starts_with("http://") {
        let prefix = if url.starts_with("https://") { 8 } else { 7 };
        if let Some(slash) = url[prefix..].find('/') {
            return url[prefix + slash + 1..].to_string();
        }
    }
    // SSH: git@host:owner/repo
    if let Some(pos) = url.rfind(':') {
        let after = &url[pos + 1..];
        if after.contains('/') {
            return after.to_string();
        }
    }
    url.to_string()
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(PathBuf::from)
}

fn save_repo_field(repo: &str, field: &HomeEditField, value: &str) -> Result<(), String> {
    let flag = match field {
        HomeEditField::Description => "--description",
        HomeEditField::Homepage => "--homepage",
    };
    let out = Command::new("gh")
        .args(["repo", "edit", repo, flag, value])
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

// ── Memory helpers ────────────────────────────────────────────────────────────

fn global_memory_path() -> PathBuf {
    dirs_home()
        .unwrap_or_default()
        .join(".pemguin")
        .join("memory")
}

/// Claude Code sanitizes project paths by replacing all non-alphanumeric chars with '-'.
fn claude_memory_path(project_path: &Path) -> PathBuf {
    let s = project_path.to_string_lossy();
    let sanitized: String = s
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();
    dirs_home()
        .unwrap_or_default()
        .join(".claude")
        .join("projects")
        .join(sanitized)
        .join("memory")
}

fn load_memory_files(dir: &Path) -> Vec<MemoryFile> {
    if !dir.is_dir() {
        return vec![];
    }
    let mut entries: Vec<_> = fs::read_dir(dir)
        .map(|r| r.filter_map(|e| e.ok()).collect())
        .unwrap_or_default();
    entries.sort_by_key(|e: &std::fs::DirEntry| e.file_name());
    entries
        .iter()
        .filter_map(|e| {
            let path = e.path();
            if path.extension().and_then(|s| s.to_str()) != Some("md") {
                return None;
            }
            let name = path.file_stem()?.to_str()?.to_string();
            let content = fs::read_to_string(&path).unwrap_or_default();
            Some(MemoryFile {
                name,
                path,
                content,
            })
        })
        .collect()
}

fn append_to_memory_index(dir: &Path, filename: &str) -> io::Result<()> {
    let index = dir.join("MEMORY.md");
    if !index.exists() {
        fs::write(&index, MEMORY_INDEX_TEMPLATE)?;
    }
    let entry = format!("- [{filename}]({filename}) — \n");
    let mut f = fs::OpenOptions::new().append(true).open(&index)?;
    use std::io::Write;
    f.write_all(entry.as_bytes())
}

// ── Skills / MCP loading ──────────────────────────────────────────────────────

fn load_skills(path: &Path) -> Vec<Skill> {
    let lock_path = path.join("skills-lock.json");
    let content = match fs::read_to_string(&lock_path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    let json: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return vec![],
    };
    let skills_obj = match json.get("skills").and_then(|v| v.as_object()) {
        Some(o) => o.clone(),
        None => return vec![],
    };
    let mut skills = Vec::new();
    for (name, val) in &skills_obj {
        let source = val
            .get("source")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        // Try to read description from .agents/skills/<name>/SKILL.md frontmatter
        let skill_md = path
            .join(".agents")
            .join("skills")
            .join(name)
            .join("SKILL.md");
        let description = fs::read_to_string(&skill_md)
            .ok()
            .and_then(|content| {
                content
                    .lines()
                    .find(|l| l.starts_with("description:"))
                    .map(|l| l.trim_start_matches("description:").trim().to_string())
            })
            .unwrap_or_default();
        skills.push(Skill {
            name: name.clone(),
            source,
            description,
        });
    }
    skills.sort_by(|a, b| a.name.cmp(&b.name));
    skills
}

fn load_mcp_servers(path: &Path) -> Vec<McpServer> {
    let mcp_path = path.join(".mcp.json");
    let content = match fs::read_to_string(&mcp_path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    let json: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return vec![],
    };
    let servers = match json.get("mcpServers").and_then(|v| v.as_object()) {
        Some(o) => o.clone(),
        None => return vec![],
    };
    let mut result = Vec::new();
    for (name, val) in &servers {
        let command = val
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let args: Vec<String> = val
            .get("args")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        result.push(McpServer {
            name: name.clone(),
            command,
            args,
        });
    }
    result.sort_by(|a, b| a.name.cmp(&b.name));
    result
}

// ── Avatar (chafa) ────────────────────────────────────────────────────────────

fn avatar_dir() -> PathBuf {
    dirs_home()
        .unwrap_or_default()
        .join(".pemguin")
        .join("avatars")
}

/// Download owner avatar and render via chafa. Returns raw ANSI string.
fn fetch_avatar(owner: &str) -> Option<String> {
    let dir = avatar_dir();
    let _ = fs::create_dir_all(&dir);
    let png = dir.join(format!("{owner}.png"));

    if !png.exists() {
        let url = format!("https://github.com/{owner}.png?size=128");
        let ok = Command::new("curl")
            .args(["-s", "-L", "-o", png.to_str().unwrap_or(""), &url])
            .status()
            .ok()?
            .success();
        if !ok {
            return None;
        }
    }

    let out = Command::new("chafa")
        .args([
            "--size",
            "20x10",
            "--format",
            "symbols",
            png.to_str().unwrap_or(""),
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Parse chafa ANSI output into ratatui Lines.
fn ansi_to_lines(s: &str) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut style = Style::default();
    let mut text = String::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' && chars.peek() == Some(&'[') {
            chars.next(); // consume '['
            let mut seq = String::new();
            for nc in chars.by_ref() {
                if nc == 'm' {
                    break;
                }
                seq.push(nc);
            }
            if !text.is_empty() {
                spans.push(Span::styled(text.clone(), style));
                text.clear();
            }
            style = apply_sgr(style, &seq);
        } else if c == '\n' {
            if !text.is_empty() {
                spans.push(Span::styled(text.clone(), style));
                text.clear();
            }
            lines.push(Line::from(spans.clone()));
            spans.clear();
        } else {
            text.push(c);
        }
    }
    if !text.is_empty() {
        spans.push(Span::styled(text, style));
    }
    if !spans.is_empty() {
        lines.push(Line::from(spans));
    }
    lines
}

fn apply_sgr(mut style: Style, seq: &str) -> Style {
    let codes: Vec<u32> = seq.split(';').filter_map(|s| s.parse().ok()).collect();
    let mut i = 0;
    while i < codes.len() {
        match codes[i] {
            0 => style = Style::default(),
            1 => style = style.add_modifier(Modifier::BOLD),
            39 => style = style.fg(Color::Reset),
            49 => style = style.bg(Color::Reset),
            38 if codes.get(i + 1) == Some(&2) && i + 4 < codes.len() => {
                style = style.fg(Color::Rgb(
                    codes[i + 2] as u8,
                    codes[i + 3] as u8,
                    codes[i + 4] as u8,
                ));
                i += 4;
            }
            48 if codes.get(i + 1) == Some(&2) && i + 4 < codes.len() => {
                style = style.bg(Color::Rgb(
                    codes[i + 2] as u8,
                    codes[i + 3] as u8,
                    codes[i + 4] as u8,
                ));
                i += 4;
            }
            38 if codes.get(i + 1) == Some(&5) && i + 2 < codes.len() => {
                style = style.fg(ansi256(codes[i + 2] as u8));
                i += 2;
            }
            48 if codes.get(i + 1) == Some(&5) && i + 2 < codes.len() => {
                style = style.bg(ansi256(codes[i + 2] as u8));
                i += 2;
            }
            _ => {}
        }
        i += 1;
    }
    style
}

fn ansi256(n: u8) -> Color {
    match n {
        0 => Color::Black,
        1 => Color::Red,
        2 => Color::Green,
        3 => Color::Yellow,
        4 => Color::Blue,
        5 => Color::Magenta,
        6 => Color::Cyan,
        7 => Color::White,
        8 => Color::DarkGray,
        9 => Color::LightRed,
        10 => Color::LightGreen,
        11 => Color::LightYellow,
        12 => Color::LightBlue,
        13 => Color::LightMagenta,
        14 => Color::LightCyan,
        15 => Color::White,
        16..=231 => {
            let v = n - 16;
            let b = (v % 6) * 51;
            let g = ((v / 6) % 6) * 51;
            let r = (v / 36) * 51;
            Color::Rgb(r, g, b)
        }
        232..=255 => {
            let v = (n - 232) * 10 + 8;
            Color::Rgb(v, v, v)
        }
    }
}

// ── App ───────────────────────────────────────────────────────────────────────

impl App {
    fn new(config: Config) -> Self {
        let (async_tx, async_rx) = mpsc::channel();
        let projects = vec![];
        let project_entries = vec![];
        let global_prompts = load_prompts_from(&global_prompts_dir());

        let mut prompt_ls = ListState::default();
        if !global_prompts.is_empty() {
            prompt_ls.select(Some(0));
        }
        let mut project_ls = ListState::default();
        if let Some(first_item) = project_entries
            .iter()
            .position(|e| matches!(e, ProjectEntry::Item(_)))
        {
            project_ls.select(Some(first_item));
        }
        let mut setup_ls = ListState::default();
        setup_ls.select(Some(0));

        let prompts = global_prompts.clone();
        let mut app = App {
            config,
            screen: Screen::Projects,
            global_prompts,
            project_prompts: vec![],
            prompts_view: PromptsView::Global,
            prompts,
            prompt_state: PromptState::Browse {
                list_state: prompt_ls,
            },
            issues: vec![],
            issue_list_state: ListState::default(),
            issues_error: None,
            issues_loaded: false,
            issues_loading: false,
            projects,
            project_entries,
            project_list_state: project_ls,
            active_project_idx: None,
            projects_msg: None,
            projects_loading: false,
            scan_generation: 0,
            home_data: None,
            home_remote_loaded: false,
            home_loading: false,
            home_edit: None,
            home_edit_input: String::new(),
            home_save_msg: None,
            setup_items: vec![],
            setup_list_state: setup_ls,
            setup_message: None,
            meta_cache: load_meta_cache(),
            avatar_cache: HashMap::new(),
            avatar_loading_owner: None,
            memories_view: MemoriesView::Project,
            memory_files: vec![],
            memory_list_state: ListState::default(),
            memory_message: None,
            memories_loaded: false,
            memory_input: String::new(),
            memory_inputting: false,
            pending_editor: None,
            skills: vec![],
            skills_list_state: {
                let mut s = ListState::default();
                s.select(Some(0));
                s
            },
            skills_loaded: false,
            mcp_servers: vec![],
            mcp_list_state: {
                let mut s = ListState::default();
                s.select(Some(0));
                s
            },
            mcp_loaded: false,
            context: String::new(),
            repo: String::new(),
            async_tx,
            async_rx,
        };
        app.start_projects_scan(false);
        app
    }

    fn switch_prompts_view(&mut self, view: PromptsView) {
        self.prompts_view = view.clone();
        self.prompts = match view {
            PromptsView::Global => self.global_prompts.clone(),
            PromptsView::Project => self.project_prompts.clone(),
        };
        let mut ls = ListState::default();
        if !self.prompts.is_empty() {
            ls.select(Some(0));
        }
        self.prompt_state = PromptState::Browse { list_state: ls };
    }

    fn refresh_setup(&mut self) {
        if let Some(idx) = self.active_project_idx {
            if let Some(p) = self.projects.get(idx) {
                let path = p.path.clone();
                self.setup_items = scan_setup(&path);
                if !self.setup_items.is_empty() {
                    self.setup_list_state.select(Some(0));
                }
                // Reload project prompts in case .prompts/ was just created
                self.project_prompts = load_prompts_from(&path.join(".prompts"));
                if self.prompts_view == PromptsView::Project {
                    self.prompts = self.project_prompts.clone();
                }
            }
        } else {
            self.setup_items = vec![];
        }
        self.setup_message = None;
    }

    fn auto_values(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        if !self.repo.is_empty() {
            map.insert("REPO".to_string(), self.repo.clone());
        }
        map
    }

    fn selected_prompt_idx(&self) -> Option<usize> {
        if let PromptState::Browse { list_state } = &self.prompt_state {
            list_state.selected()
        } else {
            None
        }
    }

    fn issue_prompt_body(&self) -> String {
        self.prompts
            .iter()
            .find(|p| p.name.contains("issue") || p.name.contains("work-on"))
            .map(|p| p.body.clone())
            .unwrap_or_else(|| DEFAULT_ISSUE_PROMPT.to_string())
    }

    fn switch_project(&mut self, idx: usize) {
        let Some(project) = self.projects.get(idx) else {
            return;
        };
        self.repo = project.repo.clone();
        self.context = make_context(&project.repo, &project.branch);
        self.active_project_idx = Some(idx);
        let path = project.path.clone();
        // Load project-local prompts; default to project view if any exist
        self.project_prompts = load_prompts_from(&path.join(".prompts"));
        let view = if !self.project_prompts.is_empty() {
            PromptsView::Project
        } else {
            PromptsView::Global
        };
        self.switch_prompts_view(view);
        // Load only cheap local data on project open; heavier tab data loads lazily.
        self.home_data = Some(load_home_data_local(&path, &self.repo.clone()));
        self.home_remote_loaded = false;
        self.home_loading = false;
        self.issues = vec![];
        self.issue_list_state = ListState::default();
        self.issues_error = None;
        self.issues_loaded = false;
        self.issues_loading = false;
        // Load setup
        self.setup_items = scan_setup(&path);
        if !self.setup_items.is_empty() {
            self.setup_list_state.select(Some(0));
        }
        self.setup_message = None;
        // Load memories — default to Claude view if it has content, else Project
        let claude_dir = claude_memory_path(&path);
        let has_claude = claude_dir.is_dir()
            && fs::read_dir(&claude_dir)
                .map(|mut d| d.next().is_some())
                .unwrap_or(false);
        self.memories_view = if has_claude {
            MemoriesView::Claude
        } else {
            MemoriesView::Project
        };
        self.memory_files = vec![];
        self.memory_list_state = ListState::default();
        self.memory_message = None;
        self.memories_loaded = false;
        self.skills = vec![];
        self.skills_list_state = ListState::default();
        self.skills_loaded = false;
        self.mcp_servers = vec![];
        self.mcp_list_state = ListState::default();
        self.mcp_loaded = false;
        let repo = self.repo.clone();
        self.start_home_load(&path, &repo);
        // Drill in
        self.screen = Screen::InProject(ProjectTab::Home);
    }

    fn ensure_tab_loaded(&mut self, tab: &ProjectTab) {
        let Some(idx) = self.active_project_idx else {
            return;
        };
        let Some(project) = self.projects.get(idx) else {
            return;
        };
        let path = project.path.clone();

        match tab {
            ProjectTab::Home if !self.home_remote_loaded && !self.home_loading => {
                let repo = self.repo.clone();
                self.start_home_load(&path, &repo);
            }
            ProjectTab::Issues if !self.issues_loaded && !self.issues_loading => {
                let repo = self.repo.clone();
                self.start_issues_load(&repo);
            }
            ProjectTab::Memories if !self.memories_loaded => {
                self.reload_memories();
                self.memories_loaded = true;
            }
            ProjectTab::Skills if !self.skills_loaded => {
                self.skills = load_skills(&path);
                self.skills_list_state = {
                    let mut s = ListState::default();
                    if !self.skills.is_empty() {
                        s.select(Some(0));
                    }
                    s
                };
                self.skills_loaded = true;
            }
            ProjectTab::Mcp if !self.mcp_loaded => {
                self.mcp_servers = load_mcp_servers(&path);
                self.mcp_list_state = {
                    let mut s = ListState::default();
                    if !self.mcp_servers.is_empty() {
                        s.select(Some(0));
                    }
                    s
                };
                self.mcp_loaded = true;
            }
            _ => {}
        }
    }

    fn set_project_tab(&mut self, tab: ProjectTab) {
        self.screen = Screen::InProject(tab.clone());
        self.ensure_tab_loaded(&tab);
    }

    fn start_home_load(&mut self, path: &Path, repo: &str) {
        self.home_loading = true;
        let tx = self.async_tx.clone();
        let repo_owned = repo.to_string();
        let path_owned = path.to_path_buf();
        std::thread::spawn(move || {
            let data = load_home_data(&path_owned, &repo_owned);
            let _ = tx.send(AsyncResult::Home {
                repo: repo_owned,
                data,
            });
        });

        let owner = repo.split('/').next().unwrap_or("").to_string();
        if !owner.is_empty()
            && !self.avatar_cache.contains_key(&owner)
            && self.avatar_loading_owner.as_deref() != Some(owner.as_str())
        {
            self.avatar_loading_owner = Some(owner.clone());
            let tx = self.async_tx.clone();
            std::thread::spawn(move || {
                let ansi = fetch_avatar(&owner);
                let _ = tx.send(AsyncResult::Avatar { owner, ansi });
            });
        }
    }

    fn start_issues_load(&mut self, repo: &str) {
        self.issues_loading = true;
        self.issues_error = None;
        let tx = self.async_tx.clone();
        let repo_owned = repo.to_string();
        std::thread::spawn(move || {
            let result = load_issues(&repo_owned);
            let _ = tx.send(AsyncResult::Issues {
                repo: repo_owned,
                result,
            });
        });
    }

    fn start_projects_scan(&mut self, preserve_message: bool) {
        self.scan_generation += 1;
        self.projects_loading = true;
        if !preserve_message {
            self.projects_msg = Some("scanning projects...".to_string());
        }
        let tx = self.async_tx.clone();
        let generation = self.scan_generation;
        let config = self.config.clone();
        std::thread::spawn(move || {
            let projects = scan_projects(&config);
            let _ = tx.send(AsyncResult::Projects {
                generation,
                projects,
            });
        });
    }

    fn process_async_results(&mut self) {
        while let Ok(msg) = self.async_rx.try_recv() {
            match msg {
                AsyncResult::Home { repo, data } => {
                    if self.repo == repo {
                        self.home_data = Some(data);
                        self.home_remote_loaded = true;
                        self.home_loading = false;
                    }
                }
                AsyncResult::Issues { repo, result } => {
                    if self.repo == repo {
                        match result {
                            Ok(issues) => {
                                let mut ls = ListState::default();
                                if !issues.is_empty() {
                                    ls.select(Some(0));
                                }
                                self.issues = issues;
                                self.issue_list_state = ls;
                                self.issues_error = None;
                            }
                            Err(e) => {
                                self.issues = vec![];
                                self.issues_error = Some(e);
                            }
                        }
                        self.issues_loaded = true;
                        self.issues_loading = false;
                    }
                }
                AsyncResult::Avatar { owner, ansi } => {
                    if let Some(ansi) = ansi {
                        self.avatar_cache.insert(owner.clone(), ansi);
                    }
                    if self.avatar_loading_owner.as_deref() == Some(owner.as_str()) {
                        self.avatar_loading_owner = None;
                    }
                }
                AsyncResult::Projects {
                    generation,
                    projects,
                } => {
                    if generation == self.scan_generation {
                        self.projects = projects;
                        self.project_entries = build_project_entries(&self.projects);
                        self.project_list_state = {
                            let mut ls = ListState::default();
                            if let Some(first_item) = self
                                .project_entries
                                .iter()
                                .position(|e| matches!(e, ProjectEntry::Item(_)))
                            {
                                ls.select(Some(first_item));
                            }
                            ls
                        };
                        self.projects_loading = false;
                        self.projects_msg =
                            Some(format!("{} projects loaded", self.projects.len()));
                    }
                }
            }
        }
    }

    fn memory_dir(&self) -> PathBuf {
        match self.memories_view {
            MemoriesView::Project => self
                .active_project_idx
                .and_then(|i| self.projects.get(i))
                .map(|p| p.path.join(".memory"))
                .unwrap_or_default(),
            MemoriesView::Global => global_memory_path(),
            MemoriesView::Claude => self
                .active_project_idx
                .and_then(|i| self.projects.get(i))
                .map(|p| claude_memory_path(&p.path))
                .unwrap_or_default(),
        }
    }

    fn reload_memories(&mut self) {
        let dir = self.memory_dir();
        self.memory_files = load_memory_files(&dir);
        let mut ls = ListState::default();
        if !self.memory_files.is_empty() {
            ls.select(Some(0));
        }
        self.memory_list_state = ls;
    }

    fn switch_memories_view(&mut self, view: MemoriesView) {
        self.memories_view = view;
        self.reload_memories();
        self.memories_loaded = true;
        self.memory_message = None;
    }
}

fn make_context(repo: &str, branch: &str) -> String {
    if repo.is_empty() {
        format!("no repo ({branch})")
    } else {
        format!("{repo} ({branch})")
    }
}

const DEFAULT_ISSUE_PROMPT: &str = "Work on issue #{ISSUE} in {REPO}.\n\nBefore writing any code:\n1. Read AGENT.md and SPEC.md in the project root\n2. Read the issue in full: gh issue view {ISSUE}\n3. Identify only the files relevant to the issue\n\nDo the work. Then:\n1. Run vp check — fix any errors before committing\n2. Run vp build — must succeed\n3. Commit: \"fix: <description> (closes #{ISSUE})\"\n\nDo not close the issue. Do not open a PR. Stop after the commit.";

const MEMORY_INDEX_TEMPLATE: &str = "# Memory Index\n\nAgent memory for this project. Read this first, then load only the files relevant to the current task.\n\n> Format: `- [filename.md](filename.md) — one-line description`\n\n<!-- add entries below as memories are created -->\n";

// ── Event handling ────────────────────────────────────────────────────────────

fn handle_key(app: &mut App, key: KeyCode, modifiers: KeyModifiers) -> bool {
    if key == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL) {
        return true;
    }

    match &app.screen {
        Screen::Projects => handle_projects(app, key),
        Screen::InProject(_) => {
            // Fill/Done, home-edit, and memory-input capture all keys before global nav
            let in_flow = matches!(
                &app.prompt_state,
                PromptState::Fill { .. } | PromptState::Done(_)
            ) || app.home_edit.is_some()
                || app.memory_inputting;
            if !in_flow {
                match key {
                    KeyCode::Esc => {
                        app.screen = Screen::Projects;
                        return false;
                    }
                    KeyCode::Char('q') => return true,
                    KeyCode::Tab => {
                        let next = match &app.screen {
                            Screen::InProject(ProjectTab::Home) => ProjectTab::Issues,
                            Screen::InProject(ProjectTab::Issues) => ProjectTab::Setup,
                            Screen::InProject(ProjectTab::Setup) => ProjectTab::Prompts,
                            Screen::InProject(ProjectTab::Prompts) => ProjectTab::Memories,
                            Screen::InProject(ProjectTab::Memories) => ProjectTab::Skills,
                            Screen::InProject(ProjectTab::Skills) => ProjectTab::Mcp,
                            Screen::InProject(ProjectTab::Mcp) => ProjectTab::Pane,
                            Screen::InProject(ProjectTab::Pane) => ProjectTab::Home,
                            _ => ProjectTab::Home,
                        };
                        app.set_project_tab(next);
                        return false;
                    }
                    KeyCode::Char('1') => {
                        app.set_project_tab(ProjectTab::Home);
                        return false;
                    }
                    KeyCode::Char('2') => {
                        app.set_project_tab(ProjectTab::Issues);
                        return false;
                    }
                    KeyCode::Char('3') => {
                        app.set_project_tab(ProjectTab::Setup);
                        return false;
                    }
                    KeyCode::Char('4') => {
                        app.set_project_tab(ProjectTab::Prompts);
                        return false;
                    }
                    KeyCode::Char('5') => {
                        app.set_project_tab(ProjectTab::Memories);
                        return false;
                    }
                    KeyCode::Char('6') => {
                        app.set_project_tab(ProjectTab::Skills);
                        return false;
                    }
                    KeyCode::Char('7') => {
                        app.set_project_tab(ProjectTab::Mcp);
                        return false;
                    }
                    KeyCode::Char('8') => {
                        app.set_project_tab(ProjectTab::Pane);
                        return false;
                    }
                    _ => {}
                }
            }
            // Dispatch to sub-screen handler
            let tab = if let Screen::InProject(t) = &app.screen {
                t.clone()
            } else {
                return false;
            };
            match tab {
                ProjectTab::Home => handle_home(app, key),
                ProjectTab::Issues => handle_issues(app, key),
                ProjectTab::Setup => handle_setup(app, key),
                ProjectTab::Prompts => handle_prompts(app, key),
                ProjectTab::Memories => handle_memories(app, key),
                ProjectTab::Skills => handle_skills(app, key),
                ProjectTab::Mcp => handle_mcp(app, key),
                ProjectTab::Pane => false,
            }
        }
    }
}

fn handle_prompts(app: &mut App, key: KeyCode) -> bool {
    // Subnav: g = global, p = project
    if matches!(&app.prompt_state, PromptState::Browse { .. }) {
        match key {
            KeyCode::Char('g') => {
                app.switch_prompts_view(PromptsView::Global);
                return false;
            }
            KeyCode::Char('p') => {
                app.switch_prompts_view(PromptsView::Project);
                return false;
            }
            _ => {}
        }
    }

    let auto = app.auto_values();
    let fillable_cache: Vec<String> =
        if let PromptState::Fill { prompt_idx, .. } = &app.prompt_state {
            let idx = *prompt_idx;
            app.prompts[idx]
                .placeholders
                .iter()
                .filter(|p| !auto.contains_key(*p))
                .cloned()
                .collect()
        } else {
            Vec::new()
        };

    match &mut app.prompt_state {
        PromptState::Browse { list_state } => {
            let len = app.prompts.len();
            match key {
                KeyCode::Down | KeyCode::Char('j') => {
                    let n = (list_state.selected().unwrap_or(0) + 1) % len;
                    list_state.select(Some(n));
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    let n = list_state
                        .selected()
                        .map(|i| if i == 0 { len - 1 } else { i - 1 })
                        .unwrap_or(0);
                    list_state.select(Some(n));
                }
                KeyCode::Enter => {
                    if let Some(idx) = list_state.selected() {
                        let prompt = &app.prompts[idx];
                        let fillable: Vec<String> = prompt
                            .placeholders
                            .iter()
                            .filter(|p| !auto.contains_key(*p))
                            .cloned()
                            .collect();
                        if fillable.is_empty() {
                            let filled = fill(&prompt.body, &auto);
                            copy_to_clipboard(&filled);
                            app.prompt_state = PromptState::Done(filled);
                        } else {
                            app.prompt_state = PromptState::Fill {
                                prompt_idx: idx,
                                field_idx: 0,
                                values: auto.clone(),
                                input: String::new(),
                            };
                        }
                    }
                }
                _ => {}
            }
        }
        PromptState::Fill {
            prompt_idx,
            field_idx,
            values,
            input,
        } => {
            let fillable = &fillable_cache;
            match key {
                KeyCode::Esc => {
                    let idx = *prompt_idx;
                    let mut ls = ListState::default();
                    ls.select(Some(idx));
                    app.prompt_state = PromptState::Browse { list_state: ls };
                }
                KeyCode::Backspace => {
                    input.pop();
                }
                KeyCode::Char(c) => {
                    input.push(c);
                }
                KeyCode::Enter => {
                    if *field_idx < fillable.len() {
                        values.insert(fillable[*field_idx].clone(), input.clone());
                        *input = String::new();
                        *field_idx += 1;
                        if *field_idx >= fillable.len() {
                            let v = values.clone();
                            let b = app.prompts[*prompt_idx].body.clone();
                            let filled = fill(&b, &v);
                            copy_to_clipboard(&filled);
                            app.prompt_state = PromptState::Done(filled);
                        }
                    }
                }
                _ => {}
            }
        }
        PromptState::Done(_) => {
            let mut ls = ListState::default();
            ls.select(Some(0));
            app.prompt_state = PromptState::Browse { list_state: ls };
        }
    }
    false
}

fn handle_home(app: &mut App, key: KeyCode) -> bool {
    // Edit mode: capture all keys
    if let Some(field) = app.home_edit.clone() {
        match key {
            KeyCode::Esc => {
                app.home_edit = None;
                app.home_edit_input.clear();
            }
            KeyCode::Backspace => {
                app.home_edit_input.pop();
            }
            KeyCode::Char(c) => {
                app.home_edit_input.push(c);
            }
            KeyCode::Enter => {
                let repo = app.repo.clone();
                let value = app.home_edit_input.trim().to_string();
                match save_repo_field(&repo, &field, &value) {
                    Ok(_) => {
                        if let Some(data) = &mut app.home_data {
                            match field {
                                HomeEditField::Description => {
                                    data.gh_description =
                                        if value.is_empty() { None } else { Some(value) }
                                }
                                HomeEditField::Homepage => {
                                    data.homepage =
                                        if value.is_empty() { None } else { Some(value) }
                                }
                            }
                        }
                        app.home_save_msg = Some("Saved.".to_string());
                    }
                    Err(e) => {
                        app.home_save_msg = Some(format!("Error: {e}"));
                    }
                }
                app.home_edit = None;
                app.home_edit_input.clear();
            }
            _ => {}
        }
        return false;
    }

    match key {
        KeyCode::Char('r') => {
            if let Some(idx) = app.active_project_idx {
                if let Some(p) = app.projects.get(idx) {
                    let path = p.path.clone();
                    let repo = app.repo.clone();
                    app.start_home_load(&path, &repo);
                    app.home_save_msg = None;
                }
            }
        }
        KeyCode::Char('e') => {
            let current = app
                .home_data
                .as_ref()
                .and_then(|d| d.gh_description.clone())
                .unwrap_or_default();
            app.home_edit = Some(HomeEditField::Description);
            app.home_edit_input = current;
            app.home_save_msg = None;
        }
        KeyCode::Char('u') => {
            let current = app
                .home_data
                .as_ref()
                .and_then(|d| d.homepage.clone())
                .unwrap_or_default();
            app.home_edit = Some(HomeEditField::Homepage);
            app.home_edit_input = current;
            app.home_save_msg = None;
        }
        KeyCode::Char('y') => {
            if let Some(data) = &app.home_data {
                if !data.url.is_empty() {
                    copy_to_clipboard(&data.url);
                }
            }
        }
        _ => {}
    }
    false
}

fn handle_issues(app: &mut App, key: KeyCode) -> bool {
    let len = app.issues.len();
    match key {
        KeyCode::Down | KeyCode::Char('j') if len > 0 => {
            let n = (app.issue_list_state.selected().unwrap_or(0) + 1) % len;
            app.issue_list_state.select(Some(n));
        }
        KeyCode::Up | KeyCode::Char('k') if len > 0 => {
            let n = app
                .issue_list_state
                .selected()
                .map(|i| if i == 0 { len - 1 } else { i - 1 })
                .unwrap_or(0);
            app.issue_list_state.select(Some(n));
        }
        KeyCode::Enter => {
            if let Some(idx) = app.issue_list_state.selected() {
                let number = app.issues[idx].number.to_string();
                let body = app.issue_prompt_body();
                let mut values = app.auto_values();
                values.insert("ISSUE".to_string(), number);
                let filled = fill(&body, &values);
                copy_to_clipboard(&filled);
                app.screen = Screen::InProject(ProjectTab::Prompts);
                app.prompt_state = PromptState::Done(filled);
            }
        }
        KeyCode::Char('r') => {
            let repo = app.repo.clone();
            app.start_issues_load(&repo);
        }
        _ => {}
    }
    false
}

fn next_item_entry(entries: &[ProjectEntry], from: usize, step: isize) -> usize {
    let len = entries.len();
    let mut i = ((from as isize + step).rem_euclid(len as isize)) as usize;
    for _ in 0..len {
        if matches!(entries[i], ProjectEntry::Item(_)) {
            return i;
        }
        i = ((i as isize + step).rem_euclid(len as isize)) as usize;
    }
    from
}

fn handle_projects(app: &mut App, key: KeyCode) -> bool {
    let elen = app.project_entries.len();
    match key {
        KeyCode::Char('q') => return true, // quit from root
        KeyCode::Down | KeyCode::Char('j') if elen > 0 => {
            let cur = app.project_list_state.selected().unwrap_or(0);
            app.project_list_state
                .select(Some(next_item_entry(&app.project_entries, cur, 1)));
        }
        KeyCode::Up | KeyCode::Char('k') if elen > 0 => {
            let cur = app.project_list_state.selected().unwrap_or(0);
            app.project_list_state
                .select(Some(next_item_entry(&app.project_entries, cur, -1)));
        }
        KeyCode::Enter => {
            if let Some(entry_idx) = app.project_list_state.selected() {
                if let Some(ProjectEntry::Item(proj_idx)) = app.project_entries.get(entry_idx) {
                    let idx = *proj_idx;
                    app.switch_project(idx);
                }
            }
        }
        KeyCode::Char('r') => {
            if let Some(entry_idx) = app.project_list_state.selected() {
                if let Some(ProjectEntry::Item(proj_idx)) =
                    app.project_entries.get(entry_idx).cloned()
                {
                    let p = &app.projects[proj_idx];
                    let (path, group) = (p.path.clone(), p.group.clone());
                    if let Some(fresh) = project_info(&path, group) {
                        let repo = fresh.repo.clone();
                        app.projects[proj_idx] = fresh;
                        app.project_entries = build_project_entries(&app.projects);
                        // gh meta refresh (blocking but only one repo)
                        match refresh_project_meta(&repo) {
                            Some(meta) => {
                                app.meta_cache.insert(repo.clone(), meta);
                                save_meta_cache(&app.meta_cache);
                                app.projects_msg = Some(format!("{repo} refreshed"));
                            }
                            None => {
                                app.projects_msg =
                                    Some(format!("{repo} — git refreshed (gh meta unavailable)"));
                            }
                        }
                    }
                }
            }
        }
        KeyCode::Char('R') => {
            app.start_projects_scan(false);
        }
        _ => {}
    }
    false
}

fn fill(body: &str, values: &HashMap<String, String>) -> String {
    let mut r = body.to_string();
    for (k, v) in values {
        r = r.replace(&format!("{{{k}}}"), v);
    }
    r
}

fn copy_to_clipboard(text: &str) {
    if let Ok(mut cb) = Clipboard::new() {
        let _ = cb.set_text(text);
    }
}

// ── Drawing ───────────────────────────────────────────────────────────────────

fn draw(frame: &mut Frame, app: &App) {
    match (&app.screen, &app.prompt_state) {
        (
            _,
            PromptState::Fill {
                prompt_idx,
                field_idx,
                values,
                input,
            },
        ) => draw_fill(frame, app, *prompt_idx, *field_idx, values, input),
        (_, PromptState::Done(text)) => draw_done(frame, text),
        (Screen::Projects, _) => draw_projects(frame, app),
        (Screen::InProject(ProjectTab::Home), _) => draw_home(frame, app),
        (Screen::InProject(ProjectTab::Issues), _) => draw_issues(frame, app),
        (Screen::InProject(ProjectTab::Setup), _) => draw_setup(frame, app),
        (Screen::InProject(ProjectTab::Prompts), _) => draw_prompts(frame, app),
        (Screen::InProject(ProjectTab::Memories), _) => draw_memories(frame, app),
        (Screen::InProject(ProjectTab::Skills), _) => draw_skills(frame, app),
        (Screen::InProject(ProjectTab::Mcp), _) => draw_mcp(frame, app),
        (Screen::InProject(ProjectTab::Pane), _) => draw_pane(frame, app),
    }
}

fn tab_span(icon: &str, label: &str, n: u8, active: bool) -> Vec<Span<'static>> {
    let text = if icon.is_empty() {
        format!(" {n} {label} ")
    } else {
        format!(" {icon} {n} {label} ")
    };
    if active {
        vec![Span::styled(
            text,
            Style::default()
                .fg(SEL_FG)
                .bg(ACCENT)
                .add_modifier(Modifier::BOLD),
        )]
    } else {
        vec![Span::styled(text, Style::default().fg(FG_DIM))]
    }
}

fn header_row(app: &App) -> Line<'static> {
    let badge = Span::styled(
        " 🐧 pm ",
        Style::default()
            .fg(SEL_FG)
            .bg(ACCENT)
            .add_modifier(Modifier::BOLD),
    );
    match &app.screen {
        Screen::Projects => Line::from(vec![
            badge,
            Span::raw("  "),
            Span::styled(
                format!(" {I_PROJECTS} projects "),
                Style::default()
                    .fg(SEL_FG)
                    .bg(ACCENT)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Screen::InProject(_) => {
            let repo_short = app.repo.split('/').last().unwrap_or(&app.repo).to_string();
            let branch = app
                .context
                .split('(')
                .nth(1)
                .unwrap_or("")
                .trim_end_matches(')')
                .to_string();
            let mut spans = vec![
                badge,
                Span::raw("  "),
                Span::styled(
                    repo_short,
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
            ];
            if !branch.is_empty() {
                spans.push(Span::styled(
                    format!("  {I_BRANCH} {branch}"),
                    Style::default().fg(FG_DIM),
                ));
            }
            Line::from(spans)
        }
    }
}

fn nav_row(app: &App) -> Line<'static> {
    let Screen::InProject(active_tab) = &app.screen else {
        return Line::from("");
    };
    let repo_short = app.repo.split('/').last().unwrap_or(&app.repo).to_string();
    let mut spans: Vec<Span> = Vec::new();
    let tabs: &[(&str, &str, u8, bool)] = &[
        (
            "",
            repo_short.as_str(),
            1u8,
            *active_tab == ProjectTab::Home,
        ),
        (I_ISSUES, "issues", 2, *active_tab == ProjectTab::Issues),
        (I_SETUP, "setup", 3, *active_tab == ProjectTab::Setup),
        (I_PROMPTS, "prompts", 4, *active_tab == ProjectTab::Prompts),
        (I_MEMORY, "memories", 5, *active_tab == ProjectTab::Memories),
        (I_SKILLS, "skills", 6, *active_tab == ProjectTab::Skills),
        (I_MCP, "mcp", 7, *active_tab == ProjectTab::Mcp),
        (I_PANE, "pane", 8, *active_tab == ProjectTab::Pane),
    ];
    for (icon, label, n, active) in tabs {
        spans.extend(tab_span(icon, label, *n, *active));
        spans.push(Span::raw(" "));
    }
    Line::from(spans)
}

fn draw_home(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let bottom_h = if app.home_edit.is_some() || app.home_save_msg.is_some() {
        3u16
    } else {
        0
    };
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(bottom_h),
            Constraint::Length(1),
        ])
        .split(area);

    frame.render_widget(Paragraph::new(header_row(app)), outer[0]);
    frame.render_widget(Paragraph::new(nav_row(app)), outer[1]);

    let title = format!(" {} ", app.repo);
    let block = Block::default().borders(Borders::ALL).title(title);

    if let Some(data) = &app.home_data {
        let inner = block.inner(outer[2]);
        frame.render_widget(block, outer[2]);

        let owner = app.repo.split('/').next().unwrap_or("");
        let avatar_ansi = app.avatar_cache.get(owner);
        let avatar_w = if avatar_ansi.is_some() { 22u16 } else { 0 };

        let split = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(avatar_w),
                Constraint::Percentage(55),
                Constraint::Percentage(45),
            ])
            .split(inner);

        // Avatar column
        if let Some(ansi) = avatar_ansi {
            let avatar_lines = ansi_to_lines(ansi);
            frame.render_widget(Paragraph::new(avatar_lines), split[0]);
        }

        // Left column (info)
        let left_area = split[1];
        let mut left: Vec<Line> = vec![Line::from("")];

        // URL row (always shown if known)
        if !data.url.is_empty() {
            left.push(Line::from(vec![
                Span::styled("  url      ", Style::default().fg(FG_DIM)),
                Span::styled(data.url.clone(), Style::default().fg(C_PURPLE)),
                Span::styled("  y copy", Style::default().fg(FG_XDIM)),
            ]));
        }

        // Description
        left.push(Line::from(""));
        if let Some(desc) = &data.gh_description {
            left.push(Line::from(vec![
                Span::styled("  desc     ", Style::default().fg(FG_DIM)),
                Span::styled(desc.clone(), Style::default().fg(Color::White)),
            ]));
        } else if app.home_loading {
            left.push(Line::from(vec![
                Span::styled("  desc     ", Style::default().fg(FG_DIM)),
                Span::styled("loading...", Style::default().fg(FG_XDIM)),
            ]));
        } else {
            left.push(Line::from(vec![
                Span::styled("  desc     ", Style::default().fg(FG_DIM)),
                Span::styled("not set", Style::default().fg(FG_XDIM)),
            ]));
        }

        // Homepage
        if let Some(home) = &data.homepage {
            left.push(Line::from(vec![
                Span::styled("  homepage ", Style::default().fg(FG_DIM)),
                Span::styled(home.clone(), Style::default().fg(C_PURPLE)),
            ]));
        } else if app.home_loading {
            left.push(Line::from(vec![
                Span::styled("  homepage ", Style::default().fg(FG_DIM)),
                Span::styled("loading...", Style::default().fg(FG_XDIM)),
            ]));
        }

        // Stack
        if let Some(stack) = &data.stack {
            left.push(Line::from(""));
            left.push(Line::from(vec![
                Span::styled("  stack    ", Style::default().fg(FG_DIM)),
                Span::styled(stack.clone(), Style::default().fg(C_PURPLE)),
            ]));
        }

        // Topics
        if let Some(meta) = app
            .active_project_idx
            .and_then(|i| app.projects.get(i))
            .and_then(|p| app.meta_cache.get(&p.repo))
        {
            if !meta.topics.is_empty() {
                left.push(Line::from(""));
                let mut spans = vec![Span::styled("  topics   ", Style::default().fg(FG_DIM))];
                for (i, t) in meta.topics.iter().enumerate() {
                    if i > 0 {
                        spans.push(Span::raw("  "));
                    }
                    spans.push(Span::styled(t.clone(), Style::default().fg(FG_DIM)));
                }
                left.push(Line::from(spans));
            }
        }

        // Setup score
        left.push(Line::from(""));
        let setup_color = if data.setup_ok == data.setup_total {
            C_GREEN
        } else {
            C_YELLOW
        };
        left.push(Line::from(vec![
            Span::styled("  setup    ", Style::default().fg(FG_DIM)),
            Span::styled(
                format!("{}/{} configured", data.setup_ok, data.setup_total),
                Style::default().fg(setup_color),
            ),
            if data.setup_ok < data.setup_total {
                Span::styled("  → [3 setup]", Style::default().fg(FG_XDIM))
            } else {
                Span::raw("")
            },
        ]));

        frame.render_widget(Paragraph::new(left).wrap(Wrap { trim: false }), left_area);

        // Right: recent commits
        let mut right: Vec<Line> = vec![
            Line::from(Span::styled(
                format!("  {I_COMMIT} recent commits"),
                Style::default().fg(FG_XDIM),
            )),
            Line::from(""),
        ];
        if data.recent_commits.is_empty() {
            right.push(Line::from(Span::styled(
                "  no commits",
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            for commit in &data.recent_commits {
                let (hash, rest) = commit.split_once(' ').unwrap_or(("", commit));
                right.push(Line::from(vec![
                    Span::styled(
                        format!("  {I_COMMIT} {hash}  "),
                        Style::default().fg(FG_DIM),
                    ),
                    Span::raw(rest.to_string()),
                ]));
            }
        }
        frame.render_widget(Paragraph::new(right).wrap(Wrap { trim: false }), split[2]);
    } else {
        frame.render_widget(
            Paragraph::new(Span::styled(
                "  Loading…",
                Style::default().fg(Color::DarkGray),
            ))
            .block(block),
            outer[2],
        );
    }

    // Edit input or save message
    if let Some(field) = &app.home_edit {
        let label = match field {
            HomeEditField::Description => "description",
            HomeEditField::Homepage => "homepage url",
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("> ", Style::default().fg(ACCENT)),
                Span::raw(app.home_edit_input.clone()),
                Span::styled("█", Style::default().fg(ACCENT)),
            ]))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" edit {label} ")),
            ),
            outer[3],
        );
    } else if let Some(msg) = &app.home_save_msg {
        let (icon, color) = if msg.starts_with("Error") {
            (I_CROSS, C_RED)
        } else {
            (I_CHECK, C_GREEN)
        };
        frame.render_widget(
            Paragraph::new(Span::styled(
                format!("  {icon}  {msg}"),
                Style::default().fg(color),
            ))
            .block(Block::default().borders(Borders::ALL)),
            outer[3],
        );
    }

    let footer_hints = if app.home_edit.is_some() {
        footer(&[("enter", "save"), ("esc", "cancel")])
    } else {
        footer(&[
            ("r", "reload"),
            ("e", "edit desc"),
            ("u", "edit homepage"),
            ("y", "copy url"),
            ("2", "issues"),
            ("3", "setup"),
            ("esc", "back"),
        ])
    };
    frame.render_widget(Paragraph::new(footer_hints), outer[4]);
}

fn footer(hints: &[(&'static str, &'static str)]) -> Line<'static> {
    let mut spans: Vec<Span> = Vec::new();
    for (key, desc) in hints {
        spans.push(Span::styled(
            format!(" {key} "),
            Style::default().fg(ACCENT),
        ));
        spans.push(Span::styled(
            format!("{desc}  "),
            Style::default().fg(FG_DIM),
        ));
    }
    Line::from(spans)
}

fn hl() -> Style {
    Style::default()
        .fg(SEL_FG)
        .bg(ACCENT)
        .add_modifier(Modifier::BOLD)
}

fn draw_prompts(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    frame.render_widget(Paragraph::new(header_row(app)), outer[0]);
    frame.render_widget(Paragraph::new(nav_row(app)), outer[1]);

    // Split left pane into subnav + list
    let left_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(outer[2]);

    let list_split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(left_area[0]);

    // Subnav
    let global_active = app.prompts_view == PromptsView::Global;
    let project_active = app.prompts_view == PromptsView::Project;
    let has_project = !app.project_prompts.is_empty();
    let subnav = Line::from(vec![
        Span::styled(
            " g global ",
            if global_active {
                Style::default()
                    .fg(SEL_FG)
                    .bg(ACCENT)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(FG_DIM)
            },
        ),
        Span::raw(" "),
        Span::styled(
            " p project ",
            if project_active {
                Style::default()
                    .fg(SEL_FG)
                    .bg(ACCENT)
                    .add_modifier(Modifier::BOLD)
            } else if has_project {
                Style::default().fg(FG_DIM)
            } else {
                Style::default().fg(FG_XDIM)
            },
        ),
    ]);
    frame.render_widget(Paragraph::new(subnav), list_split[0]);

    let items: Vec<ListItem> = app
        .prompts
        .iter()
        .map(|p| ListItem::new(p.name.clone()))
        .collect();
    let empty_hint = if !project_active {
        String::new()
    } else {
        " no .prompts/ — run setup ".to_string()
    };
    let block_title = if items.is_empty() && project_active {
        empty_hint.as_str()
    } else {
        " prompts "
    };
    let mut ls = if let PromptState::Browse { list_state } = &app.prompt_state {
        list_state.clone()
    } else {
        ListState::default()
    };
    frame.render_stateful_widget(
        List::new(items)
            .block(Block::default().borders(Borders::ALL).title(block_title))
            .highlight_style(hl())
            .highlight_symbol("> "),
        list_split[1],
        &mut ls,
    );

    let preview = app
        .selected_prompt_idx()
        .and_then(|i| app.prompts.get(i))
        .map(|p| p.preview.as_str())
        .unwrap_or("");
    frame.render_widget(
        Paragraph::new(preview)
            .block(Block::default().borders(Borders::ALL).title(" preview "))
            .wrap(Wrap { trim: false }),
        left_area[1],
    );
    frame.render_widget(
        Paragraph::new(footer(&[
            ("g", "global"),
            ("p", "project"),
            ("↑↓/jk", "navigate"),
            ("enter", "select"),
            ("esc", "back"),
            ("tab", "switch"),
            ("q", "quit"),
        ])),
        outer[3],
    );
}

fn draw_issues(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    frame.render_widget(Paragraph::new(header_row(app)), outer[0]);
    frame.render_widget(Paragraph::new(nav_row(app)), outer[1]);

    if let Some(err) = &app.issues_error {
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(
                    format!("  {I_CROSS}  {err}"),
                    Style::default().fg(C_RED),
                )),
            ])
            .block(Block::default().borders(Borders::ALL).title(" issues ")),
            outer[2],
        );
    } else if app.issues_loading {
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  Loading issues...",
                    Style::default().fg(FG_XDIM),
                )),
            ])
            .block(Block::default().borders(Borders::ALL).title(" issues ")),
            outer[2],
        );
    } else if app.issues.is_empty() {
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  No open issues.",
                    Style::default().fg(Color::DarkGray),
                )),
            ])
            .block(Block::default().borders(Borders::ALL).title(" issues ")),
            outer[2],
        );
    } else {
        let main = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(outer[2]);

        let items: Vec<ListItem> = app
            .issues
            .iter()
            .map(|issue| {
                let lbl = if issue.labels.is_empty() {
                    String::new()
                } else {
                    format!(" [{}]", issue.labels.join(", "))
                };
                ListItem::new(Line::from(vec![
                    Span::styled(format!("#{:<5}", issue.number), Style::default().fg(FG_DIM)),
                    Span::raw(issue.title.clone()),
                    Span::styled(lbl, Style::default().fg(FG_DIM)),
                ]))
            })
            .collect();

        let mut ls = app.issue_list_state.clone();
        frame.render_stateful_widget(
            List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" open issues "),
                )
                .highlight_style(hl())
                .highlight_symbol("> "),
            main[0],
            &mut ls,
        );

        let preview: Vec<Line> = app
            .issue_list_state
            .selected()
            .and_then(|i| app.issues.get(i))
            .map(|issue| {
                let mut lines = vec![
                    Line::from(Span::styled(
                        format!("{I_ISSUES} #{} — {}", issue.number, issue.title),
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    )),
                    Line::from(""),
                ];
                if !issue.labels.is_empty() {
                    lines.push(Line::from(vec![
                        Span::styled("Labels: ", Style::default().fg(Color::DarkGray)),
                        Span::raw(issue.labels.join(", ")),
                    ]));
                    lines.push(Line::from(""));
                }
                for l in issue.body.lines() {
                    lines.push(Line::from(l.to_string()));
                }
                lines
            })
            .unwrap_or_default();

        frame.render_widget(
            Paragraph::new(preview)
                .block(Block::default().borders(Borders::ALL).title(" body "))
                .wrap(Wrap { trim: false }),
            main[1],
        );
    }
    frame.render_widget(
        Paragraph::new(footer(&[
            ("↑↓/jk", "navigate"),
            ("enter", "copy prompt"),
            ("r", "refresh"),
            ("esc", "back"),
            ("tab", "switch"),
            ("q", "quit"),
        ])),
        outer[3],
    );
}

fn draw_projects(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    frame.render_widget(Paragraph::new(header_row(app)), outer[0]);

    if app.projects_loading && app.projects.is_empty() {
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  Scanning projects...",
                    Style::default().fg(FG_DIM),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  The UI stays live while the scan runs.",
                    Style::default().fg(Color::DarkGray),
                )),
            ])
            .block(Block::default().borders(Borders::ALL).title(" projects ")),
            outer[1],
        );
    } else if app.projects.is_empty() {
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  No projects found.",
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  Set $PEMGUIN_PROJECTS_DIR or place projects under ~/Projects/",
                    Style::default().fg(Color::DarkGray),
                )),
            ])
            .block(Block::default().borders(Borders::ALL).title(" projects ")),
            outer[1],
        );
    } else {
        // Split inner area: column header row (1 line) + list
        let block = Block::default().borders(Borders::ALL).title(" projects ");
        let inner = block.inner(outer[1]);
        frame.render_widget(block, outer[1]);

        let split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(inner);

        // Compute repo column width from actual names, bounded by terminal width.
        // Fixed columns consume ~52 chars (marker+lang+branch+status+ahead+cfg+pushed+gaps).
        let fixed_cols: usize = 52;
        let max_name = app
            .projects
            .iter()
            .map(|p| p.repo.split('/').last().unwrap_or(&p.repo).len())
            .max()
            .unwrap_or(16);
        let available = (frame.area().width as usize).saturating_sub(fixed_cols);
        let repo_col = max_name.clamp(16, available.max(16));

        // Column header
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!(
                    "  {:<repo_col$}  {:<4}  {:<12}  {:<6}  {:<4}  {:<4}  {}",
                    "repo", "lang", "branch", "status", "↑", "cfg", "pushed"
                ),
                Style::default().fg(FG_XDIM),
            ))),
            split[0],
        );

        // Build list items from entries (groups + projects)
        let items: Vec<ListItem> = app
            .project_entries
            .iter()
            .map(|entry| {
                match entry {
                    ProjectEntry::Group(name) => ListItem::new(Line::from(Span::styled(
                        format!("  {name}"),
                        Style::default().fg(C_PURPLE).add_modifier(Modifier::BOLD),
                    ))),
                    ProjectEntry::Item(proj_idx) => {
                        let p = &app.projects[*proj_idx];
                        let meta = app.meta_cache.get(&p.repo);
                        let active = app.active_project_idx == Some(*proj_idx);

                        let lang = meta
                            .and_then(|m| m.language.as_deref())
                            .map(lang_short)
                            .unwrap_or("");
                        let pushed = meta
                            .and_then(|m| m.pushed_at.as_deref())
                            .map(relative_date)
                            .unwrap_or_default();

                        let repo_style = if active {
                            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default()
                        };
                        let marker = if active { "• " } else { "  " };
                        let repo_display = p.repo.split('/').last().unwrap_or(&p.repo);
                        let branch_display = if p.branch.len() > 12 {
                            format!("{}~", &p.branch[..11])
                        } else {
                            p.branch.clone()
                        };

                        // Icons are double-width in terminal; compensate padding to keep columns stable
                        let status = if p.is_dirty {
                            Span::styled(
                                format!(" {I_WARN} dirty  "),
                                Style::default().fg(C_YELLOW),
                            )
                        } else {
                            Span::styled(
                                format!(" {I_CHECK} clean  "),
                                Style::default().fg(C_GREEN),
                            )
                        };
                        // Both states must be same cell width: icon(2) + space(1) + 2chars + padding = 5 cells
                        let ahead = if p.commits_ahead > 0 {
                            Span::styled(
                                format!("{I_AHEAD} {:<2} ", p.commits_ahead),
                                Style::default().fg(C_PURPLE),
                            )
                        } else {
                            Span::raw("     ")
                        };
                        let cfg_color = if p.setup_ok == p.setup_total {
                            C_GREEN
                        } else if p.setup_ok == 0 {
                            C_RED
                        } else {
                            C_YELLOW
                        };
                        // Marker: both states = 3 cells (NF icon is 2 cells + 1 space, or 3 plain spaces)
                        let marker = if active {
                            format!("{I_BULLET} ")
                        } else {
                            "   ".to_string()
                        };

                        ListItem::new(Line::from(vec![
                            Span::styled(marker, Style::default().fg(ACCENT)),
                            Span::styled(format!("{:<repo_col$}", repo_display), repo_style),
                            Span::styled(format!(" {:<3} ", lang), Style::default().fg(C_PURPLE)),
                            Span::styled(
                                format!(" {I_BRANCH} {:<9}", branch_display),
                                Style::default().fg(FG_DIM),
                            ),
                            status,
                            ahead,
                            Span::styled(
                                format!("{:<4}", format!("{}/{}", p.setup_ok, p.setup_total)),
                                Style::default().fg(cfg_color),
                            ),
                            Span::styled(format!("  {:<5}", pushed), Style::default().fg(FG_XDIM)),
                        ]))
                    }
                }
            })
            .collect();

        let mut ls = app.project_list_state.clone();
        frame.render_stateful_widget(
            List::new(items)
                .highlight_style(hl())
                .highlight_symbol("> "),
            split[1],
            &mut ls,
        );
    }

    let footer_line = if let Some(msg) = &app.projects_msg {
        let mut spans = footer(&[
            ("↑↓/jk", "navigate"),
            ("enter", "open"),
            ("r", "refresh"),
            ("R", "rescan all"),
            ("q", "quit"),
        ])
        .spans;
        spans.push(Span::styled(
            format!("  {msg}"),
            Style::default().fg(FG_DIM),
        ));
        Line::from(spans)
    } else if app.projects_loading {
        let mut spans = footer(&[
            ("↑↓/jk", "navigate"),
            ("enter", "open"),
            ("r", "refresh"),
            ("R", "rescan all"),
            ("q", "quit"),
        ])
        .spans;
        spans.push(Span::styled(
            "  scanning projects...",
            Style::default().fg(FG_DIM),
        ));
        Line::from(spans)
    } else {
        footer(&[
            ("↑↓/jk", "navigate"),
            ("enter", "open"),
            ("r", "refresh"),
            ("R", "rescan all"),
            ("q", "quit"),
        ])
    };
    frame.render_widget(Paragraph::new(footer_line), outer[2]);
}

fn handle_memories(app: &mut App, key: KeyCode) -> bool {
    // Name-input mode (creating a new memory file)
    if app.memory_inputting {
        match key {
            KeyCode::Esc => {
                app.memory_inputting = false;
                app.memory_input.clear();
            }
            KeyCode::Backspace => {
                app.memory_input.pop();
            }
            KeyCode::Char(c) => {
                app.memory_input.push(c);
            }
            KeyCode::Enter => {
                let raw = app.memory_input.trim().to_string();
                if !raw.is_empty() {
                    let filename = if raw.ends_with(".md") {
                        raw.clone()
                    } else {
                        format!("{raw}.md")
                    };
                    let dir = app.memory_dir();
                    let _ = fs::create_dir_all(&dir);
                    let path = dir.join(&filename);
                    let title = raw.trim_end_matches(".md");
                    match fs::write(&path, format!("# {title}\n\n")) {
                        Ok(_) => {
                            let _ = append_to_memory_index(&dir, &filename);
                            app.memory_inputting = false;
                            app.memory_input.clear();
                            app.reload_memories();
                            app.pending_editor = Some(path);
                        }
                        Err(e) => {
                            app.memory_message = Some(format!("Error: {e}"));
                            app.memory_inputting = false;
                            app.memory_input.clear();
                        }
                    }
                }
            }
            _ => {}
        }
        return false;
    }

    match key {
        KeyCode::Char('p') => {
            app.switch_memories_view(MemoriesView::Project);
        }
        KeyCode::Char('g') => {
            app.switch_memories_view(MemoriesView::Global);
        }
        KeyCode::Char('c') => {
            app.switch_memories_view(MemoriesView::Claude);
        }
        KeyCode::Down | KeyCode::Char('j') if !app.memory_files.is_empty() => {
            let n = (app.memory_list_state.selected().unwrap_or(0) + 1) % app.memory_files.len();
            app.memory_list_state.select(Some(n));
        }
        KeyCode::Up | KeyCode::Char('k') if !app.memory_files.is_empty() => {
            let len = app.memory_files.len();
            let n = app
                .memory_list_state
                .selected()
                .map(|i| if i == 0 { len - 1 } else { i - 1 })
                .unwrap_or(0);
            app.memory_list_state.select(Some(n));
        }
        KeyCode::Enter | KeyCode::Char('e') => {
            if let Some(idx) = app.memory_list_state.selected() {
                if let Some(f) = app.memory_files.get(idx) {
                    app.pending_editor = Some(f.path.clone());
                }
            }
        }
        KeyCode::Char('n') => {
            app.memory_inputting = true;
            app.memory_input.clear();
            app.memory_message = None;
        }
        KeyCode::Char('d') => {
            if let Some(idx) = app.memory_list_state.selected() {
                if let Some(f) = app.memory_files.get(idx) {
                    let path = f.path.clone();
                    let name = f.name.clone();
                    match fs::remove_file(&path) {
                        Ok(_) => {
                            app.memory_message = Some(format!("Deleted {name}.md"));
                            app.reload_memories();
                        }
                        Err(e) => {
                            app.memory_message = Some(format!("Error: {e}"));
                        }
                    }
                }
            }
        }
        KeyCode::Char('m') if app.memories_view == MemoriesView::Claude => {
            if let Some(idx) = app.memory_list_state.selected() {
                if let Some(f) = app.memory_files.get(idx) {
                    if let Some(proj) = app.active_project_idx.and_then(|i| app.projects.get(i)) {
                        let dst_dir = proj.path.join(".memory");
                        let _ = fs::create_dir_all(&dst_dir);
                        let dst = dst_dir.join(format!("{}.md", f.name));
                        let src = f.path.clone();
                        match fs::copy(&src, &dst) {
                            Ok(_) => {
                                let _ = append_to_memory_index(&dst_dir, &format!("{}.md", f.name));
                                app.memory_message =
                                    Some(format!("Migrated {} → .memory/", f.name));
                                app.switch_memories_view(MemoriesView::Project);
                            }
                            Err(e) => {
                                app.memory_message = Some(format!("Error: {e}"));
                            }
                        }
                    }
                }
            }
        }
        KeyCode::Char('r') => {
            app.reload_memories();
            app.memory_message = None;
        }
        _ => {}
    }
    false
}

fn handle_setup(app: &mut App, key: KeyCode) -> bool {
    match key {
        KeyCode::Down | KeyCode::Char('j') if !app.setup_items.is_empty() => {
            let n = (app.setup_list_state.selected().unwrap_or(0) + 1) % app.setup_items.len();
            app.setup_list_state.select(Some(n));
        }
        KeyCode::Up | KeyCode::Char('k') if !app.setup_items.is_empty() => {
            let len = app.setup_items.len();
            let n = app
                .setup_list_state
                .selected()
                .map(|i| if i == 0 { len - 1 } else { i - 1 })
                .unwrap_or(0);
            app.setup_list_state.select(Some(n));
        }
        KeyCode::Enter => {
            if let Some(sel) = app.setup_list_state.selected() {
                if let Some(idx) = app.active_project_idx {
                    if let Some(p) = app.projects.get(idx) {
                        let path = p.path.clone();
                        let item = app.setup_items[sel].clone();
                        if item.status != SetupStatus::Ok {
                            app.setup_message = Some(
                                apply_setup_item(&path, &item)
                                    .unwrap_or_else(|e| format!("Error: {e}")),
                            );
                            app.refresh_setup();
                        }
                    }
                }
            }
        }
        KeyCode::Char('a') => {
            if let Some(idx) = app.active_project_idx {
                if let Some(p) = app.projects.get(idx) {
                    let path = p.path.clone();
                    app.setup_message =
                        Some(apply_all_setup(&path).unwrap_or_else(|e| format!("Error: {e}")));
                    app.refresh_setup();
                }
            }
        }
        KeyCode::Char('r') => {
            app.refresh_setup();
        }
        _ => {}
    }
    false
}

fn draw_setup(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    frame.render_widget(Paragraph::new(header_row(app)), outer[0]);
    frame.render_widget(Paragraph::new(nav_row(app)), outer[1]);

    if app.active_project_idx.is_none() {
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  No project selected.",
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  Go to [3 projects] and press enter to set the active project.",
                    Style::default().fg(Color::DarkGray),
                )),
            ])
            .block(Block::default().borders(Borders::ALL).title(" setup ")),
            outer[2],
        );
    } else if app.setup_items.is_empty() {
        frame.render_widget(
            Paragraph::new(Span::styled(
                "  Scanning…",
                Style::default().fg(Color::DarkGray),
            ))
            .block(Block::default().borders(Borders::ALL).title(" setup ")),
            outer[2],
        );
    } else {
        let inner = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(if app.setup_message.is_some() { 3 } else { 0 }),
            ])
            .split(outer[2]);

        let items: Vec<ListItem> = app
            .setup_items
            .iter()
            .map(|item| {
                let (icon, icon_style) = match item.status {
                    SetupStatus::Ok => (I_CHECK, Style::default().fg(C_GREEN)),
                    SetupStatus::Missing => (I_CROSS, Style::default().fg(C_RED)),
                    SetupStatus::Stale => (I_WARN, Style::default().fg(C_YELLOW)),
                };
                let action_hint = match item.status {
                    SetupStatus::Ok => "",
                    SetupStatus::Missing => "  → press enter to apply",
                    SetupStatus::Stale => "  → press enter to remove",
                };
                ListItem::new(Line::from(vec![
                    Span::styled(format!(" {icon}  "), icon_style),
                    Span::styled(format!("{:<30}", item.label), Style::default()),
                    Span::styled(item.detail, Style::default().fg(FG_DIM)),
                    Span::styled(action_hint, Style::default().fg(FG_XDIM)),
                ]))
            })
            .collect();

        let mut ls = app.setup_list_state.clone();
        frame.render_stateful_widget(
            List::new(items)
                .block(Block::default().borders(Borders::ALL).title(" setup "))
                .highlight_style(hl())
                .highlight_symbol("> "),
            inner[0],
            &mut ls,
        );

        if let Some(msg) = &app.setup_message {
            frame.render_widget(
                Paragraph::new(Span::styled(
                    format!("  {I_CHECK} {msg}"),
                    Style::default().fg(C_GREEN),
                ))
                .block(Block::default().borders(Borders::ALL)),
                inner[1],
            );
        }
    }

    frame.render_widget(
        Paragraph::new(footer(&[
            ("↑↓/jk", "navigate"),
            ("enter", "apply item"),
            ("a", "apply all"),
            ("r", "rescan"),
            ("esc", "back"),
            ("tab", "switch"),
            ("q", "quit"),
        ])),
        outer[3],
    );
}

fn draw_memories(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let input_h = if app.memory_inputting || app.memory_message.is_some() {
        3u16
    } else {
        0
    };
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(input_h),
            Constraint::Length(1),
        ])
        .split(area);

    frame.render_widget(Paragraph::new(header_row(app)), outer[0]);
    frame.render_widget(Paragraph::new(nav_row(app)), outer[1]);

    // Main area: list (left) + preview (right)
    let main = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(outer[2]);

    // Left: subnav + file list
    let left_split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(main[0]);

    let p_active = app.memories_view == MemoriesView::Project;
    let g_active = app.memories_view == MemoriesView::Global;
    let c_active = app.memories_view == MemoriesView::Claude;
    let subnav = Line::from(vec![
        Span::styled(
            " p project ",
            if p_active {
                Style::default()
                    .fg(SEL_FG)
                    .bg(ACCENT)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(FG_DIM)
            },
        ),
        Span::raw(" "),
        Span::styled(
            " g global ",
            if g_active {
                Style::default()
                    .fg(SEL_FG)
                    .bg(ACCENT)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(FG_DIM)
            },
        ),
        Span::raw(" "),
        Span::styled(
            " c claude ",
            if c_active {
                Style::default()
                    .fg(SEL_FG)
                    .bg(ACCENT)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(FG_DIM)
            },
        ),
    ]);
    frame.render_widget(Paragraph::new(subnav), left_split[0]);

    let dir_label = match app.memories_view {
        MemoriesView::Project => " .memory/ ",
        MemoriesView::Global => " ~/.pemguin/memory/ ",
        MemoriesView::Claude => " .claude/…/memory/ ",
    };
    let items: Vec<ListItem> = if app.memory_files.is_empty() {
        vec![ListItem::new(Span::styled(
            "  (empty)",
            Style::default().fg(FG_XDIM),
        ))]
    } else {
        app.memory_files
            .iter()
            .map(|f| ListItem::new(f.name.clone()))
            .collect()
    };
    let mut ls = app.memory_list_state.clone();
    frame.render_stateful_widget(
        List::new(items)
            .block(Block::default().borders(Borders::ALL).title(dir_label))
            .highlight_style(hl())
            .highlight_symbol("> "),
        left_split[1],
        &mut ls,
    );

    // Right: preview
    let preview = app
        .memory_list_state
        .selected()
        .and_then(|i| app.memory_files.get(i))
        .map(|f| f.content.as_str())
        .unwrap_or("");
    frame.render_widget(
        Paragraph::new(preview)
            .block(Block::default().borders(Borders::ALL).title(" preview "))
            .wrap(Wrap { trim: false }),
        main[1],
    );

    // Input or message
    if app.memory_inputting {
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("> ", Style::default().fg(ACCENT)),
                Span::raw(app.memory_input.clone()),
                Span::styled("█", Style::default().fg(ACCENT)),
            ]))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" new memory name "),
            ),
            outer[3],
        );
    } else if let Some(msg) = &app.memory_message {
        let (icon, color) = if msg.starts_with("Error") {
            (I_CROSS, C_RED)
        } else {
            (I_CHECK, C_GREEN)
        };
        frame.render_widget(
            Paragraph::new(Span::styled(
                format!("  {icon}  {msg}"),
                Style::default().fg(color),
            ))
            .block(Block::default().borders(Borders::ALL)),
            outer[3],
        );
    }

    let footer_hints = if app.memory_inputting {
        footer(&[("enter", "create + open"), ("esc", "cancel")])
    } else if c_active {
        footer(&[
            ("↑↓/jk", "navigate"),
            ("e/enter", "edit"),
            ("m", "migrate → .memory/"),
            ("n", "new"),
            ("d", "delete"),
            ("r", "reload"),
            ("esc", "back"),
            ("p/g/c", "view"),
        ])
    } else {
        footer(&[
            ("↑↓/jk", "navigate"),
            ("e/enter", "edit"),
            ("n", "new"),
            ("d", "delete"),
            ("r", "reload"),
            ("esc", "back"),
            ("p/g/c", "view"),
            ("tab", "switch"),
        ])
    };
    frame.render_widget(Paragraph::new(footer_hints), outer[4]);
}

fn draw_skills(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    frame.render_widget(Paragraph::new(header_row(app)), outer[0]);
    frame.render_widget(Paragraph::new(nav_row(app)), outer[1]);

    let main = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(outer[2]);

    let items: Vec<ListItem> = app
        .skills
        .iter()
        .map(|s| ListItem::new(s.name.clone()))
        .collect();

    if items.is_empty() {
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  no skills installed",
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  npx skills add <owner/repo> --skill <name> -y",
                    Style::default().fg(FG_XDIM),
                )),
            ])
            .block(Block::default().borders(Borders::ALL).title(" skills ")),
            outer[2],
        );
    } else {
        let mut ls = app.skills_list_state.clone();
        frame.render_stateful_widget(
            List::new(items)
                .block(Block::default().borders(Borders::ALL).title(" skills "))
                .highlight_style(hl())
                .highlight_symbol("> "),
            main[0],
            &mut ls,
        );

        let preview = app
            .skills_list_state
            .selected()
            .and_then(|i| app.skills.get(i))
            .map(|s| {
                let mut lines = vec![
                    Line::from(Span::styled(
                        s.name.clone(),
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    )),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("source  ", Style::default().fg(FG_DIM)),
                        Span::raw(s.source.clone()),
                    ]),
                ];
                if !s.description.is_empty() {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::raw(s.description.clone())));
                }
                lines
            })
            .unwrap_or_default();

        frame.render_widget(
            Paragraph::new(preview)
                .block(Block::default().borders(Borders::ALL).title(" detail "))
                .wrap(Wrap { trim: false }),
            main[1],
        );
    }

    frame.render_widget(
        Paragraph::new(footer(&[
            ("↑↓/jk", "navigate"),
            ("esc", "back"),
            ("tab", "switch"),
            ("q", "quit"),
        ])),
        outer[3],
    );
}

fn draw_mcp(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    frame.render_widget(Paragraph::new(header_row(app)), outer[0]);
    frame.render_widget(Paragraph::new(nav_row(app)), outer[1]);

    let main = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(outer[2]);

    let items: Vec<ListItem> = app
        .mcp_servers
        .iter()
        .map(|s| ListItem::new(s.name.clone()))
        .collect();

    if items.is_empty() {
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  no .mcp.json found",
                    Style::default().fg(Color::DarkGray),
                )),
            ])
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" mcp servers "),
            ),
            outer[2],
        );
    } else {
        let mut ls = app.mcp_list_state.clone();
        frame.render_stateful_widget(
            List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" mcp servers "),
                )
                .highlight_style(hl())
                .highlight_symbol("> "),
            main[0],
            &mut ls,
        );

        let preview = app
            .mcp_list_state
            .selected()
            .and_then(|i| app.mcp_servers.get(i))
            .map(|s| {
                let cmd = if s.args.is_empty() {
                    s.command.clone()
                } else {
                    format!("{} {}", s.command, s.args.join(" "))
                };
                vec![
                    Line::from(Span::styled(
                        s.name.clone(),
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    )),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("command  ", Style::default().fg(FG_DIM)),
                        Span::raw(cmd),
                    ]),
                ]
            })
            .unwrap_or_default();

        frame.render_widget(
            Paragraph::new(preview)
                .block(Block::default().borders(Borders::ALL).title(" detail "))
                .wrap(Wrap { trim: false }),
            main[1],
        );
    }

    frame.render_widget(
        Paragraph::new(footer(&[
            ("↑↓/jk", "navigate"),
            ("esc", "back"),
            ("tab", "switch"),
            ("q", "quit"),
        ])),
        outer[3],
    );
}

fn draw_pane(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    frame.render_widget(Paragraph::new(header_row(app)), outer[0]);
    frame.render_widget(Paragraph::new(nav_row(app)), outer[1]);

    let inner = Block::default().borders(Borders::ALL).title(" pane ");
    let content_area = inner.inner(outer[2]);
    frame.render_widget(inner, outer[2]);

    let h = content_area.height as usize;
    let w = content_area.width as usize;

    // Center the placeholder art vertically
    let art: &[&str] = &[
        r"  _  _  _  _ ",
        r" |            ",
        r" |_   _ |\|  /",
        r" |   / \| |\/",
        r" |  |___| |  \",
        r"              ",
        r" child pane   ",
        r"              ",
        r" yazi · helix · … ",
    ];
    let art_h = art.len();
    let pad_top = h.saturating_sub(art_h) / 2;

    let mut lines: Vec<Line> = (0..pad_top).map(|_| Line::from("")).collect();
    for row in art {
        let pad_left = w.saturating_sub(row.len()) / 2;
        lines.push(Line::from(Span::styled(
            format!("{:pad_left$}{row}", ""),
            Style::default().fg(FG_DIM),
        )));
    }

    frame.render_widget(Paragraph::new(lines), content_area);
    frame.render_widget(
        Paragraph::new(footer(&[("esc", "back"), ("tab", "switch"), ("q", "quit")])),
        outer[3],
    );
}

fn handle_skills(app: &mut App, key: KeyCode) -> bool {
    let len = app.skills.len();
    if len == 0 {
        return false;
    }
    match key {
        KeyCode::Down | KeyCode::Char('j') => {
            let n = (app.skills_list_state.selected().unwrap_or(0) + 1) % len;
            app.skills_list_state.select(Some(n));
        }
        KeyCode::Up | KeyCode::Char('k') => {
            let n = app
                .skills_list_state
                .selected()
                .map(|i| if i == 0 { len - 1 } else { i - 1 })
                .unwrap_or(0);
            app.skills_list_state.select(Some(n));
        }
        _ => {}
    }
    false
}

fn handle_mcp(app: &mut App, key: KeyCode) -> bool {
    let len = app.mcp_servers.len();
    if len == 0 {
        return false;
    }
    match key {
        KeyCode::Down | KeyCode::Char('j') => {
            let n = (app.mcp_list_state.selected().unwrap_or(0) + 1) % len;
            app.mcp_list_state.select(Some(n));
        }
        KeyCode::Up | KeyCode::Char('k') => {
            let n = app
                .mcp_list_state
                .selected()
                .map(|i| if i == 0 { len - 1 } else { i - 1 })
                .unwrap_or(0);
            app.mcp_list_state.select(Some(n));
        }
        _ => {}
    }
    false
}

fn draw_fill(
    frame: &mut Frame,
    app: &App,
    prompt_idx: usize,
    field_idx: usize,
    values: &HashMap<String, String>,
    input: &str,
) {
    let area = frame.area();
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    let prompt = &app.prompts[prompt_idx];
    let auto = app.auto_values();
    let fillable: Vec<&String> = prompt
        .placeholders
        .iter()
        .filter(|p| !auto.contains_key(*p))
        .collect();

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                " 🐧 pm ",
                Style::default()
                    .fg(SEL_FG)
                    .bg(ACCENT)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(&prompt.name, Style::default().fg(Color::White)),
        ])),
        outer[0],
    );

    let mut lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Fill in placeholders",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];
    for placeholder in &prompt.placeholders {
        if let Some(v) = auto.get(placeholder) {
            lines.push(Line::from(vec![
                Span::raw(format!("  {placeholder:<14}")),
                Span::styled(v.clone(), Style::default().fg(FG_DIM)),
                Span::styled("  (auto)", Style::default().fg(FG_XDIM)),
            ]));
            continue;
        }
        let fi = fillable.iter().position(|p| *p == placeholder).unwrap_or(0);
        if fi < field_idx {
            let val = values.get(placeholder).map(|s| s.as_str()).unwrap_or("");
            lines.push(Line::from(vec![
                Span::raw(format!("  {placeholder:<14}")),
                Span::styled(val.to_string(), Style::default().fg(C_GREEN)),
                Span::styled(format!("  {I_CHECK}"), Style::default().fg(C_GREEN)),
            ]));
        } else if fi == field_idx {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {placeholder:<14}"),
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ),
                Span::styled(format!("{input}█"), Style::default().fg(Color::White)),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::raw(format!("  {placeholder:<14}")),
                Span::styled("...", Style::default().fg(FG_XDIM)),
            ]));
        }
    }
    frame.render_widget(
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL)),
        outer[1],
    );
    frame.render_widget(
        Paragraph::new(footer(&[("enter", "confirm"), ("esc", "back")])),
        outer[2],
    );
}

fn draw_done(frame: &mut Frame, text: &str) {
    let area = frame.area();
    let mut lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(format!("  {I_CHECK}  "), Style::default().fg(C_GREEN)),
            Span::styled(
                "Copied to clipboard",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  ─────────────────────────────",
            Style::default().fg(FG_XDIM),
        )),
        Line::from(""),
    ];
    for line in text.lines().take(20) {
        lines.push(Line::from(Span::styled(
            format!("  {line}"),
            Style::default().fg(Color::DarkGray),
        )));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  any key to continue",
        Style::default().fg(Color::DarkGray),
    )));
    frame.render_widget(
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(" pemguin ")),
        area,
    );
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() -> io::Result<()> {
    let config = load_config();
    let mut app = App::new(config);
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let result = run(&mut terminal, &mut app);
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> io::Result<()> {
    loop {
        app.process_async_results();
        terminal.draw(|f| draw(f, app))?;

        // Open editor if requested (suspends TUI, resumes after)
        if let Some(path) = app.pending_editor.take() {
            let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
            disable_raw_mode()?;
            execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
            let _ = Command::new(&editor).arg(&path).status();
            enable_raw_mode()?;
            execute!(terminal.backend_mut(), EnterAlternateScreen)?;
            terminal.clear()?;
            app.reload_memories();
            continue;
        }

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if handle_key(app, key.code, key.modifiers) {
                    break;
                }
            }
        }
    }
    Ok(())
}
