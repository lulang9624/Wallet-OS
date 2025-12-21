mod db;
mod handlers;
mod models;

use axum::{
    routing::{get, post, delete},
    Router,
};
use std::net::SocketAddr;
use tower_http::services::ServeDir;
use tower_http::cors::CorsLayer;

/// 应用程序的主入口点
/// Main entry point of the application
#[tokio::main]
async fn main() {
    // 初始化日志系统，便于调试和监控
    // Initialize the logging system for debugging and monitoring
    tracing_subscriber::fmt::init();

    // 初始化数据库连接池
    // Initialize the database connection pool
    let pool = db::init_db().await.expect("Failed to initialize DB");

    // 构建应用程序路由
    // Build the application router
    let app = Router::new()
        // 设置 API 路由：获取和创建订阅
        // Set up API routes: get and create subscriptions
        .route("/api/subscriptions", get(handlers::list_subscriptions).post(handlers::create_subscription))
        // 设置 API 路由：根据 ID 删除订阅
        // Set up API route: delete subscription by ID
        .route("/api/subscriptions/:id", delete(handlers::delete_subscription))
        // 提供静态文件服务（前端页面）
        // Serve static files (frontend pages)
        .nest_service("/", ServeDir::new("static"))
        // 添加 CORS 中间件，允许跨域请求
        // Add CORS middleware to allow cross-origin requests
        .layer(CorsLayer::permissive())
        // 将数据库连接池作为状态注入到应用中
        // Inject the database connection pool as state into the application
        .with_state(pool);

    // 定义监听地址和端口
    // Define the listening address and port
    let addr = SocketAddr::from(([0, 0, 0, 0], 80));
    println!("Listening on {}", addr);
    
    // Bind the TCP listener
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    
    // 启动服务器
    // Start the server
    axum::serve(listener, app).await.unwrap();
}
