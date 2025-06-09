#!/bin/bash

# 热力图功能演示脚本 - 快速测试

echo "🚀 热力图功能演示..."

# 检查输入文件是否存在
if [ ! -f "../input.mp4" ]; then
    echo "❌ 错误：找不到 ../input.mp4 文件"
    echo "请将测试视频文件命名为 input.mp4 并放在项目根目录"
    exit 1
fi

echo "✅ 找到 ../input.mp4 文件"

# 构建项目
echo "🔨 构建项目..."
cd .. && cargo build --release && cd testing

# 定义测试模式（只测试几个模式，减少时间）
modes=("ffmpeg" "manual")
test_count=2

echo "📊 开始演示测试..."
echo "🔢 每个模式测试 $test_count 次"

# 确保results目录存在
mkdir -p results

# 清理旧的监控文件
rm -f results/*_monitor.csv 2>/dev/null

# 对每个模式进行测试
for mode in "${modes[@]}"; do
    echo "🔄 测试模式: $mode"
    
    monitor_file="results/${mode}_monitor.csv"
    
    for i in $(seq 1 $test_count); do
        echo -n "  运行 $i/$test_count... "
        
        # 启动系统监控（后台运行）
        if [ -x "./monitor_system.sh" ]; then
            ./monitor_system.sh "$monitor_file" 10 "sub_snap" &
            monitor_pid=$!
        fi
        
        # 执行测试
        output=$(cd .. && cargo run --release -- --mode "$mode" --input input.mp4 --frames 5 2>&1)
        exit_code=$?
        
        # 停止监控
        if [ -n "$monitor_pid" ]; then
            kill $monitor_pid 2>/dev/null || true
            wait $monitor_pid 2>/dev/null || true
        fi
        
        if [ $exit_code -eq 0 ]; then
            time_taken=$(echo "$output" | grep "耗时" | sed -n 's/.*耗时 \([0-9]*\.[0-9]*\)秒.*/\1/p')
            if [ -z "$time_taken" ]; then
                time_taken=$(echo "$output" | grep "耗时" | sed -n 's/.*耗时 \([0-9]*\)秒.*/\1/p')
            fi
            
            if [ -n "$time_taken" ]; then
                echo "✅ ${time_taken}秒"
            else
                echo "⚠️  无法解析时间"
            fi
        else
            echo "❌ 失败"
        fi
        
        sleep 1  # 稍微间隔一下
    done
    echo ""
done

# 检查监控数据
echo "📊 检查监控数据..."
monitor_files=$(ls results/*_monitor.csv 2>/dev/null)
if [ -n "$monitor_files" ]; then
    for file in $monitor_files; do
        if [ -s "$file" ]; then
            line_count=$(wc -l < "$file")
            echo "  ✅ $file: $line_count 行数据"
        else
            echo "  ⚠️  $file: 文件为空"
        fi
    done
else
    echo "  ❌ 未找到监控数据文件"
fi

# 生成热力图
echo ""
echo "🎨 生成演示热力图..."
if command -v python3 >/dev/null 2>&1; then
    python3 -c "import pandas, matplotlib, seaborn" 2>/dev/null
    if [ $? -eq 0 ]; then
        if [ -f "generate_heatmap.py" ]; then
            python3 generate_heatmap.py --data "results/*_monitor.csv" --output "results/demo_charts"
            if [ -d "results/demo_charts" ]; then
                echo "📊 演示热力图已生成在 results/demo_charts/ 目录："
                ls -la results/demo_charts/
            fi
        else
            echo "⚠️  未找到 generate_heatmap.py 脚本"
        fi
    else
        echo "⚠️  缺少Python库，请运行: ./setup_heatmap.sh"
    fi
else
    echo "⚠️  未找到Python3"
fi

echo ""
echo "🎉 演示完成！" 