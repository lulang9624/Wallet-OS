/// 数据库连接管理模块
/// Database connection management module
///
/// 负责初始化数据库连接池、创建数据库文件（如果不存在）以及执行数据库迁移（创建表结构）。
/// Handles database connection pool initialization, database file creation (if missing),
/// and database migrations (table schema creation).

use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Pool, Sqlite};
use std::env;

/// 定义数据库连接池类型别名
/// Define type alias for database connection pool
/// 
/// 使用 `Pool<Sqlite>` 表示 SQLite 数据库的连接池。
/// Uses `Pool<Sqlite>` to represent the connection pool for SQLite database.
pub type DbPool = Pool<Sqlite>;

/// 初始化数据库
/// Initialize the database
///
/// 该函数会执行以下步骤：
/// 1. 获取数据库连接 URL（优先读取 `DATABASE_URL` 环境变量）。
/// 2. 检查数据库文件是否存在，如果不存在则自动创建文件及父目录。
/// 3. 创建并配置 SQLite 连接池。
/// 4. 运行 SQL 脚本以确保 `subscriptions` 表存在。
///
/// This function performs the following steps:
/// 1. Retrieves the database URL (prefers `DATABASE_URL` environment variable).
/// 2. Checks if the database file exists; creates it and parent directories if missing.
/// 3. Creates and configures the SQLite connection pool.
/// 4. Runs SQL scripts to ensure the `subscriptions` table exists.
pub async fn init_db() -> Result<DbPool, sqlx::Error> {
    // 1. 获取数据库配置
    //    Get database configuration
    //    默认为 "sqlite:wallet-os.db"
    //    Defaults to "sqlite:wallet-os.db"
    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:wallet-os.db".to_string());
    
    // 2. 处理数据库文件路径
    //    Handle database file path
    //    去除 "sqlite:" 前缀以获取文件系统路径
    //    Strip "sqlite:" prefix to get the filesystem path
    let db_path = database_url.strip_prefix("sqlite:").unwrap_or(&database_url);
    
    // 检查并创建数据库文件
    // Check and create database file
    let path = std::path::Path::new(db_path);
    if !path.exists() {
        // 确保父目录存在
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        // 创建空文件
        // Create empty file
        std::fs::File::create(path).expect("Failed to create database file");
    }

    // 3. 创建连接池
    //    Create connection pool
    //    max_connections(10): 设置最大并发连接数为 10
    //    max_connections(10): Sets maximum concurrent connections to 10
    
    // 配置连接选项以启用日志
    // Configure connection options to enable logging
    use std::str::FromStr;
    use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode};
    use sqlx::ConnectOptions; // 必须引入此 trait 才能调用 log_statements
    
    // 解析数据库连接字符串
    // Parse database connection string
    let connect_options = SqliteConnectOptions::from_str(&database_url)?
        .journal_mode(SqliteJournalMode::Wal) // 开启 WAL 模式以提高并发性能 / Enable WAL mode for better concurrency
        .create_if_missing(true)
        .log_statements(log::LevelFilter::Info); // 链式调用 log_statements

    let pool = SqlitePoolOptions::new()
        .max_connections(10)
        .connect_with(connect_options)
        .await?;

    // 4. 数据库迁移 (创建表)
    //    Database Migration (Create Table)
    //    使用 `CREATE TABLE IF NOT EXISTS` 确保表结构存在。
    //    Uses `CREATE TABLE IF NOT EXISTS` to ensure table schema exists.
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS subscriptions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            price REAL NOT NULL,
            currency TEXT DEFAULT 'CNY',
            next_payment DATE,
            frequency INTEGER DEFAULT 1, -- 1=月付 Monthly, 12=年付 Yearly
            url TEXT,
            logo TEXT,
            active BOOLEAN DEFAULT 1
        );
        CREATE INDEX IF NOT EXISTS idx_subscriptions_next_payment ON subscriptions(next_payment);
        CREATE INDEX IF NOT EXISTS idx_subscriptions_name ON subscriptions(name);
        "#
    )
    .execute(&pool)
    .await?;

    Ok(pool)
}
