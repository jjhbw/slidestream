mod generator;

use actix_files as fs;
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use generator::DeepZoomGenerator;
use image::{DynamicImage, ImageOutputFormat};
use std::path::Path;

#[get("/some_slide_files/{level}/{col}_{row}.jpg")]
async fn tile_endpoint(
    gen: web::Data<DeepZoomGenerator>,
    web::Path((level, col, row)): web::Path<(u64, u64, u64)>,
) -> HttpResponse {
    // TODO: ensure errors are presented in the frontend.
    let tile: DynamicImage = gen.get_tile(level, col, row).unwrap();
    let mut buffer = Vec::new();

    // TODO: evaluate performance of jpg quality
    tile.write_to(&mut buffer, ImageOutputFormat::Jpeg(80))
        .unwrap();
    HttpResponse::Ok().content_type("image/jpeg").body(buffer)
}

#[get("/some_slide.dzi")]
async fn dzi(gen: web::Data<DeepZoomGenerator>) -> impl Responder {
    HttpResponse::Ok().body(gen.get_dzi())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        let args: Vec<String> = std::env::args().collect();
        let filename = Path::new(&args[1]);
        let g = DeepZoomGenerator::new(filename).expect("Could not start DeepZoomGenerator");
        App::new()
            .data(g)
            .service(dzi)
            .service(tile_endpoint)
            .service(fs::Files::new("/static", "./public/static").show_files_listing())
            .service(fs::Files::new("/", "./public/index.html").show_files_listing())
    })
    // TODO: get_tile is mostly I/O-bound. Upping number of HTTP workers is crude way to scale that. Use a thread pool with futures instead.
    .workers(10)
    .bind("localhost:8080")?
    .run()
    .await
}
