# Husk startup and service lifecycle

The native CLI follows this lifecycle:

1. Load `HUSK_HOME/.env` and initialize the terminal REPL.
2. Spawn the Python FastAPI/Uvicorn backend as a child process. `spawn()` returns immediately; startup does not wait for the health endpoint.
3. Accept terminal chat while the backend initializes. Terminal chat calls the configured OpenAI-compatible LLM endpoint directly through `curl`.
4. On `/ui`, wait up to 45 seconds for `GET /api/v1/health`. Once healthy, start only the Vite frontend and point it at the already-running backend.
5. On `/cli`, stop only the frontend. Keep the backend child alive and reuse it on the next `/ui`.
6. On `/exit` (or EOF), stop the frontend and backend child processes.

Backend and frontend logs are written to `HUSK_HOME/logs/backend.log` and `HUSK_HOME/logs/frontend.log`.
