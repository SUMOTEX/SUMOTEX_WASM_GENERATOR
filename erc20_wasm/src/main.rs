use actix_web::{web, App, HttpServer, HttpResponse, Responder};
use serde::Deserialize;
use std::process::Command;
use std::path::Path;
use std::fs;

#[derive(Deserialize)]
struct CompileRequest {
    source_code: String,
}

async fn compile_and_serve_erc721(req: web::Json<CompileRequest>) -> impl Responder {
    let source_code = &req.source_code;
    let project_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let src_dir = project_dir.join("src");
    
    if !src_dir.exists() {
        if let Err(e) = fs::create_dir_all(&src_dir) {
            return HttpResponse::InternalServerError().body(format!("Error creating src directory: {}", e));
        }
    }

    let source_file_path = src_dir.join("lib.rs");
    if let Err(e) = fs::write(&source_file_path, source_code) {
        return HttpResponse::InternalServerError().body(format!("Error writing to source file: {}", e));
    }

    let output = Command::new("cargo")
        .arg("build")
        .arg("--target=wasm32-wasi")
        .arg("--release")
        .current_dir(project_dir)
        .output();

    match output {
        Ok(output) if output.status.success() => {
            let wasm_file_path = project_dir.join("target/wasm32-wasi/release/sample721.wasm");
            match fs::read(&wasm_file_path) {
                Ok(data) => HttpResponse::Ok().content_type("application/wasm").body(data),
                Err(e) => HttpResponse::InternalServerError().body(format!("Error reading WASM file: {}", e)),
            }
        },
        Ok(output) => {
            let error_message = String::from_utf8_lossy(&output.stderr).to_string();
            HttpResponse::InternalServerError().body(format!("Cargo build failed: {}", error_message))
        },
        Err(e) => HttpResponse::InternalServerError().body(format!("Failed to execute cargo build: {}", e)),
    }
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .route("/compile", web::post().to(compile_and_serve_erc721))
    })
    .bind("127.0.0.1:8000")?
    .run()
    .await
}
