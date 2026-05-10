# Coach Suggested Quality Sessions Plan

## Product Thesis

The Running Coach should not only answer "what quality session should I run next?" as prose. It should be able to
propose concrete future quality sessions, let the user accept or reject them, track whether an accepted session
happened,
compare the executed activity to the plan, and use that result in future coaching.

This feature is explicitly scoped to quality sessions. It is not a general run planner, calendar, or full training plan
generator. The coach may still advise easy runs, chill runs, long runs, rest days, and general volume changes in prose,
but those should not be persisted through this suggested-session system.

This is worth building because it turns the coach from an advice surface into a training loop:

1. The coach understands the athlete and recent training.
2. The coach proposes one or more quality-session options.
3. The runner commits to one (or more) option, potentially edits it, or rejects all options. This happens with a
   dedicated UI.
4. The app keeps track of accepted quality sessions and their execution status, in the reworked training plan.
5. The app watches future synced activities.
6. The app detects likely completion. The user can also manually mark as completed (and link with an activity).
7. The coach can later reason about planned vs executed training.

The most important design constraint: do not reintroduce generic chat flows. This should extend the Running Coach and
Training features, not create separate "AI chat about a suggestion" surfaces.

## Current State

Relevant existing pieces:

- `backend/intervals/src/types.rs` defines the executed interval representation: segments, reps, and interval result.
- `backend/intervals/src/lib.rs` and related algorithms can parse an executed activity into interval reps.
- Interval parsing is currently mostly used for activities tagged as `intervals`; this makes analysis dependent on the
  activity tag being correct.
- `backend/domain/src/training.rs` defines `Training` as a dated training block with name, description, race distance,
  race objective, and start/end dates.
- Training activities are currently derived from time window plus quality tags, mainly `intervals` and `long_run`.
  This feature narrows the coach-suggested planning workflow to workout-quality sessions, not long runs.
- The Running Coach has read-only session tools: search sessions, get latest sessions, get sessions in a date range,
  and get detailed session description.
- The coach does not currently have first-class tools to read or write trainings.
- Training insights can produce interval suggestions, but these are persisted as generated insight text, not as planned
  workouts with lifecycle state.

## Core Product Shape

The feature should introduce a first-class concept of a planned quality session.

Planned quality sessions should be top-level user objects (they do not belongs to trainings). Trainings should be
optional context, not required containers.

The coach can propose planned quality sessions. The user chooses what to accept. Accepted sessions become trackable
objects. Synced Strava activities can then be matched against accepted planned quality sessions.

The product should support three separate concepts:

- Suggested quality session: an option proposed by the coach but not accepted.
- Planned quality session: a user-accepted workout-quality session that should happen in a time window.
- Executed session: a real activity imported from Strava and optionally matched to a planned quality session.

Keeping these separate is important. The coach may suggest three options. The user may accept one, edit it, schedule it,
or ignore all of them. A real activity may partially match a plan, fully match it, or be unrelated.

Recommended model direction:

- `Training`: the block or campaign, for example "10K build May-June". Already exists, will remain untouched.
- `TrainingSession`: a quality session owned by the user, with nullable `training_id`. A coach-proposed session is
  the same row with `status = 'suggested'`; acceptance flips the status, no row copy.
- `TrainingSessionActivityMatch`: explicit link between a planned quality session and a Strava activity, with match
  confidence and user override state.

This keeps training blocks meaningful without making them mandatory. The Running Coach can suggest a quality session
even
when the user has no training block; that suggestion remains standalone until the user or app links it to a relevant
training.

Trainings are optional context for planned quality sessions.

## Planned Quality Session Model

A planned session in this feature means a planned quality session. It can be broader than formal intervals, but it
should
remain inside the workout-quality domain.

Included examples:

- Track or road intervals.
- Threshold or tempo sessions.
- Hill repeats.
- Fartlek workouts.
- Race-pace workouts.
- Progression workouts when the workout intent is explicit.
- Strides-focused sessions when they are the quality stimulus, not just a few strides after an easy run.
- Tune-up races or time trials when treated as training-quality efforts.

Excluded examples:

- Easy runs.
- Recovery or chill runs.
- Long runs.
- Rest days.
- General weekly volume targets.
- Cross-training unless it becomes a separate future product decision.

The coach can still recommend excluded session types in prose. It should just not create durable suggested-session
objects for them.

Suggested fields:

- `id`
- `user_id`
- `training_id` nullable
- `source`: `coach`, `manual`, maybe later `template`
- `status`: `suggested`, `planned`, `done`, `skipped`, `rejected` (a planned session with a `scheduled_for` date is
  still `planned`; we don't need a separate `scheduled` state)
- `title`
- `purpose`: short human-readable reason, for example "VO2max stimulus without too much fatigue"
- `session_type`: `intervals`, `tempo`, `threshold`, `hill`, `fartlek`, `progression`, `race_pace`, `time_trial`,
  `strides`, `other_quality`
- `scheduled_for` nullable date
- `earliest_start` nullable timestamp
- `latest_start` nullable timestamp
- `estimated_duration_s` nullable
- `estimated_distance_m` nullable
- `intensity_summary`: concise display text
- `prescription_json`: structured details
- `coach_message_id` nullable, linking back to the assistant message that proposed it
- `created_at`, `updated_at`

For quality sessions, `prescription_json` should be structured rather than just prose. The exact schema can evolve, but
the first useful version should support:

- Warmup: duration or distance, optional strides.
- Repeated work blocks: repeat count, work target, recovery target.
- Targets: pace range, speed range, percent MAS range, heart-rate range, RPE, or open effort.
- Cooldown: duration or distance.
- Notes: terrain, constraints, abort/modification guidance.

Easy running can appear inside a quality prescription as warmup, cooldown, or recovery. It should not be the proposed
session's primary type.

Example shape:

```json
{
  "kind": "intervals",
  "warmup": {
    "duration_s": 1200,
    "notes": "easy plus drills"
  },
  "sets": [
    {
      "repeat": 6,
      "work": {
        "duration_s": 180,
        "target": {
          "type": "pace",
          "min_s_per_km": 230,
          "max_s_per_km": 240
        }
      },
      "recovery": {
        "duration_s": 120,
        "target": {
          "type": "effort",
          "label": "easy jog"
        }
      }
    }
  ],
  "cooldown": {
    "duration_s": 600
  },
  "notes": "Keep the last two reps controlled; stop at 5 reps if form drops."
}
```

This schema should not try to perfectly encode every possible workout on day one. It only needs to be structured enough
for display, matching, and coach reasoning.

## Executed Interval Model Relationship

The existing interval result is an executed-session description. It should not be reused as the planned-session schema.

Instead, planned interval prescriptions and executed interval results should be comparable:

- Planned: "6 x 3 min at 3:50-4:00/km with 2 min jog".
- Executed: parsed reps, durations, distances, paces, recoveries, heart rate, cadence.
- Comparison: completion count, duration/distance tolerance, intensity fit, recovery fit, consistency, notes.

This implies a new comparison layer that consumes:

- a planned quality-session prescription,
- an activity,
- an interval parse result when relevant,
- basic activity metrics when interval parsing is not relevant.

## Coach Tooling

The coach needs tools in stages.

Stage 1 read tools:

- `list_planned_sessions`: expose accepted and suggested quality sessions, including standalone sessions, filtered by
  status/date/training.

Stage 2 write tools:

- `propose_sessions`: create one or more suggested sessions from structured payloads.
- `update_planned_session`: edit date/status/prescription/title.
- `mark_planned_session_status`: skipped, rejected, done.

Stage 3 (optional tools)

- `list_session_matches`: show candidate activity matches for a planned quality session.
- `link_activity_to_planned_session`: user-confirmed override from chat or UI.
- `unlink_activity_from_planned_session`: undo a wrong match.
- `explain_session_execution`: compare a plan to the linked activity and return coach-readable execution notes.
- `list_trainings`: expose active/recent training blocks. It may return an empty list.
- `get_training`: expose one training block with planned quality sessions, quality activities, and recent insights.

Important safety rule: the coach should not silently commit the runner to a plan. The initial implementation should let
the coach create suggestions, but acceptance should be a user action.

## Coach Behavior

When the user asks for a next session, the coach should usually return a normal coaching answer plus structured
suggestions only when the intended next session is a quality session.

The UI should make the structured suggestions inspectable and selectable.

The coach should avoid over-suggesting. If the user asks a general question, do not create a planned quality session. If
the runner asks for the next run and the right answer is easy mileage, a long run, or rest, the coach should answer in
prose without persisting a suggestion. Use suggestion creation only when the user asks for a future quality session, a
workout, an interval/tempo/hill session, or explicitly wants a workout-quality session scheduled.

## UI Surfaces

Minimum useful surfaces:

- Running Coach message area:
    - Assistant replies can include suggestion cards. These can either be a message that says "see suggestion" and then
      a pop-up opens, or directly cards displayed in the chat.
    - Each card has Accept and Reject. "Edit" is intentionally limited in v1 to title, notes, `scheduled_for`, and
      date window — to change the prescription itself, the user re-asks the coach. A full structured-prescription
      editor is deferred.
- Training detail page:
    - Add a "Quality Sessions" page which displays the pending sessions and the history of executed sesions.
    - Show status, scheduled date/window, session type, title, and linked activity if done.
    - Let the user manually mark skipped/done or link an activity.
- Activity detail page:
    - If an activity matches a planned quality session, show "Matched planned quality session".
    - If it is a candidate match, show "This may complete planned quality session X" with confirm/dismiss.

## Matching Engine

Automatic detection should be conservative. A wrong auto-match is worse than asking for confirmation.

Inputs:

- Planned quality session type and schedule window.
- Activity date/time.
- Sport type and coach run scope.
- Basic metrics: distance, duration, pace, elevation.
- Activity tag and Strava workout type.
- Activity name, if useful.
- Parsed interval result, when the plan or candidate activity is interval-like.

Candidate generation:

- Find activities after the planned quality session was accepted.
- Prefer activities inside the scheduled window.
- For unscheduled sessions, use a bounded window, for example next 7 days or until another accepted planned quality
  session.
- Include run-scope activities even if they are not tagged as intervals.

Scoring:

- Date proximity.
- Output of the interval parsing algorithm
- Duration/distance fit.
- Penalize clear mismatches, for example an easy run or long run matching any planned quality session.
- Session type compatibility (optionally it would be better to not use this)

Match states:

- `none`: no candidate.
- `candidate`: possible match, needs confirmation.
- `auto_matched`: high-confidence match.
- `confirmed`: user confirmed.
- `rejected`: user rejected candidate.
- `manual`: user manually linked an activity.

## Interval Parsing Implications

The current reliance on `activity.tag == intervals` is too narrow for this feature.

We will run interval-parsing on all activites, and we need a way to detect intervals failures easily.

Needed changes:

- do not display the output of the interval parsing algorithm if the confidence is low.
- improve the UI of the interval parsing display to clarify that it is an automatic computation.

Best: if the sessions has manual laps that are structurally shaped, we could use those instead. Can be left as a next
step.

## Data Flow

Suggested quality-session flow:

1. User asks the Running Coach for the next quality session or workout.
2. Coach reads relevant all relevant data to generate a suggestion(s).
3. Coach replies with advice and calls `propose_sessions` with structured options.
4. Backend persists suggestions linked to the coach message.
5. Frontend renders suggestion cards inside the coach reply.
6. User accepts, edits, rejects, or ignores suggestions. User can ask directly in chat for a more sepcific suggestion,
   meaning the coach should have the ability to discard suggestion (if the user hasn't done it)
7. Accepted suggestion becomes a planned quality session.

Completion flow:

1. Strava sync imports new activities.
2. Matching job checks open planned quality sessions against new run-scope activities.
3. Candidate matches are stored.
4. UI and coach context surface pending confirmations.
5. User confirms or overrides.
6. Confirmed match updates planned quality session status to `done`.
7. Coach context includes upcoming planned quality sessions, completed planned quality sessions, and execution notes.

## Coach Context Additions

The automatic coach context should eventually include:

- Active/recent training blocks.
- Explicit note when there is no active training block.
- Next accepted planned quality sessions.
- Pending suggestions awaiting user decision, if recent.
- Planned quality sessions needing match confirmation.
- Recently completed planned quality sessions with execution summary.
- Skipped/rejected sessions only when recent or relevant.

Tool payloads and automatic context must stay documented in `doc/ai-coach-data-inputs.md` when implemented.

## Suggested Implementation Phases

### Phase 0: Trustworthy interval parsing on all activities

The matching engine in Phase 5 is only as good as the interval parsing it consumes. Today, parsing is gated by
`activity.tag == intervals` in the frontend, which means many real workouts are never parsed and the algorithm's
score has never been calibrated against non-interval activities. Phase 0 fixes that before any planned-session work
depends on it.

Scope: **road running only** (`sport_type == "Run"`) for v1. Trail running (`sport_type == "TrailRun"`) is skipped
end-to-end — the parser is not run on it, and the matching engine in Phase 5 will not treat trail activities as
candidates for road-quality planned sessions. The fixture corpus contains a `trail` bucket so we have data on hand
when we revisit this, but it is excluded from threshold calibration.

Goals:

- Run the interval parser on every road-running activity at sync time, store the `IntervalResult` keyed by algorithm
  version (we already key by algorithm — keep that). Skip non-`Run` sport types.
- Define and document a normalized confidence in `[0, 1]` derived from `interval_score`. The current score is an
  unbounded float; pick a normalization and a display threshold by labeling a small set of real activities (clearly
  interval / clearly not / borderline) and choosing the cutoff that keeps false positives near zero.
- Decouple three concerns that are tangled today:
    - **Compute and store**: always, regardless of confidence. This is cheap insurance; storing low-confidence
      results lets us re-evaluate retroactively as the algorithm improves.
    - **Display**: only render interval results when confidence ≥ threshold, and clearly mark the result as an
      automatic computation (not as user-curated truth).
    - **Tag**: do *not* let the parser overwrite `activity.tag`. Tags are user-editable and we should not silently
      fight a user override. Auto-tagging can come later as an explicit suggestion ("this looks like an interval
      session — re-tag?"), but tag changes stay user-driven for now.
- Add an "advanced" / debug toggle in the activity UI that reveals low-confidence parses, so we keep collecting the
  examples that break the algorithm.

What we don't want:

- Non-interval activities surfaced as weird intervals like "1 × 10km @ 5:45".
- The activity list polluted by silently flipped tags.
- Throwing away low-confidence results — we lose the ability to retroactively re-evaluate when the algorithm improves.

Done when: every new activity has a stored `IntervalResult`; the frontend only renders results above the threshold
and labels them as automatic; no activity has its tag changed by the parser; we have a small labeled set used to
justify the threshold.

Optional: the AI-coach is given context. We could feed the output of the interval-parsing algorithm to the coach context
to all of the activites for which the stored interval_score is above a const threshold.

#### Phase 0 status notes (2026-05-10) [DONE]

**Score redesign (done).** The original `interval_score` weighted rep coun``t, recovery-alternation, and per-rep
speed CV. On the labeled corpus (7 intervals, 5 races, 9 runs, 5 trails) it scored races *higher* than intervals
(median 0.96 vs 0.74) — completely inverted. Replaced in `intervals/src/algorithms/mod.rs` with a v2 score that
combines four signals normalized to `[0, 1]`:

- gap between cluster_high and cluster_low speeds (35%)
- duration-weighted overall speed CV across all segments (30%)
- recovery cluster slowness — low cluster ≤ 10 km/h ideal (20%)
- rep count, saturating at 7 (15%)

Class medians on the corpus moved to: intervals 0.91, runs 0.60, races 0.41. `is_interval_workout` now requires
`score ≥ 0.55` in addition to `reps ≥ min_work_segments`.

**Remaining failure modes.** The 0.55 threshold misclassifies ~3 of 9 easy runs as intervals — those are runs
with frequent slow stretches (traffic lights, walk breaks) that look bimodal on the speed signal alone. Without
manual lap structure or activity-name parsing, this ambiguity is irreducible. One borderline interval (a hybrid
2k+5k tempo session) scores 0.52. Acceptable for v1; tune the threshold or add signals later.

**Sync wiring (shipped).** `sync_user_activities` now returns a `SyncOutcome { synced, strava_ids }`. Both call
sites (the manual `POST /activities/sync` route and the post-link initial-sync background task) call
`spawn_post_sync_interval_parsing(app, user_id, strava_ids)`. The spawned task looks each strava_id up via the
new `Storage::get_activity_by_strava_id` (canonical UUIDs — needed because `upsert_activities` preserves
existing UUIDs on conflict), filters to `sport_type == "Run"`, and calls `resolve_intervals` serially. Errors
are isolated per-activity. No semaphore: serial calls stay comfortably under Strava's 100-req/15-min quota for
realistic sync sizes.

**Frontend display (shipped).** `ActivityDetailPage` fetches intervals when `sport_type === "Run"` (was
`tag === 'intervals'`). The recap renders when `is_interval_workout` is true (which now means
`score >= INTERVAL_WORKOUT_THRESHOLD`, i.e. 0.55). When the parser flags an activity the user hasn't tagged
`intervals`, an italic "Auto-detected interval workout" label is shown above the rep table to clarify it's
not user-curated. The algorithm-selector is gated on the same condition, not on the tag.

**Backfill (intentionally none).** Old activities get parsed lazily the first time the user opens their detail
page, because the same `GET /api/activities/{id}/intervals` route now fires for any Run activity. No explicit
backfill script needed; activities the user never views stay un-parsed, which is fine.

**Threshold constant.** `INTERVAL_WORKOUT_THRESHOLD = 0.55` lives in `intervals/src/algorithms/mod.rs` with a
doc comment recording the corpus medians (intervals 0.91 / runs 0.60 / races 0.41) and re-calibration steps.

### Phase 1: Domain and Storage Foundation [DONE]

Goal: durable `TrainingSession` objects in the DB so the coach (Phase 3) has somewhere to write to.

What shipped:

- Domain types in `backend/domain/src/training_session.rs`: `TrainingSession`, `TrainingSessionActivityMatch`,
  and the four enums (`SessionSource`, `SessionStatus`, `SessionType`, `MatchStatus`) with serde / `Display` /
  `FromStr`. Round-trip tests pass.
- SQLite tables `training_sessions` (full schema) and `training_session_activity_matches` (schema only — no
  callers in Phase 1), plus indexes.
- `Storage` trait methods: `create_training_session`, `get_training_session`, `list_training_sessions(status?)`,
  `update_training_session_status`. No methods for the match table.
- Backend routes: `GET /api/training-sessions?status=`, `GET /api/training-sessions/{id}`,
  `PATCH /api/training-sessions/{id}/status`. No POST, no DELETE — coach is the only writer.
- Frontend: a placeholder page at `/training-sessions` rendering "Feature under development — coming soon",
  plus a "Sessions" navbar entry. No list, no API client, no Training-detail integration.

Intentionally **not** in Phase 1 (deferred or dropped):

- Manual create flow — the feature is coach-suggested.
- Any `Training` coupling — `training_id` is a nullable column kept for Phase 3; never populated yet.
- Test seeding mechanism — Phase 2 hand-seeds rows via `sqlite3` to exercise the new display component.
- Match-table reads/writes — Phase 4.

### Phase 2: Prescription Schema and Display

Goal: pin down a typed Rust schema for `prescription_json` and ship a frontend component that renders it,
before the coach starts producing JSON in Phase 3. 

Tasks:

- **Define the typed schema** in `backend/domain/src/prescription.rs`. Single struct `Prescription` with
  optional warmup, a `Vec<Set>` body, optional cooldown, and free-text notes. The interesting Rust enum is
  `Target`, with variants `Pace { min_s_per_km, max_s_per_km }`, `Speed { ... }`, `HeartRate { ... }`,
  `PercentMas { ... }`, `Rpe { ... }`, `Effort { label }`. Use serde's internally-tagged representation
  (`#[serde(tag = "type")]`) so JSON looks like `{"type": "pace", "min_s_per_km": 230, ...}`.
- **Don't reuse the `intervals` crate types directly.** That crate models *executed* sessions: `Rep` has
  measured `avg_pace_s_per_km`, `pace_std`, `pct_mas`. A planned `Set` has targets (ranges, distance-or-
  duration choice). The semantics are different even when units overlap; define prescription types from
  scratch and let Phase 4's comparison helpers map planned `Target` → executed `Rep` for diffing.
- **Schema parsing helper.** A small `Prescription::parse(s: &str) -> Result<Prescription, ...>` so callers
  get a proper error when storage hands them a malformed blob (forward-compatibility — old rows missing
  newly-required fields shouldn't crash the renderer; fail-soft to "raw text" display).
- **Round-trip tests.** A handful of representative JSON examples (track intervals, tempo, hill repeats,
  fartlek) deserialized → serialized → deserialized again to lock the wire format. Commit the JSON examples
  as fixtures under `backend/domain/src/prescription_fixtures/` or similar.
- **Frontend type definitions** in `frontend/src/types.ts` mirroring the Rust shape. Hand-written for now;
  generator (e.g. `ts-rs`) only if drift becomes a real problem.
- **`PrescriptionDisplay` component** in `frontend/src/components/`. Renders the structured prescription as
  human-readable workout shorthand:
    > **Warmup** — 20 min easy + 4 × 100 m strides
    > **6 × 800 m @ 3:20–3:30/km** with 2 min easy jog recovery
    > **Cooldown** — 10 min
    > _Stop at 5 reps if form breaks down._
  Fail-soft when JSON is malformed: render the raw string in a `<pre>` block with a small "couldn't parse"
  notice. Used later in Phase 3 by suggestion cards and on the real `/training-sessions` list.
- **Local test page.** Replace the placeholder content of `TrainingSessionsPage.tsx` with the real list view:
  fetch via `GET /api/training-sessions`, render each session's title + status + `<PrescriptionDisplay>`,
  per-row status actions (Mark done, Skip). Calls the existing PATCH `/status` route shipped in Phase 1.
- **Seeding for testing.** Document a 3-line `sqlite3` recipe in this doc (or a one-shot
  `cli seed-training-session --json fixtures/example.json` subcommand) so we can exercise the page without
  the coach. Lives in development workflow only; not a user-facing feature.

Done when:

- A handful of hand-crafted prescription JSON files round-trip cleanly through `Prescription::parse`.
- A row inserted via `sqlite3` shows up on `/training-sessions` rendered as readable workout shorthand by
  `PrescriptionDisplay`.
- Status actions on the page flip the row through the existing PATCH route.
- Phase 3 can pick up `Prescription` as a known-good type and focus on coach prompt + tool wiring.

### Phase 3: Coach Suggestions

Goal: turn the coach from an advice surface into a writer of structured `TrainingSession` rows the user can
accept or reject.

Tasks:

- **Define the `prescription_json` schema.** Pick a first-cut shape (warmup / sets / cooldown / targets per the
  plan example) sufficient for display and matching. Document it once, in code; iterate later. The schema is
  enforced at the `propose_sessions` tool boundary — invalid payloads from the LLM are dropped (or retried
  with a hint), not stored.
- **Coach context.** Add the user's current/next `TrainingSession` rows to the automatic context built in
  `coach-memory/src/context.rs` — counts and titles only, not full prescriptions. Explicit "no upcoming
  sessions" line when empty so the coach knows the absence is real.
- **Coach tools.**
    - `list_planned_sessions(status?)` — read-only, filters by status.
    - `propose_sessions(payload)` — validates against the prescription schema; on success, persists rows with
      `status = 'suggested'`, `source = 'coach'`, and `coach_message_id` set.
    - `update_planned_session_status(id, status)` — coach-driven status transitions (e.g. mark `superseded` /
      `rejected` on the user's behalf when the conversation makes it explicit).
- **Frontend.** Replace the placeholder at `/training-sessions` with the real list page (status filter chips,
  per-row actions: Accept, Reject, Skip, Mark done). In the Running Coach chat, render any sessions a reply
  produced as inspectable cards inline below the message — Accept flips the row to `planned` via the existing
  PATCH route, Reject flips it to `rejected`. No structured-prescription editor; "edit before accepting" is
  out of scope.
- **Prompt.** Tell the coach when to call `propose_sessions` (only when the user is asking for a quality
  session) vs. when to answer in prose (easy run, long run, rest day). Default to **one** suggestion unless
  the user explicitly asks for options. Update `doc/ai-coach-data-inputs.md` to reflect the new context and
  tool surfaces.

Done when: a user can ask the coach for a workout, see a structured suggestion card in the chat reply, accept
it, and find the resulting `planned` session on `/training-sessions`. The coach can also list existing sessions
to avoid double-proposing.

### Phase 4: Matching, Confirmation, and Comparison

Goal: connect executed Strava activities back to the planned sessions they completed, and produce
planned-vs-executed summaries the coach can reason about.

V1 scope is deliberately narrow: only mark a match as `auto_matched` when (a) the activity date falls inside
the planned session's window, (b) interval-parse confidence is high, and (c) `session_type` is compatible.
Everything else surfaces as a `candidate` requiring user confirmation. We can broaden the auto-match rule
once we have real matched/unmatched data to calibrate against.

Tasks:

- **Matching engine.** Post-sync background task that walks newly-synced activities and proposes matches
  against open `planned` sessions. Implement the narrow v1 rule above; store the score breakdown for later
  tuning.
- **Storage methods for the match table** — first real callers of `training_session_activity_matches`.
- **Comparison helpers.** Given a matched `(TrainingSession, Activity, IntervalResult)`, produce execution
  summaries like "planned 6 × 800 m at 3:25/km, executed 5 reps averaging 3:28, recovery longer than
  planned." Lives in a small new helper crate or in `intervals/` next to the existing parser.
- **Frontend.** "Needs confirmation" UI in Activity detail (when a candidate match exists) and on the session
  list (badge on the session). Manual link/unlink fallback for cases the engine misses.
- **Coach tools.** `list_session_matches`, `link_activity_to_planned_session`,
  `unlink_activity_from_planned_session`, `explain_session_execution` — each is a thin wrapper over storage
  + the comparison helpers.
- **Coach context.** Include recently-completed sessions (with execution summaries) and pending confirmations
  in the automatic context.

Done when: a synced Strava activity that completes a planned interval session gets auto-matched (or surfaces
as a candidate), the user can confirm/correct, and the coach can read "you ran 5 of 6 reps, second-rep pace
4 s slow" via its tools.

### Phase 5: Product Refinement

Goal: make the loop feel like coaching rather than bookkeeping.

Tasks:

- "Next planned session" surfaced on the Running Coach page so the user sees what's queued without leaving
  the chat.
- Coach reflection on execution after a confirmed match — natural prose follow-up the next time the user
  opens the coach.
- Lightweight dashboard indicators for pending confirmations.
- Better suggestion templates for common goals (5K, 10K, half marathon).

Done when: the coach reliably uses planned and executed quality sessions to guide future workout decisions
without the user having to remind it of state.

## Risks

- The feature can become a calendar/planning product before the quality-session coaching loop is proven.
- The coach may create too many suggestions and make the UI noisy --> by default, the suggestion should only include 1
  item, unless if the user specifically asked for me.
- Matching can be brittle if the planned schema is too precise or if activities are not tagged well.
