use std::{env, fs::{self, File}, path::{Path, PathBuf}, process::{Child, Command, Stdio}, thread, time::{Duration, Instant}};
use crate::config;

pub struct Services { backend: Option<Child>, frontend: Option<Child>, pub backend_url: String, pub frontend_url: String }
impl Services {
    pub fn new() -> Self { Self { backend: None, frontend: None, backend_url: String::new(), frontend_url: String::new() } }
    fn project_dir(name: &str) -> Option<PathBuf> {
        let home = config::home();
        if home.join(name).is_dir() { return Some(home.join(name)); }
        let exe = env::current_exe().ok()?;
        for base in [exe.parent()?.to_path_buf(), exe.parent()?.parent()?.to_path_buf(), env::current_dir().ok()?] {
            if base.join(name).is_dir() { return Some(base.join(name)); }
        }
        None
    }
    fn python(backend: &Path) -> Option<String> {
        #[cfg(windows)] let candidates = vec![backend.join(".venv/Scripts/python.exe"), config::home().join("venv/Scripts/python.exe")];
        #[cfg(not(windows))] let candidates = vec![backend.join(".venv/bin/python"), config::home().join("venv/bin/python")];
        for p in candidates { if p.exists() && Command::new(&p).args(["-c", "import fastapi, uvicorn"]).stdout(Stdio::null()).stderr(Stdio::null()).status().map(|s| s.success()).unwrap_or(false) { return Some(p.to_string_lossy().into_owned()); } }
        for p in ["python", "python3"] { if Command::new(p).args(["-c", "import fastapi, uvicorn"]).stdout(Stdio::null()).stderr(Stdio::null()).status().map(|s| s.success()).unwrap_or(false) { return Some(p.into()); } }
        None
    }
    fn port_free(port: u16) -> u16 { use std::net::TcpListener; if let Ok(listener) = TcpListener::bind(("127.0.0.1", port)) { return listener.local_addr().map(|a| a.port()).unwrap_or(port); } TcpListener::bind(("127.0.0.1", 0)).and_then(|l| l.local_addr()).map(|a| a.port()).unwrap_or(port) }
    fn healthy(url: &str) -> bool { Command::new("curl").args(["-fsS", "--max-time", "2", &format!("{url}/api/v1/health")]).stdout(Stdio::null()).stderr(Stdio::null()).status().map(|s| s.success()).unwrap_or(false) }
    /// Spawn Uvicorn and return immediately. Readiness is checked only when /ui is requested.
    pub fn start_backend(&mut self) -> Result<(), String> {
        // Reuse an already-running Husk API on the standard port where possible.
        let standard_url = "http://127.0.0.1:8000";
        if Self::healthy(standard_url) {
            self.backend_url = standard_url.into();
            println!("Backend already running: {}", self.backend_url);
            return Ok(());
        }

        // Do not spawn a duplicate child if this Services instance already owns a live one.
        if let Some(child) = self.backend.as_mut() {
            if child.try_wait().ok().flatten().is_none() && !self.backend_url.is_empty() {
                return Ok(());
            }
        }

        let backend = Self::project_dir("husk").ok_or("Could not find the husk/ backend directory")?;
        let py = Self::python(&backend).ok_or("Python environment with FastAPI and Uvicorn not found. Run the Husk installer first.")?;
        let port = if Self::port_free(8000) == 8000 { 8000 } else { Self::port_free(0) };
        self.backend_url = format!("http://127.0.0.1:{port}");
        let home = config::home();
        fs::create_dir_all(home.join("logs")).map_err(|e| e.to_string())?;
        let log = File::create(home.join("logs/backend.log")).map_err(|e| e.to_string())?;
        let mut c = Command::new(py);
        c.args(["-m", "uvicorn", "assistant.main:app", "--app-dir"])
            .arg(backend.join("src"))
            .args(["--host", "127.0.0.1", "--port"])
            .arg(port.to_string())
            .current_dir(&backend)
            .env("HUSK_HOME", &home)
            .env("PYTHONUNBUFFERED", "1")
            .env("CORS_ORIGINS", "http://127.0.0.1:5173,http://localhost:5173")
            .stdout(Stdio::from(log.try_clone().map_err(|e| e.to_string())?))
            .stderr(Stdio::from(log));
        self.backend = Some(match c.spawn() {
            Ok(child) => child,
            Err(e) => {
                self.backend_url.clear();
                return Err(format!("Backend launch failed: {e}"));
            }
        });
        println!("Backend starting in background: {}", self.backend_url);
        Ok(())
    }

    /// Wait for the background backend when a feature actually needs the API.
    fn wait_backend_ready(&mut self, timeout: Duration) -> Result<(), String> {
        let until = Instant::now() + timeout;
        while Instant::now() < until {
            if Self::healthy(&self.backend_url) { return Ok(()); }
            if let Some(child) = self.backend.as_mut() {
                if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
                    return Err(format!("Backend exited before becoming healthy ({status}). Check {}/logs/backend.log", config::home().display()));
                }
            }
            thread::sleep(Duration::from_millis(400));
        }
        Err(format!("Backend is still not healthy. Check {}/logs/backend.log", config::home().display()))
    }
    pub fn start_ui(&mut self) -> Result<(), String> {
        // The backend was pre-warmed during CLI startup; wait here, not in the REPL startup path.
        if self.backend_url.is_empty() { self.start_backend()?; }
        self.wait_backend_ready(Duration::from_secs(45))?;
        if self.frontend.as_mut().and_then(|p| p.try_wait().ok().flatten()).is_none() && !self.frontend_url.is_empty() { return Ok(()); }
        let ui = Self::project_dir("ui").ok_or("Could not find the ui/ frontend directory")?;
        let node = Command::new("node").arg("--version").stdout(Stdio::null()).stderr(Stdio::null()).status().map_err(|_| "Node.js 20+ is required for /ui")?;
        if !node.success() { return Err("Node.js is required for /ui".into()); }
        let modules = ui.join("node_modules");
        if !modules.exists() { let status = npm(&ui, &["install", "--no-audit", "--no-fund"])?; if !status.success() { return Err("npm install failed".into()); } }
        let port = Self::port_free(5173); self.frontend_url = format!("http://127.0.0.1:{port}");
        let home = config::home(); let log = File::create(home.join("logs/frontend.log")).map_err(|e| e.to_string())?;
        let mut c = { let mut x = npm_command(&["run", "dev", "--", "--host", "127.0.0.1", "--strictPort"]); x.arg("--port").arg(port.to_string()); x };
        c.current_dir(ui).env("HUSK_HOME", &home).env("VITE_API_BASE_URL", format!("{}/api/v1", self.backend_url)).env("CORS_ORIGINS", &self.frontend_url).env("PUBLIC_BASE_URL", &self.backend_url).env("FRONTEND_URL", &self.frontend_url).stdout(Stdio::from(log.try_clone().map_err(|e| e.to_string())?)).stderr(Stdio::from(log));
        self.frontend = Some(c.spawn().map_err(|e| format!("Frontend launch failed: {e}"))?);
        thread::sleep(Duration::from_secs(2));
        if self.frontend.as_mut().and_then(|p| p.try_wait().ok().flatten()).is_some() { return Err(format!("Frontend failed. Check {}", home.join("logs/frontend.log").display())); }
        println!("Web UI: {}\nAPI:    {}\nDocs:   {}/docs", self.frontend_url, self.backend_url, self.backend_url);
        Ok(())
    }
    pub fn stop_ui(&mut self) { stop_child(&mut self.frontend); self.frontend_url.clear(); println!("Web UI stopped; backend remains warm."); }
    pub fn stop_all(&mut self) { self.stop_ui(); stop_child(&mut self.backend); self.backend_url.clear(); }
}
impl Drop for Services { fn drop(&mut self) { self.stop_all(); } }
#[cfg(windows)] fn stop_child(child: &mut Option<Child>) { if let Some(mut p) = child.take() { let _ = Command::new("taskkill").args(["/PID", &p.id().to_string(), "/T", "/F"]).stdout(Stdio::null()).stderr(Stdio::null()).status(); let _ = p.wait(); } }
#[cfg(not(windows))] fn stop_child(child: &mut Option<Child>) { if let Some(mut p) = child.take() { let _ = p.kill(); let _ = p.wait(); } }
#[cfg(windows)] fn npm_command(args: &[&str]) -> Command { let mut c = Command::new("cmd"); c.args(["/C", "npm"]).args(args); c }
#[cfg(not(windows))] fn npm_command(args: &[&str]) -> Command { let mut c = Command::new("npm"); c.args(args); c }
fn npm(dir: &Path, args: &[&str]) -> Result<std::process::ExitStatus, String> { npm_command(args).current_dir(dir).status().map_err(|e| format!("npm could not start: {e}")) }
