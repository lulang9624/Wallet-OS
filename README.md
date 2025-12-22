# Wallet OS

Wallet OS 是一个现代化的个人订阅管理工具，帮助您轻松跟踪和管理各类周期性支出（如 Netflix, Spotify, iCloud 等）。

灵感来源于 Wallos 项目，由 Vibe Coding 驱动开发。

![Version](https://img.shields.io/badge/version-0.1.0-blue)
![Rust](https://img.shields.io/badge/Rust-1.70+-orange)
![License](https://img.shields.io/badge/license-MIT-green)

<br/>

<div align="center">
    <!-- 请替换下方链接为您的真实项目截图，建议放置在 docs/screenshot.png -->
    <!-- Please replace the link below with your actual project screenshot, e.g., docs/screenshot.png -->
    <img src="docs/screenshot.png" alt="Wallet OS Dashboard" width="100%" style="border-radius: 10px; box-shadow: 0 4px 8px rgba(0,0,0,0.2);">
</div>

<br/>

## ✨ 核心功能 (Features)

- **💰 费用追踪**: 自动计算每月总支出，支持多币种显示。
- **🔋 续费倒计时**: 独特的“电池电量”可视化效果，直观展示距离下次扣费的天数（绿色->红色->灰色）。
- **🔍 智能图标匹配**: 
  - 输入订阅名称（如 "iqiyi"）自动搜索并匹配官方高清图标。
  - 支持“三级回退”策略 (Google -> DuckDuckGo -> UI Avatars)，确保 100% 有图显示。
  - **秒级响应**: 采用 Promise 预加载技术，在您填写表单时后台自动完成搜索。
- **🛡️ 安全删除**: 删除订阅时需要输入名称确认，防止误操作。
- **⚡ 高性能**: 基于 Rust + Axum 构建，占用资源极低，响应速度极快。
- **🐳 轻松部署**: 提供 Docker 和 Docker Compose 支持，一键启动。

## 🛠️ 技术栈 (Tech Stack)

本项目采用全栈 Rust 构建，追求极致的性能与安全性：

- **后端 (Backend)**: 
  - [Rust](https://www.rust-lang.org/) (语言核心)
  - [Axum](https://github.com/tokio-rs/axum) (高性能异步 Web 框架)
  - [SQLx](https://github.com/launchbadge/sqlx) (类型安全的异步 SQL 工具)
  - [Reqwest](https://github.com/seanmonstar/reqwest) (HTTP 客户端，用于图标搜索 API)
  - [Scraper](https://github.com/causal-agent/scraper) (HTML 解析，用于辅助域名查找)
- **数据库 (Database)**: 
  - [SQLite](https://www.sqlite.org/) (轻量级嵌入式数据库，数据持久化)
- **前端 (Frontend)**: 
  - 原生 HTML5 / CSS3 (Flexbox & Grid 布局)
  - Vanilla JavaScript (ES6+, 无繁重框架依赖)
- **运维 (DevOps)**: 
  - Docker (多阶段构建，极致压缩镜像体积)
  - Docker Compose (一键编排)

## 🚀 快速开始 (Getting Started)

### 方式一：Docker 部署 (推荐)

如果您不想配置本地 Rust 环境，这是最快的方式。

1. **克隆项目**:
   ```bash
   git clone https://github.com/yourusername/wallet-OS.git
   cd wallet-OS
   ```

2. **启动服务**:
   ```bash
   # 构建并启动容器
   docker-compose up -d --build
   ```

3. **访问应用**:
   打开浏览器访问 `http://localhost:8081`。
   *数据将自动持久化到 `./wallet_os_data` 目录。*

### 方式二：本地开发 (Local Development)

1. **安装依赖**:
   确保已安装 [Rust](https://rustup.rs/) (最新稳定版)。

2. **运行项目**:
   ```bash
   # 首次运行会自动下载依赖并编译
   cargo run
   ```
默认监听端口为 `80`，在 Linux 下可能需要使用 `sudo cargo run`。

### 环境变量 (Environment)

- `DATABASE_URL`: SQLite 连接字符串，默认值为 `sqlite:wallet-os.db`。
  - 示例（自定义路径）：`export DATABASE_URL=sqlite:./wallet_os_data/wallet-os.db`
  - 首次启动会自动创建数据库文件与父目录。

### 日志 (Logs)

- 运行时会将日志写入 `./logs/wallet-os-YYYY-MM-DD.log`（按日滚动）。
- Docker 部署已将宿主机 `./logs` 挂载到容器 `/app/logs`，方便持久化与查看。

### 构建与发布 (Build & Release)

```bash
# 生成发布版二进制
cargo build --release

# 运行发布版（可指定数据库路径）
export DATABASE_URL=sqlite:./wallet_os_data/wallet-os.db
sudo ./target/release/wallet-os
```

3. **访问应用**:
   打开浏览器访问 `http://localhost`。

## 📂 项目结构 (Project Structure)

```
.
├── src/
│   ├── main.rs      # 程序入口，路由注册，跨域配置
│   ├── handlers.rs  # 核心业务逻辑 (API Controller)，含 DuckDuckGo 搜索逻辑
│   ├── models.rs    # 数据结构定义 (Subscription, SearchResult 等)
│   └── db.rs        # 数据库连接池初始化与迁移
├── static/          # 前端资源
│   ├── index.html   # 单页应用入口 (含 JS 逻辑：预加载、动画、表单验证)
│   └── style.css    # 样式表 (响应式设计、卡片布局、电池动画)
├── Cargo.toml       # Rust 项目依赖配置
├── Dockerfile       # 多阶段 Docker 构建文件 (Builder -> Runtime)
├── docker-compose.yml # 容器编排配置
└── README.md        # 项目文档
```

## 🔌 API 接口 (API Endpoints)

| 方法 | 路径 | 描述 |
|------|------|------|
| `GET` | `/api/subscriptions` | 获取所有订阅列表 (按下次付款时间排序) |
| `POST` | `/api/subscriptions` | 创建新订阅 (自动触发图标匹配) |
| `DELETE` | `/api/subscriptions/:id` | 删除指定订阅 |
| `PUT` | `/api/subscriptions/:id` | 更新指定订阅 |
| `GET` | `/api/search?q={name}` | **核心功能**: 根据名称搜索官网域名 (DuckDuckGo 源) |

## 🤝 贡献 (Contributing)

欢迎提交 Issue 或 Pull Request！

1. Fork 本项目
2. 创建您的特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交您的更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启一个 Pull Request

## 📝 许可证 (License)

本项目基于 [MIT License](https://opensource.org/licenses/MIT) 开源。