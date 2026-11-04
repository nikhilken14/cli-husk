mod chat;
mod config;
mod services;

use std::{collections::HashMap, io::{self, Write}};
use services::Services;

fn configure(cfg: &mut HashMap<String, String>) {
    for (key, label) in [("LLM_PROVIDER", "Provider"), ("LLM_ENDPOINT", "Endpoint URL"), ("LLM_MODEL", "Model"), ("LLM_API_KEY", "API key (input is visible)")] {
        let current = config::get(cfg, key).to_string(); print!("{label} [{}]: ", if key == "LLM_API_KEY" && !current.is_empty() { "configured" } else { &current }); let _ = io::stdout().flush();
        let mut value = String::new(); if io::stdin().read_line(&mut value).is_ok() { let value = value.trim(); if !value.is_empty() { cfg.insert(key.into(), value.into()); } }
    }
    match config::save(cfg) { Ok(()) => println!("Configuration saved to {}", config::env_path().display()), Err(e) => eprintln!("Could not save config: {e}") }
}
fn help() { println!("\nCommands:\n  /help       Show commands\n  /llm        Configure LLM endpoint/model/key\n  /ui         Start frontend (backend stays warm)\n  /cli        Stop frontend only\n  /status     Show service and model status\n  /new        Clear conversation history\n  /exit       Stop services and quit\n\nType any other text to chat.\n"); }
fn main() {
    let mut cfg = config::load();
    let _ = std::fs::create_dir_all(config::home().join("logs"));
    println!("HUSK v1.3 — Local AI Assistant\nProvider: {} | Model: {}", config::get(&cfg, "LLM_PROVIDER"), config::get(&cfg, "LLM_MODEL"));
    if config::get(&cfg, "LLM_ENDPOINT").is_empty() { configure(&mut cfg); }
    let mut services = Services::new();
    match services.start_backend() {
        Ok(()) => println!("Backend launched in background: {}\nTerminal chat is ready; /ui will wait for the API if it is still starting.", services.backend_url),
        Err(e) => eprintln!("Backend could not be launched: {e}\nTerminal chat can still be used; retry with /ui after fixing the backend."),
    }
    let mut history: Vec<(String, String)> = Vec::new();
    loop {
        print!("\nhusk> "); let _ = io::stdout().flush();
        let mut line = String::new(); if io::stdin().read_line(&mut line).is_err() { break; }
        let line = line.trim(); if line.is_empty() { continue; }
        if line == "/exit" || line == "/quit" { break; }
        if line == "/help" { help(); }
        else if line == "/llm" { configure(&mut cfg); }
        else if line == "/ui" { if let Err(e) = services.start_ui() { eprintln!("{e}"); } }
        else if line == "/cli" { services.stop_ui(); }
        else if line == "/new" { history.clear(); println!("Conversation cleared."); }
        else if line == "/status" { println!("Provider: {}\nEndpoint: {}\nModel: {}\nAPI: {}\nUI: {}", config::get(&cfg,"LLM_PROVIDER"), config::get(&cfg,"LLM_ENDPOINT"), config::get(&cfg,"LLM_MODEL"), if services.backend_url.is_empty() { "stopped" } else { &services.backend_url }, if services.frontend_url.is_empty() { "stopped" } else { &services.frontend_url }); }
        else if line.starts_with('/') { println!("Unknown command. Type /help."); }
        else if let Err(e) = chat::send(&cfg, &mut history, line) { eprintln!("Chat error: {e}"); }
    }
    services.stop_all();
    println!("Husk stopped. Goodbye!");
}
