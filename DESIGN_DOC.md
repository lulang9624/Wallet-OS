# Wallet OS 项目设计文档

## 1. 引言 (Introduction)

### 1.1 项目背景
Wallet OS 是一个现代化的个人订阅管理工具，旨在帮助用户轻松跟踪和管理各类周期性支出（如 Netflix, Spotify, iCloud 等）。项目灵感来源于 Wallos，目标是构建一个高性能、轻量级且具备 AI 辅助功能的自托管解决方案。

### 1.2 设计目标
- **高性能**: 采用 Rust 构建后端，确保极低的资源占用和极快的响应速度。
- **用户体验**: 提供现代化、响应式的 Web 界面，支持暗色模式。
- **智能化**: 集成 AI 能力，实现自动填单和财务分析建议。
- **易部署**: 提供 Docker 和 Docker Compose 支持，实现一键部署。
- **数据隐私**: 数据完全本地化存储 (SQLite)，用户拥有完全的控制权。

## 2. 系统架构 (System Architecture)

### 2.1 技术栈 (Tech Stack)
- **后端**: Rust + Axum (Web 框架) + SQLx (ORM) + Tokio (异步运行时)
- **数据库**: SQLite (嵌入式关系型数据库)
- **前端**: 原生 HTML5 / CSS3 / Vanilla JavaScript (无框架依赖)
- **外部服务**:
  - OpenAI Compatible API (用于智能填单与分析)
  - DuckDuckGo API/HTML (用于域名搜索)
  - Google Favicon Service (用于图标获取)

### 2.2 架构图
```mermaid
graph TD
    User[用户 Browser] <-->|HTTP/SSE| WebServer[Axum Web Server]
    WebServer <-->|SQL| DB[(SQLite Database)]
    WebServer <-->|HTTPS| LLM[LLM Provider (OpenAI/DeepSeek)]
    WebServer <-->|HTTPS| DDG[DuckDuckGo]
    WebServer <-->|HTTPS| GFav[Google Favicons]
    
    subgraph Backend [Rust Backend]
        Router[路由层]
        Handlers[业务逻辑层]
        Models[数据模型]
        DBPool[数据库连接池]
        Cache[内存缓存 (Icons/Search)]
    end
    
    WebServer --- Backend
```

## 3. 数据库设计 (Database Design)

### 3.1 表结构：`subscriptions`
用于存储所有订阅服务的信息。

| 字段名 | 类型 | 约束 | 说明 |
| :--- | :--- | :--- | :--- |
| `id` | INTEGER | PRIMARY KEY AUTOINCREMENT | 唯一标识符 |
| `name` | TEXT | NOT NULL | 订阅名称 (如 Netflix) |
| `price` | REAL | NOT NULL | 价格 |
| `currency` | TEXT | DEFAULT 'CNY' | 货币代码 (ISO 4217) |
| `next_payment` | DATE | NULLABLE | 下次付款日期 (YYYY-MM-DD) |
| `frequency` | INTEGER | DEFAULT 1 | 频率: -1=Daily, 1=Monthly, 3=Quarterly, 12=Yearly, 0=Lifetime |
| `url` | TEXT | NULLABLE | 官网链接 |
| `logo` | TEXT | NULLABLE | Logo 图片 URL |
| `start_date` | DATE | NULLABLE | 订阅开始日期 (YYYY-MM-DD) |
| `active` | BOOLEAN | DEFAULT 1 | 激活状态 (逻辑删除用) |

### 3.2 索引 (Indexes)
- `idx_subscriptions_next_payment`: 优化按下次付款日期排序的查询 (`ORDER BY next_payment ASC`)。
- `idx_subscriptions_name`: 优化名称搜索（预留）。

## 4. 接口设计 (API Design)

所有 API 均位于 `/api` 路径下。

### 4.1 订阅管理
- **GET /api/subscriptions**: 获取订阅列表。
  - 响应: `[Subscription]` JSON 数组，按 `next_payment` 升序排列。
- **POST /api/subscriptions**: 创建新订阅。
  - 请求: `CreateSubscription` JSON。
  - 响应: 创建成功的完整 `Subscription` 对象。
- **PUT /api/subscriptions/:id**: 更新订阅。
  - 请求: `CreateSubscription` JSON。
- **DELETE /api/subscriptions/:id**: 删除订阅。

### 4.2 辅助功能
- **GET /api/search?q={query}**: 搜索服务官网域名。
  - 逻辑: 优先 DuckDuckGo API，失败则回退至 HTML 解析。包含内存缓存。
- **GET /api/icon?domain={domain}&sz={size}**: 获取并缓存网站图标。
  - 逻辑: 检查本地 `static/icons`，无则从 Google 获取并保存。
- **POST /api/smart-parse**: AI 智能文本解析。
  - 请求: `{ "text": "..." }`
  - 逻辑: 调用 LLM 提取 JSON 结构；若无 Key 则使用本地关键词 Mock。
- **POST /api/analyze**: AI 财务分析。
  - 逻辑: 汇总当前订阅数据，发送给 LLM 获取优化建议。

### 4.3 实时更新
- **GET /api/stream**: SSE (Server-Sent Events) 端点。
  - 逻辑: 后端数据变更（增删改）时，通过 `tokio::sync::broadcast` 推送 `"update"` 事件，前端接收后自动刷新列表。

## 5. 详细模块设计 (Detailed Design)

### 5.1 后端模块 (src/)

#### 5.1.1 应用程序入口 (`main.rs`)
- 初始化日志系统 (`tracing-subscriber`)，支持控制台和文件日志滚动。
- 初始化数据库连接池 (`db::init_db`)，执行自动迁移。
- 构建 Axum Router，挂载 API 路由和静态文件服务 (`ServeDir`)。
- 配置 CORS 和 Gzip 压缩中间件。
- 启动 Tokio 运行时和 TCP 监听。

#### 5.1.2 业务逻辑 (`handlers.rs`)
- **智能解析 (`smart_parse`)**:
  - 读取 `OPENAI_API_KEY`。存在则构造 Prompt 调用 `/chat/completions`。
  - Prompt 模板从 `static/prompts.json` 加载，支持热更新。
  - Mock 模式：若无 Key，通过关键词 (netflix, spotify 等) 匹配生成演示数据。
- **域名搜索 (`search_domain`)**:
  - 实现三级策略：Cache -> DDG API (JSON) -> DDG HTML Parsing。
  - HTML 解析使用 `scraper` 库提取真实 URL，处理重定向链。
- **图标代理 (`get_icon`)**:
  - 充当反向代理 + 缓存层。避免前端直接请求第三方导致的隐私泄漏和 CORS 问题。
  - 首次请求下载并写入磁盘，后续请求直接服务本地文件。

#### 5.1.3 数据层 (`db.rs`, `models.rs`)
- 使用 `sqlx` 实现异步数据库交互。
- 开启 SQLite WAL (Write-Ahead Logging) 模式以支持更高的并发读写。
- 启动时检查 `wallet-os.db` 是否存在，不存在则自动创建文件和 Schema。

### 5.2 前端模块 (static/)

#### 5.2.1 界面交互 (`index.html`)
- **单页应用 (SPA)**: 无需路由跳转，所有操作在当前页面完成。
- **状态管理**: 使用全局变量 `currentSubs` 存储当前数据，`searchCache` 缓存搜索 Promise。
- **模态框 (Modal)**: 手写原生 Modal 逻辑，处理表单的展示与隐藏。
- **防抖 (Debounce)**: 虽然主要依赖后端缓存，但前端在输入搜索时可预留防抖逻辑。

#### 5.2.2 视觉设计 (`style.css`)
- **配色系统**: 定义 CSS 变量 (`--bg-color`, `--text-color`, `--accent`) 支持一键切换 Light/Dark 模式。
- **响应式布局**: 使用 Grid 布局展示统计卡片，Flexbox 处理列表项，适配移动端。
- **电池电量特效**: 根据剩余天数计算 CSS `width` 和 `background-color` (绿->黄->红)，直观展示续费紧迫度。

## 6. 安全与性能设计 (Security & Performance)

### 6.1 安全性
- **输入校验**: 后端对所有输入字段（如名称、频率、日期格式）进行严格校验。
- **SQL 注入防护**: 使用 SQLx 的参数化查询 (`bind`)，杜绝注入风险。
- **隐私保护**: 所有的图标和域名搜索均由后端代理，不在前端直接暴露用户 IP 给第三方。
- **错误处理**: 统一的错误响应格式，不向前端暴露具体的数据库堆栈信息。

### 6.2 性能优化
- **数据库**: 启用 SQLite WAL 模式；关键字段建立索引。
- **并发处理**: 利用 Rust/Tokio 的异步非阻塞特性处理 I/O 密集型任务（网络请求、文件读写）。
- **缓存**:
  - **内存缓存**: 域名搜索结果缓存 (`SEARCH_CACHE`)。
  - **文件缓存**: 网站图标持久化存储。
  - **浏览器缓存**: 静态资源和图标接口设置 `Cache-Control` 头。
- **前端优化**:
  - 使用 SSE 替代轮询 (Polling)，减少无效请求。
  - 图片懒加载（原生 `loading="lazy"`，虽然本项目列表较短，但设计上应支持）。

## 7. 部署与运维 (Deployment)

### 7.1 Docker 化
- **构建阶段 (Builder)**: 使用 `rust:latest` 镜像，编译发布版二进制 (`cargo build --release`)。
- **运行阶段 (Runtime)**: 使用 `debian:bookworm-slim`，安装必要的系统库 (openssl, ca-certificates)。
- **体积优化**: 清理构建缓存，最终镜像仅包含二进制文件、配置模板和静态资源。

### 7.2 配置管理
- 所有的可变配置通过环境变量注入：
  - `DATABASE_URL`: 数据库路径。
  - `PORT`: 监听端口。
  - `OPENAI_*`: AI 相关配置。
- 数据持久化通过 Docker Volume 挂载 `/app/data` 和 `/app/logs`。

## 8. 开发指南 (Development Guide)

1.  **环境准备**: 安装 Rust (cargo), Docker。
2.  **本地运行**:
    ```bash
    cargo run
    ```
    访问 `http://localhost:80`。
3.  **构建镜像**:
    ```bash
    docker build -t wallet-os .
    ```
4.  **功能验证**:
    - 添加一条订阅，观察列表刷新。
    - 检查 `./wallet_os_data` 下是否生成数据库文件。
    - 检查 `./logs` 下是否有日志输出。

---
*文档生成日期: 2025-12-23*
