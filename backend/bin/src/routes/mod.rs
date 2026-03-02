mod activity_routes;
mod admin_routes;
mod auth_routes;
mod chat_routes;
mod profile_routes;
mod strava_routes;
mod training_routes;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .service(
                web::scope("/auth")
                    .route(
                        "/register/start",
                        web::post().to(auth_routes::register_start),
                    )
                    .route(
                        "/register/finish",
                        web::post().to(auth_routes::register_finish),
                    )
                    .route("/login/start", web::post().to(auth_routes::login_start))
                    .route("/login/finish", web::post().to(auth_routes::login_finish))
                    .route("/strava/start", web::post().to(auth_routes::strava_auth_start))
                    .route("/logout", web::post().to(auth_routes::logout))
                    .route("/me", web::get().to(auth_routes::me))
                    .route("/mas", web::get().to(profile_routes::get_mas))
                    .route("/mas", web::patch().to(profile_routes::update_mas))
                    .route("/profile", web::get().to(profile_routes::profile))
                    .route(
                        "/ai-cost-summary",
                        web::get().to(profile_routes::ai_cost_summary),
                    )
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
                web::scope("/admin")
                    .route("/stats", web::get().to(admin_routes::stats))
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
                    ),
            )
            .service(
                web::scope("/chats")
                    .route("", web::post().to(chat_routes::create_chat))
                    .route("", web::get().to(chat_routes::list_chats))
                    .route("/models", web::get().to(chat_routes::list_models))
                    .route(
                        "/models/cost-tiers",
                        web::get().to(chat_routes::list_model_cost_tiers),
                    )
                    .route(
                        "/from-insight/{insight_id}",
                        web::post().to(chat_routes::create_from_insight),
                    )
                    .route("/{id}", web::get().to(chat_routes::get_chat))
                    .route("/{id}", web::patch().to(chat_routes::update_chat))
                    .route("/{id}", web::delete().to(chat_routes::delete_chat))
                    .route("/{id}/messages", web::post().to(chat_routes::send_message))
                    .route("/{id}/context", web::post().to(chat_routes::add_context)),
            ),
    );
}
