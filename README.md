# SubSnap - 多模式YUV到RGB转换性能测试工具

一个支持多种YUV到RGB转换实现的性能测试和对比工具。

## 🚀 功能特性

### 支持的转换模式

1. **FFmpeg模式** (`ffmpeg`) - 使用FFmpeg SWScale进行CPU转换
   - ✅ 默认启用，无需额外feature
   - 🔧 成熟稳定，兼容性好
   - 💻 CPU优化，支持多种像素格式

2. **OpenCV模式** (`opencv`) - 使用OpenCV库进行CPU转换
   - ✅ 默认启用，无需额外feature
   - 🎨 使用OpenCV的cvt_color函数进行标准化转换
   - 📚 适合使用OpenCV库进行色彩空间转换

3. **手工模式** (`manual`) - 使用手工实现进行CPU转换
   - ✅ 默认启用，无需额外feature
   - 🎨 手工实现的YUV到RGB转换算法
   - 📚 适合学习和理解转换原理

4. **WGPU模式** (`wgpu`) - 使用WGPU进行GPU加速转换
   - 🎯 需要 `--features wgpu-mode` 启用
   - 🚀 GPU计算着色器加速
   - ⚡ 适合批量处理大量帧

5. **yuvutils-rs模式** (`yuvutils`) - 使用yuvutils-rs进行高性能CPU转换
   - 🎯 需要 `--features yuvutils-mode` 启用
   - 🔥 纯Rust实现，SIMD优化
   - 🏆 专门针对YUV420P格式优化

## 📦 安装和编译

### 基础功能（FFmpeg + OpenCV模式）
```bash
cargo build --release
```

### 启用WGPU GPU加速
```bash
cargo build --release --features wgpu-mode
```

### 启用yuvutils-rs高性能模式
```bash
cargo build --release --features yuvutils-mode
```

### 启用所有模式
```bash
cargo build --release --features all-modes
```

## 🎯 使用方法

### 1. 查看可用模式
```bash
cargo run -- --list-modes
```

### 2. 测试单个模式
```bash
# 测试FFmpeg模式
cargo run -- --mode ffmpeg --frames 10

# 测试OpenCV模式
cargo run -- --mode opencv --frames 10

# 测试手工模式
cargo run -- --mode manual --frames 10

# 测试WGPU模式（需要对应feature）
cargo run --features wgpu-mode -- --mode wgpu --frames 10

# 测试yuvutils模式（需要对应feature）
cargo run --features yuvutils-mode -- --mode yuvutils --frames 10
```

### 3. 运行性能基准测试
```bash
# 对比所有启用的模式
cargo run --features all-modes -- --benchmark --frames 50

# 仅对比FFmpeg和WGPU
cargo run --features wgpu-mode -- --benchmark --frames 50
```

### 4. 保存转换后的图片
```bash
# 测试并保存图片
cargo run -- --mode ffmpeg --frames 5 --save-images

# 性能测试后保存最佳模式的结果
cargo run --features all-modes -- --benchmark --frames 10 --save-images
```

### 5. 使用自定义输入文件
```bash
cargo run -- --mode ffmpeg --input my_video.mp4 --frames 5 --output my_output
```

## 📊 性能对比示例

```bash
$ cargo run --features all-modes -- --benchmark --frames 50

🏆 运行性能基准测试...
📸 提取了 50 帧用于性能测试 (1920x1080, YUV420P)

🚀 开始测试 使用FFmpeg SWScale进行CPU转换 模式...
📊 使用FFmpeg SWScale进行CPU转换 转换性能统计:
  🎞️  处理帧数: 50
  ⏱️  总耗时: 0.05秒
  📈 平均每帧: 1.00ms
  🚀 转换FPS: 1000.0

🚀 开始测试 使用OpenCV进行CPU转换 模式...
📊 使用OpenCV进行CPU转换 转换性能统计:
  🎞️  处理帧数: 50
  ⏱️  总耗时: 0.05秒
  📈 平均每帧: 1.00ms
  🚀 转换FPS: 1000.0

🚀 开始测试 使用手工实现进行CPU转换 模式...
📊 使用手工实现进行CPU转换 转换性能统计:
  🎞️  处理帧数: 50
  ⏱️  总耗时: 0.16秒
  📈 平均每帧: 3.12ms
  🚀 转换FPS: 320.5

🚀 开始测试 使用WGPU进行GPU加速转换 模式...
📊 使用WGPU进行GPU加速转换 转换性能统计:
  🎞️  处理帧数: 50
  ⏱️  总耗时: 0.25秒
  📈 平均每帧: 4.90ms
  🚀 转换FPS: 203.7

🚀 开始测试 使用yuvutils-rs进行高性能CPU转换 模式...
📊 使用yuvutils-rs进行高性能CPU转换 转换性能统计:
  🎞️  处理帧数: 50
  ⏱️  总耗时: 0.05秒
  📈 平均每帧: 1.00ms
  🚀 转换FPS: 1000.0

🏆 性能对比总结:
┌─────────────────┬──────────┬──────────┬──────────┐
│      模式       │ 帧数     │ 平均耗时 │   FPS    │
├─────────────────┼──────────┼──────────┼──────────┤
│ ffmpeg          │       50 │   1.00ms │ 1000.0   │
│ opencv          │       50 │   1.00ms │ 1000.0   │
│ yuvutils        │       50 │   1.00ms │ 1000.0   │
│ manual          │       50 │   3.12ms │  320.5   │
│ wgpu            │       50 │   4.90ms │  203.7   │
└─────────────────┴──────────┴──────────┴──────────┘
🥇 最快模式: ffmpeg/opencv/yuvutils (1000.0 FPS)
```

### 性能分析

1. **FFmpeg模式** - 🥇 顶级性能 (1000.0 FPS)
   - 高度优化的C库实现
   - 使用硬件加速和SIMD指令
   - 适合生产环境的高性能需求

2. **OpenCV模式** - 🥇 顶级性能 (1000.0 FPS)
   - 使用OpenCV的原生转换函数
   - 计算机视觉领域的标准库
   - 适合与OpenCV项目集成

3. **yuvutils-rs模式** - 🥇 顶级性能 (1000.0 FPS)
   - 专为YUV转换优化的Rust库
   - SIMD优化，纯Rust实现
   - 平衡了性能和内存安全

4. **Manual模式** - 🥈 良好性能 (320.5 FPS)
   - 手工实现的转换算法
   - 主要用于教育和理解原理
   - 算法逻辑清晰透明

5. **WGPU模式** - 🥉 GPU加速 (203.7 FPS)
   - GPU计算，适合批量处理
   - 并行优势在大量数据时体现
   - 现代GPU API支持

## 🔧 命令行选项

```
SubSnap - 多模式YUV到RGB转换性能测试工具

Usage: sub-snap [OPTIONS]

Options:
  -m, --mode <MODE>         转换模式 [possible values: ffmpeg, opencv, manual, wgpu, yuvutils]
  -b, --benchmark           是否运行所有模式的性能对比测试
  -i, --input <INPUT>       输入视频文件路径 [default: input.mp4]
  -f, --frames <FRAMES>     要提取的帧数（用于测试） [default: 10]
  -s, --save-images         是否保存转换后的图片
  -o, --output <OUTPUT>     输出目录 [default: extracted_frames]
  -l, --list-modes          列出可用的转换模式
  -h, --help                Print help
  -V, --version             Print version
```

## 📋 系统要求

### 基础要求
- Rust 1.70+
- FFmpeg 开发库
- OpenCV 开发库

### WGPU模式额外要求
- 支持现代图形API的GPU (Vulkan/Metal/DX12)
- 适当的图形驱动程序

### yuvutils-rs模式额外要求
- 支持SIMD指令集的CPU (大部分现代CPU都支持)

## 🛠️ 开发和调试

### 自动化性能测试
```bash
# 运行完整的性能测试脚本
./test_performance.sh
```

### 启用日志
```bash
RUST_LOG=debug cargo run -- --mode ffmpeg --frames 3
```

### 仅编译检查
```bash
cargo check --features all-modes
```

### 运行测试
```bash
cargo test --features all-modes
```

### 性能调优
```bash
# 启用CPU特定优化
RUSTFLAGS="-C target-cpu=native" cargo build --release --features all-modes
```

## 🎯 使用场景

### 1. 性能评估
- 测试不同转换模式在特定硬件上的性能
- 找出最适合当前环境的转换方案

### 2. 算法验证
- 验证不同实现的转换结果一致性
- 调试转换质量问题

### 3. 批处理优化
- 为大规模视频处理选择最优方案
- 根据硬件配置选择合适的并行策略

### 4. 学习和研究
- 比较不同实现的转换算法
- 理解YUV到RGB转换的原理

## 🏗️ 架构设计

```
src/
├── main.rs                     # 命令行界面和主程序逻辑
├── converter.rs                # 核心转换器框架和性能测试
├── converters/                 # 模块化转换器实现
│   ├── mod.rs                  # 模块导出
│   ├── ffmpeg_converter.rs     # FFmpeg SWScale实现
│   ├── opencv_converter.rs     # 真正的OpenCV实现
│   ├── manual_converter.rs     # 手工实现转换器
│   ├── wgpu_converter.rs       # WGPU GPU实现
│   └── yuvutils_converter.rs   # yuvutils-rs实现
├── wgpu_processor.rs           # WGPU GPU处理器
├── yuv_to_rgb.wgsl            # WGPU计算着色器
└── (其他模块...)               # 配置、帧处理等支持模块
```

## 📈 性能优化建议

1. **对于生产环境**: 使用FFmpeg模式获得成熟稳定的最佳性能
2. **对于计算机视觉项目**: 使用OpenCV模式与现有OpenCV代码集成
3. **对于纯Rust环境**: 使用yuvutils-rs提供内存安全的高性能选择
4. **对于学习研究**: 使用Manual模式理解转换原理
5. **对于GPU丰富环境**: WGPU模式适合大规模并行处理

## 📖 文档

- [📖 README.md](./README.md) - 快速开始和基本使用
- [🔧 TECHNICAL_DETAILS.md](./TECHNICAL_DETAILS.md) - 技术实现详情
- [🧪 test_performance.sh](./test_performance.sh) - 自动化性能测试脚本

## 🤝 贡献

欢迎提交Issue和Pull Request！

## �� 许可证

MIT License
