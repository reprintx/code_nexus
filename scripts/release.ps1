# CodeNexus 发布脚本
# 用法: .\scripts\release.ps1 -Version "0.2.0"

param(
    [Parameter(Mandatory=$true)]
    [string]$Version,
    
    [switch]$DryRun = $false
)

# 颜色输出函数
function Write-ColorOutput($ForegroundColor) {
    $fc = $host.UI.RawUI.ForegroundColor
    $host.UI.RawUI.ForegroundColor = $ForegroundColor
    if ($args) {
        Write-Output $args
    } else {
        $input | Write-Output
    }
    $host.UI.RawUI.ForegroundColor = $fc
}

function Write-Success { Write-ColorOutput Green $args }
function Write-Warning { Write-ColorOutput Yellow $args }
function Write-Error { Write-ColorOutput Red $args }
function Write-Info { Write-ColorOutput Cyan $args }

# 检查是否在项目根目录
if (-not (Test-Path "Cargo.toml")) {
    Write-Error "❌ 请在项目根目录运行此脚本"
    exit 1
}

Write-Info "🚀 开始准备发布 CodeNexus v$Version"

# 验证版本格式
if ($Version -notmatch '^\d+\.\d+\.\d+$') {
    Write-Error "❌ 版本格式错误，请使用语义化版本格式 (例如: 1.0.0)"
    exit 1
}

# 检查工作目录是否干净
$gitStatus = git status --porcelain
if ($gitStatus -and -not $DryRun) {
    Write-Error "❌ 工作目录不干净，请先提交或暂存所有更改"
    Write-Info "未提交的更改:"
    git status --short
    exit 1
}

# 检查是否在主分支
$currentBranch = git branch --show-current
if ($currentBranch -ne "main" -and $currentBranch -ne "master" -and -not $DryRun) {
    Write-Warning "⚠️  当前不在主分支 ($currentBranch)，确定要继续吗？"
    $confirm = Read-Host "输入 'yes' 继续"
    if ($confirm -ne "yes") {
        Write-Info "取消发布"
        exit 0
    }
}

# 更新 Cargo.toml 中的版本
Write-Info "📝 更新 Cargo.toml 版本到 $Version"
if (-not $DryRun) {
    $cargoContent = Get-Content "Cargo.toml" -Raw
    # 只更新 [package] 部分的 version，使用更精确的正则表达式
    $cargoContent = $cargoContent -replace '(\[package\][\s\S]*?)version\s*=\s*"[^"]*"', "`$1version = `"$Version`""
    Set-Content "Cargo.toml" -Value $cargoContent -NoNewline
}

# 运行测试
Write-Info "🧪 运行测试..."
if (-not $DryRun) {
    cargo test
    if ($LASTEXITCODE -ne 0) {
        Write-Error "❌ 测试失败，取消发布"
        exit 1
    }
    Write-Success "✅ 所有测试通过"
}



# 生成 changelog
Write-Info "📋 生成 changelog..."
if (-not $DryRun) {
    # 检查是否安装了 git-cliff
    $gitCliff = Get-Command git-cliff -ErrorAction SilentlyContinue
    if (-not $gitCliff) {
        Write-Warning "⚠️  git-cliff 未安装，跳过 changelog 生成"
        Write-Info "可以通过以下命令安装: cargo install git-cliff"
    } else {
        git-cliff --tag "v$Version" --output CHANGELOG.md
        Write-Success "✅ Changelog 已生成"
    }
}

# 提交更改
Write-Info "📦 提交版本更改..."
if (-not $DryRun) {
    git add Cargo.toml Cargo.lock
    if (Test-Path "CHANGELOG.md") {
        git add CHANGELOG.md
    }
    git commit -m "chore: bump version to $Version"
    Write-Success "✅ 版本更改已提交"
}

# 创建标签
Write-Info "🏷️  创建 git 标签 v$Version..."
if (-not $DryRun) {
    git tag -a "v$Version" -m "Release v$Version"
    Write-Success "✅ 标签已创建"
}

# 推送到远程
Write-Info "🚀 推送到远程仓库..."
if ($DryRun) {
    Write-Info "DryRun: 将执行以下命令:"
    Write-Info "  git push origin $currentBranch"
    Write-Info "  git push origin v$Version"
} else {
    git push origin $currentBranch
    git push origin "v$Version"
    Write-Success "✅ 已推送到远程仓库"
}

Write-Success "🎉 发布准备完成！"
Write-Info ""
Write-Info "接下来的步骤:"
Write-Info "1. GitHub Actions 将自动构建多平台二进制文件"
Write-Info "2. 自动创建 GitHub Release"
Write-Info "3. 查看发布状态: https://github.com/your-username/code-nexus/actions"
Write-Info ""
Write-Info "如果需要取消发布，可以删除标签:"
Write-Info "  git tag -d v$Version"
Write-Info "  git push origin :refs/tags/v$Version"
