# AI Coach Data Inputs

This document specifies the data that is visible to the Running Coach LLM. It covers the automatic grounding context
built for every coach exchange and the extra activity data available through coach tool calls.

## Source Files

- Automatic context assembly: [`backend/coach-memory/src/context.rs`](../backend/coach-memory/src/context.rs)
- Coach request and tool loop: [`backend/coach-memory/src/coach.rs`](../backend/coach-memory/src/coach.rs)
- Coach tool definitions and payloads: [
  `backend/bin/src/adapters/coach_tools.rs`](../backend/bin/src/adapters/coach_tools.rs)
- Detailed activity description used by `get_session_detail`: [
  `backend/bin/src/helpers/activity_description.rs`](../backend/bin/src/helpers/activity_description.rs)
- Stored activity, lap, and stream models: [`backend/domain/src/activity.rs`](../backend/domain/src/activity.rs), [
  `backend/domain/src/lap.rs`](../backend/domain/src/lap.rs), [
  `backend/domain/src/stream.rs`](../backend/domain/src/stream.rs)

## Per-Exchange LLM Messages

Each `/api/coach/messages` request stores the user message, rebuilds context, then sends the LLM these messages:

1. Fixed coach system prompt.
2. Coach personality from `running_coach_settings.personality`.
3. The generated `# Running Coach Grounding Context`.
4. Tool-use instructions when tools are enabled.
5. The last 24 stored user/assistant coach messages, including the current user message that was stored immediately
   before context generation.

Tool calling is enabled for the coach route. The LLM may call one tool at a time for up to 4 tool-loop steps. Tool
outputs are inserted back into the message list as tool messages before the final assistant reply.

## Automatic Grounding Context

The context is generated from stored database data, not directly from Strava live calls.

### Header

- Current date.
- Athlete username.
- Current MAS in km/h, when available.
- Coach run scope:
    - `Run` only by default.
    - `Run` and `TrailRun` when `consider_trail_runs_as_runs` is enabled.

### User Profile

Included from `IdentityProfile`:

- Name.
- Age.
- Gender.

Not currently included from `IdentityProfile`:

- Email.
- Height.
- Weight.

Included from `AthleteProfile`:

- Goal description.
- Goal date.
- Goal distance.
- Goal target time.
- Additional info.

Not currently included from `AthleteProfile`:

- Goal sport type.
- Goal elevation gain.

### Volume

The coach receives volume over the last `volume_weeks` weeks, clamped to 1-24 and defaulting to 8.

Included:

- Number of run-scope activities.
- Total distance.
- Total moving time.
- Up to 10 weekly lines with run count, distance, and moving time.

This section uses `get_activities_in_range` and only includes activities considered runs by coach settings.

### Recent Quality Sessions

The coach receives compact summary lines for:

- Last interval-tagged run sessions, count from `last_workouts_count`, clamped to 1-25 and defaulting to 8.
- Last long-run-tagged run sessions, count from `last_long_runs_count`, clamped to 1-25 and defaulting to 6.
- Last races, count from `last_races_count`, clamped to 1-25 and defaulting to 4.

A race is included when it is in coach run scope and either:

- `activity.tag == race`, or
- `activity.workout_type == 1`.

Each activity summary line contains:

- Activity name.
- Activity date.
- Distance.
- Moving time.
- Average pace.
- Total elevation gain.
- Tag.

Each activity summary line does not contain:

- Internal `activity_id`.
- Strava ID.
- Sport type.
- Lap data.
- Stream data.
- Altitude/elevation profile.
- Heart rate, cadence, power, or calories.

### New Activities Since Last Exchange

The coach receives up to `new_activities_count` activities whose `start_date` is newer than
`running_coach_state.last_seen_activity_start_date`. On first use, the same section is an initial snapshot of the newest
activities. The count is clamped to 1-25 and defaults to 8.

Important details:

- This section is not filtered to runs. It uses the newest stored activities directly.
- It uses the same compact activity summary line described above.
- After the exchange, `last_seen_activity_start_date` advances to the newest activity seen in this section.

When a newly synced race appears here, the coach receives the race summary line. It gets total elevation gain, but it
does not get laps or stream-derived elevation profile data from the automatic context.

### Recent Tool Results

The context includes up to 6 compact summaries of recent tool usage from prior exchanges. These summaries identify
recent lookup actions and top matched session labels, but they do not replay full tool payloads.

### Coach Memory Snapshot

The context includes:

- Pinned facts.
- Active coaching plan.
- Episodic memory.
- Rolling summary.

Memory is updated after each exchange by a classifier LLM call. It is periodically normalized after
`normalizer_every_n_messages`, clamped to 1-20 and defaulting to 6.

## Coach Tools

### `search_sessions`

Input:

- `query`.
- Optional `limit`, clamped to 1-20 and defaulting to 5.

Behavior:

- If the query is an exact internal activity UUID and belongs to the user, it returns that one match.
- Otherwise, it searches the latest 500 activities by name, tag, sport type, and date.

Output match fields:

- `activity_id`.
- `strava_id`.
- `name`.
- `start_date`.
- `tag`.
- `sport_type`.
- `distance_km`.
- `moving_time_s`.
- `elevation_gain_m`.
- `pace`.
- `score`.

No lap data or stream data is returned.

### `get_last_sessions`

Input:

- Optional `limit`, clamped to 1-20 and defaulting to 1.
- Optional `sport_type`.
- Optional `tag`: `normal`, `intervals`, `race`, or `long_run`.

Behavior:

- Searches the latest 500 activities.
- Sorts by `start_date` descending.
- If `sport_type` is `Run`, `TrailRun` inclusion follows the coach setting.

Output:

- Same match fields as `search_sessions`.
- Includes total elevation gain as `elevation_gain_m`.
- Does not include laps or streams.

### `get_sessions_in_time_range`

Input:

- `start_date` in `YYYY-MM-DD`.
- `end_date` in `YYYY-MM-DD`.
- Optional `limit`, clamped to 1-20 and defaulting to 10.
- Optional `sport_type`.
- Optional `tag`: `normal`, `intervals`, `race`, or `long_run`.

Behavior:

- Searches the latest 500 activities in the inclusive UTC date range.
- Sorts by `start_date` descending.
- If `sport_type` is `Run`, `TrailRun` inclusion follows the coach setting.

Output:

- Same match fields as `search_sessions`.
- Includes total elevation gain as `elevation_gain_m`.
- Does not include laps or streams.

### `get_session_detail`

Input:

- `activity_id`.
- Optional `detail_mode`: `auto`, `intervals`, `race`, `long_run`, or `normal`.

Behavior:

- Loads the stored activity.
- Resolves detail mode from the explicit mode, tag, or Strava `workout_type`.
- Loads laps from storage or fetches them from Strava and caches them.
- Loads streams from storage or fetches them from Strava and caches non-GPS streams.

Requested stream keys from Strava:

- `time`.
- `distance`.
- `latlng`.
- `altitude`.
- `heartrate`.
- `cadence`.
- `watts`.
- `velocity_smooth`.
- `moving`.

Storage does not persist `latlng`. Other returned streams, including `altitude`, may be cached.

Output:

- JSON with `activity_id`, `detail_mode`, and `description_markdown`.

The markdown contains:

- Identity: internal activity ID, Strava ID, name, start date, sport, tag, selected mode.
- Core metrics: distance, moving time, elapsed time, average pace, average speed, max speed, total elevation gain,
  optional heart rate, cadence, power, and calories.
- Mode analysis.
- Notes/data quality.

Race mode analysis includes:

- Estimated first-half and second-half pace from time/distance streams when available.
- Split trend.
- Lap pacing summary from laps: lap count, average pace, fastest pace, slowest pace.

Race mode analysis does not currently include:

- Full per-lap table.
- Per-lap elevation gain.
- Altitude/elevation profile or climb distribution.

So when the coach requests one activity through `get_session_detail`, it does have access to fetched lap data
indirectly, but only the rendered summary is sent to the LLM. It also receives total elevation gain in core metrics.
Although altitude streams may be fetched and cached, the current description does not render the altitude stream or an
elevation profile.

## Direct Answers

### What data is fed to the AI coach?

The coach receives fixed instructions, personality settings, automatic grounding context, optional tool instructions,
the last 24 coach messages, and any tool outputs it requests during the exchange.

The automatic context includes profile fields, MAS, run-scope setting, volume summaries, recent interval sessions,
recent long runs, recent races, new activities since the last exchange, recent tool-result summaries, and coach memory.
Activity rows in the automatic context are compact summaries only.

### When the coach receives the update of one race, does it get the laps?

No. A newly synced race enters the automatic context as a compact activity summary. That summary does not include laps
and does not trigger a Strava lap fetch.

### When the coach receives the update of one race, does it get the elevation?

Yes, but only as total elevation gain from the stored `Activity.total_elevation_gain`. It does not get an altitude
stream or elevation profile in the automatic update.

### When the coach requests one activity via tool calls, does it get the laps?

For `search_sessions`, `get_last_sessions`, and `get_sessions_in_time_range`: no. These return match metadata only.

For `get_session_detail`: yes, the backend loads or fetches laps, but the LLM receives only the rendered markdown
summary. In race mode, that currently means lap pacing count/average/fastest/slowest, not every raw lap.

### When the coach requests one activity via tool calls, does it get the elevation?

For match tools: yes, total elevation gain as `elevation_gain_m`.

For `get_session_detail`: yes, total elevation gain in core metrics. Altitude stream data may be fetched and cached, but
the current rendered markdown does not expose an elevation profile.
