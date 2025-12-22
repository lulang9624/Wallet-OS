mod db;
mod handlers;
mod models;

use axum::{
    routing::{get, delete},
    Router,
};
use std::net::SocketAddr;
use tower_http::services::ServeDir;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// 应用程序的主入口点
/// Main entry point of the application
#[tokio::main]
async fn main() {
    // 1. 初始化日志系统
    //    Initialize the logging system.
    
    // 设置日志文件存放目录和文件名前缀
    // Set log file directory and filename prefix
    let file_appender = tracing_appender::rolling::daily("logs", "wallet-os.log");
    
    // 使用 non_blocking 包装器以避免阻塞主线程
    // Use non_blocking wrapper to avoid blocking the main thread
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // 配置控制台输出层
    // Configure console output layer
    let stdout_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stdout);

    // 配置文件输出层
    // Configure file output layer
    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false); // 文件中不包含 ANSI 颜色代码 / No ANSI color codes in file

    // 配置环境变量过滤器
    // Configure environment variable filter
    // 默认开启 wallet_os=debug, tower_http=debug, sqlx=info
    // Default to wallet_os=debug, tower_http=debug, sqlx=info
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "wallet_os=debug,tower_http=debug,sqlx=info".into());

    // 注册订阅者
    // Register subscriber
    tracing_subscriber::registry()
        .with(env_filter)
        .with(stdout_layer)
        .with(file_layer)
        .init();

    // 2. 初始化数据库连接池
    //    调用 db 模块的 init_db 函数，建立与 SQLite 数据库的连接。
    //    如果初始化失败，程序将 panic 并退出。
    //    Initialize the database connection pool.
    //    Calls init_db from the db module to establish a connection with SQLite.
    //    If initialization fails, the program will panic and exit.
    let pool = db::init_db().await.expect("Failed to initialize DB");

    // 3. 构建应用程序路由 (Router)
    //    定义 URL 路径与处理函数之间的映射关系。
    //    Build the application router.
    //    Defines the mapping between URL paths and handler functions.
    let app = Router::new()
        // API 路由：获取所有订阅 (GET) 和 创建新订阅 (POST)
        // API Routes: Get all subscriptions (GET) and Create new subscription (POST)
        .route("/api/subscriptions", get(handlers::list_subscriptions).post(handlers::create_subscription))
        
        // API 路由：根据 ID 删除特定订阅 (DELETE) 或 更新特定订阅 (PUT)
        // API Routes: Delete a specific subscription by ID (DELETE) or Update specific subscription (PUT)
        .route("/api/subscriptions/:id", delete(handlers::delete_subscription).put(handlers::update_subscription))

        // API 路由：搜索域名 (GET)
        // API Routes: Search domain (GET)
        .route("/api/search", get(handlers::search_domain))
        
        // 静态文件服务
        // 将根路径 "/" 映射到本地的 "static" 目录，用于托管前端页面 (HTML, CSS, JS)。
        // Static file service.
        // Maps the root path "/" to the local "static" directory to serve frontend assets.
        .nest_service("/", ServeDir::new("static"))
        
        // 中间件：CORS (跨域资源共享)
        // 允许来自不同源的请求，方便开发阶段的前后端调试。
        // Middleware: CORS (Cross-Origin Resource Sharing).
        // Allows requests from different origins, facilitating frontend-backend debugging.
        .layer(CorsLayer::permissive())

        // 中间件：Trace (日志追踪)
        // 自动记录 HTTP 请求日志
        // Middleware: Trace (Logging)
        // Automatically log HTTP requests
        .layer(TraceLayer::new_for_http())
        
        // 状态共享
        // 将数据库连接池注入到应用状态中，使所有处理函数都能访问数据库。
        // State Sharing.
        // Injects the database connection pool into the app state, making it accessible to all handlers.
        .with_state(pool);

    // 4. 配置服务器监听地址
    //    监听所有网络接口 (0.0.0.0) 的 80 端口。
    //    Configure server listening address.
    //    Listens on port 80 of all network interfaces (0.0.0.0).
    let addr = SocketAddr::from(([0, 0, 0, 0], 80));
    tracing::info!("Listening on {}", addr);
    
    // 绑定 TCP 监听器
    // Bind the TCP listener.
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    
    // 5. 启动 Axum 服务器
    //    开始接收并处理请求。
    //    Start the Axum server.
    //    Begins accepting and handling requests.
    axum::serve(listener, app).await.unwrap();
}
