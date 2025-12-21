# Wallet OS

Wallet OS 是一个简单的个人财务和订阅管理工具，旨在帮助您跟踪每月的固定支出。

## 🛠️ 技术栈 (Tech Stack)

本项目采用现代化的 Rust 技术栈构建：

- **后端**: [Rust](https://www.rust-lang.org/)
- **Web 框架**: [Axum](https://github.com/tokio-rs/axum) (高性能、符合人体工程学的 Web 框架)
- **数据库**: [SQLite](https://www.sqlite.org/) (轻量级嵌入式数据库)
- **ORM/SQL 工具**: [SQLx](https://github.com/launchbadge/sqlx) (类型安全的异步 SQL 工具)
- **前端**: 原生 HTML/CSS/JavaScript (位于 `static/` 目录)
- **容器化**: Docker & Docker Compose

## 🚀 快速开始 (Getting Started)

### 前置要求

- [Rust](https://rustup.rs/) (最新稳定版)
- [Docker](https://www.docker.com/) (可选，用于容器化部署)

### 📦 本地运行 (Local Development)

1. **安装依赖并运行**:
   ```bash
   # 运行项目 (首次运行会自动编译)
   cargo run
   ```
   默认监听端口为 `80` (Linux 下可能需要 `sudo` 权限，或者修改 `src/main.rs` 中的端口为 `8080`)。

2. **数据库初始化**:
   应用启动时会自动创建 `wallos.db` 文件并初始化数据表，无需手动配置。

3. **访问应用**:
   打开浏览器访问 `http://localhost` (如果修改了端口则是 `http://localhost:8080`)。

### 🐳 使用 Docker 运行 (Docker)

如果您不想安装 Rust 环境，可以直接使用 Docker：

```bash
# 构建并启动容器
docker-compose up --build
```
容器启动后，应用将运行在 `http://localhost:8081`。

## 📂 项目结构 (Project Structure)

```
.
├── src/
│   ├── main.rs      # 程序入口，路由配置
│   ├── handlers.rs  # API 处理逻辑 (Controller)
│   ├── models.rs    # 数据模型定义
│   └── db.rs        # 数据库连接和初始化
├── static/          # 前端静态资源 (HTML/CSS/JS)
├── Cargo.toml       # Rust 项目依赖配置
├── Dockerfile       # Docker 构建文件
├── docker-compose.yml # Docker Compose 编排文件
└── README.md        # 项目文档
```

## 🔌 API 接口 (API Endpoints)

- `GET /api/subscriptions`: 获取所有订阅列表
- `POST /api/subscriptions`: 创建新订阅
- `DELETE /api/subscriptions/:id`: 删除指定订阅

## 📝 许可证 (License)

MIT License
