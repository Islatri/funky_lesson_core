#!/bin/bash

# 简单的发布脚本
# 使用方法: ./release.sh v0.1.1

set -e

# 检查参数
if [ $# -eq 0 ]; then
    echo "❌ 请提供版本号!"
    echo "使用方法: $0 v0.1.1"
    exit 1
fi

VERSION=$1

# 验证版本号格式
if [[ ! $VERSION =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "❌ 版本号格式错误! 请使用 vX.Y.Z 格式 (如: v0.1.1)"
    exit 1
fi

echo "🚀 准备发布版本: $VERSION"

# 检查是否有未提交的更改
if [ -n "$(git status --porcelain)" ]; then
    echo "❌ 检测到未提交的更改，请先提交所有更改!"
    git status --short
    exit 1
fi

# 检查当前分支是否为main
CURRENT_BRANCH=$(git branch --show-current)
if [ "$CURRENT_BRANCH" != "main" ]; then
    echo "❌ 请在main分支上进行发布! 当前分支: $CURRENT_BRANCH"
    exit 1
fi

# 更新Cargo.toml中的版本号
VERSION_NUMBER=${VERSION#v}  # 移除前缀v
echo "📝 更新Cargo.toml版本号为: $VERSION_NUMBER"

# 使用sed更新版本号（跨平台兼容）
if [[ "$OSTYPE" == "darwin"* ]]; then
    # macOS
    sed -i '' "s/^version = \".*\"/version = \"$VERSION_NUMBER\"/" Cargo.toml
else
    # Linux
    sed -i "s/^version = \".*\"/version = \"$VERSION_NUMBER\"/" Cargo.toml
fi

# 更新README中的版本徽章
echo "📝 更新README.md版本徽章"
if [[ "$OSTYPE" == "darwin"* ]]; then
    # macOS
    sed -i '' "s/version-[^-]*-blue/version-$VERSION_NUMBER-blue/" Readme.md
else
    # Linux
    sed -i "s/version-[^-]*-blue/version-$VERSION_NUMBER-blue/" Readme.md
fi

# 检查编译是否正常
echo "🔧 检查编译..."
cargo check --release

# 提交版本更新
echo "📝 提交版本更新..."
git add Cargo.toml Readme.md
git commit -m "chore: bump version to $VERSION"

# 创建并推送标签
echo "🏷️  创建版本标签: $VERSION"
git tag -a "$VERSION" -m "Release $VERSION"

echo "📤 推送到远程仓库..."
git push origin main
git push origin "$VERSION"

echo ""
echo "🎉 发布流程已启动!"
echo "📋 版本: $VERSION"
echo "🔗 查看构建进度: https://github.com/Islatri/funky_lesson_core/actions"
echo "📦 发布完成后可在此查看: https://github.com/Islatri/funky_lesson_core/releases"
echo ""
echo "⏳ 预计5-10分钟后完成构建，请耐心等待..."
