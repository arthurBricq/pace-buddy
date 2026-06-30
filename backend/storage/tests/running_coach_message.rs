//! Storage tests for `running_coach_messages` lifecycle, focused on the
//! rollback path: when the coach's tool loop fails after the user message
//! has been persisted, we delete that row so subsequent turns don't load
//! a dangling user message and steer the LLM into satisfying a stale
//! request. See `coach.rs` `send_message_internal`.

use chrono::Utc;
use domain::{RunningCoachMessage, User};
use storage::{SqliteStorage, Storage};
use uuid::Uuid;

#[tokio::test]
async fn delete_running_coach_message_removes_only_the_targeted_row() {
    let path = format!("/tmp/storage_msg_test_{}.db", Uuid::new_v4().simple());
    let url = format!("sqlite:{path}?mode=rwc");
    let db = SqliteStorage::new(&url).await.expect("open storage");

    let user = User::new("msgtest".into(), "Msg Test".into(), None);
    db.create_user(&user).await.expect("create user");

    let mk = |role: &str, content: &str, offset_secs: i64| RunningCoachMessage {
        id: Uuid::new_v4(),
        user_id: user.id,
        role: role.into(),
        content: content.into(),
        prompt_tokens: 0,
        completion_tokens: 0,
        total_tokens: 0,
        cost: 0.0,
        created_at: Utc::now() + chrono::Duration::seconds(offset_secs),
    };

    let assistant = mk("assistant", "earlier reply", 0);
    let user_msg = mk("user", "doomed prompt", 1);
    let later_user = mk("user", "kept prompt", 2);

    db.store_running_coach_message(&assistant)
        .await
        .expect("store assistant");
    db.store_running_coach_message(&user_msg)
        .await
        .expect("store user");
    db.store_running_coach_message(&later_user)
        .await
        .expect("store later user");

    let before = db
        .list_running_coach_messages(user.id, 50)
        .await
        .expect("list before");
    assert_eq!(before.len(), 3);

    db.delete_running_coach_message(user_msg.id)
        .await
        .expect("delete user msg");

    let after = db
        .list_running_coach_messages(user.id, 50)
        .await
        .expect("list after");
    assert_eq!(after.len(), 2);
    assert!(after.iter().all(|m| m.id != user_msg.id));
    assert!(after.iter().any(|m| m.id == assistant.id));
    assert!(after.iter().any(|m| m.id == later_user.id));

    // Deleting a missing id is a no-op, not an error.
    db.delete_running_coach_message(Uuid::new_v4())
        .await
        .expect("delete missing");

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(format!("{path}-wal"));
    let _ = std::fs::remove_file(format!("{path}-shm"));
}
