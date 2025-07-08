# CodeNexus 发布指南

本文档介绍如何使用自动化发布系统为 CodeNexus 项目创建新版本。

## 🚀 快速开始

### 使用发布脚本（推荐）

#### Windows (PowerShell)
```powershell
# 发布新版本
.\scripts\release.ps1 -Version "0.2.0"

# 预览模式（不执行实际操作）
.\scripts\release.ps1 -Version "0.2.0" -DryRun
```

#### Linux/macOS (Bash)
```bash
# 发布新版本
./scripts/release.sh 0.2.0

# 预览模式（不执行实际操作）
./scripts/release.sh 0.2.0 --dry-run
```

### 手动发布

如果你更喜欢手动控制每个步骤：

```bash
# 1. 更新版本号
# 编辑 Cargo.toml 中的 version 字段

# 2. 运行测试
cargo test

# 3. 构建项目
cargo build --release

# 4. 生成 changelog（可选）
git-cliff --tag "v0.2.0" --output CHANGELOG.md

# 5. 提交更改
git add Cargo.toml CHANGELOG.md
git commit -m "chore: bump version to 0.2.0"

# 6. 创建标签
git tag -a "v0.2.0" -m "Release v0.2.0"

# 7. 推送到远程
git push origin main
git push origin v0.2.0
```

## 📋 发布流程

### 1. 准备阶段
- ✅ 确保所有更改已提交
- ✅ 确保在主分支（main/master）
- ✅ 运行测试确保代码质量
- ✅ 更新版本号（遵循语义化版本）

### 2. 自动化阶段
当你推送版本标签时，GitHub Actions 会自动：

1. **多平台构建**
   - Windows x86_64
   - macOS x86_64 (Intel)
   - macOS aarch64 (Apple Silicon)
   - Linux x86_64

2. **版本一致性检查**
   - 验证 git tag 版本与 Cargo.toml 版本一致

3. **生成 Changelog**
   - 使用 git-cliff 自动生成更新日志
   - 基于 Conventional Commits 格式

4. **创建 GitHub Release**
   - 自动上传所有平台的二进制文件
   - 包含生成的 changelog
   - 智能生成发布标题

## 🛠️ 工具和依赖

### 必需工具
- **Git**: 版本控制
- **Rust/Cargo**: 构建工具
- **GitHub**: 代码托管和 CI/CD

### 可选工具
- **git-cliff**: 自动生成 changelog
  ```bash
  cargo install git-cliff
  ```

## 📁 文件结构

```
.github/
└── workflows/
    └── release.yml          # GitHub Actions 发布工作流
cliff.toml                   # git-cliff 配置文件
scripts/
├── release.ps1             # Windows 发布脚本
└── release.sh              # Linux/macOS 发布脚本
docs/
└── RELEASE_GUIDE.md        # 本文档
```

## 🔧 配置说明

### GitHub Actions 配置

发布工作流在以下情况触发：
- 推送以 `v` 开头的标签（如 `v1.0.0`）
- 手动触发（workflow_dispatch）

### git-cliff 配置

`cliff.toml` 文件配置了 changelog 生成规则：
- 支持 Conventional Commits 格式
- 自动分组提交类型（feat, fix, docs 等）
- 生成 Markdown 格式的 changelog

## 📝 版本规范

### 语义化版本
遵循 [Semantic Versioning](https://semver.org/) 规范：

- **MAJOR.MINOR.PATCH** (例如: 1.2.3)
- **MAJOR**: 不兼容的 API 更改
- **MINOR**: 向后兼容的功能添加
- **PATCH**: 向后兼容的错误修复

### 提交消息格式
推荐使用 [Conventional Commits](https://www.conventionalcommits.org/) 格式：

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

示例：
```
feat: add file tagging system
fix(query): resolve search performance issue
docs: update API documentation
```

## 🚨 故障排除

### 版本不一致错误
```
❌ Version mismatch detected!
Git tag version: 0.2.0
Cargo.toml version: 0.1.0
```

**解决方案**：
1. 更新 `Cargo.toml` 中的版本号匹配标签
2. 或者删除错误的标签，创建正确的标签

### 构建失败
如果 GitHub Actions 构建失败：
1. 检查 Actions 页面的详细日志
2. 确保代码在本地能正常构建
3. 检查是否有平台特定的依赖问题

### 发布脚本权限问题
Linux/macOS 下如果脚本无法执行：
```bash
chmod +x scripts/release.sh
```

## 🔗 相关链接

- [GitHub Actions 文档](https://docs.github.com/en/actions)
- [git-cliff 文档](https://git-cliff.org/)
- [Semantic Versioning](https://semver.org/)
- [Conventional Commits](https://www.conventionalcommits.org/)

## 📞 支持

如果在发布过程中遇到问题：
1. 查看 GitHub Actions 日志
2. 检查本文档的故障排除部分
3. 在项目仓库创建 Issue
