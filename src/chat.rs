use std::{collections::HashMap, process::{Command, Stdio}, io::Write};
use crate::config;

fn json_escape(s: &str) -> String {
    let mut out = String::new();
    for ch in s.chars() { match ch { '"' => out.push_str("\\\""), '\\' => out.push_str("\\\\"), '\n' => out.push_str("\\n"), '\r' => out.push_str("\\r"), '\t' => out.push_str("\\t"), c if c.is_control() => out.push(' '), c => out.push(c) } }
    out
}

fn extract_content(s: &str) -> Option<String> {
    let marker = "\"content\":";
    let start = s.find(marker)? + marker.len();
    let bytes = s.as_bytes();
    let mut i = start;
    while i < bytes.len() && bytes[i].is_ascii_whitespace() { i += 1; }
    if bytes.get(i) != Some(&b'"') { return None; }
    i += 1;
    let mut out = String::new();
    while i < bytes.len() {
        match bytes[i] {
            b'"' => return Some(out),
            b'\\' => { i += 1; if i >= bytes.len() { break; } match bytes[i] { b'n' => out.push('\n'), b'r' => out.push('\r'), b't' => out.push('\t'), b'"' => out.push('"'), b'\\' => out.push('\\'), b'/' => out.push('/'), _ => out.push(bytes[i] as char) } },
            b => out.push(b as char),
        }
        i += 1;
    }
    None
}

pub fn send(cfg: &HashMap<String, String>, history: &mut Vec<(String, String)>, message: &str) -> Result<(), String> {
    let endpoint = config::get(cfg, "LLM_ENDPOINT").trim_end_matches('/');
    let url = format!("{}/chat/completions", endpoint);
    let mut msgs = String::from("{\"role\":\"system\",\"content\":\"You are Husk, a helpful local AI assistant.\"}");
    for (role, content) in history.iter() { msgs.push_str(&format!(",{{\"role\":\"{}\",\"content\":\"{}\"}}", role, json_escape(content))); }
    msgs.push_str(&format!(",{{\"role\":\"user\",\"content\":\"{}\"}}", json_escape(message)));
    let body = format!("{{\"model\":\"{}\",\"messages\":[{}],\"stream\":false}}", json_escape(config::get(cfg, "LLM_MODEL")), msgs);
    let mut cmd = Command::new("curl");
    cmd.args(["-sS", "--fail-with-body", "--max-time", "300", "-X", "POST", &url, "-H", "Content-Type: application/json"]);
    if !config::get(cfg, "LLM_API_KEY").is_empty() { cmd.arg("-H").arg(format!("Authorization: Bearer {}", config::get(cfg, "LLM_API_KEY"))); }
    cmd.args(["--data-binary", "@-"]).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| format!("Could not start curl (install curl and ensure it is on PATH): {e}"))?;
    child.stdin.take().ok_or("Could not open curl stdin")?.write_all(body.as_bytes()).map_err(|e| e.to_string())?;
    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    if !output.status.success() { return Err(String::from_utf8_lossy(&output.stderr).to_string() + &String::from_utf8_lossy(&output.stdout)); }
    let raw = String::from_utf8_lossy(&output.stdout);
    let reply = extract_content(&raw).ok_or_else(|| format!("Unexpected LLM response: {}", raw.chars().take(500).collect::<String>()))?;
    println!("\n{reply}");
    history.push(("user".into(), message.into()));
    history.push(("assistant".into(), reply));
    Ok(())
}
