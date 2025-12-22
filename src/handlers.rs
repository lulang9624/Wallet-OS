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
};
use serde::{Deserialize, Serialize};
use tracing::{info, warn, debug};

#[derive(Deserialize)]
pub struct SearchQuery {
    q: String,
}

#[derive(Serialize)]
pub struct SearchResult {
    domain: String,
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
pub async fn search_domain(
    Query(params): Query<SearchQuery>,
) -> Result<Json<SearchResult>, String> {
    let query = params.q.trim();
    if query.is_empty() {
        return Err("Query is empty".to_string());
    }

    info!("Searching for: {}", query);

    // 1. 尝试 DuckDuckGo API (JSON)
    // Try DuckDuckGo API (JSON) first
    let api_url = format!("https://api.duckduckgo.com/?q={}&format=json", urlencoding::encode(query));
    
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
        .build()
        .map_err(|e| e.to_string())?;

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
                                    return Ok(Json(SearchResult {
                                        domain: domain.to_string(),
                                    }));
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
                                        return Ok(Json(SearchResult {
                                            domain: domain.to_string(),
                                        }));
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
                                            return Ok(Json(SearchResult {
                                                domain: domain.to_string(),
                                            }));
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
    let resp = client.get(&url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
        .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8")
        .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
        .header("Referer", "https://html.duckduckgo.com/")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;
    
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
                    return Ok(Json(SearchResult {
                        domain: domain.to_string(),
                    }));
                }
            }
        }
    }

    Err("No domain found".to_string())
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
        INSERT INTO subscriptions (name, price, currency, next_payment, frequency, url, logo)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        "#
    )
    .bind(&payload.name)
    .bind(price)
    .bind(&payload.currency)
    .bind(&next_payment)
    .bind(payload.frequency)
    .bind(&payload.url)
    .bind(&payload.logo)
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
        active: true, // 默认为激活状态 Default to active
    };

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
        SET name = ?, price = ?, currency = ?, next_payment = ?, frequency = ?, url = ?, logo = ?
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
        active: true,
    };

    Ok(Json(sub))
}
