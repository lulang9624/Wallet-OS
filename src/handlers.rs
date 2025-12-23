/// HTTP 请求处理模块
/// HTTP Request Handlers Module
///
/// 包含所有 API 接口的具体实现逻辑。
/// Contains implementation logic for all API endpoints.

use crate::db::DbPool;
use crate::models::{CreateSubscription, Subscription};
use axum::{
    extract::{Path, State, Query},
    Json,
    http::StatusCode,
    response::IntoResponse,
};
use axum::response::Response;
use axum::http::header::{CONTENT_TYPE, CACHE_CONTROL};
use axum::http::HeaderValue;
use tokio::fs;
use tokio::io::AsyncReadExt;
use serde::{Deserialize, Serialize};
use tracing::{info, warn, debug};
use std::time::Duration;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use parking_lot::RwLock;
use tokio::sync::broadcast;
use axum::response::sse::{Sse, Event, KeepAlive};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use std::convert::Infallible;
use chrono::Datelike;

#[derive(Deserialize)]
pub struct SmartParseRequest {
    text: String,
}

/// 智能解析订阅信息 (POST /api/smart-parse)
/// Smart parse subscription info
#[axum::debug_handler]
pub async fn smart_parse(
    Json(payload): Json<SmartParseRequest>,
) -> impl IntoResponse {
    let text = payload.text;
    info!("Smart parse request: {}", text);

    // 1. 尝试从环境变量获取 API Key
    // 1. Try to get API Key from env vars
    let api_key = std::env::var("OPENAI_API_KEY").ok();
    let api_base = std::env::var("OPENAI_API_BASE").unwrap_or_else(|_| "https://api.openai.com/v1".to_string());

    if let Some(key) = api_key {
        // 调用 LLM API
        // Call LLM API
        let client = reqwest::Client::new();
        let model = std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-3.5-turbo".to_string());
        
        let prompts = get_prompts();
        let template = prompts.smart_parse_user_template.unwrap_or_else(|| default_prompts().smart_parse_user_template.unwrap());
        let prompt = template.replace("{text}", &text);
        debug!("smart_parse prompt loaded from prompts.json");

        let body = serde_json::json!({
            "model": model,
            "messages": [
                {"role": "system", "content": prompts.smart_parse_system.unwrap_or_else(|| default_prompts().smart_parse_system.unwrap())},
                {"role": "user", "content": prompt}
            ],
            "temperature": 0.1
        });

        match client.post(format!("{}/chat/completions", api_base))
            .header("Authorization", format!("Bearer {}", key))
            .json(&body)
            .send()
            .await 
        {
            Ok(res) => {
                if let Ok(json) = res.json::<serde_json::Value>().await {
                    if let Some(content) = json["choices"][0]["message"]["content"].as_str() {
                        // 尝试清理 markdown 代码块标记
                        // Try to clean markdown code block markers
                        let clean_content = content.trim().trim_start_matches("```json").trim_start_matches("```").trim_end_matches("```");
                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(clean_content) {
                            return Json(parsed).into_response();
                        }
                    }
                }
            }
            Err(e) => {
                warn!("LLM API call failed: {}", e);
            }
        }
    }

    // 2. 降级方案 / 演示模式 (Mock Mode)
    // 2. Fallback / Mock Mode
    // 简单的关键词匹配用于演示
    let mut mock_data = serde_json::json!({
        "name": "Unknown Subscription",
        "price": 0,
        "currency": "CNY",
        "frequency": 1,
        "start_date": "2025-12-23",
        "next_payment": "2026-01-23"
    });

    let lower_text = text.to_lowercase();
    if lower_text.contains("netflix") {
        mock_data["name"] = "Netflix".into();
        mock_data["price"] = 15.99.into();
        mock_data["currency"] = "USD".into();
    } else if lower_text.contains("spotify") {
        mock_data["name"] = "Spotify".into();
        mock_data["price"] = 10.99.into();
        mock_data["currency"] = "USD".into();
    } else if lower_text.contains("chatgpt") {
        mock_data["name"] = "ChatGPT Plus".into();
        mock_data["price"] = 20.00.into();
        mock_data["currency"] = "USD".into();
    }

    // 尝试提取数字作为价格
    // Try to extract number as price
    if let Some(price_match) = text.split_whitespace().find(|s| s.parse::<f64>().is_ok()) {
        if let Ok(p) = price_match.parse::<f64>() {
            mock_data["price"] = p.into();
        }
    }

    Json(mock_data).into_response()
}

/// 财务分析 (POST /api/analyze)
/// Financial Analysis
#[axum::debug_handler]
pub async fn analyze_spending(
    State(pool): State<DbPool>,
) -> impl IntoResponse {
    // 1. 获取所有活跃订阅
    // 1. Get all active subscriptions
    let subs = match sqlx::query_as::<_, Subscription>("SELECT * FROM subscriptions")
        .fetch_all(&pool)
        .await 
    {
        Ok(s) => s,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "DB Error").into_response(),
    };

    // 2. 构造 Prompt 数据
    // 2. Construct Prompt Data
    let mut data_str = String::new();
    for sub in &subs {
        let freq_str = match sub.frequency {
            -1 => "Daily",
            1 => "Monthly",
            3 => "Quarterly",
            12 => "Yearly",
            0 => "Lifetime",
            _ => "Unknown"
        };
        let start = sub.start_date.as_deref().unwrap_or("N/A");
        let end = sub.next_payment.as_deref().unwrap_or("N/A");
        data_str.push_str(&format!("- {} | {} | price={} {} | start={} | end={}\n", sub.name, freq_str, sub.price, sub.currency, start, end));
    }

    

    // 3. 调用 LLM (或 Mock) 获取建议文本（不让其改动金额，只做建议描述）
    // 3. Call LLM (or Mock) to get advisory text (do not change computed amounts)
    let api_key = std::env::var("OPENAI_API_KEY").ok();
    let api_base = std::env::var("OPENAI_API_BASE").unwrap_or_else(|_| "https://api.openai.com/v1".to_string());

    let advisory_text = if let Some(key) = api_key {
        let client = reqwest::Client::new();
        let model = std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-3.5-turbo".to_string());

        let prompts = get_prompts();
        let template = prompts.analyze_user_template.unwrap_or_else(|| default_prompts().analyze_user_template.unwrap());
        let prompt = template.replace("{list}", &data_str);
        debug!("analyze prompt loaded from prompts.json");

        let body = serde_json::json!({
            "model": model,
            "messages": [
                {"role": "system", "content": prompts.analyze_system.unwrap_or_else(|| default_prompts().analyze_system.unwrap())},
                {"role": "user", "content": prompt}
            ],
            "temperature": 0.3
        });

        match client.post(format!("{}/chat/completions", api_base))
            .header("Authorization", format!("Bearer {}", key))
            .json(&body)
            .send()
            .await 
        {
            Ok(res) => {
                if let Ok(json) = res.json::<serde_json::Value>().await {
                    json["choices"][0]["message"]["content"].as_str().unwrap_or("建议生成失败").to_string()
                } else {
                    "建议生成失败".to_string()
                }
            }
            Err(_) => "建议生成失败".to_string()
        }
    } else {
        // Mock 建议
        "- 检查是否存在功能重叠的订阅，避免重复付费。\n- 长期使用的工具优先考虑年度方案或一次性买断。\n- 对低使用频率的订阅进行降级或暂停。".to_string()
    };

    let final_md = format!("### 订阅优化建议\n\n{}", advisory_text);
    Json(serde_json::json!({ "analysis": final_md })).into_response()
}

/// 域名搜索结果内存缓存
/// In-memory cache for domain search results
///
/// 目的：避免对同一查询名称的重复网络请求，提升响应速度并降低外部 API 负载。
/// Purpose: Prevent duplicate network requests for the same query name, improving
/// response time and reducing external API load.
static SEARCH_CACHE: Lazy<RwLock<HashMap<String, String>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

static BROADCAST: Lazy<broadcast::Sender<()>> = Lazy::new(|| {
    let (tx, _rx) = broadcast::channel(100);
    tx
});

#[derive(Clone, Deserialize)]
struct Prompts {
    smart_parse_system: Option<String>,
    smart_parse_user_template: Option<String>,
    analyze_system: Option<String>,
    analyze_user_template: Option<String>,
}

fn default_prompts() -> Prompts {
    Prompts {
        smart_parse_system: Some("You are a helpful assistant that extracts JSON.".to_string()),
        smart_parse_user_template: Some("You are a subscription data extractor. Extract details from this text: '{text}'. Return ONLY a valid JSON object with these fields: name (string), price (number), currency (string, e.g. CNY, USD), start_date (string YYYY-MM-DD, assume today is 2025-12-23 if 'today'), next_payment (string YYYY-MM-DD, synonymous with end_date), frequency (number: -1=daily, 1=monthly, 3=quarterly, 12=yearly, 0=lifetime). If missing, guess or leave null.".to_string()),
        analyze_system: Some("You are a financial advisor.".to_string()),
        analyze_user_template: Some("作为订阅优化顾问，请仅依据下面的订阅信息给出 3–5 条中文建议（Markdown 列表）。不要进行任何金额计算或估算。关注冗余订阅、升级/降级机会、取消指引、以及临近到期的提醒。\n\n列表：\n{list}".to_string()),
    }
}

fn get_prompts() -> Prompts {
    let loaded = std::fs::read_to_string("static/prompts.json")
        .ok()
        .and_then(|s| serde_json::from_str::<Prompts>(&s).ok())
        .unwrap_or_else(|| default_prompts());
    loaded
}

#[derive(Deserialize)]
pub struct SearchQuery {
    q: String,
}

#[derive(Serialize)]
pub struct SearchResult {
    domain: String,
}

#[derive(Deserialize)]
pub struct IconQuery {
    domain: String,
    sz: Option<u32>,
}

/// 获取网站图标 (GET /api/icon?domain=example.com&sz=64)
/// Fetch website icon
///
/// 行为：
/// 1. 尝试从本地缓存目录 `static/icons` 读取，如果存在则直接返回并设置缓存头。
/// 2. 若不存在，调用 Google Favicon 服务下载 PNG，并写入本地以供后续命中。
/// 3. 所有成功响应附带 `Cache-Control`，允许前端浏览器进行缓存。
/// Behavior:
/// 1. Try reading from local cache dir `static/icons`; return if present with cache headers.
/// 2. If missing, fetch PNG via Google Favicon, then persist for future hits.
/// 3. Successful responses include `Cache-Control` for browser caching.
#[axum::debug_handler]
pub async fn get_icon(
    Query(params): Query<IconQuery>,
) -> Response {
    let mut domain = params.domain.to_lowercase();
    domain.retain(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-');
    if domain.is_empty() {
        return (StatusCode::BAD_REQUEST, "invalid domain").into_response();
    }
    let sz = params.sz.unwrap_or(64);
    let file_name = format!("{}_{}.png", domain, sz);
    let dir = "static/icons";
    let path = format!("{}/{}", dir, file_name);

    if let Ok(mut f) = fs::File::open(&path).await {
        let mut buf = Vec::new();
        if f.read_to_end(&mut buf).await.is_ok() {
            let mut resp = Response::new(buf.into());
            resp.headers_mut().insert(CONTENT_TYPE, HeaderValue::from_static("image/png"));
            resp.headers_mut().insert(CACHE_CONTROL, HeaderValue::from_static("public, max-age=604800"));
            return resp;
        }
    }

    let _ = fs::create_dir_all(dir).await;
    let url = format!("https://www.google.com/s2/favicons?domain={}&sz={}", domain, sz);
    let client = match reqwest::Client::builder()
        .user_agent("Mozilla/5.0")
        .timeout(Duration::from_secs(8))
        .build() {
        Ok(c) => c,
        Err(e) => return (StatusCode::BAD_GATEWAY, e.to_string()).into_response(),
    };

    match client.get(&url).send().await {
        Ok(resp) => {
            match resp.bytes().await {
                Ok(bytes) => {
                    let _ = fs::write(&path, &bytes).await;
                    let mut resp = Response::new(bytes.into());
                    resp.headers_mut().insert(CONTENT_TYPE, HeaderValue::from_static("image/png"));
                    resp.headers_mut().insert(CACHE_CONTROL, HeaderValue::from_static("public, max-age=604800"));
                    return resp;
                },
                Err(e) => return (StatusCode::BAD_GATEWAY, e.to_string()).into_response(),
            }
        },
        Err(e) => (StatusCode::BAD_GATEWAY, e.to_string()).into_response(),
    }
}

/// 实时更新流 (GET `/api/stream`)
/// Server-Sent Events stream
///
/// 当订阅数据发生变化时，向前端推送一个轻量事件 `"update"`，
/// 前端接收到事件后会主动调用列表刷新接口以获取最新数据。
/// Pushes a lightweight `"update"` event when subscription data changes.
/// The frontend listens to this SSE and refreshes the list on message.
#[axum::debug_handler]
pub async fn stream_updates() -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let rx = BROADCAST.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|msg| match msg {
        Ok(_) => Some(Ok(Event::default().data("update"))),
        Err(_) => None,
    });
    Sse::new(stream).keep_alive(KeepAlive::new())
}

/// 搜索域名 API (GET /api/search?q=name)
/// Search Domain API
///
/// 使用 DuckDuckGo HTML 搜索，解析结果获取官网域名。
/// Uses DuckDuckGo HTML search, parses results to get official website domain.
#[derive(serde::Deserialize)]
struct DdgResponse {
    #[serde(rename = "OfficialWebsite")]
    official_website: Option<String>,
    #[serde(rename = "Results")]
    results: Option<Vec<DdgResult>>,
    #[serde(rename = "AbstractURL")]
    abstract_url: Option<String>,
}

#[derive(serde::Deserialize)]
struct DdgResult {
    #[serde(rename = "FirstURL")]
    first_url: Option<String>,
}

/// 搜索域名 API (GET /api/search?q=name)
/// Search Domain API
///
/// 使用 DuckDuckGo API 和 HTML 搜索，解析结果获取官网域名。
/// Uses DuckDuckGo API and HTML search, parses results to get official website domain.
#[axum::debug_handler]
pub async fn search_domain(
    State(_pool): State<DbPool>,
    Query(params): Query<SearchQuery>,
) -> axum::response::Response {
    let query = params.q.trim();
    if query.is_empty() {
        return (StatusCode::BAD_REQUEST, "Query is empty".to_string()).into_response();
    }

    info!("Searching for: {}", query);

    if let Some(cached) = { SEARCH_CACHE.read().get(query).cloned() } {
        info!("Cache hit: {}", cached);
        return Json(SearchResult { domain: cached }).into_response();
    }

    // 1. 尝试 DuckDuckGo API (JSON)
    // Try DuckDuckGo API (JSON) first
    let api_url = format!("https://api.duckduckgo.com/?q={}&format=json", urlencoding::encode(query));
    
    let client = match reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
        .timeout(Duration::from_secs(8))
        .build() {
        Ok(c) => c,
        Err(e) => return (StatusCode::BAD_GATEWAY, e.to_string()).into_response(),
    };

    match client.get(&api_url).send().await {
        Ok(resp) => {
            if let Ok(text) = resp.text().await {
                // println!("API Response: {}", text); // Debug log
                if let Ok(ddg_resp) = serde_json::from_str::<DdgResponse>(&text) {
                    // 优先级 1: OfficialWebsite
                    if let Some(url) = ddg_resp.official_website {
                        if !url.is_empty() {
                            if let Ok(parsed) = url::Url::parse(&url) {
                                if let Some(domain) = parsed.host_str() {
                                    info!("Found via API (OfficialWebsite): {}", domain);
                                    { SEARCH_CACHE.write().insert(query.to_string(), domain.to_string()); }
                                    return Json(SearchResult { domain: domain.to_string() }).into_response();
                                }
                            }
                        }
                    }

                    // 优先级 2: AbstractURL
                    if let Some(url) = ddg_resp.abstract_url {
                         if !url.is_empty() {
                            if let Ok(parsed) = url::Url::parse(&url) {
                                if let Some(domain) = parsed.host_str() {
                                    if !domain.contains("wikipedia.org") {
                                        info!("Found via API (AbstractURL): {}", domain);
                                        { SEARCH_CACHE.write().insert(query.to_string(), domain.to_string()); }
                                        return Json(SearchResult { domain: domain.to_string() }).into_response();
                                    }
                                }
                            }
                         }
                    }

                    // 优先级 3: Results 中的 FirstURL
                    if let Some(results) = ddg_resp.results {
                        for result in results {
                            if let Some(url) = result.first_url {
                                if !url.is_empty() {
                                    if let Ok(parsed) = url::Url::parse(&url) {
                                        if let Some(domain) = parsed.host_str() {
                                            info!("Found via API (Results): {}", domain);
                                            { SEARCH_CACHE.write().insert(query.to_string(), domain.to_string()); }
                                            return Json(SearchResult { domain: domain.to_string() }).into_response();
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        },
        Err(e) => warn!("API request failed: {}", e),
    }

    info!("API failed to find domain, falling back to HTML search...");

    // 2. 回退到 DuckDuckGo HTML 搜索
    // Fallback to DuckDuckGo HTML search
    let url = format!("https://html.duckduckgo.com/html/?q={}", urlencoding::encode(query));
    debug!("Searching: {}", url);

    // 发送请求
    let resp = match client.get(&url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
        .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8")
        .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
        .header("Referer", "https://html.duckduckgo.com/")
        .send()
        .await {
        Ok(r) => r,
        Err(e) => return (StatusCode::BAD_GATEWAY, e.to_string()).into_response(),
    };
    let resp = match resp.text().await {
        Ok(t) => t,
        Err(e) => return (StatusCode::BAD_GATEWAY, e.to_string()).into_response(),
    };
    
    debug!("Response length: {}", resp.len());
    // if resp.len() < 1000 {
    //      println!("Response body: {}", resp);
    // }

    // 3. 解析 HTML 提取第一个非广告链接
    let document = scraper::Html::parse_document(&resp);
    // 尝试更广泛的选择器
    let selector = scraper::Selector::parse(".result__a, .result__url, .links_main a").unwrap();

    for element in document.select(&selector) {
        if let Some(href) = element.value().attr("href") {
            debug!("Found candidate link: {}", href);
            
            let actual_url = if href.starts_with("/l/") {
                if let Some(start) = href.find("uddg=") {
                    let encoded = &href[start + 5..];
                    let end = encoded.find('&').unwrap_or(encoded.len());
                    if let Ok(decoded) = urlencoding::decode(&encoded[..end]) {
                         decoded.into_owned()
                    } else {
                        continue;
                    }
                } else {
                    continue; 
                }
            } else {
                href.to_string()
            };

            // 提取域名
            if let Ok(parsed_url) = url::Url::parse(&actual_url) {
                if let Some(domain) = parsed_url.host_str() {
                    if domain.contains("duckduckgo.com") || parsed_url.scheme() != "http" && parsed_url.scheme() != "https" {
                        continue;
                    }
                    
                    info!("Found domain: {}", domain);
                    { SEARCH_CACHE.write().insert(query.to_string(), domain.to_string()); }
                    return Json(SearchResult { domain: domain.to_string() }).into_response();
                }
            }
        }
    }

    (StatusCode::NOT_FOUND, "No domain found".to_string()).into_response()
}

/// 获取所有订阅列表 (GET /api/subscriptions)
/// Get list of all subscriptions
///
/// 从数据库中查询所有订阅记录，并按“下次付款日期”升序排列。
/// Queries all subscription records from the database, ordered by "next payment date" ascending.
pub async fn list_subscriptions(
    // 从应用状态中提取数据库连接池
    // Extract database connection pool from application state
    State(pool): State<DbPool>,
) -> Result<Json<Vec<Subscription>>, String> {
    // 执行 SQL 查询
    // Execute SQL query
    // query_as 将查询结果映射为 Subscription 结构体
    // query_as maps query results to Subscription struct
    let subs = sqlx::query_as::<_, Subscription>("SELECT * FROM subscriptions ORDER BY next_payment ASC")
        .fetch_all(&pool)
        .await
        .map_err(|e| e.to_string())?;

    // 返回 JSON 格式的数据
    // Return data in JSON format
    Ok(Json(subs))
}

/// 创建新订阅 (POST /api/subscriptions)
/// Create a new subscription
///
/// 接收 JSON 格式的订阅数据，验证必填字段，并将其保存到数据库。
/// Receives subscription data in JSON format, validates required fields, and saves to database.
pub async fn create_subscription(
    State(pool): State<DbPool>,
    // 解析请求体中的 JSON 数据
    // Parse JSON data from request body
    Json(payload): Json<CreateSubscription>,
) -> Result<Json<Subscription>, String> {
    // 1. 数据验证
    //    Data Validation
    //    确保订阅名称不为空
    //    Ensure subscription name is not empty
    if payload.name.trim().is_empty() {
        return Err("Name is required".to_string());
    }

    // 处理价格和日期逻辑
    // Handle price and date logic
    if ![-1,0,1,3,12].contains(&payload.frequency) {
        return Err("Invalid frequency".to_string());
    }
    let (price, next_payment) = if payload.frequency == 0 {
        // 永久订阅：价格可选 (默认为 0)，无需下次付款日期
        // Lifetime: Price optional (default 0), no next payment date
        (payload.price.unwrap_or(0.0), None)
    } else {
        // 普通订阅：价格和日期必填
        // Normal: Price and Date required
        if payload.price.is_none() {
            return Err("Price is required for non-lifetime subscriptions".to_string());
        }
        if payload.next_payment.is_none() {
            return Err("Next payment date is required for non-lifetime subscriptions".to_string());
        }
        (payload.price.unwrap(), payload.next_payment)
    };

    // 2. 插入数据库
    //    Insert into database
    //    执行 INSERT 语句并获取新生成的 ID
    //    Execute INSERT statement and get the newly generated ID
    let id = sqlx::query(
        r#"
        INSERT INTO subscriptions (name, price, currency, next_payment, frequency, url, logo, start_date)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#
    )
    .bind(&payload.name)
    .bind(price)
    .bind(&payload.currency)
    .bind(&next_payment)
    .bind(payload.frequency)
    .bind(&payload.url)
    .bind(&payload.logo)
    .bind(&payload.start_date)
    .execute(&pool)
    .await
    .map_err(|e| e.to_string())?
    .last_insert_rowid();

    // 3. 构建返回对象
    //    Construct response object
    //    将插入的数据和新 ID 组合成完整的 Subscription 对象返回
    //    Combine inserted data and new ID into a full Subscription object to return
    let sub = Subscription {
        id,
        name: payload.name,
        price,
        currency: payload.currency,
        next_payment,
        frequency: payload.frequency,
        url: payload.url,
        logo: payload.logo,
        start_date: payload.start_date,
        active: true, // 默认为激活状态 Default to active
    };

    let _ = BROADCAST.send(());
    Ok(Json(sub))
}

/// 删除指定订阅 (DELETE /api/subscriptions/:id)
/// Delete specific subscription
///
/// 根据路径参数中的 ID 删除对应的订阅记录。
/// Deletes the subscription record corresponding to the ID in the path parameter.
pub async fn delete_subscription(
    State(pool): State<DbPool>,
    // 从 URL 路径中提取 ID 参数
    // Extract ID parameter from URL path
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, String> {
    // 执行删除操作
    // Execute delete operation
    sqlx::query("DELETE FROM subscriptions WHERE id = ?")
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|e| e.to_string())?;

    // 返回简单的成功状态 JSON
    // Return simple success status JSON
    let _ = BROADCAST.send(());
    Ok(Json(serde_json::json!({ "status": "deleted" })))
}

/// 更新指定订阅 (PUT /api/subscriptions/:id)
/// Update specific subscription
pub async fn update_subscription(
    State(pool): State<DbPool>,
    Path(id): Path<i64>,
    Json(payload): Json<CreateSubscription>,
) -> Result<Json<Subscription>, String> {
    // 1. 数据验证 (与 Create 逻辑相同)
    if payload.name.trim().is_empty() {
        return Err("Name is required".to_string());
    }

    // 处理价格和日期逻辑
    if ![-1,0,1,3,12].contains(&payload.frequency) {
        return Err("Invalid frequency".to_string());
    }
    let (price, next_payment) = if payload.frequency == 0 {
        (payload.price.unwrap_or(0.0), None)
    } else {
        if payload.price.is_none() {
            return Err("Price is required for non-lifetime subscriptions".to_string());
        }
        if payload.next_payment.is_none() {
            return Err("Next payment date is required for non-lifetime subscriptions".to_string());
        }
        (payload.price.unwrap(), payload.next_payment)
    };

    // 2. 更新数据库
    let result = sqlx::query(
        r#"
        UPDATE subscriptions 
        SET name = ?, price = ?, currency = ?, next_payment = ?, frequency = ?, url = ?, logo = ?, start_date = ?
        WHERE id = ?
        "#
    )
    .bind(&payload.name)
    .bind(price)
    .bind(&payload.currency)
    .bind(&next_payment)
    .bind(payload.frequency)
    .bind(&payload.url)
    .bind(&payload.logo)
    .bind(&payload.start_date)
    .bind(id)
    .execute(&pool)
    .await
    .map_err(|e| e.to_string())?;

    if result.rows_affected() == 0 {
        return Err("Subscription not found".to_string());
    }

    // 3. 返回更新后的对象
    let sub = Subscription {
        id,
        name: payload.name,
        price,
        currency: payload.currency,
        next_payment,
        frequency: payload.frequency,
        url: payload.url,
        logo: payload.logo,
        start_date: payload.start_date,
        active: true,
    };

    let _ = BROADCAST.send(());
    Ok(Json(sub))
}
