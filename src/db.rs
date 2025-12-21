use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Pool, Sqlite};
use std::env;

/// 定义数据库连接池类型别名
/// Define type alias for database connection pool
pub type DbPool = Pool<Sqlite>;

/// 初始化数据库
/// Initialize the database
///
/// 该函数会：
/// 1. 获取数据库 URL（默认为 sqlite:wallos.db）
/// 2. 如果数据库文件不存在，则创建它
/// 3. 创建连接池
/// 4. 如果表不存在，则创建 subscriptions 表
///
/// This function will:
/// 1. Get database URL (defaults to sqlite:wallos.db)
/// 2. Create the database file if it doesn't exist
/// 3. Create a connection pool
/// 4. Create the subscriptions table if it doesn't exist
pub async fn init_db() -> Result<DbPool, sqlx::Error> {
    // 获取环境变量中的 DATABASE_URL，如果没有则使用默认值
    // Get DATABASE_URL from environment variables, use default if not present
    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:wallos.db".to_string());
    
    // 如果数据库文件不存在，则创建它
    // Create the database file if it doesn't exist
    if !std::path::Path::new("wallos.db").exists() {
        std::fs::File::create("wallos.db").expect("Failed to create database file");
    }

    // 配置并建立数据库连接池
    // Configure and establish database connection pool
    let pool = SqlitePoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await?;

    // 如果表不存在则创建
    // Create tables if they don't exist
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS subscriptions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            price REAL NOT NULL,
            currency TEXT DEFAULT 'USD',
            next_payment DATE,
            frequency INTEGER DEFAULT 1, -- 1=Monthly, 12=Yearly etc, simplified for now
            url TEXT,
            logo TEXT,
            active BOOLEAN DEFAULT 1
        );
        "#
    )
    .execute(&pool)
    .await?;

    Ok(pool)
}
