mod generator;

use actix_files as fs;
use actix_web::{
    error,
    http::{header::ContentType, StatusCode},
    middleware, web, App, HttpResponse, HttpServer,
};
use derive_more::{Display, Error};
use env_logger::Env;
use generator::DeepZoomGenerator;
use image::ImageOutputFormat;
use log::error;
use std::{collections::HashMap, path::Path};

#[derive(Debug, Display, Error)]
enum DZIRetrievalError {
    #[display(fmt = "An internal error occurred.")]
    InternalError,

    #[display(fmt = "Tile request invalid.")]
    TileRequestInvalid,

    #[display(fmt = "Could not find slide.")]
    SlideNotFound,
}

impl error::ResponseError for DZIRetrievalError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::plaintext())
            .body(self.to_string())
    }

    fn status_code(&self) -> StatusCode {
        match *self {
            DZIRetrievalError::InternalError => StatusCode::INTERNAL_SERVER_ERROR,
            DZIRetrievalError::TileRequestInvalid => StatusCode::BAD_REQUEST,
            DZIRetrievalError::SlideNotFound => StatusCode::NOT_FOUND,
        }
    }
}

async fn get_tile(
    viewers: web::Data<HashMap<String, DeepZoomGenerator>>,
    path: web::Path<(String, u64, u64, u64)>,
) -> Result<HttpResponse, DZIRetrievalError> {
    let (slide, level, col, row) = path.into_inner();
    let gen = viewers.get(&slide).expect("slide not found");
    let tile = match gen.get_tile(level, col, row) {
        Ok(tile) => tile,
        Err(err) => {
            error!("Could not retrieve tile: {:?}", err);
            return Err(DZIRetrievalError::TileRequestInvalid);
        }
    };

    let mut buffer = Vec::new();
    match tile.write_to(&mut buffer, ImageOutputFormat::Jpeg(80)) {
        Ok(()) => (),
        Err(err) => {
            error!("Jpeg conversion failed: {:?}", err);
            return Err(DZIRetrievalError::InternalError);
        }
    };

    Ok(HttpResponse::Ok()
        .content_type(ContentType::jpeg())
        // TODO: caching is very aggressive and not private. Ensure URL is unique.
        .insert_header(("Cache-Control", "public, max-age=604800, immutable"))
        .body(buffer))
}

async fn get_dzi(
    viewers: web::Data<HashMap<String, DeepZoomGenerator>>,
    path: web::Path<String>,
) -> Result<HttpResponse, DZIRetrievalError> {
    let slide = path.into_inner();
    let gen = match viewers.get(slide.as_str()) {
        Some(tile) => tile,
        None => {
            error!("Could not find slide: {}", slide);
            return Err(DZIRetrievalError::SlideNotFound);
        }
    };
    Ok(HttpResponse::Ok()
        // TODO: caching is very aggressive and not private. Ensure URL is unique.
        .insert_header(("Cache-Control", "public, max-age=604800, immutable"))
        .body(gen.get_dzi()))
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
            .route("/{slide}.dzi", web::get().to(get_dzi))
            .route(
                "/{slide}_files/{level}/{col}_{row}.jpg",
                web::get().to(get_tile),
            )
            .service(fs::Files::new("/static", "./public/static").show_files_listing())
            .service(fs::Files::new("/", "./public/index.html").show_files_listing())
    })
    .bind("localhost:8080")?
    .run()
    .await
}
