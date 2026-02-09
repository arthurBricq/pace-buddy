CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY NOT NULL,
    username TEXT UNIQUE NOT NULL,
    display_name TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS passkeys (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL REFERENCES users(id),
    passkey_json TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS strava_tokens (
    user_id TEXT PRIMARY KEY NOT NULL REFERENCES users(id),
    strava_athlete_id INTEGER NOT NULL,
    access_token TEXT NOT NULL,
    refresh_token TEXT NOT NULL,
    expires_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS activities (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id),
    strava_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    sport_type TEXT NOT NULL,
    start_date TEXT NOT NULL,
    elapsed_time INTEGER NOT NULL,
    moving_time INTEGER NOT NULL,
    distance REAL NOT NULL,
    total_elevation_gain REAL NOT NULL,
    average_speed REAL NOT NULL,
    max_speed REAL NOT NULL,
    average_heartrate REAL,
    max_heartrate REAL,
    average_cadence REAL,
    average_watts REAL,
    calories REAL,
    tag TEXT NOT NULL DEFAULT 'normal',
    summary_polyline TEXT,
    workout_type INTEGER,
    streams_loaded INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    UNIQUE(user_id, strava_id)
);

CREATE INDEX IF NOT EXISTS idx_activities_user_date ON activities(user_id, start_date DESC);

CREATE TABLE IF NOT EXISTS activity_streams (
    activity_id TEXT NOT NULL REFERENCES activities(id),
    stream_type TEXT NOT NULL,
    data_json TEXT NOT NULL,
    PRIMARY KEY (activity_id, stream_type)
);

CREATE TABLE IF NOT EXISTS trainings (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    description TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_trainings_user ON trainings(user_id);

CREATE TABLE IF NOT EXISTS training_activities (
    training_id TEXT NOT NULL REFERENCES trainings(id) ON DELETE CASCADE,
    activity_id TEXT NOT NULL REFERENCES activities(id) ON DELETE CASCADE,
    PRIMARY KEY (training_id, activity_id)
);

CREATE INDEX IF NOT EXISTS idx_training_activities_training ON training_activities(training_id);
CREATE INDEX IF NOT EXISTS idx_training_activities_activity ON training_activities(activity_id);
