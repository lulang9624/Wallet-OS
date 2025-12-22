# Stage 1: Build
# 阶段 1: 构建阶段
# 使用 Rust 官方镜像作为构建环境，基于 Debian Bookworm Slim 版本
# Uses official Rust image as build environment, based on Debian Bookworm Slim
FROM rust:1-slim-bookworm AS builder

WORKDIR /usr/src/app

# Install dependencies for compilation
# 安装编译所需的系统依赖
# pkg-config 和 libssl-dev 是编译某些 Rust crate (如 reqwest, openssl) 所必需的
# pkg-config and libssl-dev are required for compiling certain Rust crates
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Copy manifest
# 复制 Cargo.toml 配置文件
COPY Cargo.toml .

# Create dummy main to cache deps
# 创建一个虚拟的 main.rs 并进行一次构建
# 这样做的目的是为了缓存依赖项。只要 Cargo.toml 不变，这一层就会被 Docker 缓存。
# Create a dummy main.rs and run a build to cache dependencies.
# As long as Cargo.toml doesn't change, this layer will be cached.
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

# Copy source
# 复制实际的源代码
# Copy actual source code
COPY src ./src

# Touch main to force rebuild
# 更新 main.rs 的时间戳，强制 Cargo 重新编译项目代码（而不是依赖项）
# Update timestamp of main.rs to force Cargo to recompile project code (not dependencies)
RUN touch src/main.rs
RUN cargo build --release

# 验证构建产物是否存在
# Verify build artifact exists
RUN ls -l /usr/src/app/target/release/

# Stage 2: Runtime
# 阶段 2: 运行时环境
# 使用轻量级的 Debian 镜像作为运行环境，减小最终镜像体积
# Use lightweight Debian image for runtime to reduce final image size
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime libs
# 安装运行时所需的库
# libssl3: SSL/TLS 支持
# ca-certificates: HTTPS 请求所需的根证书
# sqlite3: SQLite 数据库支持
# Install runtime libraries (SSL, CA certs, SQLite)
RUN apt-get update && apt-get install -y libssl3 ca-certificates sqlite3 && rm -rf /var/lib/apt/lists/*

# Copy binary
# 从构建阶段复制编译好的二进制文件到运行时镜像
# Copy compiled binary from builder stage
COPY --from=builder /usr/src/app/target/release/wallet-os /app/wallet-os

# Copy static files
# 复制前端静态资源文件 (HTML, CSS, JS)
# Copy frontend static assets
COPY static /app/static

# 暴露 80 端口
# Expose port 80
EXPOSE 80

# 设置环境变量
# DATABASE_URL: 数据库连接字符串，默认为本地 SQLite 文件
# Set environment variables
ENV DATABASE_URL=sqlite:wallet-os.db

# 启动应用
# Start the application
CMD ["./wallet-os"]
