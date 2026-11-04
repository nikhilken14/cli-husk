# Husk native CLI

The Rust executable is the terminal launcher. The FastAPI backend remains Python and the web UI remains the existing Node/Vite application. The crate uses only Rust's standard library; terminal chat calls the OpenAI-compatible endpoint through `curl`.

## Source layout

```text
cli-rs/
├── Cargo.toml
├── README.md
└── src/
    ├── main.rs       # REPL and commands
    ├── config.rs     # ~/.husk/.env
    ├── services.rs   # backend/frontend process lifecycle
    └── chat.rs       # curl-based LLM chat
```

## Build

Install Rust only on the machine that builds/releases Husk:

```powershell
cd cli-rs
cargo build --release
```

Windows output: `target\release\husk.exe`
Linux/macOS output: `target/release/husk`

The end user does not need Rust or Cargo. Package the resulting executable in the installer payload.

## Runtime layout

```text
.husk/
├── bin/husk[.exe]    # native CLI launcher
├── husk/             # Python backend package
├── ui/               # frontend
├── venv/             # Python environment with backend dependencies
├── .env
├── data/
└── logs/
```

`HUSK_HOME` can override `~/.husk`. At startup the CLI spawns the Python backend in the background and immediately enters the terminal REPL; it does not wait for backend health. Terminal chat is available while Uvicorn initializes. `/ui` waits for `/api/v1/health` (up to 45 seconds) and then starts only the frontend, reusing the already-running backend. `/cli` stops only the frontend and leaves the backend warm. `/exit` stops both child processes. Logs are written to `logs/backend.log` and `logs/frontend.log`.

## Runtime prerequisites

- Python environment with `fastapi` and `uvicorn` for the backend.
- Node.js 20+ and npm for `/ui`.
- `curl` on PATH for health checks and terminal chat.
- A reachable OpenAI-compatible LLM endpoint.

## Security note

This standard-library-only version stores `LLM_API_KEY` in `~/.husk/.env` so it can load the same configuration format. On Unix it sets file permissions to `0600`. On Windows, protect the `.husk` directory with the current user's filesystem permissions. For OS credential-store integration, add a platform keyring crate or keep the existing Python keyring implementation; that cannot be delivered as a zero-external-crate build.

## Current scope

The CLI supports `/help`, `/llm`, `/ui`, `/cli`, `/status`, `/new`, and `/exit`. It does not yet expose the Python CLI's OAuth wizard or agent inspection command. Those integrations remain available in the web UI/backend.
