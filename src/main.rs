use actix_cors::Cors;
use actix_web::{web, App, HttpResponse, HttpServer, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

async fn get_public_ip() -> String {
    // Try to get the public IP address using external services
    let services = [
        "https://api.ipify.org",
        "https://ifconfig.me/ip",
        "https://icanhazip.com",
    ];
    
    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();
    
    for service in &services {
        if let Ok(response) = client.get(*service).send().await {
            if let Ok(ip) = response.text().await {
                let ip = ip.trim();
                if !ip.is_empty() && ip.parse::<std::net::IpAddr>().is_ok() {
                    return format!("{}:8888", ip);
                }
            }
        }
    }
    
    "unknown:8888".to_string() // Fallback if all services fail
}

#[derive(Deserialize)]
struct ProxyRequest {
    url: String,
}

#[derive(Serialize)]
struct ProxyResponse {
    html: String,
    status: u16,
    server_ip: String,
}

async fn proxy_handler(req: web::Json<ProxyRequest>) -> Result<HttpResponse> {
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .unwrap();

    println!("Fetching URL: {}", req.url);

    match client.get(&req.url)
        .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8")
        .header("Accept-Language", "en-US,en;q=0.5")
        .header("DNT", "1")
        .header("Connection", "keep-alive")
        .header("Upgrade-Insecure-Requests", "1")
        .send()
        .await {
        Ok(response) => {
            let status = response.status().as_u16();
            let headers = response.headers().clone();
            
            println!("Response status: {}", status);
            println!("Content-Type: {:?}", headers.get("content-type"));
            
            match response.text().await {
                Ok(mut html) => {
                    // Fix relative URLs to absolute URLs
                    let base_url = &req.url;
                    if let Ok(parsed_url) = url::Url::parse(base_url) {
                        let origin = format!("{}://{}", parsed_url.scheme(), parsed_url.host_str().unwrap_or(""));
                        
                        // Replace relative URLs with absolute URLs
                        html = html.replace("href=\"/", &format!("href=\"{}/", origin));
                        html = html.replace("src=\"/", &format!("src=\"{}/", origin));
                        html = html.replace("action=\"/", &format!("action=\"{}/", origin));
                        
                        // Fix CSS url() references
                        html = html.replace("url(/_next/", &format!("url({}//_next/", origin));
                        html = html.replace("url(/", &format!("url({}/", origin));
                        
                        // Also handle protocol-relative URLs
                        html = html.replace("href=\"//", "href=\"https://");
                        html = html.replace("src=\"//", "src=\"https://");
                    }
                    
                    println!("HTML content length: {} chars", html.len());
                    
                    // Get the server's public IP address
                    let server_ip = get_public_ip().await;
                    
                    let proxy_response = ProxyResponse {
                        html,
                        status,
                        server_ip,
                    };
                    Ok(HttpResponse::Ok().json(proxy_response))
                }
                Err(e) => {
                    println!("Failed to read response body: {}", e);
                    Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                        "error": format!("Failed to read response body: {}", e)
                    })))
                }
            }
        }
        Err(e) => {
            println!("Failed to fetch URL: {}", e);
            Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "error": format!("Failed to fetch the URL: {}", e)
            })))
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("🦀 Proxy Server starting on http://localhost:8888");
    
    HttpServer::new(|| {
        let cors = Cors::default()
            .allowed_origin("http://localhost:5173")
            .allowed_origin("http://localhost:5174")
            .allowed_origin("http://localhost:3000")
            .allowed_origin_fn(|origin, _req_head| {
                origin.as_bytes().starts_with(b"file://") || 
                origin.as_bytes().starts_with(b"http://localhost") ||
                origin.as_bytes().starts_with(b"http://127.0.0.1")
            })
            .allowed_methods(vec!["GET", "POST", "OPTIONS"])
            .allowed_headers(vec![
                actix_web::http::header::AUTHORIZATION, 
                actix_web::http::header::ACCEPT,
                actix_web::http::header::CONTENT_TYPE,
                actix_web::http::header::ACCESS_CONTROL_ALLOW_ORIGIN
            ])
            .supports_credentials();

        App::new()
            .wrap(cors)
            .route("/proxy", web::post().to(proxy_handler))
    })
    .bind("127.0.0.1:8888")?
    .run()
    .await
}
