//! Terminal presentation helpers (ANSI only — no external crates).
//! Visual language mirrors husk.py (Rich): accent #2fa6f5, rounded panels, logo.

use std::io::{self, Write};

pub const ACCENT: &str = "\x1b[38;2;47;166;245m";
pub const BOLD: &str = "\x1b[1m";
pub const DIM: &str = "\x1b[38;5;245m";
pub const GREEN: &str = "\x1b[32m";
pub const RED: &str = "\x1b[31m";
pub const YELLOW: &str = "\x1b[33m";
pub const WHITE: &str = "\x1b[97m";
pub const RESET: &str = "\x1b[0m";

const LOGO: &[&str] = &[
    "██╗  ██╗██╗   ██╗███████╗██╗  ██╗",
    "██║  ██║██║   ██║██╔════╝██║ ██╔╝",
    "███████║██║   ██║███████╗█████╔╝ ",
    "██╔══██║██║   ██║╚════██║██╔═██╗ ",
    "██║  ██║╚██████╔╝███████║██║  ██╗",
    "╚═╝  ╚═╝ ╚═════╝ ╚══════╝╚═╝  ╚═╝",
];

pub fn term_width() -> usize {
    // Best-effort; default 80 when COLUMNS is unset.
    std::env::var("COLUMNS")
        .ok()
        .and_then(|s| s.parse().ok())
        .filter(|&w| w >= 40)
        .unwrap_or(80)
}

pub fn clear_screen() {
    print!("\x1b[2J\x1b[H");
    let _ = io::stdout().flush();
}

fn hline(width: usize, left: &str, mid: &str, right: &str) -> String {
    format!("{left}{}{right}", mid.repeat(width.saturating_sub(2)))
}

/// Rounded panel like Rich box.ROUNDED.
pub fn panel(title: &str, body_lines: &[String], width: usize) {
    let inner = width.saturating_sub(2);
    let title_plain = strip_ansi(title);
    let title_w = title_plain.chars().count();
    // top: ╭─ title ─╮
    let pad = inner.saturating_sub(title_w.saturating_add(2));
    let left_pad = 1;
    let right_pad = pad.saturating_sub(left_pad);
    println!(
        "{ACCENT}╭{} {} {}{}╮{RESET}",
        "─".repeat(left_pad),
        title,
        "─".repeat(right_pad),
        ""
    );
    for line in body_lines {
        let plain = strip_ansi(line);
        let len = plain.chars().count();
        let space = inner.saturating_sub(len);
        println!("{ACCENT}│{RESET}{}{}{ACCENT}│{RESET}", line, " ".repeat(space));
    }
    println!("{ACCENT}{}{RESET}", hline(width, "╰", "─", "╯"));
}

fn strip_ansi(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();
                while let Some(ch) = chars.next() {
                    if ch.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

pub fn header(version: &str, user: &str, provider: &str, model: &str, cwd: &str, llm_ok: bool, oauth_ok: bool, api_ok: bool) {
    let w = term_width().min(100).max(60);
    clear_screen();

    let mut body: Vec<String> = Vec::new();
    body.push(format!("  {BOLD}{WHITE}Welcome back, {user}!{RESET}"));
    body.push(String::new());
    for line in LOGO {
        // center logo roughly
        let pad = ((w as isize - 2 - 34) / 2).max(0) as usize;
        body.push(format!("{}{ACCENT}{line}{RESET}", " ".repeat(pad)));
    }
    body.push(format!(
        "{}{DIM}{BOLD}Local AI Assistant{RESET}",
        " ".repeat(((w as isize - 2 - 18) / 2).max(0) as usize)
    ));
    body.push(String::new());
    body.push(format!("  {DIM}{model} · {provider}{RESET}"));
    body.push(format!("  {DIM}{cwd}{RESET}"));
    body.push(String::new());
    body.push(format!("  {BOLD}{ACCENT}Tips{RESET}"));
    body.push(format!("  {DIM}Type a message to chat, or /help{RESET}"));
    body.push(format!("  {DIM}/ui starts the web UI · /llm configures the model{RESET}"));
    body.push(String::new());
    body.push(format!("  {BOLD}{ACCENT}Quick status{RESET}"));
    body.push(format!(
        "  {} LLM    {provider} ({model})",
        if llm_ok { format!("{GREEN}●{RESET}") } else { format!("{RED}●{RESET}") }
    ));
    body.push(format!(
        "  {} API    {}",
        if api_ok { format!("{GREEN}●{RESET}") } else { format!("{DIM}○{RESET}") },
        if api_ok { "Backend reachable" } else { "Backend starting / offline" }
    ));
    body.push(format!(
        "  {} OAuth  {}",
        if oauth_ok { format!("{GREEN}●{RESET}") } else { format!("{DIM}○{RESET}") },
        if oauth_ok { "Configured" } else { "Not configured" }
    ));
    body.push(format!("  {GREEN}●{RESET} Store  Local SQLite (~/.husk/data)"));

    let title = format!("{ACCENT}{BOLD} Husk v{version} {RESET}{ACCENT}");
    panel(&title, &body, w);
    println!();
    println!("{DIM}  /help for commands · Ctrl+C or /exit to quit{RESET}");
    println!();
}

pub fn rule() {
    let w = term_width();
    println!("{DIM}{}{RESET}", "─".repeat(w));
}

pub fn prompt_line() -> Option<String> {
    rule();
    // Colored ❯ like husk.py
    print!("{BOLD}{ACCENT}❯{RESET} ");
    let _ = io::stdout().flush();
    let mut line = String::new();
    if io::stdin().read_line(&mut line).is_err() {
        println!();
        return None;
    }
    rule();
    Some(line.trim().to_string())
}

pub fn ok(msg: &str) {
    println!("{GREEN}+{RESET} {msg}");
}

pub fn warn(msg: &str) {
    println!("{YELLOW}!{RESET} {msg}");
}

pub fn err(msg: &str) {
    println!("{RED}x{RESET} {msg}");
}

pub fn dim(msg: &str) {
    println!("{DIM}{msg}{RESET}");
}

pub fn info_panel(title: &str, lines: &[&str]) {
    let w = term_width().min(88).max(50);
    let body: Vec<String> = lines.iter().map(|s| format!("  {s}")).collect();
    let t = format!("{ACCENT}{BOLD} {title} {RESET}{ACCENT}");
    panel(&t, &body, w);
}

pub fn help_table() {
    let rows = [
        ("/help", "Show this help"),
        ("/chat <msg>", "Send a message (plain text works too)"),
        ("/new", "Start a fresh conversation"),
        ("/llm", "Configure LLM (synced to shared backend DB)"),
        ("/sync", "Pull default model from backend DB"),
        ("/ui", "Start web UI (backend stays warm)"),
        ("/cli", "Stop web UI only"),
        ("/status", "Show model + service status"),
        ("/exit", "Stop services and quit"),
    ];
    let w = term_width().min(72).max(50);
    let mut body = Vec::new();
    body.push(format!("  {BOLD}{ACCENT}Commands{RESET}"));
    body.push(String::new());
    for (cmd, desc) in rows {
        body.push(format!("  {BOLD}{ACCENT}{cmd:<14}{RESET} {DIM}{desc}{RESET}"));
    }
    body.push(String::new());
    body.push(format!("  {DIM}When the backend is up, chat uses the same models & DB as the web UI.{RESET}"));
    let t = format!("{ACCENT}{BOLD} Help {RESET}{ACCENT}");
    panel(&t, &body, w);
}

pub fn status_table(provider: &str, endpoint: &str, model: &str, api: &str, ui: &str, conv: &str) {
    let w = term_width().min(72).max(50);
    let body = vec![
        format!("  {DIM}Provider{RESET}     {provider}"),
        format!("  {DIM}Endpoint{RESET}     {endpoint}"),
        format!("  {DIM}Model{RESET}        {model}"),
        format!("  {DIM}API{RESET}          {api}"),
        format!("  {DIM}UI{RESET}           {ui}"),
        format!("  {DIM}Conversation{RESET} {conv}"),
    ];
    let t = format!("{ACCENT}{BOLD} Status {RESET}{ACCENT}");
    panel(&t, &body, w);
}

pub fn ask(label: &str, default: &str) -> String {
    if default.is_empty() {
        print!("{ACCENT}?{RESET} {label} {DIM}(skip){RESET}: ");
    } else {
        print!("{ACCENT}?{RESET} {label} {DIM}[{default}]{RESET}: ");
    }
    let _ = io::stdout().flush();
    let mut value = String::new();
    if io::stdin().read_line(&mut value).is_err() {
        return default.to_string();
    }
    let value = value.trim();
    if value.is_empty() {
        default.to_string()
    } else {
        value.to_string()
    }
}

pub fn goodbye() {
    println!("{DIM}Goodbye!{RESET}");
}
