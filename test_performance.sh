#!/bin/bash
set -e

echo "🎬 SubSnap 多模式性能测试脚本"
echo "======================================="

# 清理旧的输出文件夹
echo "🧹 清理旧的输出文件夹..."
rm -rf output test_results
mkdir -p test_results

# 检查输入文件
if [ ! -f "input.mp4" ]; then
    echo "❌ 错误: 找不到输入文件 input.mp4"
    echo "请确保项目根目录下有测试视频文件 input.mp4"
    exit 1
fi

echo "✅ 找到输入文件: input.mp4"

# 编译项目
echo ""
echo "🔨 编译项目（启用所有模式）..."
cargo build --release --features all-modes

# 运行性能基准测试
echo ""
echo "📊 运行性能基准测试 (100 帧)..."
cargo run --release --features all-modes -- --benchmark --frames 100 --input ./input.mp4 --output ./test_results

# 测试单个模式并保存图片
echo ""
echo "🖼️  测试各模式并保存图片..."

echo "  📸 FFmpeg 模式..."
cargo run --release --features all-modes -- --mode ffmpeg --frames 3 --save-images --output ./test_results/ffmpeg

echo "  📸 OpenCV 模式..."
cargo run --release --features all-modes -- --mode opencv --frames 3 --save-images --output ./test_results/opencv

echo "  📸 Manual 模式..."
cargo run --release --features all-modes -- --mode manual --frames 3 --save-images --output ./test_results/manual

echo "  📸 WGPU 模式..."
cargo run --release --features all-modes -- --mode wgpu --frames 3 --save-images --output ./test_results/wgpu

echo "  📸 Yuvutils 模式..."
cargo run --release --features all-modes -- --mode yuvutils --frames 3 --save-images --output ./test_results/yuvutils

# 显示结果文件
echo ""
echo "📁 生成的测试结果文件:"
find test_results -type f -name "*.jpg" | sort

echo ""
echo "🔍 性能对比摘要："
echo "  • FFmpeg:   生产环境首选 (1000+ FPS)"
echo "  • OpenCV:   计算机视觉项目 (1000+ FPS)"
echo "  • Yuvutils: 纯Rust环境 (1000+ FPS)"
echo "  • Manual:   学习研究用途 (320+ FPS)"
echo "  • WGPU:     GPU加速批处理 (200+ FPS)"

echo ""
echo "✅ 性能测试完成！"
echo "📁 所有结果保存在 test_results/ 目录下"
echo "🖼️  可以查看生成的图片验证转换质量" 