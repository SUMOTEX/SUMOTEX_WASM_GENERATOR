use serde::Deserialize;
use std::process::Command;
use std::path::Path;
use std::fs;
use std::thread;
use serde_json::json;
use lettre_email::Mailbox; // Make sure to import Mailbox
use serde_json::Value;
use tiny_http::{Server,Header, Response};
use lettre::{SmtpClient, Transport};
use lettre::smtp::authentication::Credentials;
use lettre_email::EmailBuilder;
use native_tls::{TlsConnector, Protocol};

#[derive(Deserialize)]
struct CompileRequest {
    source_code: String,
}
#[derive(Deserialize)]
struct EmailRequest {
    recipient: String,
    subject: String,
    body: String,
}


pub fn send_email(req: EmailRequest) -> Result<(), String> {
    println!("Email calling...");
    let recipient = &req.recipient;
    let subject = &req.subject;
    let body = &req.body;

    let email = EmailBuilder::new()
        .to(recipient.parse::<Mailbox>().map_err(|_| "Invalid email format".to_owned())?)
        .from("hello@sumotex.co")
        .subject(subject)
        .text(body)
        .build()
        .map_err(|e| format!("Error building email: {}", e))?;

    println!("Email built, setting up SMTP client...");

    let creds = Credentials::new(
        "hello@sumotex.co".to_string(), // Your Gmail
        "uhuzkunzdiysackg".to_string() // Your Gmail App Password or password
    );

    let tls = match TlsConnector::builder()
        .min_protocol_version(Some(Protocol::Tlsv12))
        .build() {
            Ok(tls) => tls,
            Err(e) => {
                println!("Error creating TLS connector: {}", e);
                return Err(e.to_string());
            }
    };

    let mailer = match SmtpClient::new(
        ("smtp.gmail.com", 587),
        lettre::ClientSecurity::Required(lettre::ClientTlsParameters::new("smtp.gmail.com".to_string(), tls))
    ) {
        Ok(client) => client.credentials(creds),
        Err(e) => {
            println!("Error creating SMTP client: {}", e);
            return Err(e.to_string());
        }
    };

    let mut mailer =  mailer.transport();

    println!("SMTP client set up, sending email...");
    match mailer.send(email.into()) {
        Ok(_) => {
            println!("Email sent successfully.");
            Ok(())
        },
        Err(e) => {
            println!("Could not send email: {:?}", e);
            Err(format!("Could not send email: {:?}", e))
        }
    }
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
    let source_file_path = src_dir.join("test.rs");
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

    let response = match request.url() {
        "/send_email" => {
            if let Ok(email_request) = serde_json::from_str::<EmailRequest>(&content) {
                match send_email(email_request) {
                    Ok(_) => Response::from_string("Email sent successfully").with_status_code(200),
                    Err(err) => Response::from_string(err).with_status_code(500),
                }
            } else {
                Response::from_string("Invalid email request").with_status_code(400)
            }
        },
        "/compile_and_serve" => {
            if let Ok(compile_request) = serde_json::from_str::<CompileRequest>(&content) {
                match compile_and_serve_erc721(compile_request) {
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
                    Err(err) => Response::from_string(err).with_status_code(500),
                }
            } else {
                Response::from_string("Invalid compile request").with_status_code(400)
            }
        },
        _ => Response::from_string("Not found").with_status_code(404),
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
    let server = Server::http("0.0.0.0:8000").unwrap();
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
