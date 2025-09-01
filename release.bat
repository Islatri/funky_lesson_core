@echo off
setlocal enabledelayedexpansion

REM 简单的Windows发布脚本
REM 使用方法: release.bat v0.1.1

if "%1"=="" (
    echo ❌ 请提供版本号!
    echo 使用方法: %0 v0.1.1
    exit /b 1
)

set VERSION=%1

REM 验证版本号格式 (简单检查)
echo %VERSION% | findstr /r "^v[0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*$" >nul
if errorlevel 1 (
    echo ❌ 版本号格式错误! 请使用 vX.Y.Z 格式 ^(如: v0.1.1^)
    exit /b 1
)

echo 🚀 准备发布版本: %VERSION%

REM 检查是否有未提交的更改
git status --porcelain > temp_status.txt
for %%A in (temp_status.txt) do if %%~zA neq 0 (
    echo ❌ 检测到未提交的更改，请先提交所有更改!
    git status --short
    del temp_status.txt
    exit /b 1
)
del temp_status.txt

REM 检查当前分支
for /f "tokens=*" %%A in ('git branch --show-current') do set CURRENT_BRANCH=%%A
if not "%CURRENT_BRANCH%"=="main" (
    echo ❌ 请在main分支上进行发布! 当前分支: %CURRENT_BRANCH%
    exit /b 1
)

REM 移除版本号前缀v
set VERSION_NUMBER=%VERSION:~1%
echo 📝 更新Cargo.toml版本号为: %VERSION_NUMBER%

REM 使用PowerShell更新Cargo.toml
powershell -Command "(Get-Content Cargo.toml) -replace '^version = \".*\"', 'version = \"%VERSION_NUMBER%\"' | Set-Content Cargo.toml"

REM 更新README版本徽章
echo 📝 更新README.md版本徽章
powershell -Command "(Get-Content Readme.md) -replace 'version-[^-]*-blue', 'version-%VERSION_NUMBER%-blue' | Set-Content Readme.md"

REM 检查编译
echo 🔧 检查编译...
cargo check --release
if errorlevel 1 (
    echo ❌ 编译检查失败!
    exit /b 1
)

REM 提交版本更新
echo 📝 提交版本更新...
git add Cargo.toml Readme.md
git commit -m "chore: bump version to %VERSION%"

REM 创建并推送标签
echo 🏷️  创建版本标签: %VERSION%
git tag -a "%VERSION%" -m "Release %VERSION%"

echo 📤 推送到远程仓库...
git push origin main
git push origin "%VERSION%"

echo.
echo 🎉 发布流程已启动!
echo 📋 版本: %VERSION%
echo 🔗 查看构建进度: https://github.com/Islatri/funky_lesson_core/actions
echo 📦 发布完成后可在此查看: https://github.com/Islatri/funky_lesson_core/releases
echo.
echo ⏳ 预计5-10分钟后完成构建，请耐心等待...
