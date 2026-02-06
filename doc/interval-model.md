Below is a **design draft** you can treat as the backbone for interval parsing + modeling. It’s written as a
“developer-facing spec” with explicit data structures, invariants, heuristics, and an incremental implementation plan.

---

# Interval Modeling & Parsing Design Draft

## 0) Goals and non-goals

### Goals

1. Given a Strava activity (summary + streams), automatically:

- detect whether it contains structured intervals
- segment it into **work reps** and **recoveries**
- infer a **workout template** when possible (e.g., `10 × 400m`, `2 × (6 × 400m)`, `12 × 30s/30s`)
- compute per-rep and per-session metrics, including **%MAS** (when MAS is available)

2. Produce outputs that support:

- plotting reps on a **speed (or %MAS) vs distance/duration** chart
- aggregations across sessions (Feature 3)

### Non-goals (v1)

- Perfect reconstruction of the coach’s intended session text in all cases
- Full physiology modeling (grade-adjusted pace, wind, temperature) — keep metadata to refine later
- Intervals defined purely by HR zones (possible later)

# 1) Inputs from Strava

Assume we can access:

- **Activity summary**: sport type, start date, distance, elapsed/moving time, elevation gain, average pace/speed, etc.
- **Streams (time series)** where available (typical keys):
    - `time` (sec)
    - `distance` (m)
    - `velocity_smooth` (m/s) or equivalent
    - `heartrate` (bpm) (optional)
    - `cadence` (optional)
    - `altitude` (optional)
    - `moving` (bool) (optional but very useful)
    - `grade_smooth` (optional)

- **Laps / splits** if the user pressed lap button (optional but high-value when present)

### Important design principle

Parsing must **not depend on MAS**. MAS is used later for **intensity interpretation** and plotting, not to find the
intervals.

---

# 2) Core data model

## 2.1 Base segmentation primitives

### `Segment`

A contiguous time interval with uniform “type”.

- `start_t`, `end_t` (sec since activity start)
- `duration_s`
- `distance_m`
- `avg_speed_mps`, `speed_std_mps`, `max_speed_mps`
- optional: `avg_hr`, `avg_cadence`, `elev_gain_m`, etc.
- `kind` ∈ `{WARMUP, WORK, RECOVERY, COOLDOWN, PAUSE, STEADY, UNKNOWN}`
- `features`: dictionary for debug + scoring (see below)

**Invariant:** Segments partition the activity timeline (except we may drop very short noise segments during cleanup).

### `Rep`

A refined view of a “WORK” segment paired with its following recovery (if any).

- `work: Segment`
- `recovery: Segment | None` (often present)
- `rep_index`, `set_index` (optional, inferred)
- `observed`:
    - `distance_m`, `duration_s`, `avg_pace_s_per_km`, `avg_speed_mps`, `pace_std`
- `intensity` (if MAS known):
    - `%MAS = avg_speed / MAS_speed`
- `quality`:
    - steadiness score, fade score, etc.

## 2.2 Workout template model

We want a template that can represent common sessions but degrades gracefully.

### `WorkoutTemplate` (top-level)

- `type` ∈ `{REPEATED_DISTANCE, REPEATED_DURATION, MIXED_GENERIC, PYRAMID, UNKNOWN}`
- `sets: list[SetBlock]` (possibly length 1)
- `confidence` ∈ [0,1]
- `explanations`: list of “why we think this” (debug-friendly)

### `SetBlock`

- `reps: list[RepSpec]`
- `intra_rep_recovery`: `RecoverySpec` (optional; can be rep-specific)
- `inter_set_recovery`: `RecoverySpec` (optional)

### `RepSpec`

Rep intention we infer from clustering:

- `target_type` ∈ `{DISTANCE, DURATION}`
- `target_value` (meters or seconds)
- `count` (M)
- `pace_or_speed_target` (optional, inferred as median observed)
- `tolerance` used for match (e.g., ±15m or ±5%)

### `RecoverySpec`

- `target_type` ∈ `{DURATION, DISTANCE, NONE}`
- `target_value`
- `style` ∈ `{JOG, WALK, STOP, UNKNOWN}` (inferred from speed/moving)

**Key point:** Even if we infer a repeated-distance template, we still store the actual `Rep` objects so the UI can show
reality (e.g., rep 7 was slower).

---

# 3) Interval “models” supported

## Model A — Repeated distance (canonical track-style)

Rep pattern:

- `N × (M × d @ intensity, r1), r2`

Where:

- `d` is target distance (meters)
- `r1` is recovery between reps in a set
- `r2` is longer recovery between sets

**Examples**

- `10 × 400m, r = 90s`
- `2 × (6 × 400m), r1 = 60s, r2 = 4min`
- `3 × 2km, r = 3min`

## Model B — Repeated duration (canonical fartlek/time reps)

- `N × (M × t @ intensity, r1), r2`

**Examples**

- `12 × (30s hard / 30s easy)`
- `5 × (3min hard / 2min easy)`

## Model C — Mixed / generic interval sequence

- `d1@p1, r1 + d2@p2, r2 + …` (distance or duration reps can mix)

**Examples**

- `1k @ 4:15, 2k @ 4:30`
- ladder: `200-400-600-800-600-400-200`

# 4) Parsing pipeline (v1 → robust)

We separate parsing into **three layers**:

1. preprocessing (clean signals)
2. segmentation (work vs recovery)
3. structure inference (sets, repetition, template classification)

## 4.1 Preprocessing

### Inputs

Time-indexed arrays (possibly uneven sampling).

### Steps

1. **Align to common time base** (usually 1s resolution is enough for running).
    - If streams are already per-second, keep them.
    - Else resample.

2. **Compute derived signals**
    - `speed(t)` in m/s (prefer smooth stream if available)
    - `pace(t)` = `1000 / speed(t)` (sec/km) for non-zero speed
    - `is_pause(t)`:
        - if `moving` exists: `moving == false`
        - else: `speed < pause_speed_threshold` for ≥ `pause_min_duration` (e.g., 2–3s)

3. **Smoothing**
    - apply rolling median (e.g., window 5s) to reduce jitter
    - optionally clamp impossible spikes

4. **Mask pauses**
    - treat pauses separately (stoplights, etc.) so they don’t corrupt the work/rest threshold discovery

### Preprocessing outputs

- cleaned `speed_smooth[t]`
- `pause_mask[t]`

## 4.2 Segmentation: detect WORK and RECOVERY regions

### Design choice (v1)

Use **distribution-based thresholding + hysteresis** (fast, explainable, tunable).  
Avoid ML/HMM initially; keep hooks so we can upgrade later.

### Step A — determine a “work threshold”

We want a threshold that separates “hard” segments from “easy” within the same activity.
Approach:

- consider only non-pause samples
- compute distribution of speed
- estimate a separating boundary using one of:
    - **K-means (k=2)** on speed values
    - or Otsu-like thresholding
    - fallback: percentile-based threshold (e.g., between 65th–75th percentile) if clustering is unstable

This yields:

- `v_low_cluster` (easy)
- `v_high_cluster` (hard)
- `v_boundary` between them

### Step B — hysteresis labeling

Use two thresholds:

- enter WORK when `speed >= v_enter`
- exit WORK when `speed <= v_exit`  
  with `v_enter > v_exit` to prevent flicker (e.g., `v_enter = v_boundary + δ`, `v_exit = v_boundary - δ`)

### Step C — cleanup constraints

After initial labeling:

- remove WORK segments shorter than `min_work_duration` (e.g., 12–15s) unless distance > `min_work_distance`
- remove RECOVERY segments shorter than `min_recovery_duration` (e.g., 8–10s) by merging neighbors
- merge adjacent WORK segments separated by tiny recovery gaps (noise)

### Output

A first list of `Segment` objects with provisional `kind`:

- WORK, RECOVERY, PAUSE, UNKNOWN

---

## 4.3 Identify warm-up / cool-down

Most sessions have:

- warm-up: steady/easy early
- cool-down: steady/easy late

Heuristic:

- find first WORK segment; everything before it is warm-up if duration >= `warmup_min` (e.g., 6–8min)
- find last WORK segment; everything after it is cool-down if duration >= `cooldown_min`
- if no WORK found: mark all as STEADY/UNKNOWN (not an interval session)

---

## 4.4 Convert segments → reps

Create a `Rep` for each WORK segment:

- `Rep.work = WORK segment`
- `Rep.recovery = next segment if RECOVERY (or PAUSE treated as recovery type STOP)`

Store:

- observed distance/duration
- stability metrics:
    - `pace_std` within rep
    - `fade`: compare first half vs second half average pace

- classify recovery style:
    - STOP if pause_mask dominates
    - WALK if avg_speed very low
    - JOG otherwise

---

# 5) Structure inference (template recognition)

We want to recognize:

- repeated distance
- repeated duration
- sets (r2)
- pyramids/ladders
- otherwise fallback to generic

## 5.1 Determine “is this an interval workout?”

Compute a simple score:

- number of WORK segments ≥ 3
- alternation pattern quality:
    - fraction of WORK segments followed by a recovery ≥ 60%
- WORK segments have relatively consistent intensity (not just random surges)

If score too low → `WorkoutTemplate.type = UNKNOWN`, still store segments/reps for plotting (can be useful).

## 5.2 Identify sets (long recoveries)

Given recovery durations after each rep:

- compute typical recovery `r_typ = median(recovery_durations)`
- detect “long rests” as:
    - `recovery_duration > max(r_typ * α, r_typ + β)` (e.g., α=2.0, β=60s)
    - or use robust stats: > Q3 + 1.5*IQR

Split reps into blocks at long rests → candidate `SetBlock`s.

## 5.3 Repeated distance detection (within each set)

For each set (or globally if no sets):

- cluster work distances (meters) with tolerance:
    - prefer relative: within ±3–5%
    - plus absolute: within ±15–25m for short reps (400m can be GPS-noisy)

- if one dominant cluster covers ≥ 70–80% of reps and count ≥ 3:
    - infer `target_distance_m = median(cluster)`
    - infer `M = reps_in_set`
    - infer `r1 = median(short recoveries)`
    - if multiple sets share same `d` and same `M` → infer N sets with r2

## 5.4 Repeated duration detection

Same method, but on work durations:

- tolerance maybe ±5–8% (short reps) or ±10s absolute
- infer `target_duration_s`

## 5.6 Generic fallback

If no clean structure:

- output `MIXED_GENERIC` with ordered rep list (each rep has observed metrics)
- still compute per-rep intensity (%MAS if available)

---

# 6) Intensity mapping (%MAS) and plotting

Once MAS is available for that date (from Feature 1 or manual):

- `MAS_speed_mps = MAS_kmh / 3.6`
- for each rep:
    - `%MAS = rep.avg_speed_mps / MAS_speed_mps`

### Plot coordinates (recommended)

Support both views:

- **distance-based plot**: x = rep.distance_m, y = %MAS (or speed)
- **duration-based plot**: x = rep.duration_s, y = %MAS

This covers both repeated distance and repeated duration workouts cleanly.

---

# 7) Quality, confidence, and explainability

## 7.1 Template confidence scoring

Compute confidence from:

- repetition purity: fraction of reps matching the inferred target cluster
- consistency of recoveries (r1 CV small)
- presence of clear set breaks (if sets inferred)
- low number of “orphan” work segments (stray surges)

`confidence ∈ [0,1]` should be stored, and the UI can show:

- “Detected: 2 × (6 × 400m), confidence 0.82”
- Provide a debug panel (dev mode) listing thresholds, cluster stats.

## 7.2 Rep quality scoring

Per rep, compute:

- stability: low pace_std
- fade: second half slower than first half
- recovery sufficiency vs typical recovery  
  This supports “execution quality” reporting.

---

# 8) Edge cases and how we handle them

1. **Stoplights in the middle of reps**

- pauses detected as `PAUSE` and treated as recovery/STOP
- if pause occurs inside a work segment:
    - split the work segment at pause; mark as low-quality rep


2. **Hills**

- v1: keep as-is but store grade/elevation metadata
- later: optionally compute grade-adjusted pace and re-run segmentation


3. **Treadmill / poor GPS**

- if distance stream is unreliable but time/speed exists, duration-based parsing still works
- if both are noisy, fallback to generic and lower confidence

4. **Progression runs / tempo runs**

- may produce 1 long WORK segment rather than repetitions → not an interval workout, still analyzable

5. **Strides (6×20s) embedded in easy run**

- v1 can detect as very short WORK segments; keep min_work_duration low enough (e.g., 12–15s) and allow small clusters.

---

# 9) Output artifacts (what gets saved)

For each analyzed activity:

- raw metadata: activity id, date, sport, etc.
- chosen MAS reference (value + source)
- list of Segments (for debugging + reprocessing)
- list of Reps (primary for UI + analytics)
- WorkoutTemplate (optional, with confidence)
- derived summaries:
    - time in intensity bands
    - rep distributions by duration/distance
    - best rep metrics

**Important:** store enough that you can re-run improved parsers later without refetching (ideally store streams or
compacted features).

---

# 10) Incremental implementation plan

### V0 (fast baseline)

- fetch streams
- smooth + pause detection
- k=2 speed clustering threshold
- segment work/recovery with hysteresis
- extract reps
- compute %MAS per rep (if MAS exists)
- no template recognition yet, just show rep list + plot

### V1 (first “structured” recognition)

- detect sets via long recoveries
- repeated distance recognition + confidence
- repeated duration recognition + confidence
- save WorkoutTemplate

### V1.5

- better warm-up/cool-down detection
- handle intra-rep pause splitting
- ladder/pyramid detection (lightweight)

### V2

- user correction UI (“these are reps”, “merge these”, “this was 8×400”)
- grade-adjusted pace option
- more advanced models if needed (HMM / change-point)

