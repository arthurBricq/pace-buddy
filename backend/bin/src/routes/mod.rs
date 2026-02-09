pub mod activity_routes;
pub mod auth_routes;
pub mod strava_routes;

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
                    .route("/users", web::get().to(auth_routes::list_all_users)),
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
                    .route("/{id}/intervals", web::get().to(activity_routes::get_intervals)),
            ),
    );
}
