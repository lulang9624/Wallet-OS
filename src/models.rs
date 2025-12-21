/// 数据模型定义模块
/// Data models definition module
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 订阅模型结构体，对应数据库中的 subscriptions 表
/// Subscription model struct, corresponding to the subscriptions table in the database
#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct Subscription {
    /// 唯一标识符
    /// Unique identifier
    pub id: i64,
    /// 订阅名称 (例如: Netflix, Spotify)
    /// Subscription name (e.g., Netflix, Spotify)
    pub name: String,
    /// 价格
    /// Price
    pub price: f64,
    /// 货币类型 (例如: USD, EUR)
    /// Currency type (e.g., USD, EUR)
    pub currency: String,
    /// 下次付款日期
    /// Next payment date
    pub next_payment: Option<String>,
    /// 付款频率 (1=月付, 12=年付)
    /// Payment frequency (1=Monthly, 12=Yearly)
    pub frequency: i64,
    /// 官网链接 (可选)
    /// Official website URL (optional)
    pub url: Option<String>,
    /// Logo 图片链接 (可选)
    /// Logo image URL (optional)
    pub logo: Option<String>,
    /// 是否处于激活状态
    /// Whether it is active
    pub active: bool,
}

/// 创建订阅时的请求载荷结构体
/// Request payload struct for creating a subscription
#[derive(Debug, Deserialize)]
pub struct CreateSubscription {
    /// 订阅名称
    /// Subscription name
    pub name: String,
    /// 价格
    /// Price
    pub price: f64,
    /// 货币类型
    /// Currency type
    pub currency: String,
    /// 下次付款日期
    /// Next payment date
    pub next_payment: String,
    /// 付款频率
    /// Payment frequency
    pub frequency: i64,
    /// 官网链接
    /// Official website URL
    pub url: Option<String>,
    /// Logo 图片链接
    /// Logo image URL
    pub logo: Option<String>,
}
