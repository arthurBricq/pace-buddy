mod activity_routes;
mod admin_routes;
mod auth_routes;
mod coach_routes;
mod llm_routes;
mod profile_routes;
mod strava_routes;
mod training_routes;
mod training_session_routes;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .service(
                web::scope("/auth")
                    // Public login start endpoint. The handler lives in
                    // `strava_routes` so start/callback stay documented together.
                    .route(
                        "/strava/start",
                        web::post().to(strava_routes::strava_auth_start),
                    )
                    .route("/logout", web::post().to(auth_routes::logout))
                    .route("/me", web::get().to(auth_routes::me))
                    .route("/mas", web::get().to(profile_routes::get_mas))
                    .route("/mas", web::patch().to(profile_routes::update_mas))
                    .route(
                        "/mas/recompute",
                        web::post().to(profile_routes::recompute_mas),
                    )
                    .route(
                        "/profile/status",
                        web::get().to(profile_routes::runner_profile_status),
                    )
                    .route(
                        "/profile/identity",
                        web::get().to(profile_routes::get_identity_profile),
                    )
                    .route(
                        "/profile/identity",
                        web::put().to(profile_routes::upsert_identity_profile),
                    )
                    .route(
                        "/profile/athlete",
                        web::get().to(profile_routes::get_athlete_profile),
                    )
                    .route(
                        "/profile/athlete",
                        web::put().to(profile_routes::upsert_athlete_profile),
                    )
                    .route(
                        "/mas/estimates",
                        web::get().to(profile_routes::mas_estimates),
                    )
                    .route("/profile", web::get().to(profile_routes::profile))
                    .route("/quota", web::get().to(profile_routes::quota_status))
                    .route(
                        "/quota/request",
                        web::post().to(profile_routes::request_quota),
                    ),
            )
            .service(
                web::scope("/strava")
                    .route("/link", web::get().to(strava_routes::link))
                    .route("/callback", web::get().to(strava_routes::callback))
                    .route("/status", web::get().to(strava_routes::status))
                    .route("/disconnect", web::post().to(strava_routes::disconnect))
                    .route("/webhook", web::get().to(strava_routes::webhook_validate))
                    .route("/webhook", web::post().to(strava_routes::webhook_event)),
            )
            .service(
                web::scope("/activities")
                    .route("/sync", web::post().to(activity_routes::sync_activities))
                    .route("/sync/status", web::get().to(activity_routes::sync_status))
                    .route("", web::get().to(activity_routes::list_activities))
                    .route("/{id}", web::get().to(activity_routes::get_activity))
                    .route("/{id}/tag", web::patch().to(activity_routes::update_tag))
                    .route(
                        "/{id}/intervals",
                        web::get().to(activity_routes::get_intervals),
                    )
                    .route(
                        "/{id}/trainings",
                        web::get().to(training_routes::get_activity_trainings),
                    ),
            )
            .service(
                web::scope("/trainings")
                    .route("", web::post().to(training_routes::create_training))
                    .route("", web::get().to(training_routes::list_trainings))
                    .route("/{id}", web::get().to(training_routes::get_training))
                    .route("/{id}", web::patch().to(training_routes::update_training))
                    .route("/{id}", web::delete().to(training_routes::delete_training))
                    .route(
                        "/{id}/activities",
                        web::get().to(training_routes::get_training_activities),
                    )
                    .route(
                        "/{id}/insight",
                        web::post().to(training_routes::training_insight),
                    )
                    .route(
                        "/{id}/insights",
                        web::get().to(training_routes::list_training_insights),
                    ),
            )
            .service(
                web::scope("/training-sessions")
                    .route(
                        "",
                        web::get().to(training_session_routes::list_training_sessions),
                    )
                    .route(
                        "/{id}",
                        web::get().to(training_session_routes::get_training_session),
                    )
                    .route(
                        "/{id}/status",
                        web::patch().to(training_session_routes::update_training_session_status),
                    ),
            )
            .service(
                web::scope("/admin")
                    .route("/stats", web::get().to(admin_routes::stats))
                    .route("/users", web::get().to(admin_routes::users_by_quota_spent))
                    .route(
                        "/coach-contexts",
                        web::get().to(admin_routes::list_coach_contexts),
                    )
                    .route(
                        "/coach-contexts/{user_id}",
                        web::get().to(admin_routes::get_coach_context),
                    )
                    .route(
                        "/quota-requests",
                        web::get().to(admin_routes::list_quota_requests),
                    )
                    .route(
                        "/delete-all-data",
                        web::post().to(admin_routes::delete_all_data),
                    )
                    .route(
                        "/quota-requests/{id}/approve",
                        web::post().to(admin_routes::approve_quota_request),
                    )
                    .route(
                        "/quota-requests/{id}/reject",
                        web::post().to(admin_routes::reject_quota_request),
                    )
                    .route(
                        "/invite-codes",
                        web::post().to(admin_routes::create_invite_code),
                    )
                    .route(
                        "/invite-codes",
                        web::get().to(admin_routes::list_invite_codes),
                    )
                    .route(
                        "/invite-codes/{id}/revoke",
                        web::post().to(admin_routes::revoke_invite_code),
                    )
                    .route(
                        "/activities/{id}/dump",
                        web::get().to(admin_routes::dump_activity),
                    ),
            )
            .service(
                web::scope("/llm")
                    .route("/models", web::get().to(llm_routes::list_models))
                    .route(
                        "/models/cost-tiers",
                        web::get().to(llm_routes::list_model_cost_tiers),
                    ),
            )
            .service(
                web::scope("/coach")
                    .route("", web::get().to(coach_routes::get_coach))
                    .route(
                        "/settings",
                        web::put().to(coach_routes::update_coach_settings),
                    )
                    .route(
                        "/messages",
                        web::post().to(coach_routes::send_coach_message),
                    )
                    .route("/reset", web::post().to(coach_routes::reset_coach)),
            ),
    );
}
