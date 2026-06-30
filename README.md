# Pace Buddy

Pace Buddy is a Strava-connected training analysis app for self-coached runners. It combines activity sync, interval
workout parsing, training-block reviews, and a persistent Running Coach that can use stored training context when
answering.

The project is in active development. It is useful as a personal training tool today, but it still needs a policy,
security, and product-readiness pass before it should be operated as a public service.

## What It Does

- Authenticates users with Strava OAuth only.
- Syncs Strava activities, laps, and streams into local SQLite storage.
- Lets runners tag activities as `normal`, `intervals`, `long_run`, or `race`.
- Parses interval-tagged workouts into work/recovery repetitions.
- Groups activities into training blocks and generates persisted AI insights.
- Provides a persistent Running Coach with automatic context, tool access, and coach memory.
- Estimates MAS (Maximum Aerobic Speed) from race-tagged activities, with user overrides.
- Tracks LLM usage cost and quota in the backend.
- Provides admin views for users, quota, invite codes, and coach-context inspection.

The old standalone generic AI chat experience has been removed. New conversational work should build on the Running
Coach.

## Repository Layout

- `backend/`: Rust workspace for the API server, domain model, storage, Strava client, LLM client, coach memory, and
  interval parsing.
- `frontend/`: React + TypeScript + Vite app.
- `doc/`: design notes and data-contract documentation.
- `.github/workflows/`: CI and manual Fly deployment workflows.

Core backend crates:

- `backend/bin`: Actix Web server, routes, static frontend serving, background tasks.
- `backend/storage`: SQLite persistence layer.
- `backend/intervals`: interval parsing algorithms.
- `backend/coach-memory`: Running Coach context, memory, classifier, and normalizer.
- `backend/llm`: OpenRouter client abstraction.
- `backend/strava-client`: Strava API client and DTO conversions.
- `backend/domain`: shared domain models and errors.

## Documentation

- [AI coach data inputs](doc/ai-coach-data-inputs.md): what the Running Coach can see automatically and through tools.
- [Interval model](doc/interval-model.md): design notes for interval parsing and workout modeling.
- [Coach suggested sessions plan](doc/coach-suggested-sessions-plan.md): planning notes for coach-driven session
  suggestions.
- [Compliance notes](COMPLIANCE.md): public-readiness checklist for Strava data, AI usage, retention, and deletion.
- [Roadmap](ROADMAP.md): near-term known gaps and planned product work.
- [Security policy](SECURITY.md): how to handle vulnerabilities, secrets, and private athlete data.

## Local Development

### Prerequisites

- Rust toolchain, stable channel.
- Node.js 22+ and npm.
- A Strava API application for local development.
- Optional: an OpenRouter API key for AI features.

Copy the example environment file and fill in local values:

```bash
cp .env.example .env
```

The backend reads configuration from environment variables. It also supports local files named
`backend/strava_client_id`, `backend/strava_client_secret`, and `backend/openrouter_key`; these files are ignored by
Git.

### Backend

From `backend/`:

```bash
cargo run
```

Useful local variables:

- `DATABASE_URL=sqlite:data.db?mode=rwc`
- `BASE_URL=http://localhost:5173`
- `STRAVA_REDIRECT_URI=http://localhost:5173/api/strava/callback`
- `FRONTEND_URL=http://localhost:5173`
- `ADMIN_STRAVA_ID=<your-strava-athlete-id>`

`OPENROUTER_API_KEY` is optional. When it is missing, LLM-powered features are disabled.

### Frontend

From `frontend/`:

```bash
npm install
npm run dev
```

The Vite dev server proxies `/api` to `http://localhost:8080`.

## Checks

Backend:

```bash
cd backend
cargo test
cargo clippy --all-features --all-targets --workspace
```

Frontend:

```bash
cd frontend
npm test
npm run lint
```

## Configuration

Main backend environment variables:

- `DATABASE_URL` (default: `sqlite:data.db?mode=rwc`)
- `JWT_SECRET` (required outside local development)
- `STRAVA_CLIENT_ID`
- `STRAVA_CLIENT_SECRET`
- `STRAVA_REDIRECT_URI`
- `FRONTEND_URL`
- `BASE_URL`
- `OPENROUTER_API_KEY`
- `STRAVA_WEBHOOK_VERIFY_TOKEN`
- `ADMIN_STRAVA_ID`
- `QUOTA_MARKUP_RATIO`
- `HOST`
- `PORT`

Use `.env.example` as the source of truth for local variable names and expected formats.

## Deployment

The repository includes a Dockerfile and a manual Fly.io deployment workflow.

Production secrets should be set through the deployment platform, not committed to Git:

```bash
fly secrets set JWT_SECRET=...
fly secrets set STRAVA_CLIENT_ID=...
fly secrets set STRAVA_CLIENT_SECRET=...
fly secrets set STRAVA_WEBHOOK_VERIFY_TOKEN=...
fly secrets set OPENROUTER_API_KEY=...
fly secrets set ADMIN_STRAVA_ID=...
```

The Fly workflow is manual (`workflow_dispatch`) and uses `FLY_API_TOKEN` from GitHub Actions secrets.

## Public Release Status

Before making this repository public or inviting external users, complete these items:

- Rotate the Strava webhook verification token that was previously committed in `fly.toml`.
- Review `COMPLIANCE.md` against the current Strava API Agreement and API Policy.
- Decide on a repository license and add a `LICENSE` file if external use or contributions should be allowed.
- Remove local scratch files from the working tree (`fix.sh`, one-off SQL files, local databases, recovered backups).
- Confirm CI is green on a clean checkout.

## License

No open-source license has been selected yet. Until a license is added, default copyright restrictions apply.
