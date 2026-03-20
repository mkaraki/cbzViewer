use actix_files as afs;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};

mod config;
mod img;
mod list;
mod pathutils;
mod read;
mod thumb;

async fn legal_handler() -> impl Responder {
    match std::fs::read_to_string("dist/legal.txt") {
        Ok(content) => HttpResponse::Ok()
            .content_type("text/plain; charset=utf-8")
            .body(content),
        Err(e) => {
            log::error!("Failed to read legal.txt: {}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

async fn frontend_handler(req: HttpRequest) -> impl Responder {
    let path = req.uri().path();

    if path == "/favicon.ico" || path == "/robots.txt" {
        return HttpResponse::NotFound().finish();
    }

    let html = match std::fs::read_to_string("dist/index.html") {
        Ok(h) => h,
        Err(e) => {
            log::error!("Failed to read dist/index.html: {}", e);
            return HttpResponse::InternalServerError()
                .body("Couldn't prepare frontend HTML.");
        }
    };

    let host = req
        .headers()
        .get("host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("")
        .to_string();

    let sentry_dsn = std::env::var("SENTRY_DSN").unwrap_or_default();

    // Replace Go template placeholders with runtime values.
    let html = html
        .replace("{{ .SentryBaggage }}", "")
        .replace("{{ .SentryTrace }}", "")
        .replace("{{ .SentryDsn }}", &sentry_dsn)
        .replace("{{ .ServerHost }}", &host);

    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let cfg = config::load_config().expect("Failed to load config.json");
    let cfg = web::Data::new(cfg);

    log::info!("Starting cbzViewer on :8080");

    HttpServer::new(move || {
        App::new()
            .app_data(cfg.clone())
            .route("/api/list", web::get().to(list::list_handler))
            .route("/api/read", web::get().to(read::read_handler))
            .route("/api/img", web::get().to(img::img_handler))
            .route("/api/thumb", web::get().to(thumb::thumb_handler))
            .route("/api/thumb_dir", web::get().to(thumb::dir_thumb_handler))
            .route("/legal", web::get().to(legal_handler))
            .service(afs::Files::new("/assets", "dist/assets/").prefer_utf8(true))
            .default_service(web::get().to(frontend_handler))
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}
