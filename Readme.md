<!-- markdownlint-disable MD036 MD029 -->
# 🚀 Funky Lesson Core

[![Rust](https://img.shields.io/badge/language-Rust-orange.svg)](https://rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Version](https://img.shields.io/badge/version-0.1.0-blue.svg)](https://github.com/Islatri/funky_lesson_core)
[![CI](https://github.com/Islatri/funky_lesson_core/actions/workflows/ci.yml/badge.svg)](https://github.com/Islatri/funky_lesson_core/actions/workflows/ci.yml)
[![Release](https://github.com/Islatri/funky_lesson_core/actions/workflows/release.yml/badge.svg)](https://github.com/Islatri/funky_lesson_core/actions/workflows/release.yml)

**高性能、多线程的吉林大学抢课自动化工具**

一个基于 Rust 开发的高效抢课脚本，提供稳定的多线程选课支持和自动重连功能。

![funky_lesson_core GIF演示](./funky-lesson-core.gif)

## ✨ 特性

- 🔥 **高性能多线程**: 默认8线程并发轮询，大大提高选课成功率
- 🔄 **智能轮询**: 每个线程从不同课程开始轮询，避免冲突
- 🌐 **自动重连**: 网络中断自动重连，保持持续选课
- 🛡️ **安全加密**: 使用AES加密保护用户凭据
- 📱 **跨平台**: 支持 Windows、macOS、Linux
- 🎯 **精准控制**: 500ms请求间隔，平衡效率与服务器负载
- 🔧 **灵活配置**: 支持WASM和原生两种运行模式

## 🏗️ 技术架构

本项目是 [funky-lesson](https://github.com/ZoneHerobrine/funky-lesson) 的核心库，采用现代化技术栈：

- **核心语言**: Rust (Edition 2024)
- **异步运行时**: Tokio
- **HTTP客户端**: Reqwest (支持 rustls-tls)
- **加密算法**: AES-128-ECB
- **序列化**: Serde + JSON
- **GUI版本**: 基于 Leptos + Actix + Tauri 的纯 Rust 实现

### 📦 GUI版本

完整的图形界面版本已经发布！

- 🎯 **下载地址**: [Release v0.0.4](https://github.com/ZoneHerobrine/funky-lesson/releases/tag/release)
- 🖥️ **技术栈**: Leptos + Actix + Tauri
- 📱 **开箱即用**: 无需配置环境，下载即可使用

## 📋 项目起源

本项目基于 [MoonWX](https://github.com/MoonWX/Fuck-Lesson) 从 [H4ckF0rFun](https://github.com/H4ckF0rFun) 同学的 Python 抢课脚本重写而成。

- 📁 **原版脚本**: `raw.py` (保留原始Python实现)
- 🦀 **Rust重写**: `examples/standalone.rs` (单文件Rust实现)
- 📚 **库版本**: `src/` (模块化库实现，适配GUI应用)

## 🙏 致谢

感谢以下开发者的贡献：

- **[H4ckF0rFun](https://github.com/H4ckF0rFun)**: 原始Python抢课脚本的创作者
- **[MoonWX](https://github.com/MoonWX/Fuck-Lesson)**: Python脚本的优化和维护者

> 注：由于原仓库未附带开源许可证，在此进行口头致谢。原Python脚本已完整保留在 `raw.py` 文件中。

## 🚀 快速开始

### 前置准备

1. **添加课程到收藏**: 在教务系统网站上将要选的课程添加到收藏列表
2. **获取选课信息**: 确认选课轮次（从0开始计数）

### 方式一：开发者模式 (源码运行)

#### 环境要求

- 📦 **Rust环境**: 请先安装 Rust 工具链
  - 官方安装指南: [https://www.rust-lang.org/learn/get-started](https://www.rust-lang.org/learn/get-started)
  - 推荐版本: Rust 1.70.0 或更高版本

#### 运行步骤

1. **克隆仓库**

```bash
git clone https://github.com/Islatri/funky_lesson_core.git
cd funky_lesson_core
```

2. **运行程序**

```bash
cargo run <用户名> <密码> <选课轮次> [是否循环]
```

**参数说明**:

- `<用户名>`: 教务系统登录用户名
- `<密码>`: 教务系统登录密码  
- `<选课轮次>`: 选课轮次编号（从0开始）
- `[是否循环]`: 可选参数，填写任意数字启用循环模式

**示例**:

```bash
# 单次选课
cargo run 114514 1919810 0

# 循环选课模式
cargo run 114514 1919810 0 1
```

1. **输入验证码**
   - 程序会自动下载验证码图片到 `captcha.png`
   - 在终端中输入验证码（不区分大小写）
   - 程序开始自动选课

### 方式二：可执行文件模式

#### 下载预编译版本

1. **下载**: 从 [Releases](https://github.com/Islatri/funky_lesson_core/releases) 页面下载最新版本
2. **解压**: 将 `funky_lesson_core.exe` 解压到任意目录

#### 运行命令

**PowerShell** (推荐):

```powershell
./funky_lesson_core.exe <用户名> <密码> <选课轮次> [是否循环]
```

**命令提示符 (CMD)**:

```cmd
funky_lesson_core.exe <用户名> <密码> <选课轮次> [是否循环]
```

**使用示例**:

```powershell
# PowerShell - 循环模式
./funky_lesson_core.exe 114514 1919810 0 1

# CMD - 单次模式  
funky_lesson_core.exe 114514 1919810 0
```

## ⚡ 性能特性

- **🔥 多线程并发**: 8个工作线程同时运行
- **🎯 智能调度**: 各线程从不同课程开始，避免竞争
- **⏱️ 精确间隔**: 500ms请求间隔，平衡效率与稳定性
- **🔄 自动恢复**: 网络异常自动重连，无需手动重启
- **📊 实时反馈**: 详细的运行状态和错误信息

## 📚 API文档

### 库特性 (Features)

- `default = ["no-wasm", "tui"]`: 默认特性
- `no-wasm`: 原生环境支持 (Tokio + Reqwest)
- `wasm`: WebAssembly支持 (Gloo + Web-sys)  
- `tui`: 命令行界面
- `gui`: 图形界面支持
- `proxy`: 代理支持

### 核心模块

```rust
// 基础使用示例
use funky_lesson_core::{
    client::LessonClient,
    model::LoginRequest,
    interface::ClientInterface,
};
```

## ⚠️ 重要提醒

> **⚠️ 使用须知**
>
> - 📈 **成功率**: 程序无法保证100%选课成功，请保持理性预期
> - 🌐 **网络依赖**: 教务系统服务器不够稳定，严重网络中断可能随时发生
> - 🔄 **备用方案**: 如遇脚本无响应，请同时准备浏览器手动选课
> - ⏰ **时机把握**: 在选课开放的黄金时间段使用效果最佳
> - 🔒 **账号安全**: 请勿在公共设备上使用，注意保护个人凭据

## 📄 免责声明

### 重要法律声明

- 🎓 **用途限制**: 本软件仅供学习和研究使用，请勿用于违反学校规定或法律法规的行为
- 🚫 **风险承担**: 使用本软件所产生的一切后果均由用户自行承担，开发者不承担任何直接或间接责任
- ⚖️ **合规使用**: 用户必须遵守所在机构及国家的相关法律法规，违规责任自负
- 🏛️ **非官方软件**: 本软件未经吉林大学官方授权，与吉林大学无任何官方关联
- ✅ **协议同意**: 使用本程序即代表您完全理解并同意本免责声明

## 🤝 贡献指南

欢迎提交Issue和Pull Request！

1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 创建 Pull Request

## 📜 开源许可

本项目采用 [MIT License](LICENSE) 开源协议。

```text
MIT License - Copyright (c) 2024 ChisatoZone
```

---

**⭐ 如果这个项目对你有帮助，请给一个Star！**

[🐛 报告问题](https://github.com/Islatri/funky_lesson_core/issues) • [💡 功能建议](https://github.com/Islatri/funky_lesson_core/issues) • [📖 文档](https://github.com/Islatri/funky_lesson_core/wiki)
