use actix_cors::Cors;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Result};
use reqwest::{Client, cookie::Jar};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use std::hash::Hasher;
use std::sync::{Arc, Mutex};
use rand::{thread_rng, Rng};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use base64::{Engine as _, engine::general_purpose};

mod pqc;
use pqc::PqcCrypto;

// Advanced browser fingerprint data
#[derive(Debug, Clone)]
struct BrowserFingerprint {
    user_agent: String,
    sec_ch_ua: String,
    sec_ch_ua_mobile: String,
    sec_ch_ua_platform: String,
    viewport_width: u32,
    viewport_height: u32,
    screen_width: u32,
    screen_height: u32,
    timezone_offset: i32,
    language: String,
    platform: String,
    webgl_vendor: String,
    webgl_renderer: String,
    created_at: u64,
}

impl BrowserFingerprint {
    fn new() -> Self {
        let mut rng = thread_rng();
        
        // Generate consistent but varied browser characteristics
        let platforms = [
            ("Windows NT 10.0; Win64; x64", "Windows", 1920, 1080, 1920, 1040),
            ("Macintosh; Intel Mac OS X 10_15_7", "macOS", 1440, 900, 1440, 877),
            ("X11; Linux x86_64", "Linux", 1920, 1080, 1920, 1053),
        ];
        
        let (platform_str, platform_name, sw, sh, vw, vh) = platforms[rng.gen_range(0..platforms.len())];
        
        let chrome_versions = ["120.0.0.0", "119.0.0.0", "121.0.0.0", "118.0.0.0"];
        let chrome_version = chrome_versions[rng.gen_range(0..chrome_versions.len())];
        
        let user_agent = format!(
            "Mozilla/5.0 ({}) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/{} Safari/537.36",
            platform_str, chrome_version
        );
        
        let major_version = chrome_version.split('.').next().unwrap();
        let sec_ch_ua = format!(
            r#""Not_A Brand";v="8", "Chromium";v="{}", "Google Chrome";v="{}""#,
            major_version, major_version
        );
        
        Self {
            user_agent,
            sec_ch_ua,
            sec_ch_ua_mobile: "?0".to_string(),
            sec_ch_ua_platform: format!(r#""{}""#, platform_name),
            viewport_width: vw,
            viewport_height: vh,
            screen_width: sw,
            screen_height: sh,
            timezone_offset: rng.gen_range(-720..=840), // Realistic timezone range
            language: "en-US".to_string(),
            platform: platform_name.to_string(),
            webgl_vendor: "Google Inc. (Intel)".to_string(),
            webgl_renderer: "ANGLE (Intel, Intel(R) UHD Graphics 630 Direct3D11 vs_5_0 ps_5_0, D3D11)".to_string(),
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        }
    }
}

// Enhanced session management with persistent state
#[derive(Clone)]
struct BrowserSession {
    fingerprint: BrowserFingerprint,
    cookies: Arc<Jar>,
    visited_urls: Vec<String>,
    interaction_history: Vec<String>,
    session_start: u64,
    last_activity: u64,
    request_count: u32,
}

impl BrowserSession {
    fn new() -> Self {
        let fingerprint = BrowserFingerprint::new();
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        
        Self {
            fingerprint,
            cookies: Arc::new(Jar::default()),
            visited_urls: Vec::new(),
            interaction_history: Vec::new(),
            session_start: now,
            last_activity: now,
            request_count: 0,
        }
    }
    
    fn update_activity(&mut self, url: &str) {
        self.last_activity = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        self.request_count += 1;
        if !self.visited_urls.contains(&url.to_string()) {
            self.visited_urls.push(url.to_string());
        }
    }
    
    fn get_session_duration(&self) -> u64 {
        self.last_activity - self.session_start
    }
}

// Global session storage
// Advanced session storage
type SessionStorage = Arc<Mutex<HashMap<String, Arc<Mutex<AdvancedSession>>>>>;

fn get_session_id(req: &HttpRequest) -> String {
    // Create session ID based on client IP and some randomization
    let client_ip = req.connection_info().realip_remote_addr()
        .unwrap_or("127.0.0.1").to_string();
    
    // In a real implementation, you'd use proper session management
    format!("session_{}", general_purpose::STANDARD.encode(client_ip))
}

fn get_or_create_session(session_storage: &SessionStorage, session_id: &str) -> Arc<Mutex<AdvancedSession>> {
    get_or_create_advanced_session(session_storage, session_id)
}

fn update_session(session_storage: &SessionStorage, session_id: &str, session: Arc<Mutex<AdvancedSession>>, url: &str) {
    update_advanced_session(session_storage, session_id, &session, url);
}

fn get_realistic_user_agent(_url: &str) -> &'static str {
    // This is now handled by BrowserFingerprint, but keeping for compatibility
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
}

fn generate_google_specific_headers(session: &BrowserSession, url: &str, referer: Option<&str>) -> Vec<(String, String)> {
    let mut headers = Vec::new();
    let fp = &session.fingerprint;
    
    // Header order is critical - must match real Chrome exactly
    headers.push(("Accept".to_string(), 
        "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7".to_string()));
    
    headers.push(("Accept-Encoding".to_string(), "gzip, deflate, br".to_string()));
    headers.push(("Accept-Language".to_string(), "en-US,en;q=0.9".to_string()));
    
    // Critical Chrome client hints - Google checks these extensively
    headers.push(("Sec-Ch-Ua".to_string(), fp.sec_ch_ua.clone()));
    headers.push(("Sec-Ch-Ua-Mobile".to_string(), fp.sec_ch_ua_mobile.clone()));
    headers.push(("Sec-Ch-Ua-Platform".to_string(), fp.sec_ch_ua_platform.clone()));
    
    // Always include these for Google
    headers.push(("Sec-Fetch-Dest".to_string(), "document".to_string()));
    headers.push(("Sec-Fetch-Mode".to_string(), "navigate".to_string()));
    headers.push(("Sec-Fetch-Site".to_string(), if referer.is_some() { "same-origin" } else { "none" }.to_string()));
    headers.push(("Sec-Fetch-User".to_string(), "?1".to_string()));
    
    headers.push(("Upgrade-Insecure-Requests".to_string(), "1".to_string()));
    headers.push(("User-Agent".to_string(), fp.user_agent.clone()));
    
    // Add referer if provided
    if let Some(ref_url) = referer {
        headers.push(("Referer".to_string(), ref_url.to_string()));
    }
    
    // Google-specific client data that real Chrome sends
    if url.contains("google.") {
        let client_data = generate_google_client_data(&fp);
        headers.push(("X-Client-Data".to_string(), client_data));
        
        // Additional Google-specific headers based on session history
        if session.visited_urls.iter().any(|u| u.contains("google.")) {
            headers.push(("X-Same-Domain".to_string(), "1".to_string()));
        }
        
        // Simulate browser cache behavior
        if session.request_count > 1 {
            headers.push(("Cache-Control".to_string(), "max-age=0".to_string()));
        }
    }
    
    headers
}

fn generate_google_client_data(fp: &BrowserFingerprint) -> String {
    // This generates a realistic X-Client-Data header that Chrome sends to Google
    // The format is base64 encoded protobuf data
    
    // Simulate Chrome's client data with realistic values
    let mut rng = thread_rng();
    let chrome_version: u32 = fp.user_agent.split("Chrome/").nth(1)
        .and_then(|s| s.split('.').next())
        .and_then(|s| s.parse().ok())
        .unwrap_or(120);
    
    // Generate realistic encoded client data
    // This is a simplified version - real Chrome client data is more complex
    let client_data_raw = format!(
        "{}:{}:{}:{}:{}",
        chrome_version,
        rng.gen_range(1000..9999),
        fp.viewport_width,
        fp.viewport_height,
        rng.gen_range(100..999)
    );
    
    general_purpose::STANDARD.encode(client_data_raw.as_bytes())
}

fn get_realistic_headers(url: &str, method: &str) -> Vec<(&'static str, String)> {
    // This is now deprecated in favor of generate_google_specific_headers
    // but keeping for backward compatibility
    let mut headers = Vec::new();
    
    headers.push(("Accept", if method == "GET" {
        "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7"
    } else {
        "application/json, text/plain, */*"
    }.to_string()));
    
    headers.push(("Accept-Language", "en-US,en;q=0.9".to_string()));
    headers.push(("Accept-Encoding", "gzip, deflate, br".to_string()));
    headers.push(("User-Agent", get_realistic_user_agent(url).to_string()));
    
    headers
}

fn is_hop_by_hop_header(name: &str) -> bool {
    matches!(name, 
        "connection" | "proxy-connection" | "te" | "trailer" | 
        "transfer-encoding" | "upgrade" | "keep-alive" | "proxy-authenticate" |
        "proxy-authorization"
    )
}

fn is_already_set_header(name: &str) -> bool {
    matches!(name,
        "host" | "content-length" | "accept" | "accept-language" | 
        "accept-encoding" | "user-agent" | "sec-fetch-dest" | 
        "sec-fetch-mode" | "sec-fetch-site" | "sec-fetch-user" | 
        "upgrade-insecure-requests" | "dnt" | "cache-control" | 
        "x-forwarded-for" | "x-real-ip" | "x-forwarded-proto" |
        "sec-ch-ua" | "sec-ch-ua-mobile" | "sec-ch-ua-platform" |
        "sec-ch-ua-arch" | "sec-ch-ua-bitness" | "sec-ch-ua-full-version-list" |
        "sec-ch-ua-wow64" | "sec-ch-ua-model" | "x-client-data" |
        "sec-ch-viewport-width" | "sec-ch-viewport-height" | "sec-ch-device-memory" |
        "sec-ch-prefers-color-scheme" | "sec-ch-prefers-reduced-motion" | "referer"
    )
}

// Create a persistent session for each client to maintain cookies and state
struct Session {
    client: Client,
    cookies: Arc<Jar>,
    visit_count: u32,
    last_visit: std::time::Instant,
}

impl Session {
    fn new() -> Self {
        let cookies = Arc::new(Jar::default());
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .cookie_provider(cookies.clone())
            .danger_accept_invalid_certs(false)
            .tcp_keepalive(Duration::from_secs(60))
            .pool_idle_timeout(Duration::from_secs(90))
            .pool_max_idle_per_host(10)
            .http1_only() // Force HTTP/1.1 to avoid HTTP/2 issues with bot detection
            .build()
            .unwrap();
            
        Self {
            client,
            cookies,
            visit_count: 0,
            last_visit: std::time::Instant::now(),
        }
    }
    
    fn update_visit(&mut self) {
        self.visit_count += 1;
        self.last_visit = std::time::Instant::now();
    }
}

async fn prefetch_google_homepage(client: &Client) -> Result<(), reqwest::Error> {
    // Some browsers prefetch the homepage before making search requests
    let _response = client
        .get("https://www.google.com/")
        .header("User-Agent", get_realistic_user_agent("https://www.google.com/"))
        .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8")
        .header("Accept-Language", "en-US,en;q=0.9")
        .header("sec-ch-ua", "\"Not_A Brand\";v=\"8\", \"Chromium\";v=\"120\", \"Google Chrome\";v=\"120\"")
        .header("sec-ch-ua-mobile", "?0")
        .header("sec-ch-ua-platform", "\"Windows\"")
        .timeout(Duration::from_secs(10))
        .send()
        .await?;
    
    // Small delay after prefetch
    tokio::time::sleep(Duration::from_millis(200)).await;
    Ok(())
}

async fn simulate_google_browsing_behavior(session: &BrowserSession, target_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Create client using session's cookies
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .cookie_provider(session.cookies.clone())
        .http1_only()
        .build()?;
    
    println!("Simulating realistic Google browsing behavior");
    
    // Step 1: Visit Google homepage first if we haven't been to Google recently
    if !session.visited_urls.iter().any(|u| u.contains("google.")) {
        println!("First visit to Google - loading homepage");
        
        let homepage_headers = generate_google_specific_headers(session, "https://www.google.com/", None);
        let mut homepage_request = client.get("https://www.google.com/");
        
        for (name, value) in homepage_headers {
            homepage_request = homepage_request.header(&name, &value);
        }
        
        let _ = homepage_request.send().await;
        
        // Human-like delay after loading homepage
        tokio::time::sleep(Duration::from_millis(rand::thread_rng().gen_range(1500..3000))).await;
        
        // Load some typical resources that a real browser would load
        let resources = [
            "/favicon.ico",
            "/images/branding/googleg/1x/googleg_standard_color_128dp.png",
        ];
        
        for resource in &resources {
            let resource_url = format!("https://www.google.com{}", resource);
            let resource_headers = generate_google_specific_headers(session, &resource_url, Some("https://www.google.com/"));
            let mut resource_request = client.get(&resource_url);
            
            for (name, value) in resource_headers {
                resource_request = resource_request.header(&name, &value);
            }
            
            let _ = resource_request.send().await;
            tokio::time::sleep(Duration::from_millis(rand::thread_rng().gen_range(100..500))).await;
        }
    }
    
    // Step 2: If this is a search, simulate typing delay and suggestion fetching
    if target_url.contains("/search?q=") {
        // Extract search query
        if let Some(query_start) = target_url.find("q=") {
            let query_part = &target_url[query_start + 2..];
            let query = query_part.split('&').next().unwrap_or("");
            
            if !query.is_empty() {
                println!("Simulating search query typing for: {}", query);
                
                // Simulate Google search suggestions being fetched as user types
                let decoded_query = urlencoding::decode(query).unwrap_or_default();
                let query_chars: Vec<char> = decoded_query.chars().collect();
                
                // Simulate typing with suggestions (Google's complete/search endpoint)
                for i in 1..=query_chars.len() {
                    if i > 2 && i % 2 == 0 { // Don't fetch for every character, be realistic
                        let partial_query: String = query_chars[..i].iter().collect();
                        let suggestion_url = format!(
                            "https://www.google.com/complete/search?q={}&client=gws-wiz&xssi=t&gs_ri=gws-wiz&hl=en&authuser=0",
                            urlencoding::encode(&partial_query)
                        );
                        
                        let suggestion_headers = generate_google_specific_headers(session, &suggestion_url, Some("https://www.google.com/"));
                        let mut suggestion_request = client.get(&suggestion_url);
                        
                        for (name, value) in suggestion_headers {
                            suggestion_request = suggestion_request.header(&name, &value);
                        }
                        
                        let _ = suggestion_request.send().await;
                        
                        // Realistic typing speed delay
                        tokio::time::sleep(Duration::from_millis(rand::thread_rng().gen_range(150..400))).await;
                    }
                }
                
                // Final pause before submitting search
                tokio::time::sleep(Duration::from_millis(rand::thread_rng().gen_range(500..1500))).await;
            }
        }
    }
    
    Ok(())
}

async fn simulate_browsing_behavior(client: &Client, target_url: &str) -> Result<(), reqwest::Error> {
    // Simulate a more realistic browsing pattern for Google
    if target_url.contains("google.com") {
        // First, visit the homepage
        let _homepage = client
            .get("https://www.google.com/")
            .header("User-Agent", get_realistic_user_agent("https://www.google.com/"))
            .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8")
            .header("Accept-Language", "en-US,en;q=0.9")
            .header("sec-ch-ua", "\"Not_A Brand\";v=\"8\", \"Chromium\";v=\"120\", \"Google Chrome\";v=\"120\"")
            .header("sec-ch-ua-mobile", "?0")
            .header("sec-ch-ua-platform", "\"Windows\"")
            .header("Sec-Fetch-Dest", "document")
            .header("Sec-Fetch-Mode", "navigate")
            .header("Sec-Fetch-Site", "none")
            .header("Sec-Fetch-User", "?1")
            .timeout(Duration::from_secs(10))
            .send()
            .await?;
        
        // Human-like delay
        tokio::time::sleep(Duration::from_millis(800 + rand::thread_rng().gen_range(200..800))).await;
        
        // Simulate loading Google's static resources (JavaScript, CSS)
        let resources = [
            "https://www.google.com/xjs/_/js/",
            "https://www.google.com/textinputassistant/",
            "https://ssl.gstatic.com/ui/v1/",
        ];
        
        for resource in &resources[0..1] { // Just load one to avoid too many requests
            let _res = client
                .get(*resource)
                .header("User-Agent", get_realistic_user_agent("https://www.google.com/"))
                .header("Referer", "https://www.google.com/")
                .header("Accept", "*/*")
                .header("Sec-Fetch-Dest", "script")
                .header("Sec-Fetch-Mode", "no-cors")
                .header("Sec-Fetch-Site", "same-origin")
                .timeout(Duration::from_secs(5))
                .send()
                .await; // Ignore errors for resource loading
            
            tokio::time::sleep(Duration::from_millis(100 + rand::thread_rng().gen_range(50..200))).await;
        }
    }
    
    Ok(())
}

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
struct ProxyQuery {
    url: String,
}

// PQC-related structs for existing handlers
#[derive(Deserialize)]
struct ProxyRequest {
    url: String,
}

#[derive(Serialize)]
struct ProxyResponse {
    html: String,
    status: u16,
    server_ip: String,
    pqc_session_id: String,
    pqc_public_keys: PqcPublicKeys,
}

#[derive(Deserialize)]
struct PqcProxyRequest {
    url: String,
    pqc_session: Option<pqc::PqcSharedData>,
    peer_public_keys: Option<PqcPublicKeys>,
}

#[derive(Serialize, Deserialize, Clone)]
struct PqcPublicKeys {
    kyber_pk: String,
    dilithium_pk: String,
    sphincs_pk: String,
}

#[derive(Serialize)]
struct PqcResponse {
    session_data: pqc::PqcSharedData,
    public_keys: PqcPublicKeys,
}

// Global PQC instance (in production, you'd want proper state management)
lazy_static::lazy_static! {
    static ref PQC_INSTANCE: PqcCrypto = PqcCrypto::new();
}

fn generate_session_id() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let session_id: u64 = rng.r#gen();
    format!("pqc_session_{}", session_id)
}

async fn proxy(req: HttpRequest, body: web::Bytes, query: web::Query<ProxyQuery>, session_storage: web::Data<SessionStorage>) -> Result<HttpResponse> {
    // Validate URL parameter
    if query.url.is_empty() {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": "URL parameter is required"
        })));
    }

    // Get session ID and advanced session
    let session_id = get_session_id(&req);
    let session = get_or_create_advanced_session(&session_storage, &session_id);
    
    println!("Proxying {} request to: {}", req.method(), query.url);

    // For Google requests, use advanced anti-bot techniques
    let is_google_search = query.url.contains("google.") && query.url.contains("/search");
    let is_google_request = query.url.contains("google.");
    
    if is_google_request {
        println!("🎯 Using advanced Google anti-bot techniques");
        
        // Check if session needs cooling off
        {
            let session_guard = session.lock().unwrap();
            if session_guard.needs_cooling_off() {
                println!("❄️ Session needs cooling off - adding extra delay");
                drop(session_guard);
                tokio::time::sleep(Duration::from_millis(rand::thread_rng().gen_range(5000..15000))).await;
            }
        }
        
        // Simulate realistic pre-search behavior
        let _ = simulate_advanced_browsing_behavior(&session, &query.url).await;
        
        // Smart delay to avoid detection
        smart_delay(&session).await;
    }

    // Determine if this should be a mobile request (randomly for variety)
    let is_mobile = rand::thread_rng().gen_bool(0.3);

    // Create client with session's cookie jar and advanced settings
    let client = {
        let session_guard = session.lock().unwrap();
        Client::builder()
            .timeout(Duration::from_secs(45))
            .cookie_provider(session_guard.cookies.clone())
            .danger_accept_invalid_certs(false)
            .tcp_keepalive(Duration::from_secs(60))
            .pool_idle_timeout(Duration::from_secs(90))
            .pool_max_idle_per_host(10)
            .http1_only() // Force HTTP/1.1 to avoid HTTP/2 fingerprinting
            .gzip(true)
            .brotli(true)
            .deflate(true)
            .build()
            .unwrap()
    };

    // Build the request with advanced headers
    let mut request_builder = match req.method().as_str() {
        "GET" => client.get(&query.url),
        "POST" => client.post(&query.url),
        "PUT" => client.put(&query.url),
        "DELETE" => client.delete(&query.url),
        "HEAD" => client.head(&query.url),
        "PATCH" => client.patch(&query.url),
        method => client.request(reqwest::Method::from_bytes(method.as_bytes()).unwrap(), &query.url),
    };

    // Use advanced header generation
    let headers_to_use = {
        let session_guard = session.lock().unwrap();
        generate_realistic_headers_v2(&session_guard, &query.url, is_mobile)
    };
    
    // Add headers in the exact order they appear in real browsers
    for (name, value) in headers_to_use {
        request_builder = request_builder.header(&name, &value);
    }

    // Get client IP for forwarding with advanced masking
    let client_ip = req.connection_info().realip_remote_addr()
        .map(|addr| addr.split(':').next().unwrap_or("127.0.0.1"))
        .unwrap_or("127.0.0.1")
        .to_string();

    // Generate more realistic forwarded IP
    let forwarded_ip = generate_realistic_forwarded_ip(&client_ip);

    request_builder = request_builder
        .header("X-Forwarded-For", &forwarded_ip)
        .header("X-Real-IP", &forwarded_ip)
        .header("X-Forwarded-Proto", if query.url.starts_with("https") { "https" } else { "http" });

    // Add request body if present
    if !body.is_empty() {
        request_builder = request_builder.body(body.to_vec());
    }

    // Update session before making request
    {
        let mut session_guard = session.lock().unwrap();
        session_guard.update_activity();
    }

    // Send the request with retry logic for Google
    let mut retry_count = 0;
    let max_retries = if is_google_request { 3 } else { 1 };
    
    loop {
        let response_result = request_builder.try_clone().unwrap().send().await;
        
        match response_result {
            Ok(response) => {
                let status = response.status();
                let headers = response.headers().clone();
                
                println!("Response status: {} for {}", status, query.url);
                
                // Get response body
                let body_bytes = match response.bytes().await {
                    Ok(bytes) => bytes,
                    Err(e) => {
                        println!("Failed to read response body: {}", e);
                        return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                            "error": format!("Failed to read response body: {}", e)
                        })));
                    }
                };

                let body_str = String::from_utf8_lossy(&body_bytes);

                // Check for anti-bot responses
                if is_google_request && handle_anti_bot_response(&body_str, &session) {
                    if retry_count < max_retries {
                        retry_count += 1;
                        println!("🔄 Retry attempt {} for anti-bot response", retry_count);
                        
                        // Longer delay before retry
                        tokio::time::sleep(Duration::from_millis(rand::thread_rng().gen_range(3000..8000))).await;
                        continue;
                    } else {
                        println!("❌ Max retries reached for anti-bot response");
                    }
                }

                // Update session with this activity
                update_advanced_session(&session_storage, &session_id, &session, &query.url);
                
                // Log response details
                if status.as_u16() == 429 || status.as_u16() == 403 {
                    println!("⚠️  Potential bot detection: status {}", status);
                } else if status.is_success() {
                    println!("✅ Request successful: status {}", status);
                } else {
                    println!("⚠️  Unexpected status: {}", status);
                }
                
                // Check for Google's specific responses
                if query.url.contains("google.com") {
                    if let Some(content_type) = headers.get("content-type") {
                        if let Ok(ct) = content_type.to_str() {
                            if ct.contains("text/html") {
                                if body_str.contains("<h3") {
                                    println!("🎉 Found search result headers (h3 tags)!");
                                } else if body_str.contains("javascript") || body_str.contains("click here") {
                                    println!("🤖 Likely bot detection page (contains JavaScript requirements)");
                                } else {
                                    println!("❓ Unknown HTML response type");
                                }
                            }
                        }
                    }
                }
                
                // Create response builder with the same status
                let mut response_builder = HttpResponse::build(
                    actix_web::http::StatusCode::from_u16(status.as_u16()).unwrap()
                );

                // Forward response headers (excluding hop-by-hop headers)
                for (header_name, header_value) in &headers {
                    let name = header_name.as_str().to_lowercase();
                    
                    // Skip hop-by-hop headers and headers that actix-web manages
                    if !matches!(name.as_str(),
                        "connection" | "proxy-connection" | "te" | "trailer" | 
                        "transfer-encoding" | "upgrade" | "content-encoding"
                    ) {
                        if let Ok(value) = header_value.to_str() {
                            response_builder.insert_header((header_name.as_str(), value));
                        }
                    }
                }

                println!("Response body length: {} bytes", body_bytes.len());
                return Ok(response_builder.body(body_bytes));
            }
            Err(e) => {
                if retry_count < max_retries && is_google_request {
                    retry_count += 1;
                    println!("🔄 Network retry attempt {} due to error: {}", retry_count, e);
                    tokio::time::sleep(Duration::from_millis(rand::thread_rng().gen_range(2000..5000))).await;
                    continue;
                } else {
                    println!("Failed to proxy request: {}", e);
                    return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                        "error": format!("Failed to proxy request: {}", e)
                    })));
                }
            }
        }
    }
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
                    
                    // Generate PQC session ID and get public keys
                    let pqc_session_id = generate_session_id();
                    let (kyber_pk, dilithium_pk, sphincs_pk) = PQC_INSTANCE.get_public_keys();
                    let pqc_public_keys = PqcPublicKeys {
                        kyber_pk,
                        dilithium_pk,
                        sphincs_pk,
                    };
                    
                    let proxy_response = ProxyResponse {
                        html,
                        status,
                        server_ip,
                        pqc_session_id,
                        pqc_public_keys,
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

async fn pqc_proxy_handler(req: web::Json<PqcProxyRequest>) -> Result<HttpResponse> {
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .unwrap();

    println!("PQC Proxy: Fetching URL: {}", req.url);

    // If peer public keys are provided, establish secure session
    let mut encryption_key = None;
    if let Some(peer_keys) = &req.peer_public_keys {
        println!("Establishing PQC secure session...");
        match PQC_INSTANCE.kyber_encapsulate(&peer_keys.kyber_pk) {
            Ok((shared_secret, _)) => {
                println!("✓ Kyber key encapsulation successful");
                encryption_key = Some(shared_secret);
            }
            Err(e) => {
                println!("⚠ PQC key encapsulation failed: {}", e);
            }
        }
    }

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
                    // Fix relative URLs to absolute URLs (same as before)
                    let base_url = &req.url;
                    if let Ok(parsed_url) = url::Url::parse(base_url) {
                        let origin = format!("{}://{}", parsed_url.scheme(), parsed_url.host_str().unwrap_or(""));
                        
                        html = html.replace("href=\"/", &format!("href=\"{}/", origin));
                        html = html.replace("src=\"/", &format!("src=\"{}/", origin));
                        html = html.replace("action=\"/", &format!("action=\"{}/", origin));
                        html = html.replace("url(/_next/", &format!("url({}//_next/", origin));
                        html = html.replace("url(/", &format!("url({}/", origin));
                        html = html.replace("href=\"//", "href=\"https://");
                        html = html.replace("src=\"//", "src=\"https://");
                    }
                    
                    // Apply PQC encryption if secure session established
                    let processed_html = if let Some(key) = encryption_key {
                        println!("🔒 Applying PQC encryption to HTML content");
                        match PQC_INSTANCE.symmetric_encrypt(html.as_bytes(), &key) {
                            Ok(encrypted) => {
                                println!("✓ HTML content encrypted with PQC");
                                encrypted
                            }
                            Err(e) => {
                                println!("⚠ PQC encryption failed: {}", e);
                                html // Fallback to unencrypted
                            }
                        }
                    } else {
                        html
                    };
                    
                    println!("Processed content length: {} chars", processed_html.len());
                    
                    let server_ip = get_public_ip().await;
                    let pqc_session_id = generate_session_id();
                    let (kyber_pk, dilithium_pk, sphincs_pk) = PQC_INSTANCE.get_public_keys();
                    
                    // Create digital signature of the content hash for integrity
                    let content_hash = PQC_INSTANCE.hash_data(processed_html.as_bytes());
                    let content_signature = match PQC_INSTANCE.dilithium_sign(content_hash.as_bytes()) {
                        Ok(sig) => sig,
                        Err(e) => {
                            println!("⚠ Failed to sign content: {}", e);
                            String::new()
                        }
                    };
                    
                    let pqc_public_keys = PqcPublicKeys {
                        kyber_pk,
                        dilithium_pk,
                        sphincs_pk,
                    };
                    
                    let proxy_response = ProxyResponse {
                        html: processed_html,
                        status,
                        server_ip,
                        pqc_session_id,
                        pqc_public_keys,
                    };
                    
                    // Add PQC signature to response headers
                    Ok(HttpResponse::Ok()
                        .insert_header(("X-PQC-Content-Hash", content_hash))
                        .insert_header(("X-PQC-Content-Signature", content_signature))
                        .insert_header(("X-PQC-Enabled", "true"))
                        .json(proxy_response))
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

async fn pqc_handshake(req: web::Json<PqcPublicKeys>) -> Result<HttpResponse> {
    println!("🤝 PQC Handshake initiated");
    
    // Create secure session with the provided public key
    match PQC_INSTANCE.create_secure_session(&req.kyber_pk) {
        Ok(session_data) => {
            let (kyber_pk, dilithium_pk, sphincs_pk) = PQC_INSTANCE.get_public_keys();
            let public_keys = PqcPublicKeys {
                kyber_pk,
                dilithium_pk,
                sphincs_pk,
            };
            
            let response = PqcResponse {
                session_data,
                public_keys,
            };
            
            println!("✓ PQC Handshake completed successfully");
            Ok(HttpResponse::Ok().json(response))
        }
        Err(e) => {
            println!("⚠ PQC Handshake failed: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("PQC handshake failed: {}", e)
            })))
        }
    }
}

async fn pqc_info() -> Result<HttpResponse> {
    let (kyber_pk, dilithium_pk, sphincs_pk) = PQC_INSTANCE.get_public_keys();
    
    let info = serde_json::json!({
        "pqc_enabled": true,
        "algorithms": {
            "key_encapsulation": "Kyber-768",
            "signature_primary": "Dilithium-3", 
            "signature_alternative": "SPHINCS+-SHA256-128s-simple",
            "hash": "SHA3-256"
        },
        "public_keys": {
            "kyber": kyber_pk,
            "dilithium": dilithium_pk,
            "sphincs": sphincs_pk
        },
        "description": "Post-Quantum Cryptography enabled proxy server using NIST-approved algorithms"
    });
    
    Ok(HttpResponse::Ok().json(info))
}

// Enhanced anti-bot evasion strategies
use std::time::Instant;

#[derive(Debug, Clone)]
struct AdvancedSession {
    session_id: String,
    cookies: Arc<reqwest::cookie::Jar>,
    fingerprint: BrowserFingerprint,
    visited_urls: Vec<String>,
    interaction_history: Vec<String>,
    session_start: u64,
    last_activity: Instant,
    request_count: u32,
    success_count: u32,
    google_tokens: Vec<String>,
    captcha_attempts: u32,
    preferred_languages: Vec<String>,
    screen_resolution: String,
    timezone: String,
    connection_downlink: String,
}

impl AdvancedSession {
    fn new(session_id: String) -> Self {
        let fingerprint = BrowserFingerprint::new();
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        
        AdvancedSession {
            session_id,
            cookies: Arc::new(reqwest::cookie::Jar::default()),
            fingerprint,
            visited_urls: Vec::new(),
            interaction_history: Vec::new(),
            session_start: now,
            last_activity: Instant::now(),
            request_count: 0,
            success_count: 0,
            google_tokens: Vec::new(),
            captcha_attempts: 0,
            preferred_languages: vec!["en-US".to_string(), "en".to_string()],
            screen_resolution: "1920x1080".to_string(),
            timezone: "America/New_York".to_string(),
            connection_downlink: "10".to_string(),
        }
    }

    fn update_activity(&mut self) {
        self.last_activity = Instant::now();
        self.request_count += 1;
    }

    fn add_success(&mut self) {
        self.success_count += 1;
    }

    fn get_success_rate(&self) -> f32 {
        if self.request_count == 0 { 1.0 } else { self.success_count as f32 / self.request_count as f32 }
    }

    fn is_suspicious(&self) -> bool {
        // Check if session appears automated
        let elapsed = self.last_activity.elapsed().as_secs();
        let rate = self.request_count as f64 / (elapsed as f64 + 1.0);
        
        // Too many requests too quickly, or very low success rate
        rate > 5.0 || (self.request_count > 10 && self.get_success_rate() < 0.3)
    }

    fn needs_cooling_off(&self) -> bool {
        self.captcha_attempts > 2 || self.is_suspicious()
    }
}

// Helper function to generate realistic forwarded IPs
fn generate_realistic_forwarded_ip(original_ip: &str) -> String {
    // Generate IPs from common residential/business ranges
    let ranges = [
        // Common residential ranges
        ("192.168.", 1, 255, 1, 254),
        ("10.", 0, 255, 0, 255),
        ("172.", 16, 31, 0, 255),
        // Common business ranges  
        ("203.", 0, 255, 0, 255),
        ("74.", 125, 127, 0, 255),
        ("98.", 138, 140, 0, 255),
    ];
    
    if rand::thread_rng().gen_bool(0.8) {
        // 80% chance to use a realistic IP range
        let range = ranges[rand::thread_rng().gen_range(0..ranges.len())];
        let (prefix, start1, end1, start2, end2) = range;
        
        if prefix.ends_with('.') {
            // Two more octets needed
            format!("{}{}.{}.{}", 
                prefix,
                rand::thread_rng().gen_range(start1..=end1),
                rand::thread_rng().gen_range(start2..=end2),
                rand::thread_rng().gen_range(1..=254)
            )
        } else {
            original_ip.to_string()
        }
    } else {
        // 20% chance to slightly modify the original IP
        let ip_parts: Vec<&str> = original_ip.split('.').collect();
        if ip_parts.len() == 4 {
            if let (Ok(a), Ok(b), Ok(c), Ok(d)) = (
                ip_parts[0].parse::<u8>(),
                ip_parts[1].parse::<u8>(),
                ip_parts[2].parse::<u8>(),
                ip_parts[3].parse::<u8>(),
            ) {
                let new_d = d.wrapping_add(rand::thread_rng().gen_range(0..5));
                format!("{}.{}.{}.{}", a, b, c, new_d)
            } else {
                original_ip.to_string()
            }
        } else {
            original_ip.to_string()
        }
    }
}

// Advanced session management functions
fn get_or_create_advanced_session(session_storage: &SessionStorage, session_id: &str) -> Arc<Mutex<AdvancedSession>> {
    let mut storage = session_storage.lock().unwrap();
    
    // Clean up old sessions periodically
    let now = Instant::now();
    storage.retain(|_, session_arc| {
        if let Ok(session) = session_arc.lock() {
            now.duration_since(session.last_activity).as_secs() < 3600 // Keep for 1 hour
        } else {
            false
        }
    });
    
    storage.entry(session_id.to_string())
        .or_insert_with(|| Arc::new(Mutex::new(AdvancedSession::new(session_id.to_string()))))
        .clone()
}

fn update_advanced_session(session_storage: &SessionStorage, session_id: &str, session: &Arc<Mutex<AdvancedSession>>, url: &str) {
    let mut session_guard = session.lock().unwrap();
    session_guard.visited_urls.push(url.to_string());
    session_guard.interaction_history.push(format!("visited: {}", url));
    
    // Keep only recent history to prevent memory bloat
    if session_guard.visited_urls.len() > 50 {
        session_guard.visited_urls.drain(0..10);
    }
    if session_guard.interaction_history.len() > 100 {
        session_guard.interaction_history.drain(0..20);
    }
}

// Missing functions implementation

// Smart delay function
async fn smart_delay(session: &Arc<Mutex<AdvancedSession>>) {
    let (needs_cooling, request_count) = {
        let session_guard = session.lock().unwrap();
        (session_guard.needs_cooling_off(), session_guard.request_count)
    };
    
    let base_delay = if needs_cooling {
        // Longer delay if we're being suspicious
        rand::thread_rng().gen_range(3000..8000)
    } else if request_count % 5 == 0 {
        // Occasional longer delay to simulate reading
        rand::thread_rng().gen_range(2000..5000)
    } else {
        // Normal human delay
        rand::thread_rng().gen_range(800..2500)
    };
    
    println!("⏳ Smart delay: {}ms", base_delay);
    tokio::time::sleep(Duration::from_millis(base_delay)).await;
}

// Handle anti-bot response function
fn handle_anti_bot_response(body: &str, session: &Arc<Mutex<AdvancedSession>>) -> bool {
    let mut session_guard = session.lock().unwrap();
    
    if body.contains("captcha") || body.contains("CAPTCHA") {
        session_guard.captcha_attempts += 1;
        println!("🤖 CAPTCHA detected - attempt #{}", session_guard.captcha_attempts);
        return true;
    }
    
    if body.contains("unusual traffic") || body.contains("automated") {
        println!("🚫 Automated traffic detection");
        return true;
    }
    
    if body.contains("Please click") && body.contains("redirected") {
        println!("🔄 Redirect challenge detected");
        return true;
    }
    
    // Check for successful search results
    if body.contains("<h3") || body.contains("search-result") {
        session_guard.add_success();
        println!("✅ Successful search results detected");
        return false;
    }
    
    false
}

// Generate realistic headers v2 function
fn generate_realistic_headers_v2(session: &AdvancedSession, url: &str, is_mobile: bool) -> HashMap<String, String> {
    let mut headers = HashMap::new();
    
    // Advanced User-Agent rotation based on real browser statistics
    let user_agents = if is_mobile {
        vec![
            "Mozilla/5.0 (iPhone; CPU iPhone OS 17_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.1 Mobile/15E148 Safari/604.1",
            "Mozilla/5.0 (Linux; Android 14; SM-G998B) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Mobile Safari/537.36",
            "Mozilla/5.0 (Linux; Android 13; Pixel 7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/118.0.0.0 Mobile Safari/537.36",
        ]
    } else {
        vec![
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36",
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/118.0.0.0 Safari/537.36 Edg/118.0.2088.76",
        ]
    };
    
    let ua_index = (session.session_start % user_agents.len() as u64) as usize;
    headers.insert("User-Agent".to_string(), user_agents[ua_index].to_string());
    
    // Advanced Accept headers that match real browsers
    headers.insert("Accept".to_string(), 
        "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7".to_string());
    
    headers.insert("Accept-Language".to_string(), 
        session.preferred_languages.join(","));
    
    headers.insert("Accept-Encoding".to_string(), "gzip, deflate, br".to_string());
    
    headers.insert("DNT".to_string(), "1".to_string());
    headers.insert("Connection".to_string(), "keep-alive".to_string());
    headers.insert("Upgrade-Insecure-Requests".to_string(), "1".to_string());
    
    // Add realistic Sec-CH headers for modern browsers
    if !is_mobile {
        headers.insert("sec-ch-ua".to_string(), 
            "\"Google Chrome\";v=\"119\", \"Chromium\";v=\"119\", \"Not?A_Brand\";v=\"24\"".to_string());
        headers.insert("sec-ch-ua-mobile".to_string(), "?0".to_string());
        headers.insert("sec-ch-ua-platform".to_string(), "\"Windows\"".to_string());
        headers.insert("sec-ch-ua-platform-version".to_string(), "\"15.0.0\"".to_string());
        headers.insert("sec-ch-ua-arch".to_string(), "\"x86\"".to_string());
        headers.insert("sec-ch-ua-bitness".to_string(), "\"64\"".to_string());
        headers.insert("sec-ch-ua-model".to_string(), "\"\"".to_string());
        headers.insert("sec-ch-ua-full-version-list".to_string(), 
            "\"Google Chrome\";v=\"119.0.6045.160\", \"Chromium\";v=\"119.0.6045.160\", \"Not?A_Brand\";v=\"24.0.0.0\"".to_string());
    }
    
    // Add Google-specific headers
    if url.contains("google.com") {
        headers.insert("sec-fetch-site".to_string(), "same-origin".to_string());
        headers.insert("sec-fetch-mode".to_string(), "navigate".to_string());
        headers.insert("sec-fetch-user".to_string(), "?1".to_string());
        headers.insert("sec-fetch-dest".to_string(), "document".to_string());
        
        // Add cache control for Google
        headers.insert("Cache-Control".to_string(), "max-age=0".to_string());
        
        // Add Google's client data header
        use base64::{Engine as _, engine::general_purpose};
        let client_data = general_purpose::STANDARD.encode(format!("session={}", &session.session_id[0..8]));
        headers.insert("X-Client-Data".to_string(), client_data);
    }
    
    // Add referer based on session history
    if !session.visited_urls.is_empty() {
        let last_url = session.visited_urls.last().unwrap();
        if last_url.contains("google.com") || url.contains("google.com") {
            headers.insert("Referer".to_string(), "https://www.google.com/".to_string());
        }
    }
    
    headers
}

// Simulate advanced browsing behavior
async fn simulate_advanced_browsing_behavior(session: &Arc<Mutex<AdvancedSession>>, target_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = {
        let session_guard = session.lock().unwrap();
        Client::builder()
            .timeout(Duration::from_secs(30))
            .cookie_provider(session_guard.cookies.clone())
            .danger_accept_invalid_certs(false)
            .tcp_keepalive(Duration::from_secs(60))
            .pool_idle_timeout(Duration::from_secs(90))
            .pool_max_idle_per_host(10)
            .http1_only()
            .build()
            .unwrap()
    };

    if target_url.contains("google.com") {
        // 1. Visit Google homepage first if not already visited
        {
            let session_guard = session.lock().unwrap();
            if !session_guard.visited_urls.iter().any(|u| u.contains("google.com")) {
                drop(session_guard);
                
                println!("🌐 Simulating Google homepage visit...");
                let headers = {
                    let session_guard = session.lock().unwrap();
                    generate_realistic_headers_v2(&session_guard, "https://www.google.com", false)
                };
                
                let mut request = client.get("https://www.google.com");
                for (name, value) in headers {
                    request = request.header(&name, &value);
                }
                
                if let Ok(response) = request.send().await {
                    let mut session_guard = session.lock().unwrap();
                    session_guard.visited_urls.push("https://www.google.com".to_string());
                    session_guard.update_activity();
                    
                    if response.status().is_success() {
                        session_guard.add_success();
                        println!("✅ Google homepage visit successful");
                    }
                    
                    // Extract any tokens or cookies for later use
                    let body = response.text().await.unwrap_or_default();
                    if let Some(start) = body.find("\"FPB\":\"") {
                        if let Some(end) = body[start+7..].find("\"") {
                            let token = &body[start+7..start+7+end];
                            session_guard.google_tokens.push(token.to_string());
                        }
                    }
                }
                
                // Human-like delay
                tokio::time::sleep(Duration::from_millis(rand::thread_rng().gen_range(800..2000))).await;
            }
        }

        // 2. Simulate search suggestions request
        if target_url.contains("/search?q=") {
            if let Some(query_start) = target_url.find("q=") {
                let query_part = &target_url[query_start+2..];
                let query = query_part.split('&').next().unwrap_or("");
                let decoded_query = urlencoding::decode(query).unwrap_or_default();
                
                // Simulate typing the query gradually with suggestions
                for i in 1..=decoded_query.len().min(8) {
                    let partial_query = &decoded_query[0..i];
                    let suggest_url = format!("https://suggestqueries.google.com/complete/search?client=chrome&q={}", urlencoding::encode(partial_query));
                    
                    let headers = {
                        let session_guard = session.lock().unwrap();
                        generate_realistic_headers_v2(&session_guard, &suggest_url, false)
                    };
                    
                    let mut request = client.get(&suggest_url);
                    for (name, value) in headers {
                        request = request.header(&name, &value);
                    }
                    
                    let _ = request.send().await;
                    
                    // Typing speed simulation
                    tokio::time::sleep(Duration::from_millis(rand::thread_rng().gen_range(100..400))).await;
                }
                
                println!("⌨️ Simulated typing behavior for query: {}", decoded_query);
            }
        }
    }

    Ok(())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Server starting...");
    
    // Initialize session storage
    let session_storage: SessionStorage = Arc::new(Mutex::new(HashMap::new()));
    
    // Create and start HTTP server
    actix_web::HttpServer::new(move || {
        actix_web::App::new()
            .app_data(actix_web::web::Data::new(session_storage.clone()))
            .route("/proxy", actix_web::web::get().to(proxy))
            .route("/proxy", actix_web::web::post().to(proxy))
            .route("/pqc_info", actix_web::web::get().to(pqc_info))
            .route("/pqc-info", actix_web::web::get().to(pqc_info))  // Extension compatibility
            .route("/pqc_handshake", actix_web::web::post().to(pqc_handshake))
            .route("/", actix_web::web::get().to(|| async {
                actix_web::HttpResponse::Ok().body("VPN Server with PQC - Proxy available at /proxy")
            }))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
