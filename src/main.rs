mod chat;
mod config;
mod services;
mod ui;

use std::{collections::HashMap, env, thread, time::Duration};
use services::Services;

fn configure(cfg: &mut HashMap<String, String>, backend_base: &str) {
    ui::info_panel("Configure LLM", &["Settings are saved locally and pushed to the shared backend model DB."]);
    println!();
    let provider = ui::ask("Provider (openai/ollama/anthropic/google)", config::get(cfg, "LLM_PROVIDER"));
    cfg.insert("LLM_PROVIDER".into(), provider);
    let endpoint = ui::ask("Endpoint URL", config::get(cfg, "LLM_ENDPOINT"));
    cfg.insert("LLM_ENDPOINT".into(), endpoint);
    let model = ui::ask("Model", config::get(cfg, "LLM_MODEL"));
    cfg.insert("LLM_MODEL".into(), model);
    let key_default = if config::get(cfg, "LLM_API_KEY").is_empty() {
        ""
    } else {
        "configured"
    };
    let key = ui::ask("API key", key_default);
    if key != "configured" && !key.is_empty() {
        cfg.insert("LLM_API_KEY".into(), key);
    }
    match config::save(cfg) {
        Ok(()) => ui::ok(&format!("Local config saved to {}", config::env_path().display())),
        Err(e) => ui::err(&format!("Could not save config: {e}")),
    }
    if !backend_base.is_empty() {
        match chat::push_to_backend(cfg, backend_base) {
            Ok(msg) => ui::ok(&msg),
            Err(e) => ui::warn(&format!("Could not sync model to backend: {e}")),
        }
    }
}

fn main() {
    let mut cfg = config::load();
    let _ = std::fs::create_dir_all(config::home().join("logs"));

    let user = env::var("USER")
        .or_else(|_| env::var("USERNAME"))
        .unwrap_or_else(|_| "Developer".into());
    let cwd = env::current_dir()
        .map(|p| {
            let s = p.display().to_string();
            if let Ok(home) = env::var("HOME").or_else(|_| env::var("USERPROFILE")) {
                if s.starts_with(&home) {
                    return format!("~{}", &s[home.len()..]);
                }
            }
            s
        })
        .unwrap_or_else(|_| ".".into());

    let mut services = Services::new();
    let backend_started = match services.start_backend() {
        Ok(()) => true,
        Err(e) => {
            ui::warn(&format!("Backend could not be launched: {e}"));
            false
        }
    };

    if backend_started {
        thread::sleep(Duration::from_secs(2));
        match chat::sync_from_backend(&mut cfg, &services.backend_url) {
            Ok(()) => {}
            Err(_) => {
                let _ = chat::push_to_backend(&cfg, &services.backend_url);
            }
        }
    }

    let llm_ok = !config::get(&cfg, "LLM_MODEL").is_empty() && !config::get(&cfg, "LLM_ENDPOINT").is_empty();
    let oauth_ok = !config::get(&cfg, "GOOGLE_CLIENT_ID").is_empty()
        || !config::get(&cfg, "MICROSOFT_CLIENT_ID").is_empty();
    let api_ok = backend_started && !services.backend_url.is_empty();

    ui::header(
        "1.4",
        &user,
        config::get(&cfg, "LLM_PROVIDER"),
        config::get(&cfg, "LLM_MODEL"),
        &cwd,
        llm_ok,
        oauth_ok,
        api_ok,
    );

    if config::get(&cfg, "LLM_ENDPOINT").is_empty() {
        ui::warn("No LLM endpoint yet — run /llm to configure.");
        println!();
    } else if api_ok {
        ui::dim("  Shared model DB active (CLI + UI use ~/.husk/data/husk.db)");
        println!();
    }

    let mut history: Vec<(String, String)> = Vec::new();
    let mut conversation_id: Option<String> = None;

    loop {
        let Some(line) = ui::prompt_line() else {
            break;
        };
        if line.is_empty() {
            continue;
        }
        if line == "/exit" || line == "/quit" || line == "exit" || line == "quit" {
            break;
        }
        if line == "/help" {
            ui::help_table();
        } else if line == "/llm" {
            configure(&mut cfg, &services.backend_url);
        } else if line == "/sync" {
            match chat::sync_from_backend(&mut cfg, &services.backend_url) {
                Ok(()) => ui::ok(&format!(
                    "Pulled default model: {} | {}",
                    config::get(&cfg, "LLM_PROVIDER"),
                    config::get(&cfg, "LLM_MODEL")
                )),
                Err(e) => ui::err(&format!("Sync failed: {e}")),
            }
        } else if line == "/ui" {
            match services.start_ui() {
                Ok(()) => {
                    ui::ok("Web UI started");
                    if !services.frontend_url.is_empty() {
                        ui::info_panel(
                            "Husk is running",
                            &[
                                &format!("Web UI  {}", services.frontend_url),
                                &format!("API     {}", services.backend_url),
                                &format!("Docs    {}/docs", services.backend_url),
                            ],
                        );
                    }
                }
                Err(e) => ui::err(&e),
            }
        } else if line == "/cli" {
            services.stop_ui();
            ui::ok("Web UI stopped. Continuing in the terminal.");
        } else if line == "/new" {
            history.clear();
            conversation_id = None;
            ui::ok("Started a new conversation");
        } else if line == "/status" {
            ui::status_table(
                config::get(&cfg, "LLM_PROVIDER"),
                config::get(&cfg, "LLM_ENDPOINT"),
                config::get(&cfg, "LLM_MODEL"),
                if services.backend_url.is_empty() {
                    "stopped"
                } else {
                    &services.backend_url
                },
                if services.frontend_url.is_empty() {
                    "stopped"
                } else {
                    &services.frontend_url
                },
                conversation_id.as_deref().unwrap_or("(new)"),
            );
        } else if line.starts_with('/') {
            ui::err(&format!("Unknown command {}. Try /help", line));
        } else {
            print!("{}thinking...{}\r", ui::DIM, ui::RESET);
            let _ = std::io::Write::flush(&mut std::io::stdout());
            match chat::send(
                &cfg,
                &mut history,
                &line,
                &services.backend_url,
                &mut conversation_id,
            ) {
                Ok(reply) => {
                    print!("\x1b[2K\r"); // clear thinking line
                    println!();
                    // Indent assistant reply slightly for readability
                    for (i, line) in reply.lines().enumerate() {
                        if i == 0 {
                            println!("  {}{}{}", ui::ACCENT, line, ui::RESET);
                        } else {
                            println!("  {line}");
                        }
                    }
                    println!();
                }
                Err(e) => {
                    print!("\x1b[2K\r");
                    ui::err(&format!("Chat error: {e}"));
                }
            }
        }
        println!();
    }

    services.stop_all();
    ui::goodbye();
}
