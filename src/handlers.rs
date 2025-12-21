/// HTTP 请求处理模块
/// HTTP Request Handlers Module
///
/// 包含所有 API 接口的具体实现逻辑。
/// Contains implementation logic for all API endpoints.

use crate::db::DbPool;
use crate::models::{CreateSubscription, Subscription};
use axum::{
    extract::{Path, State},
    Json,
};

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
    .bind(payload.price)
    .bind(&payload.currency)
    .bind(&payload.next_payment)
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
        price: payload.price,
        currency: payload.currency,
        next_payment: Some(payload.next_payment),
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
