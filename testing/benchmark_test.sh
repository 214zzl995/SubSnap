#!/bin/bash

# 性能测试脚本
# 对每个转换模式执行10次测试并计算平均时间

set -e

# 全局变量用于跟踪后台进程
MONITOR_PIDS=()

# 信号处理函数 - 清理后台监控进程
cleanup() {
    echo ""
    echo "⚠️  收到中断信号，正在清理后台进程..."
    
    # 终止所有监控进程
    for pid in "${MONITOR_PIDS[@]}"; do
        if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
            echo "🔄 终止监控进程 PID: $pid"
            kill "$pid" 2>/dev/null || true
            wait "$pid" 2>/dev/null || true
        fi
    done
    
    echo "✅ 清理完成，退出测试"
    exit 1
}

# 捕获中断信号 (Ctrl+C) 和终止信号
trap cleanup SIGINT SIGTERM

# 测试配置参数
MAX_FRAMES=0         # 最大获取帧数，0 表示获取所有帧
TEST_FPS=1           # 测试用的 FPS
TEST_COUNT=10        # 每个模式的测试次数

echo "🚀 开始性能测试脚本..."
echo "📁 检查输入文件..."

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

if [ $? -ne 0 ]; then
    echo "❌ 构建失败"
    exit 1
fi

echo "✅ 构建完成"

# 定义测试模式
modes=("ffmpeg" "opencv" "manual" "wgpu" "yuvutils")

# 使用配置的测试次数
test_count=$TEST_COUNT


# 清理旧的测试数据
echo "🧹 清理旧的测试数据..."
if [ -d "results" ]; then
    # 保存 .gitignore 文件（如果存在）
    gitignore_backup=""
    if [ -f "results/.gitignore" ]; then
        gitignore_backup=$(cat "results/.gitignore")
    fi
    
    # 删除所有内容但保留目录
    find results -mindepth 1 -delete 2>/dev/null || {
        # 如果 find 命令失败，使用备用方法
        rm -rf results/*/ results/*.* 2>/dev/null || true
    }
    
    # 恢复 .gitignore 文件
    if [ -n "$gitignore_backup" ]; then
        echo "$gitignore_backup" > "results/.gitignore"
    fi
    
    echo "✅ 已清理旧数据"
else
    echo "📁 首次运行，无需清理"
fi

# 创建结果目录
mkdir -p results

# 创建 .gitignore 文件（如果不存在）
if [ ! -f "results/.gitignore" ]; then
    cat > "results/.gitignore" << 'EOF'
# 忽略所有测试结果文件
*

# 但保留 .gitignore 本身
!.gitignore

# 可选：保留 README.md 或其他文档文件
# !README.md
EOF
    echo "📝 创建了 results/.gitignore 文件"
fi

# 输出结果文件
output_file="results/benchmark_results.txt"
echo "📊 性能测试结果 - $(date)" > "$output_file"
echo "=========================================" >> "$output_file"

echo ""
echo "🧪 开始性能测试..."
echo "📝 测试参数：--frames $MAX_FRAMES --fps $TEST_FPS"
echo "🔢 每个模式测试 $test_count 次"
echo ""

# 对每个模式进行测试
for mode in "${modes[@]}"; do
    echo "🔄 测试模式: $mode"
    echo "模式: $mode" >> "$output_file"
    
    # 为这个模式创建子文件夹
    mode_dir="results/${mode}"
    mkdir -p "$mode_dir"
    
    total_time=0
    successful_runs=0
    
    for i in $(seq 1 $test_count); do
        echo -n "  运行 $i/$test_count... "
        
        # 为每次测试创建独立的监控数据文件
        run_dir="${mode_dir}/run_${i}"
        mkdir -p "$run_dir"
        monitor_file="${run_dir}/monitor.csv"
        
        # 启动系统监控（后台运行，无时间限制）
        monitor_pid=""
        if [ -x "./monitor_system.sh" ]; then
            ./monitor_system.sh "$monitor_file" 0 "sub_snap" &
            monitor_pid=$!
            # 将监控进程PID添加到全局数组
            MONITOR_PIDS+=("$monitor_pid")
        fi
        
        # 执行测试并捕获输出，同时保存到独立文件
        test_start_time=$(date "+%Y-%m-%d %H:%M:%S")
        output=$(cd .. && cargo run --release -- --mode "$mode" --input input.mp4 --frames $MAX_FRAMES --fps $TEST_FPS 2>&1)
        exit_code=$?
        test_end_time=$(date "+%Y-%m-%d %H:%M:%S")
        
        # 停止监控（如果还在运行）
        if [ -n "$monitor_pid" ]; then
            kill $monitor_pid 2>/dev/null || true
            wait $monitor_pid 2>/dev/null || true
            # 从全局数组中移除已停止的进程PID
            MONITOR_PIDS=("${MONITOR_PIDS[@]/$monitor_pid}")
        fi
        
        # 为每次测试保存详细信息到独立文件
        test_info_file="${run_dir}/test_info.txt"
        echo "测试信息 - 运行 $i" > "$test_info_file"
        echo "=========================================" >> "$test_info_file"
        echo "模式: $mode" >> "$test_info_file"
        echo "开始时间: $test_start_time" >> "$test_info_file"
        echo "结束时间: $test_end_time" >> "$test_info_file"
        echo "退出码: $exit_code" >> "$test_info_file"
        echo "完整输出:" >> "$test_info_file"
        echo "$output" >> "$test_info_file"
        echo "" >> "$test_info_file"
        
        # 为每次测试生成独立的热力图
        if [ -f "$monitor_file" ] && [ -s "$monitor_file" ]; then
            echo -n "📊 生成热力图... "
            if command -v python3 >/dev/null 2>&1; then
                # 检查是否有必要的Python库
                python3 -c "import pandas, matplotlib, seaborn" 2>/dev/null
                if [ $? -eq 0 ]; then
                    if [ -f "generate_heatmap.py" ]; then
                        python3 generate_heatmap.py --data "$monitor_file" --output "${run_dir}/charts" 2>/dev/null
                        if [ $? -eq 0 ]; then
                            echo "✅"
                        else
                            echo "❌"
                        fi
                    else
                        echo "⚠️"
                    fi
                else
                    echo "⚠️"
                fi
            else
                echo "⚠️"
            fi
        fi
        
        if [ $exit_code -eq 0 ]; then
            # 从输出中提取时间（支持各种输出格式）
            time_taken=$(echo "$output" | grep "耗时" | sed -n 's/.*耗时 \([0-9]*\.[0-9]*\)秒.*/\1/p')
            
            # 如果上面没有匹配到，尝试匹配整数秒
            if [ -z "$time_taken" ]; then
                time_taken=$(echo "$output" | grep "耗时" | sed -n 's/.*耗时 \([0-9]*\)秒.*/\1/p')
            fi
            
            if [ -n "$time_taken" ]; then
                total_time=$(echo "$total_time + $time_taken" | bc -l)
                successful_runs=$((successful_runs + 1))
                echo "✅ ${time_taken}秒"
                echo "  运行 $i: ${time_taken}秒" >> "$output_file"
                echo "执行时间: ${time_taken}秒" >> "$test_info_file"
                echo "测试状态: 成功" >> "$test_info_file"
            else
                echo "⚠️  无法解析时间"
                echo "  原始输出: $output" >&2
                echo "  运行 $i: 解析失败" >> "$output_file"
                echo "测试状态: 解析失败" >> "$test_info_file"
            fi
        else
            echo "❌ 失败"
            echo "  错误输出: $output" >&2
            echo "  运行 $i: 执行失败" >> "$output_file"
            echo "测试状态: 执行失败" >> "$test_info_file"
        fi
    done
    
    # 计算平均时间
    if [ $successful_runs -gt 0 ]; then
        avg_time=$(echo "scale=4; $total_time / $successful_runs" | bc -l)
        # 确保平均时间以0开头（处理.2410这种格式）
        if [[ $avg_time == .* ]]; then
            avg_time="0$avg_time"
        fi
        echo "📈 $mode 模式平均时间: ${avg_time}秒 (成功: $successful_runs/$test_count)"
        echo "  平均时间: ${avg_time}秒 (成功运行: $successful_runs/$test_count)" >> "$output_file"
        echo "  总时间: ${total_time}秒" >> "$output_file"
    else
        echo "❌ $mode 模式所有测试都失败了"
        echo "  所有测试都失败了" >> "$output_file"
    fi
    
    echo "" >> "$output_file"
    echo ""
done

echo "✅ 测试完成！"
echo "📄 详细结果已保存到: $output_file"
echo ""
echo "📊 测试摘要："

# 生成摘要
echo "=========================================" >> "$output_file"
echo "测试摘要:" >> "$output_file"

for mode in "${modes[@]}"; do
    # 查找该模式的平均时间行（需要查看更多行才能找到平均时间）
    avg_line=$(grep -A 15 "模式: $mode" "$output_file" | grep "平均时间" | head -1)
    if [ -n "$avg_line" ]; then
        # 提取平均时间数值，支持各种格式：0.2400、.2400、2 等
        avg_time=$(echo "$avg_line" | sed -n 's/.*平均时间: \([0-9]*\.[0-9]*\)秒.*/\1/p')
        if [ -z "$avg_time" ]; then
            avg_time=$(echo "$avg_line" | sed -n 's/.*平均时间: \(\.[0-9]*\)秒.*/\1/p')
            # 为以点开头的数字添加前导0
            if [ -n "$avg_time" ]; then
                avg_time="0$avg_time"
            fi
        fi
        if [ -z "$avg_time" ]; then
            avg_time=$(echo "$avg_line" | sed -n 's/.*平均时间: \([0-9]*\)秒.*/\1/p')
        fi
        
        if [ -n "$avg_time" ]; then
            printf "%-10s: %s秒\n" "$mode" "$avg_time"
            echo "$mode: ${avg_time}秒" >> "$output_file"
        else
            printf "%-10s: 解析失败\n" "$mode"
            echo "$mode: 解析失败 - 原始行: $avg_line" >> "$output_file"
        fi
    else
        printf "%-10s: 测试失败\n" "$mode"
        echo "$mode: 测试失败" >> "$output_file"
    fi
done

echo ""

# 生成综合热力图（所有模式汇总）
echo "🎨 生成综合性能热力图..."
if command -v python3 >/dev/null 2>&1; then
    # 检查是否有必要的Python库
    python3 -c "import pandas, matplotlib, seaborn" 2>/dev/null
    if [ $? -eq 0 ]; then
        if [ -f "generate_heatmap.py" ]; then
            python3 generate_heatmap.py --data "results/*/run_*/monitor.csv" --output "results/performance_charts"
            echo "📊 综合热力图已生成，请查看 results/performance_charts/ 目录"
        else
            echo "⚠️  未找到 generate_heatmap.py 脚本"
        fi
    else
        echo "⚠️  缺少必要的Python库。请安装："
        echo "   pip3 install pandas matplotlib seaborn"
    fi
else
    echo "⚠️  未找到Python3，跳过热力图生成"
fi

echo ""
echo "📁 数据文件结构："
echo "results/"
for mode in "${modes[@]}"; do
    if [ -d "results/${mode}" ]; then
        echo "├── ${mode}/"
        run_dirs=($(ls -1 "results/${mode}" | head -3))
        for run_dir in "${run_dirs[@]}"; do
            echo "│   ├── ${run_dir}/"
            if [ -d "results/${mode}/${run_dir}" ]; then
                ls -1 "results/${mode}/${run_dir}" | sed 's/^/│   │   ├── /'
            fi
        done
        run_count=$(ls -1 "results/${mode}" | wc -l)
        if [ $run_count -gt 3 ]; then
            echo "│   └── ... (共 $run_count 次运行)"
        fi
    fi
done
echo "├── performance_charts/ (综合分析)"

echo ""
echo "🎉 性能测试完成！" 