//! Round-trip integration test for `TrainingSession` storage.
//!
//! Was added to lock in a fix for an empty-UUID parsing error that surfaced
//! when a row had `training_id = NULL` and `coach_message_id = NULL`. The
//! original `row_to_training_session` mapper used `.try_get(...).ok()` which
//! interacted poorly with sqlx's NULL handling for `Option<String>` columns
//! and caused them to come back as `Some("")` instead of `None`. The fix
//! switched to plain `.get(col)` which is the pattern used by every other
//! row mapper in the codebase.

use chrono::Utc;
use domain::{SessionStatus, SessionType, TrainingSession, User};
use storage::{SqliteStorage, Storage};
use uuid::Uuid;

#[tokio::test]
async fn round_trip_with_null_optional_fields() {
    let path = format!("/tmp/storage_test_{}.db", Uuid::new_v4().simple());
    let url = format!("sqlite:{path}?mode=rwc");
    let db = SqliteStorage::new(&url).await.expect("open storage");

    let user = User::new("teststorage".into(), "Test Storage".into(), None);
    db.create_user(&user).await.expect("create user");

    let now = Utc::now();
    let session = TrainingSession {
        id: Uuid::new_v4(),
        user_id: user.id,
        training_id: None,
        status: SessionStatus::Suggested,
        title: "round-trip test".into(),
        session_type: SessionType::Intervals,
        expiry: None,
        estimated_duration_s: None,
        estimated_distance_m: None,
        intensity_summary: None,
        prescription_json: r#"{"sets":[]}"#.into(),
        coach_message_id: None,
        created_at: now,
        updated_at: now,
    };
    let session_id = session.id;

    db.create_training_session(&session)
        .await
        .expect("insert session");

    // The bug we're guarding against: list/get used to fail with
    // "Invalid UUID ''" when training_id and coach_message_id were NULL.
    let listed = db
        .list_training_sessions(user.id, None)
        .await
        .expect("list sessions");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, session_id);
    assert!(listed[0].training_id.is_none());
    assert!(listed[0].coach_message_id.is_none());
    assert!(listed[0].expiry.is_none());

    let one = db
        .get_training_session(session_id, user.id)
        .await
        .expect("get session");
    assert_eq!(one.title, "round-trip test");

    db.update_training_session_status(session_id, user.id, SessionStatus::Planned)
        .await
        .expect("update status");
    let after = db
        .get_training_session(session_id, user.id)
        .await
        .expect("re-get");
    assert_eq!(after.status, SessionStatus::Planned);

    // Filter by status — should match exactly the row we just flipped.
    let planned = db
        .list_training_sessions(user.id, Some(SessionStatus::Planned))
        .await
        .expect("list planned");
    assert_eq!(planned.len(), 1);
    let suggested = db
        .list_training_sessions(user.id, Some(SessionStatus::Suggested))
        .await
        .expect("list suggested");
    assert!(suggested.is_empty());

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(format!("{path}-wal"));
    let _ = std::fs::remove_file(format!("{path}-shm"));
}
