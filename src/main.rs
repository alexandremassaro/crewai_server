use actix_web::{web, App, HttpServer, Responder, HttpResponse, post, get};
use elasticsearch::{Elasticsearch, SearchParts};
use elasticsearch::http::transport::Transport;
use prometheus::{Encoder, TextEncoder, register_counter, register_gauge, Counter, Gauge};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Serialize, Deserialize)]
struct AssistRequest {
    code_base: serde_json::Value,
    open_files: serde_json::Value,
    active_edits: serde_json::Value,
}

#[derive(Serialize, Deserialize)]
struct AskRequest {
    snippet: String,
    file_path: String,
}

struct AppState {
    es_client: Elasticsearch,
    requests_counter: Counter,
    response_time_gauge: Gauge,
}

#[post("/assist")]
async fn assist(_data: web::Json<AssistRequest>, state: web::Data<Arc<Mutex<AppState>>>) -> impl Responder {
    let state = state.lock().await;
    state.requests_counter.inc();
    let _client = &state.es_client;
    // Implement logic to handle assist request
    HttpResponse::Ok().json("Assistance response based on the current project.")
}

#[post("/ask")]
async fn ask(data: web::Json<AskRequest>, state: web::Data<Arc<Mutex<AppState>>>) -> impl Responder {
    let state = state.lock().await;
    state.requests_counter.inc();
    let client = &state.es_client;
    

    println!("Will search codebase for: {}", data.snippet);
    // Implement search logic
    let response = client
        .search(SearchParts::Index(&["codebase"]))
        .body(json!({
            "query": {
                "match": {
                    "content": data.snippet
                }
            }
        }))
        .send()
        .await;

    match response {
        Ok(result) => {
            let body = result.json::<serde_json::Value>().await.unwrap();
            let hits = body["hits"]["hits"].as_array().unwrap_or(&vec![]).to_vec();
            if !hits.is_empty() {
                let content = &hits[0]["_source"]["content"];
                println!("Succesful response");
                HttpResponse::Ok().json(json!({ "result": content }))
            } else {
                println!("Empty response");
                HttpResponse::Ok().json(json!({ "result": "No relevant context found." }))
            }
        }
        Err(_) => {
            println!("Error response");
            HttpResponse::InternalServerError().finish()
        },
    }
}

#[get("/metrics")]
async fn metrics(state: web::Data<Arc<Mutex<AppState>>>) -> impl Responder {
    let state = state.lock().await;
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();
    let response = String::from_utf8(buffer).unwrap();
    HttpResponse::Ok()
        .content_type("text/plain; version=0.0.4")
        .body(response)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let es_url = "http://elasticsearch:9200";  // Docker DNS resolution
    let transport = Transport::single_node(es_url).unwrap();
    let es_client = Elasticsearch::new(transport);
    let requests_counter = register_counter!("requests_total", "Total number of requests made.").unwrap();
    let response_time_gauge = register_gauge!("response_time_seconds", "Response time in seconds.").unwrap();
    let state = Arc::new(Mutex::new(AppState {
        es_client,
        requests_counter,
        response_time_gauge,
    }));

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(state.clone()))
            .service(assist)
            .service(ask)
            .service(metrics)
    })
    .bind("0.0.0.0:5000")?
    .run()
    .await
}
