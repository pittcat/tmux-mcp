#!/bin/bash
#
# Tmux MCP Server 安装脚本
# 支持源码构建和已有二进制两种安装方式
#

set -e

# 配置项
APP_NAME="tmux-mcp-server"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
CONFIG_DIR="${CONFIG_DIR:-$HOME/.config/tmux-mcp}"
DATA_DIR="${DATA_DIR:-$HOME/.local/share/tmux-mcp}"
LOG_DIR="${DATA_DIR}/logs"

# 默认绑定地址
BIND_ADDR="${TMUX_MCP_BIND_ADDR:-127.0.0.1:8090}"
MAX_COMMANDS="${TMUX_MCP_MAX_COMMANDS:-1000}"
COMMAND_TTL="${TMUX_MCP_COMMAND_TTL:-600}"

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 打印函数
info() { echo -e "${BLUE}[INFO]${NC} $1"; }
success() { echo -e "${GREEN}[OK]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; }

# 使用说明
usage() {
    cat << 'EOF'
Usage: ./install.sh [OPTIONS]

Options:
    -b, --binary PATH       使用已有的二进制文件路径，跳过源码构建
    -s, --skip-build        跳过构建，假设二进制已在 ~/.local/bin/
    -i, --install-dir DIR   安装目录 (默认: ~/.local/bin)
    -c, --config-dir DIR    配置目录 (默认: ~/.config/tmux-mcp)
    --bind ADDR             绑定地址 (默认: 127.0.0.1:8090)
    --max-cmd N             最大命令数 (默认: 1000)
    --ttl SECONDS           命令TTL秒数 (默认: 600)
    -u, --uninstall         卸载服务
    -h, --help              显示此帮助

Examples:
    # 从源码构建并安装
    ./install.sh

    # 使用已有的二进制文件
    ./install.sh --binary ./target/release/tmux-mcp-server

    # 二进制已在 ~/.local/bin/，只配置服务
    ./install.sh --skip-build

    # 自定义安装目录
    ./install.sh --install-dir /usr/local/bin

    # 卸载
    ./install.sh --uninstall

EOF
}

# 检测平台
detect_platform() {
    case "$(uname -s)" in
        Darwin*)
            PLATFORM="macos"
            SERVICE_TYPE="launchd"
            ;;
        Linux*)
            PLATFORM="linux"
            if command -v systemctl &> /dev/null; then
                SERVICE_TYPE="systemd"
            else
                error "Linux 系统需要 systemd"
                exit 1
            fi
            ;;
        *)
            error "不支持的操作系统: $(uname -s)"
            exit 1
            ;;
    esac
    info "检测到平台: $PLATFORM ($SERVICE_TYPE)"
}

# 检查目录是否存在并创建
ensure_dir() {
    if [ ! -d "$1" ]; then
        mkdir -p "$1"
        success "创建目录: $1"
    fi
}

# 检查依赖
check_dependencies() {
    info "检查依赖..."

    # 检查 tmux
    if ! command -v tmux &> /dev/null; then
        error "未找到 tmux，请先安装"
        exit 1
    fi
    success "tmux: $(tmux -V)"

    # 如果需要构建，检查 Rust
    if [ "$SKIP_BUILD" != "true" ] && [ -z "$BINARY_PATH" ]; then
        if ! command -v cargo &> /dev/null; then
            error "未找到 Rust/Cargo，请先安装: https://rustup.rs/"
            exit 1
        fi
        success "Rust: $(cargo --version)"
    fi
}

# 源码构建
build_from_source() {
    info "从源码构建..."

    # 检查是否在项目目录
    if [ ! -f "Cargo.toml" ]; then
        error "未找到 Cargo.toml，请在项目根目录运行此脚本"
        exit 1
    fi

    # 构建 release 版本
    cargo build --release

    if [ ! -f "target/release/$APP_NAME" ]; then
        error "构建失败：未找到 target/release/$APP_NAME"
        exit 1
    fi

    BINARY_PATH="target/release/$APP_NAME"
    success "构建完成: $BINARY_PATH"
}

# 安装二进制文件
install_binary() {
    info "安装二进制文件..."

    local src="$1"
    local dst="$INSTALL_DIR/$APP_NAME"

    # 如果源文件就是目标文件，跳过
    if [ "$src" = "$dst" ]; then
        info "二进制已在目标位置，跳过复制"
        return
    fi

    # 复制并设置权限
    cp "$src" "$dst"
    chmod +x "$dst"
    success "已安装: $dst"
}

# 创建 systemd 用户服务 (Linux)
setup_systemd_service() {
    info "配置 systemd 用户服务..."

    local service_dir="$HOME/.config/systemd/user"
    local service_file="$service_dir/$APP_NAME.service"

    ensure_dir "$service_dir"

    cat > "$service_file" << EOF
[Unit]
Description=Tmux MCP Server
Documentation=https://github.com/pittcat/tmux-mcp
After=network.target

[Service]
Type=simple
ExecStart=$INSTALL_DIR/$APP_NAME
Restart=on-failure
RestartSec=5
Environment="RUST_LOG=info"
Environment="TMUX_MCP_BIND_ADDR=$BIND_ADDR"
Environment="TMUX_MCP_MAX_COMMANDS=$MAX_COMMANDS"
Environment="TMUX_MCP_COMMAND_TTL=$COMMAND_TTL"

# 日志输出到文件
StandardOutput=append:$LOG_DIR/server.log
StandardError=append:$LOG_DIR/server.log

[Install]
WantedBy=default.target
EOF

    success "创建服务文件: $service_file"

    # 重新加载 systemd
    systemctl --user daemon-reload

    # 启用并启动服务
    systemctl --user enable "$APP_NAME.service"
    systemctl --user restart "$APP_NAME.service"

    # 等待服务启动
    sleep 2

    if systemctl --user is-active --quiet "$APP_NAME.service"; then
        success "服务已启动并启用开机自启"
    else
        error "服务启动失败，请检查日志: journalctl --user -u $APP_NAME"
        exit 1
    fi
}

# 创建 launchd 服务 (macOS)
setup_launchd_service() {
    info "配置 launchd 服务..."

    local plist_dir="$HOME/Library/LaunchAgents"
    local plist_file="$plist_dir/com.pittcat.$APP_NAME.plist"

    ensure_dir "$plist_dir"

    cat > "$plist_file" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.pittcat.$APP_NAME</string>

    <key>ProgramArguments</key>
    <array>
        <string>$INSTALL_DIR/$APP_NAME</string>
    </array>

    <key>EnvironmentVariables</key>
    <dict>
        <key>RUST_LOG</key>
        <string>info</string>
        <key>TMUX_MCP_BIND_ADDR</key>
        <string>$BIND_ADDR</string>
        <key>TMUX_MCP_MAX_COMMANDS</key>
        <string>$MAX_COMMANDS</string>
        <key>TMUX_MCP_COMMAND_TTL</key>
        <string>$COMMAND_TTL</string>
    </dict>

    <key>StandardOutPath</key>
    <string>$LOG_DIR/server.log</string>

    <key>StandardErrorPath</key>
    <string>$LOG_DIR/server.log</string>

    <key>KeepAlive</key>
    <true/>

    <key>RunAtLoad</key>
    <true/>

    <key>ThrottleInterval</key>
    <integer>5</integer>
</dict>
</plist>
EOF

    success "创建 plist 文件: $plist_file"

    # 加载并启动服务
    launchctl unload "$plist_file" 2>/dev/null || true
    launchctl load "$plist_file"
    launchctl start "com.pittcat.$APP_NAME" 2>/dev/null || true

    # 等待服务启动
    sleep 2

    # 检查服务状态
    if launchctl list | grep -q "com.pittcat.$APP_NAME"; then
        success "服务已启动并启用开机自启"
    else
        warn "服务可能未启动，请手动检查: launchctl list | grep $APP_NAME"
    fi
}

# 配置服务
setup_service() {
    info "配置开机自启服务..."

    case "$SERVICE_TYPE" in
        systemd)
            setup_systemd_service
            ;;
        launchd)
            setup_launchd_service
            ;;
    esac
}

# 卸载服务
uninstall() {
    info "卸载 $APP_NAME..."

    detect_platform

    case "$SERVICE_TYPE" in
        systemd)
            systemctl --user stop "$APP_NAME.service" 2>/dev/null || true
            systemctl --user disable "$APP_NAME.service" 2>/dev/null || true
            rm -f "$HOME/.config/systemd/user/$APP_NAME.service"
            systemctl --user daemon-reload
            ;;
        launchd)
            launchctl stop "com.pittcat.$APP_NAME" 2>/dev/null || true
            launchctl unload "$HOME/Library/LaunchAgents/com.pittcat.$APP_NAME.plist" 2>/dev/null || true
            rm -f "$HOME/Library/LaunchAgents/com.pittcat.$APP_NAME.plist"
            ;;
    esac

    # 删除二进制
    rm -f "$INSTALL_DIR/$APP_NAME"

    # 询问是否删除配置和数据
    read -p "是否删除配置和数据目录? [y/N] " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        rm -rf "$CONFIG_DIR" "$DATA_DIR"
        success "已删除配置和数据"
    fi

    success "卸载完成"
}

# 验证安装
verify_installation() {
    info "验证安装..."

    # 检查二进制
    if [ ! -x "$INSTALL_DIR/$APP_NAME" ]; then
        error "二进制文件不存在或不可执行"
        exit 1
    fi

    # 检查版本
    local version_output
    version_output=$("$INSTALL_DIR/$APP_NAME" --version 2>&1 || echo "version not available")
    info "版本: $version_output"

    # 等待服务就绪
    sleep 1

    # 测试连接
    if curl -s "http://${BIND_ADDR}/mcp" > /dev/null 2>&1 || \
       curl -s "http://${BIND_ADDR}/health" > /dev/null 2>&1 || \
       curl -s "http://${BIND_ADDR}" > /dev/null 2>&1; then
        success "服务响应正常"
    else
        warn "服务可能还在启动中，请稍后手动检查"
    fi

    success "安装验证完成!"
}

# 打印安装信息
print_info() {
    echo
    echo "========================================"
    echo "  Tmux MCP Server 安装完成!"
    echo "========================================"
    echo
    echo "  安装路径: $INSTALL_DIR/$APP_NAME"
    echo "  配置文件: $CONFIG_DIR/"
    echo "  日志文件: $LOG_DIR/server.log"
    echo "  服务地址: http://$BIND_ADDR"
    echo
    echo "  常用命令:"
    case "$SERVICE_TYPE" in
        systemd)
            echo "    查看状态: systemctl --user status $APP_NAME"
            echo "    查看日志: journalctl --user -u $APP_NAME -f"
            echo "    重启服务: systemctl --user restart $APP_NAME"
            echo "    停止服务: systemctl --user stop $APP_NAME"
            ;;
        launchd)
            echo "    查看状态: launchctl list | grep $APP_NAME"
            echo "    查看日志: tail -f $LOG_DIR/server.log"
            echo "    重启服务: launchctl stop com.pittcat.$APP_NAME; launchctl start com.pittcat.$APP_NAME"
            echo "    停止服务: launchctl stop com.pittcat.$APP_NAME"
            ;;
    esac
    echo
    echo "========================================"
}

# 主函数
main() {
    # 解析参数
    SKIP_BUILD="false"
    BINARY_PATH=""
    UNINSTALL_MODE="false"

    while [[ $# -gt 0 ]]; do
        case $1 in
            -b|--binary)
                BINARY_PATH="$2"
                shift 2
                ;;
            -s|--skip-build)
                SKIP_BUILD="true"
                shift
                ;;
            -i|--install-dir)
                INSTALL_DIR="$2"
                shift 2
                ;;
            -c|--config-dir)
                CONFIG_DIR="$2"
                shift 2
                ;;
            --bind)
                BIND_ADDR="$2"
                shift 2
                ;;
            --max-cmd)
                MAX_COMMANDS="$2"
                shift 2
                ;;
            --ttl)
                COMMAND_TTL="$2"
                shift 2
                ;;
            -u|--uninstall)
                UNINSTALL_MODE="true"
                shift
                ;;
            -h|--help)
                usage
                exit 0
                ;;
            *)
                error "未知选项: $1"
                usage
                exit 1
                ;;
        esac
    done

    # 卸载模式
    if [ "$UNINSTALL_MODE" = "true" ]; then
        uninstall
        exit 0
    fi

    echo "========================================"
    echo "  Tmux MCP Server 安装脚本"
    echo "========================================"
    echo

    # 检测平台
    detect_platform

    # 检查依赖
    check_dependencies

    # 创建必要目录
    ensure_dir "$INSTALL_DIR"
    ensure_dir "$CONFIG_DIR"
    ensure_dir "$LOG_DIR"

    # 获取二进制文件
    if [ -n "$BINARY_PATH" ]; then
        # 使用指定的二进制文件
        if [ ! -f "$BINARY_PATH" ]; then
            error "指定的二进制文件不存在: $BINARY_PATH"
            exit 1
        fi
        info "使用指定的二进制文件: $BINARY_PATH"
        install_binary "$BINARY_PATH"
    elif [ "$SKIP_BUILD" = "true" ]; then
        # 跳过构建，检查是否已安装
        if [ ! -f "$INSTALL_DIR/$APP_NAME" ]; then
            error "未找到已安装的二进制文件: $INSTALL_DIR/$APP_NAME"
            info "请使用 --binary 指定二进制路径，或移除 --skip-build 从源码构建"
            exit 1
        fi
        info "使用已安装的二进制文件"
    else
        # 从源码构建
        build_from_source
        install_binary "$BINARY_PATH"
    fi

    # 配置服务
    setup_service

    # 验证安装
    verify_installation

    # 打印信息
    print_info
}

main "$@"
