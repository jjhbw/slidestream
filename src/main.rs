mod generator;

use actix_files as fs;
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use generator::DeepZoomGenerator;
use image::{DynamicImage, ImageOutputFormat};
use std::{collections::HashMap, path::Path};
use tokio::task;

#[get("/{slide}_files/{level}/{col}_{row}.jpg")]
async fn tile_endpoint(
    viewers: web::Data<HashMap<String, DeepZoomGenerator>>,
    web::Path((slide, level, col, row)): web::Path<(String, u64, u64, u64)>,
) -> HttpResponse {
    // TODO: ensure errors are presented in the frontend.
    // TODO: revisit thread safety of OpenSlide object from bindings, see https://github.com/openslide/openslide/issues/242
    let tile: DynamicImage = task::spawn_blocking(move || {
        let gen = viewers.get(&slide).expect("slide not found");
        gen.get_tile(level, col, row).unwrap()
    })
    .await
    .unwrap();
    let mut buffer = Vec::new();

    // TODO: evaluate performance of jpg quality
    tile.write_to(&mut buffer, ImageOutputFormat::Jpeg(80))
        .unwrap();
    HttpResponse::Ok()
        .content_type("image/jpeg")
        .set_header("Access-Control-Allow-Origin", "*")
        // TODO: caching is very aggressive and not private. Ensure URL is unique.
        .set_header("Cache-Control", "public, max-age=604800, immutable")
        .body(buffer)
}

#[get("/{slide}.dzi")]
async fn dzi(
    viewers: web::Data<HashMap<String, DeepZoomGenerator>>,
    web::Path(slide): web::Path<String>,
) -> impl Responder {
    let gen = viewers.get(slide.as_str()).expect("slide not found");
    HttpResponse::Ok().body(gen.get_dzi())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        let args: Vec<String> = std::env::args().collect();
        let filename = Path::new(&args[1]);
        let mut viewers = HashMap::new();
        viewers.insert(
            "slide_1".to_string(),
            DeepZoomGenerator::new(filename).expect("Could not start DeepZoomGenerator"),
        );
        App::new()
            .data(viewers)
            .service(dzi)
            .service(tile_endpoint)
            .service(fs::Files::new("/static", "./public/static").show_files_listing())
            .service(fs::Files::new("/", "./public/index.html").show_files_listing())
    })
    .workers(10)
    .bind("localhost:8080")?
    .run()
    .await
}
