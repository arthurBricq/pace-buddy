# Pace Buddy

Pace Buddy is a Strava-connected web app for self-coached runners.

It combines:

- deep interval parsing for workout analysis,
- training-block summaries and AI insights,
- contextual AI chats,
- and a persistent "Running Coach" chat with memory.

This README describes the current implemented state of the product.

## Product Scope (Current)

The current app provides these user-facing areas:

- Authentication with Strava OAuth (no local passwords).
- First-login onboarding to build runner profile context.
- Activity list + detail pages with tagging and interval parsing.
- Training blocks (date-range based) with AI insight generation.
- Generic AI chats with manual context injection.
- A persistent Running Coach chat with automatic context + memory.
- MAS (Maximum Aerobic Speed) estimation from race-tagged activities.
- Quota/cost system (backend-enforced; admin-managed approvals).
- Admin pages for users/quota/invites/coach context inspection.

## High-Level Architecture

- `backend/`: Rust workspace (Actix Web + SQLite + domain crates).
- `frontend/`: React + TypeScript + Vite.
- Data source: Strava API (activities, streams, laps, webhook events).
- Storage: SQLite (schema created at startup; no migration pipeline).

Core backend crates:

- `backend/bin`: HTTP server, routes, background tasks.
- `backend/storage`: SQLite persistence layer.
- `backend/intervals`: interval parsing algorithms.
- `backend/coach-memory`: coach context/memory/classifier/normalizer.
- `backend/llm`: OpenRouter client abstraction.
- `backend/strava-client`: Strava API client + conversions.
- `backend/domain`: domain models and errors.

## Main User Flows

### 1) Login and Account Creation

- Users start at `/login` and authenticate through Strava.
- Invite codes are supported and enforced for new accounts.
- A session cookie is issued by backend JWT auth.

### 2) Onboarding (Required for New Users)

- Two-step onboarding is enforced via auth guard:
  - General presentation (identity profile)
  - Sport presentation (athlete profile / goals)
- All fields are optional at product level, with validation on numeric/date formats.

### 3) Activities and Interval Analysis

- Activities are synced from Strava (initial sync + manual resync + webhook updates).
- Users can tag sessions: `normal`, `intervals`, `long_run`, `race`.
- Activity detail includes:
  - stats,
  - map polyline,
  - streams/laps chart,
  - interval recap when tagged as `intervals`.
- Interval algorithm can be switched per activity:
  - `speed_based`
  - `manual_laps`
- Parsed interval results are cached in DB.

### 4) Trainings and AI Insights

- A training is a date range with metadata (name, description, race distance/objective).
- Training detail derives quality sessions in that date window and exposes AI insight generation:
  - Critical overview
  - Interval suggestions
- Insights are persisted and can be turned into AI chats.

### 5) AI Chats (Manual Context)

- Users can create standalone chats or chats from insights.
- Context can be injected manually from a context panel:
  - last activities,
  - last long runs,
  - last race efforts,
  - last N days summary,
  - this week vs last week,
  - this month vs last month,
  - runner profile presentation,
  - activity detail,
  - weekly stats range,
  - training recap.

### 6) Running Coach (Persistent Single Coach)

- Dedicated `/coach` page with one persistent conversation stream.
- Coach settings include:
  - model selection,
  - personality,
  - context-window parameters (volume weeks, counts, etc.),
  - memory normalization cadence.
- Coach context is rebuilt each exchange from:
  - current profile,
  - recent activities and quality sessions,
  - new activities since last exchange,
  - compact coach memory snapshot.
- Memory update pipeline:
  - classifier extracts meaningful items,
  - normalizer periodically compacts memory.

## Interval Parsing Algorithm

The core technical differentiator remains the interval parser for interval-tagged sessions.

Pipeline:

1. Preprocess streams (smoothing + pause detection)
2. Segment work/recovery (k-means split + hysteresis + cleanup)
3. Build reps and recovery links
4. Compute intensity metrics (including %MAS when available)
5. Score interval quality

A manual-lap parser is also available for lap-driven workouts.

## MAS (Maximum Aerobic Speed)

- MAS is estimated from race-tagged activities using the latest eligible race.
- Users can:
  - recompute automatically,
  - manually override MAS.
- MAS is used in interval recap outputs and coach context when available.

## Cost and Quota Model

- LLM calls are billed in backend using provider-reported usage cost.
- A configurable markup (`QUOTA_MARKUP_RATIO`) is applied for user quota accounting.
- Model cost tiers (`economical`, `standard`, `expensive`) are periodically recomputed and shown in UI selectors.
- Admin can approve/reject quota requests and issue invite codes.

## Strava Integration Details

- OAuth login + account link/unlink.
- Background sync and explicit user-triggered resync.
- Webhook support for activity create/update/delete and deauthorization handling.
- Stream/lap caching in DB with periodic purge for old cached streams.

For compliance intent and policy notes, see `COMPLIANCE.md`.

## API Surface (Implemented Scopes)

Implemented route groups:

- `/api/auth/*` (session, onboarding/profile, MAS, quota, cost summary)
- `/api/strava/*` (linking, callback, status, disconnect, webhook)
- `/api/activities/*` (sync, listing, detail, tags, intervals)
- `/api/trainings/*` (CRUD, activities, insights)
- `/api/chats/*` (CRUD, messaging, context injection, models, cost tiers)
- `/api/coach/*` (state, settings, messaging, reset)
- `/api/admin/*` (stats, users, quota requests, invite codes, coach contexts, delete-all)

## Local Development

### Prerequisites

- Rust toolchain (stable)
- Node.js + npm
- Strava API credentials
- (Optional) OpenRouter API key for AI features

### Backend

From `backend/`:

```bash
cargo run -p bin
```

Useful options:

```bash
cargo run -p bin -- --fresh-start
cargo run -p bin -- --static-serving ../frontend/dist
```

### Frontend

From `frontend/`:

```bash
npm install
npm run dev
```

The Vite dev server proxies `/api` to `http://localhost:8080`.

### Tests and Checks

Backend:

```bash
cargo test
```

Frontend:

```bash
npm run build
npm run lint
```

## Environment Configuration

Main backend environment variables:

- `DATABASE_URL` (default: `sqlite:data.db?mode=rwc`)
- `JWT_SECRET`
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

Notes:

- If `OPENROUTER_API_KEY` is missing, AI/LLM features are disabled.
- `--fresh-start` clears local SQLite files for rapid iteration.

## Current Product Limitations

- No migration framework by design (schema is initialized directly on startup).
- Frontend lint baseline is not yet clean.
- Some product surfaces overlap (Trainings vs AI Chats vs Coach) and are still being consolidated.
- Quota request UX is backend-ready and admin-ready, but user-facing profile actions are still limited.

## Deployment

- Fly deployment is configured via GitHub Actions on `main`.
- Current workflow deploys directly; test/lint gating should be strengthened.
