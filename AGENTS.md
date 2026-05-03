I am the main developer of this application. I am a self-trained runner, and I use data science and LLMs to improve my
running performance and get better help with training decisions.

This project supports that workflow, with the goal of becoming useful for other runners too.

# Product Direction

The primary product experience is the **Running Coach**: a persistent, contextual AI coach that understands the user's
Strava history, profile, goals, training context, and prior coaching conversation.

The old standalone "AI chat" feature has been removed. Do not reintroduce generic chats, chat lists, chat-from-insight
flows, or manual "add context to chat" panels unless the user explicitly asks for that feature again. New conversational
work should build on the Running Coach.

# Features

## Running Coach

The Running Coach is the main AI feature. It should stay automatically contextualized from the user's stored Strava data,
runner profile, optional goals, recent activities, quality sessions, and coach memory.

The coach has tool access so it can query user data from the database when answering. Prefer improving this coach over
adding separate LLM conversation surfaces.

## Interval Parsing Algorithm

One of the core values of the project is the **interval parsing algorithm** in `backend/intervals/src/lib.rs`.

Every activity tagged as `intervals` can be parsed into a series of intervals, which gives high descriptive value for
workout review, training summaries, and coaching context.

## Trainings and AI Insights

Users can create training programs / training blocks that group sessions over a time window. The app derives quality
sessions from the training range and can generate AI insights for that training, such as critical overviews or interval
suggestions.

Training insights are persisted for review. They are not converted into standalone chats anymore.

# Technical Details

- Backend: Rust workspace, with the main server in `backend/bin/src/main.rs` and route wiring in
  `backend/bin/src/routes/mod.rs`.
- Frontend: React + TypeScript compiled with Vite.
- Storage: SQLite, initialized directly at startup without a migration framework.
- Data source: Strava API. Synced Strava data is cached in SQLite to avoid unnecessary API calls.
- Strava compliance is a priority. Be careful with data retention, deletion, webhook behavior, and API usage.
- Authentication and account linking use Strava OAuth only; there is no local password or passkey flow.
- LLM model listing and model cost tiers are shared infrastructure under `/api/llm/*`, not a chat feature.

# Development Mode

This project is under active development. For database changes, you can assume the local database can be dropped and
rebuilt from scratch. Do not add migration machinery unless the user explicitly asks for production migration support.
