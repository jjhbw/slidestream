mod generator;

use actix_files as fs;
use actix_web::{
    http::header::ContentType, middleware, web, App, HttpResponse, HttpServer, Responder,
};
use env_logger::Env;
use generator::DeepZoomGenerator;
use image::ImageOutputFormat;
use std::{collections::HashMap, path::Path};

async fn tile_endpoint(
    viewers: web::Data<HashMap<String, DeepZoomGenerator>>,
    path: web::Path<(String, u64, u64, u64)>,
) -> HttpResponse {
    let (slide, level, col, row) = path.into_inner();

    // TODO: ensure errors are presented in the frontend.
    let gen = viewers.get(&slide).expect("slide not found");
    let tile = gen.get_tile(level, col, row).unwrap();

    let mut buffer = Vec::new();

    // TODO: evaluate performance of jpg quality
    tile.write_to(&mut buffer, ImageOutputFormat::Jpeg(80))
        .unwrap();
    HttpResponse::Ok()
        .content_type(ContentType::jpeg())
        // TODO: caching is very aggressive and not private. Ensure URL is unique.
        .insert_header(("Cache-Control", "public, max-age=604800, immutable"))
        .body(buffer)
}

async fn dzi(
    viewers: web::Data<HashMap<String, DeepZoomGenerator>>,
    path: web::Path<String>,
) -> impl Responder {
    let slide = path.into_inner();
    let gen = viewers.get(slide.as_str()).expect("slide not found");
    HttpResponse::Ok()
        // TODO: caching is very aggressive and not private. Ensure URL is unique.
        .insert_header(("Cache-Control", "public, max-age=604800, immutable"))
        .body(gen.get_dzi())
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(Env::default().default_filter_or("info"));
    HttpServer::new(|| {
        let args: Vec<String> = std::env::args().collect();
        let filename = Path::new(&args[1]);
        let mut viewers = HashMap::new();
        viewers.insert(
            "slide_1".to_string(),
            DeepZoomGenerator::new(filename).expect("Could not start DeepZoomGenerator"),
        );
        let state = web::Data::new(viewers);
        App::new()
            .wrap(middleware::Logger::default())
            .wrap(middleware::DefaultHeaders::new().add(("Access-Control-Allow-Origin", "*")))
            .app_data(state)
            .route("/{slide}.dzi", web::get().to(dzi))
            .route(
                "/{slide}_files/{level}/{col}_{row}.jpg",
                web::get().to(tile_endpoint),
            )
            .service(fs::Files::new("/static", "./public/static").show_files_listing())
            .service(fs::Files::new("/", "./public/index.html").show_files_listing())
    })
    .workers(10)
    .bind("localhost:8080")?
    .run()
    .await
}
