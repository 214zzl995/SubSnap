# SubSnap - 字幕处理工具

一个使用 Rust 和 FFmpeg 构建的字幕处理工具。

## 功能特性

- 🎬 集成 FFmpeg 进行媒体文件处理
- 📝 字幕提取和处理
- 🎥 支持多种视频格式
- 🎵 支持音频处理

## 依赖要求

### 系统依赖

在 macOS 上，需要安装以下依赖：

```bash
# 安装 pkg-config 和 ffmpeg
brew install pkgconf ffmpeg
```

在 Linux 上：

```bash
# Ubuntu/Debian
sudo apt update
sudo apt install pkg-config libavutil-dev libavformat-dev libavcodec-dev libavdevice-dev libavfilter-dev libswscale-dev libswresample-dev

# 或者安装 FFmpeg 开发包
sudo apt install ffmpeg libffmpeg-dev
```

### Rust 依赖

项目使用 `ffmpeg-next` crate 来与 FFmpeg 库进行交互：

```toml
[dependencies]
ffmpeg-next = "7.0"
```

## 构建和运行

```bash
# 克隆项目
git clone <repository-url>
cd SubSnap

# 构建项目
cargo build

# 运行项目
cargo run
```

## 项目结构

```
SubSnap/
├── Cargo.toml          # 项目配置和依赖
├── src/
│   └── main.rs        # 主程序入口
├── README.md          # 项目说明
└── LICENSE            # 许可证
```

## 开发状态

当前项目处于初始阶段，已完成：

- ✅ FFmpeg 集成
- ✅ 基础项目结构
- ✅ 依赖配置

计划功能：

- 🔄 字幕文件解析
- 🔄 视频字幕提取
- 🔄 字幕格式转换
- 🔄 字幕时间轴调整

## 贡献

欢迎提交 Issue 和 Pull Request！

## 许可证

[根据项目需要选择适当的许可证]
