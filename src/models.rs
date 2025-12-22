/// 数据模型定义模块
/// Data models definition module
/// 
/// 本模块定义了应用程序中使用的数据结构，包括对应数据库表的结构体 (Entity)
/// 和用于 API 请求的传输对象 (DTO)。
/// This module defines the data structures used in the application, including structs
/// corresponding to database tables (Entities) and Data Transfer Objects (DTOs) for API requests.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 订阅模型结构体
/// Subscription model struct
///
/// 对应数据库中的 `subscriptions` 表。实现了 `FromRow` 以便从数据库查询结果自动映射，
/// 实现了 `Serialize` 和 `Deserialize` 以便进行 JSON 序列化和反序列化。
/// Corresponds to the `subscriptions` table in the database. Implements `FromRow` for 
/// automatic mapping from database query results, and `Serialize`/`Deserialize` for JSON handling.
#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct Subscription {
    /// 唯一标识符 (Primary Key)
    /// Unique identifier
    pub id: i64,
    
    /// 订阅名称 (例如: Netflix, Spotify)
    /// Subscription name
    pub name: String,
    
    /// 价格
    /// Price
    pub price: f64,
    
    /// 货币类型 (例如: USD, EUR, CNY)
    /// Currency type
    pub currency: String,
    
    /// 下次付款日期 (格式: YYYY-MM-DD)
    /// Next payment date
    pub next_payment: Option<String>,
    
    /// 付款频率
    /// 1 = 月付 (Monthly)
    /// 12 = 年付 (Yearly)
    /// Payment frequency
    pub frequency: i64,
    
    /// 官网链接 (可选)
    /// Official website URL (optional)
    pub url: Option<String>,
    
    /// Logo 图片链接 (可选)
    /// Logo image URL (optional)
    pub logo: Option<String>,
    
    /// 订阅开始日期 (格式: YYYY-MM-DD, 可选)
    /// Subscription start date (Format: YYYY-MM-DD, optional)
    pub start_date: Option<String>,

    /// 是否处于激活状态 (true = 激活, false = 停用)
    /// Whether it is active
    pub active: bool,
}

/// 创建订阅请求载荷结构体
/// Create Subscription Request Payload Struct
///
/// 用于接收前端 `POST /api/subscriptions` 请求提交的 JSON 数据。
/// 不需要 `id` 和 `active` 字段，因为这些在创建时会自动生成或设为默认值。
/// Used to receive JSON data submitted by the frontend `POST /api/subscriptions` request.
/// Does not require `id` and `active` fields as these are auto-generated or defaulted upon creation.
#[derive(Debug, Deserialize)]
pub struct CreateSubscription {
    /// 订阅名称 (必填)
    /// Subscription name (Required)
    pub name: String,
    
    /// 价格 (可选，永久订阅可不填)
    /// Price (Optional, can be omitted for lifetime)
    pub price: Option<f64>,
    
    /// 货币类型 (默认为 CNY)
    /// Currency type (Default: CNY)
    pub currency: String,
    
    /// 下次付款日期 (可选，永久订阅可不填)
    /// Next payment date (Optional, can be omitted for lifetime)
    pub next_payment: Option<String>,
    
    /// 付款频率 (1=Monthly, 12=Yearly, 0=Lifetime)
    /// Payment frequency
    pub frequency: i64,
    
    /// 官网链接
    /// Official website URL
    pub url: Option<String>,
    
    /// Logo 图片链接
    /// Logo image URL
    pub logo: Option<String>,

    /// 订阅开始日期 (可选)
    /// Subscription start date (Optional)
    pub start_date: Option<String>,
}
