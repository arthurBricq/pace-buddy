# TODO-CODEX

Purpose: incremental delivery of coach database-query capability, one milestone at a time.

## Status Legend
- [ ] Not started
- [~] In progress
- [x] Done

## Locked Decisions
- Scope order: incremental milestones, not big-bang.
- Milestone 1 output: Markdown text only.
- Data policy: DB-first, Strava fallback when detail is missing.
- Canonical key for later tools: internal `activity_id` UUID.
- Ambiguity policy for later tools: ask user to choose.

## Milestone 1 — Session Text Description Engine (first)
Goal: produce precise Markdown descriptions for a single activity, with mode-specific detail.

### Scope
- [x] Create one canonical helper/service that builds activity description text.
- [x] Support modes: `auto`, `intervals`, `race`, `long_run`, `normal`.
- [x] `auto` mode selects from activity tag/workout type + available data.
- [x] Include core sections: identity, main metrics, mode-specific analysis.
- [~] Reuse existing interval resolution and current formatting logic where possible.
- [x] Keep output as Markdown text only (no tool-calling yet).

### Data policy
- [x] DB-first reads for activity/streams/laps/interval cache.
- [x] Strava fallback only when needed for missing detail.

### Acceptance criteria
- [ ] Interval-tagged run includes reps/recoveries and pace details.
- [ ] Race-tagged run includes race-focused pacing summary.
- [ ] Long-run tagged run includes endurance-focused pacing summary.
- [ ] Normal run includes compact precise summary.
- [ ] Output is deterministic enough for LLM prompting.

### Tests
- [x] Unit test: mode selection (`auto`).
- [x] Unit test: required sections present per mode.
- [x] Unit test: missing optional metrics handled safely.

### Implementation Notes (Milestone 1)
- [x] Add helper module: `backend/bin/src/helpers/activity_description.rs`.
- [x] Export module in `backend/bin/src/helpers/mod.rs`.
- [x] Add `ActivityDescriptionMode` enum with parser (`auto|intervals|race|long_run|normal`).
- [x] Add one entry function:
      `build_activity_description(state: &AppState, user_id: Uuid, activity_id: Uuid, mode: ActivityDescriptionMode) -> Result<String, DomainError>`
- [x] Internally:
      fetch activity from storage,
      load streams/laps from DB,
      fallback to Strava if needed,
      call `state.resolve_intervals(...)` only when interval detail is required.
- [ ] Reuse formatting fragments from:
      `backend/bin/src/helpers/context_builder.rs`,
      `backend/bin/src/helpers/insight_builder.rs`.
- [x] Keep section order stable:
      `# Session`,
      `## Identity`,
      `## Core Metrics`,
      `## Mode Analysis`,
      `## Notes/Data Quality`.
- [x] Do not expose this via new API route in M1 unless needed for manual validation.
- [x] Add unit tests in the new module for mode dispatch and section presence.

## Milestone 2 — OpenRouter Tool-Calling Plumbing
- [ ] Add request fields: `tools`, `tool_choice`, `parallel_tool_calls`.
- [ ] Parse assistant `tool_calls` and `finish_reason=tool_calls`.
- [ ] Add tool-capable completion interface in `llm` crate.

## Milestone 3 — Coach Tool Loop + DB Query Tools
- [ ] Add coach loop: call model -> execute tools -> call model until final answer.
- [ ] Add `search_sessions` tool (returns candidate sessions + canonical UUID).
- [ ] Add `get_session_detail(activity_id, detail_mode)` tool (uses Milestone 1 output).
- [ ] Ambiguity policy: ask user to choose if multiple candidates.

## Milestone 4 — Hardening
- [ ] Fallback behavior when tool-calling unsupported.
- [ ] Iteration/token guardrails.
- [ ] End-to-end tests and logging.

## Current focus
- [~] Milestone 1 — Session Text Description Engine
