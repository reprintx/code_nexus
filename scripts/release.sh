#!/bin/bash

# CodeNexus 发布脚本
# 用法: ./scripts/release.sh 0.2.0 [--dry-run]

set -e

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

function log_info() {
    echo -e "${CYAN}$1${NC}"
}

function log_success() {
    echo -e "${GREEN}$1${NC}"
}

function log_warning() {
    echo -e "${YELLOW}$1${NC}"
}

function log_error() {
    echo -e "${RED}$1${NC}"
}

# 检查参数
if [ $# -eq 0 ]; then
    log_error "❌ 请提供版本号"
    echo "用法: $0 <version> [--dry-run]"
    echo "示例: $0 0.2.0"
    exit 1
fi

VERSION=$1
DRY_RUN=false

if [ "$2" = "--dry-run" ]; then
    DRY_RUN=true
    log_warning "🔍 DRY RUN 模式 - 不会执行实际操作"
fi

# 检查是否在项目根目录
if [ ! -f "Cargo.toml" ]; then
    log_error "❌ 请在项目根目录运行此脚本"
    exit 1
fi

log_info "🚀 开始准备发布 CodeNexus v$VERSION"

# 验证版本格式
if ! [[ $VERSION =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    log_error "❌ 版本格式错误，请使用语义化版本格式 (例如: 1.0.0)"
    exit 1
fi

# 检查工作目录是否干净
if [ -n "$(git status --porcelain)" ] && [ "$DRY_RUN" = false ]; then
    log_error "❌ 工作目录不干净，请先提交或暂存所有更改"
    log_info "未提交的更改:"
    git status --short
    exit 1
fi

# 检查是否在主分支
CURRENT_BRANCH=$(git branch --show-current)
if [ "$CURRENT_BRANCH" != "main" ] && [ "$CURRENT_BRANCH" != "master" ] && [ "$DRY_RUN" = false ]; then
    log_warning "⚠️  当前不在主分支 ($CURRENT_BRANCH)，确定要继续吗？"
    read -p "输入 'yes' 继续: " confirm
    if [ "$confirm" != "yes" ]; then
        log_info "取消发布"
        exit 0
    fi
fi

# 更新 Cargo.toml 中的版本
log_info "📝 更新 Cargo.toml 版本到 $VERSION"
if [ "$DRY_RUN" = false ]; then
    # 使用 sed 更新版本，只更新 [package] 部分的第一个 version
    if [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS
        sed -i '' '/^\[package\]/,/^\[/ { /^version = / { s/^version = "[^"]*"/version = "'"$VERSION"'"/; } }' Cargo.toml
    else
        # Linux
        sed -i '/^\[package\]/,/^\[/ { /^version = / { s/^version = "[^"]*"/version = "'"$VERSION"'"/; } }' Cargo.toml
    fi
fi

# 运行测试
log_info "🧪 运行测试..."
if [ "$DRY_RUN" = false ]; then
    if ! cargo test; then
        log_error "❌ 测试失败，取消发布"
        exit 1
    fi
    log_success "✅ 所有测试通过"
fi



# 生成 changelog
log_info "📋 生成 changelog..."
if [ "$DRY_RUN" = false ]; then
    # 检查是否安装了 git-cliff
    if ! command -v git-cliff &> /dev/null; then
        log_warning "⚠️  git-cliff 未安装，跳过 changelog 生成"
        log_info "可以通过以下命令安装: cargo install git-cliff"
    else
        git-cliff --tag "v$VERSION" --output CHANGELOG.md
        log_success "✅ Changelog 已生成"
    fi
fi

# 提交更改
log_info "📦 提交版本更改..."
if [ "$DRY_RUN" = false ]; then
    git add Cargo.toml
    if [ -f "CHANGELOG.md" ]; then
        git add CHANGELOG.md
    fi
    git commit -m "chore: bump version to $VERSION"
    log_success "✅ 版本更改已提交"
fi

# 创建标签
log_info "🏷️  创建 git 标签 v$VERSION..."
if [ "$DRY_RUN" = false ]; then
    git tag -a "v$VERSION" -m "Release v$VERSION"
    log_success "✅ 标签已创建"
fi

# 推送到远程
log_info "🚀 推送到远程仓库..."
if [ "$DRY_RUN" = true ]; then
    log_info "DryRun: 将执行以下命令:"
    log_info "  git push origin $CURRENT_BRANCH"
    log_info "  git push origin v$VERSION"
else
    git push origin "$CURRENT_BRANCH"
    git push origin "v$VERSION"
    log_success "✅ 已推送到远程仓库"
fi

log_success "🎉 发布准备完成！"
echo ""
log_info "接下来的步骤:"
log_info "1. GitHub Actions 将自动构建多平台二进制文件"
log_info "2. 自动创建 GitHub Release"
log_info "3. 查看发布状态: https://github.com/your-username/code-nexus/actions"
echo ""
log_info "如果需要取消发布，可以删除标签:"
log_info "  git tag -d v$VERSION"
log_info "  git push origin :refs/tags/v$VERSION"
