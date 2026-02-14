pub mod activity_routes;
pub mod auth_routes;
pub mod chat_routes;
pub mod strava_routes;
pub mod training_routes;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .service(
                web::scope("/auth")
                    .route("/register/start", web::post().to(auth_routes::register_start))
                    .route("/register/finish", web::post().to(auth_routes::register_finish))
                    .route("/login/start", web::post().to(auth_routes::login_start))
                    .route("/login/finish", web::post().to(auth_routes::login_finish))
                    .route("/logout", web::post().to(auth_routes::logout))
                    .route("/me", web::get().to(auth_routes::me))
                    .route("/users", web::get().to(auth_routes::list_all_users))
                    .route("/mas", web::get().to(auth_routes::get_mas))
                    .route("/mas", web::patch().to(auth_routes::update_mas))
                    .route("/profile", web::get().to(auth_routes::profile))
                    .route("/ai-cost-summary", web::get().to(auth_routes::ai_cost_summary)),
            )
            .service(
                web::scope("/strava")
                    .route("/link", web::get().to(strava_routes::link))
                    .route("/callback", web::get().to(strava_routes::callback))
                    .route("/status", web::get().to(strava_routes::status)),
            )
            .service(
                web::scope("/activities")
                    .route("/sync", web::post().to(activity_routes::sync_activities))
                    .route("", web::get().to(activity_routes::list_activities))
                    .route("/{id}", web::get().to(activity_routes::get_activity))
                    .route("/{id}/tag", web::patch().to(activity_routes::update_tag))
                    .route("/{id}/intervals", web::get().to(activity_routes::get_intervals))
                    .route("/{id}/trainings", web::get().to(training_routes::get_activity_trainings)),
            )
            .service(
                web::scope("/trainings")
                    .route("", web::post().to(training_routes::create_training))
                    .route("", web::get().to(training_routes::list_trainings))
                    .route("/{id}", web::get().to(training_routes::get_training))
                    .route("/{id}", web::patch().to(training_routes::update_training))
                    .route("/{id}", web::delete().to(training_routes::delete_training))
                    .route("/{id}/activities", web::get().to(training_routes::get_training_activities))
                    .route("/{id}/activities/{activity_id}", web::post().to(training_routes::add_activity_to_training))
                    .route("/{id}/activities/{activity_id}", web::delete().to(training_routes::remove_activity_from_training))
                    .route("/{id}/insight", web::post().to(training_routes::training_insight))
                    .route("/{id}/insights", web::get().to(training_routes::list_training_insights)),
            )
            .service(
                web::scope("/chats")
                    .route("", web::post().to(chat_routes::create_chat))
                    .route("", web::get().to(chat_routes::list_chats))
                    .route("/models", web::get().to(chat_routes::list_models))
                    .route("/from-insight/{insight_id}", web::post().to(chat_routes::create_from_insight))
                    .route("/{id}", web::get().to(chat_routes::get_chat))
                    .route("/{id}", web::patch().to(chat_routes::update_chat))
                    .route("/{id}", web::delete().to(chat_routes::delete_chat))
                    .route("/{id}/messages", web::post().to(chat_routes::send_message)),
            ),
    );
}
