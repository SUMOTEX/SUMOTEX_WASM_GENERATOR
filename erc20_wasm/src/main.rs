use serde::Deserialize;
use std::process::Command;
use std::path::Path;
use std::fs;
use std::thread;
use serde_json::json;
use serde_json::Value;
use tiny_http::{Server,Header, Response};

#[derive(Deserialize)]
struct CompileRequest {
    source_code: String,
}

fn compile_and_serve_erc721(req: CompileRequest) -> Result<(Vec<u8>,String), String> {
    let source_code = &req.source_code;
    println!("{:?}",source_code);
    let project_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let src_dir = project_dir.join("src");

    if !src_dir.exists() {
        if let Err(e) = fs::create_dir_all(&src_dir) {
            return Err(format!("Error creating src directory: {}", e));
        }
    }
    let source_file_path = src_dir.join("lib.rs");
    if let Err(e) = fs::write(&source_file_path, source_code) {
        return Err(format!("Error writing to source file: {}", e));
    }
    let output = Command::new("cargo")
        .arg("build")
        .arg("--target=wasm32-wasi")
        .arg("--release")
        .current_dir(project_dir)
        .output()
        .map_err(|e| format!("Failed to execute cargo build: {}", e))?;


    if output.status.success() {
        let wasm_file_name = "sample721.wasm"; // Replace with the actual file name
        let wasm_file_path = "/home/ubuntu/SUMOTEX_WASM_GENERATOR/target/wasm32-wasi/release/sample721.wasm";   
        let wasm_data = fs::read(&wasm_file_path)
            .map_err(|e| format!("Error reading WASM file: {}", e))?;

        // Read or generate the ABI
        let abi_file_path = "/home/ubuntu/SUMOTEX_WASM_GENERATOR/abi.json";
        let abi_data = fs::read_to_string(&abi_file_path)
            .map_err(|e| format!("Error reading ABI file: {}", e))?;

        Ok((wasm_data, abi_data))
            
    }else{
        let error_message = String::from_utf8_lossy(&output.stderr).to_string();
        Err(format!("Cargo build failed: {}", error_message))
    }
}
fn handle_request(mut request: tiny_http::Request) {
    let mut content = String::new();
    if let Err(e) = request.as_reader().read_to_string(&mut content) {
        eprintln!("Failed to read request: {}", e);
        return;
    }

    let response = if let Ok(json_value) = serde_json::from_str::<Value>(&content) {
        if let Some(source_code) = json_value["source_code"].as_str() {
            match compile_and_serve_erc721(CompileRequest { source_code: source_code.to_string() }) {
                Ok((wasm_data, abi_data)) => {
                    let response_obj = json!({
                        "wasm": base64::encode(wasm_data),
                        "abi": abi_data,
                    });
                    if let Ok(response_json) = serde_json::to_string(&response_obj) {
                        let mut response = Response::from_string(response_json);
                        for header in cors_headers() {
                            response.add_header(header);
                        }
                        response
                    } else {
                        Response::from_string("Failed to serialize response").with_status_code(500)
                    }
                },
                Err(err) => {
                    Response::from_string(err).with_status_code(500)
                }
            }
        } else {
            Response::from_string("Invalid request").with_status_code(400)
        }
    } else {
        Response::from_string("Failed to parse JSON").with_status_code(400)
    };

    let _ = request.respond(response);
}


fn cors_headers() -> Vec<Header> {
    vec![
        Header::from_bytes("Access-Control-Allow-Origin", "*").unwrap(),
        Header::from_bytes("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS").unwrap(),
        Header::from_bytes("Access-Control-Allow-Headers", "Content-Type, Authorization").unwrap(),
        Header::from_bytes("Access-Control-Allow-Credentials", "true").unwrap(),
    ]
}


fn main() {
    let server = Server::http("0.0.0.0:8001").unwrap();
    println!("Server listening on port 8000...");

    for request in server.incoming_requests() {
        let request = request; // request is already of type tiny_http::Request
        thread::spawn(move || {
            // Check if it's a preflight OPTIONS request
            if request.method() == &tiny_http::Method::Options {
                let mut response = Response::empty(204);
                for header in cors_headers() {
                    response.add_header(header);
                }
                let _ = request.respond(response);
                return;
            }
            handle_request(request);
            // Handle other requests
            // ...
        });
    }
}
