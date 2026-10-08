# distributet-systems

A master/secondary broadcast system: a Rust `master` service accepts messages and relays them to registered `secondary` replicas, backed by Redis for state, with a React UI to drive and observe it.

## Running

Run everything with Docker Compose from the repo root:

```bash
docker compose up --build
```

This starts Redis, `master` (API on `http://localhost:3000`), one `secondary` replica, and the UI (`http://localhost:5173`). Open the UI to send messages, watch delivery logs, and add/stop/start secondaries — the "Add secondary" button needs master running as a container (it uses the Docker socket to spawn new ones), so prefer Compose over running `master` with `cargo run` if you want that to work. To start with more replicas, add `--scale secondary=3` (or similar) to the command above.

For frontend-only development, run `cd ui && npm install && npm run dev` instead of building the UI's Docker image.

