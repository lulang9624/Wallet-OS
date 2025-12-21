# Stage 1: Build
# 阶段 1: 构建
FROM rust:1-slim-bookworm as builder

WORKDIR /usr/src/app

# Install dependencies for compilation
# 安装编译所需的依赖
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Copy manifest
# 复制 Cargo.toml 配置文件
COPY Cargo.toml .
# Create dummy main to cache deps
# 创建虚拟 main.rs 以利用 Docker 缓存层缓存依赖构建
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

# Copy source
# 复制源代码
COPY src ./src
# Touch main to force rebuild
# 更新文件时间戳以强制重新构建
RUN touch src/main.rs
RUN cargo build --release

# Stage 2: Runtime
# 阶段 2: 运行时环境
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime libs
# 安装运行时库 (OpenSSL, CA 证书, SQLite)
RUN apt-get update && apt-get install -y libssl3 ca-certificates sqlite3 && rm -rf /var/lib/apt/lists/*

# Copy binary
# 从构建阶段复制编译好的二进制文件
COPY --from=builder /usr/src/app/target/release/wallos-next /app/wallos-next
# Copy static files
# 复制静态文件 (前端资源)
COPY static /app/static

# 暴露 80 端口
EXPOSE 80
# 设置默认数据库 URL
ENV DATABASE_URL=sqlite:wallos.db

# 启动应用
CMD ["./wallos-next"]
