use std::{collections::HashMap, env, fs, path::PathBuf};

pub fn home() -> PathBuf {
    if let Some(p) = env::var_os("HUSK_HOME") { return PathBuf::from(p); }
    env::var_os("USERPROFILE").or_else(|| env::var_os("HOME"))
        .map(PathBuf::from).unwrap_or_else(|| PathBuf::from("." )).join(".husk")
}

pub fn env_path() -> PathBuf { home().join(".env") }

pub fn load() -> HashMap<String, String> {
    let mut c = HashMap::from([
        ("LLM_PROVIDER".into(), "Ollama".into()),
        ("LLM_ENDPOINT".into(), "http://127.0.0.1:11434/v1".into()),
        ("LLM_MODEL".into(), "qwen3".into()),
        ("LLM_API_KEY".into(), String::new()),
    ]);
    if let Ok(s) = fs::read_to_string(env_path()) {
        for line in s.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') { continue; }
            if let Some((k, v)) = line.split_once('=') {
                let key = k.trim().to_string();
                let value = v.trim().trim_matches('"').trim_matches('\'').to_string();
                c.insert(if key == "DEFAULT_LLM_KEY" { "LLM_API_KEY".into() } else { key }, value);
            }
        }
    }
    c
}

pub fn get<'a>(c: &'a HashMap<String, String>, key: &str) -> &'a str {
    c.get(key).map(String::as_str).unwrap_or("")
}

pub fn save(c: &HashMap<String, String>) -> Result<(), String> {
    let h = home();
    fs::create_dir_all(h.join("data")).map_err(|e| e.to_string())?;
    fs::create_dir_all(h.join("logs")).map_err(|e| e.to_string())?;
    let mut lines = Vec::new();
    for key in ["LLM_PROVIDER", "LLM_ENDPOINT", "LLM_MODEL", "GOOGLE_CLIENT_ID", "GOOGLE_CLIENT_SECRET", "MICROSOFT_CLIENT_ID", "MICROSOFT_CLIENT_SECRET", "MICROSOFT_TENANT"] {
        if let Some(v) = c.get(key) { lines.push(format!("{}=\"{}\"", key, v.replace('\\', "\\\\").replace('"', "\\\""))); }
    }
    if let Some(v) = c.get("LLM_API_KEY") { lines.push(format!("LLM_API_KEY=\"{}\"", v.replace('\\', "\\\\").replace('"', "\\\""))); }
    lines.push(format!("DATABASE_URL=\"sqlite:///{}\"", h.join("data").join("husk.db").display()));
    lines.push("PORT=8000".into());
    fs::write(env_path(), lines.join("\n") + "\n").map_err(|e| e.to_string())?;
    #[cfg(unix)] { use std::os::unix::fs::PermissionsExt; let _ = fs::set_permissions(env_path(), fs::Permissions::from_mode(0o600)); }
    Ok(())
}
