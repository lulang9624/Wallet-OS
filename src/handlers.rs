/// 处理 HTTP 请求的模块
/// Module for handling HTTP requests
use crate::db::DbPool;
use crate::models::{CreateSubscription, Subscription};
use axum::{
    extract::{Path, State},
    Json,
};

/// 获取所有订阅列表
/// Get list of all subscriptions
///
/// 按下次付款日期升序排序
/// Ordered by next payment date ascending
pub async fn list_subscriptions(
    State(pool): State<DbPool>,
) -> Result<Json<Vec<Subscription>>, String> {
    // 查询所有订阅数据
    // Query all subscription data
    let subs = sqlx::query_as::<_, Subscription>("SELECT * FROM subscriptions ORDER BY next_payment ASC")
        .fetch_all(&pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(Json(subs))
}

/// 创建新订阅
/// Create a new subscription
pub async fn create_subscription(
    State(pool): State<DbPool>,
    Json(payload): Json<CreateSubscription>,
) -> Result<Json<Subscription>, String> {
    // 验证必填字段
    // Validate required fields
    if payload.name.trim().is_empty() {
        return Err("Name is required".to_string());
    }

    // 插入新订阅数据到数据库
    // Insert new subscription data into database
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

    // 构建返回的订阅对象
    // Construct the returned subscription object
    let sub = Subscription {
        id,
        name: payload.name,
        price: payload.price,
        currency: payload.currency,
        next_payment: Some(payload.next_payment),
        frequency: payload.frequency,
        url: payload.url,
        logo: payload.logo,
        active: true,
    };

    Ok(Json(sub))
}

/// 删除指定 ID 的订阅
/// Delete subscription with specified ID
pub async fn delete_subscription(
    State(pool): State<DbPool>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, String> {
    // 执行删除操作
    // Execute delete operation
    sqlx::query("DELETE FROM subscriptions WHERE id = ?")
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(Json(serde_json::json!({ "status": "deleted" })))
}
