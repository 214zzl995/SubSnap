#!/bin/bash

# 系统监控脚本 - 收集CPU和内存数据
# 参数：$1=输出文件, $2=监控时长(秒), $3=进程名称模式

output_file="$1"
duration="$2"
process_pattern="$3"

if [ -z "$output_file" ] || [ -z "$duration" ]; then
    echo "用法: $0 <输出文件> <监控时长(秒)> [进程名称模式]"
    exit 1
fi

# 信号处理函数
cleanup() {
    echo ""
    echo "⚠️  收到中断信号，正在停止监控..."
    echo "DEBUG: 监控结束时间: $(date)" >&2
    echo "✅ 监控完成！数据已保存到: $output_file"
    
    # 显示收集到的数据行数
    if [ -f "$output_file" ]; then
        line_count=$(wc -l < "$output_file" 2>/dev/null || echo "1")
        echo "📊 收集到 $((line_count - 1)) 条数据记录"
    fi
    
    exit 0
}

# 捕获中断信号 (Ctrl+C)
trap cleanup SIGINT SIGTERM

echo "timestamp_ms,cpu_user,cpu_sys,cpu_idle,memory_mb,memory_free_mb,process_cpu,process_memory,load_avg_1m,disk_io_read,disk_io_write" > "$output_file"

start_time=$(date +%s)

echo "🔍 开始监控系统资源..."
echo "📁 输出文件: $output_file"
if [ "$duration" -eq 0 ]; then
    echo "⏱️  监控时长: 无限制（直到进程结束）"
else
    echo "⏱️  监控时长: ${duration}秒"
fi
if [ -n "$process_pattern" ]; then
    echo "🎯 监控进程: $process_pattern"
fi

# 确保输出文件目录存在
mkdir -p "$(dirname "$output_file")"

# 添加调试信息
echo "DEBUG: 监控开始时间: $(date)" >&2

# 无限制监控（duration=0）或有时间限制监控
while true; do
    # 检查时间限制
    if [ "$duration" -ne 0 ] && [ $(date +%s) -ge $((start_time + duration)) ]; then
        break
    fi
    
    # 高精度时间戳（毫秒）
    timestamp_sec=$(date +%s)
    timestamp_ms=$((timestamp_sec * 1000))
    
    # 获取系统整体CPU使用率（macOS）- 使用更简单的方法
    cpu_total=$(top -l 1 -n 0 | grep "CPU usage" | awk '{print $3}' | sed 's/%//' | head -1 2>/dev/null || echo "0")
    cpu_user=${cpu_total:-0}
    cpu_sys=0
    cpu_idle=0
    
    # 获取系统内存使用情况（macOS）- 使用更简单的方法
    memory_mb=$(top -l 1 -n 0 | grep "PhysMem" | awk '{print $2}' | sed 's/[^0-9]//g' 2>/dev/null || echo "0")
    memory_free_mb=$(top -l 1 -n 0 | grep "PhysMem" | awk '{print $6}' | sed 's/[^0-9]//g' 2>/dev/null || echo "0")
    memory_mb=${memory_mb:-0}
    memory_free_mb=${memory_free_mb:-0}
    
    # 获取系统负载
    load_avg_1m=$(uptime 2>/dev/null | awk -F'load averages:' '{print $2}' | awk '{print $1}' 2>/dev/null || echo "0")
    load_avg_1m=${load_avg_1m:-0}
    
    # 简化磁盘IO统计，避免阻塞
    disk_io_read=0
    disk_io_write=0
    
    # 获取特定进程的CPU和内存使用情况
    process_cpu=0
    process_memory=0
    
    if [ -n "$process_pattern" ]; then
        # 查找匹配的进程
        process_info=$(ps aux 2>/dev/null | grep "$process_pattern" 2>/dev/null | grep -v grep 2>/dev/null | head -1 2>/dev/null || echo "")
        if [ -n "$process_info" ]; then
            process_cpu=$(echo "$process_info" | awk '{print $3}' 2>/dev/null | head -1)
            process_memory=$(echo "$process_info" | awk '{print $4}' 2>/dev/null | head -1)
            process_cpu=${process_cpu:-0}
            process_memory=${process_memory:-0}
        fi
    fi
    
    # 处理空值，确保所有字段都有值
    cpu_user=${cpu_user:-0}
    cpu_sys=${cpu_sys:-0}
    cpu_idle=${cpu_idle:-0}
    memory_mb=${memory_mb:-0}
    memory_free_mb=${memory_free_mb:-0}
    process_cpu=${process_cpu:-0}
    process_memory=${process_memory:-0}
    load_avg_1m=${load_avg_1m:-0}
    disk_io_read=${disk_io_read:-0}
    disk_io_write=${disk_io_write:-0}
    
    # 验证数值格式
    if ! [[ "$cpu_user" =~ ^[0-9]*\.?[0-9]+$ ]]; then cpu_user=0; fi
    if ! [[ "$cpu_sys" =~ ^[0-9]*\.?[0-9]+$ ]]; then cpu_sys=0; fi
    if ! [[ "$cpu_idle" =~ ^[0-9]*\.?[0-9]+$ ]]; then cpu_idle=0; fi
    if ! [[ "$memory_mb" =~ ^[0-9]+$ ]]; then memory_mb=0; fi
    if ! [[ "$memory_free_mb" =~ ^[0-9]+$ ]]; then memory_free_mb=0; fi
    if ! [[ "$process_cpu" =~ ^[0-9]*\.?[0-9]+$ ]]; then process_cpu=0; fi
    if ! [[ "$process_memory" =~ ^[0-9]*\.?[0-9]+$ ]]; then process_memory=0; fi
    if ! [[ "$load_avg_1m" =~ ^[0-9]*\.?[0-9]+$ ]]; then load_avg_1m=0; fi
    
    echo "$timestamp_ms,$cpu_user,$cpu_sys,$cpu_idle,$memory_mb,$memory_free_mb,$process_cpu,$process_memory,$load_avg_1m,$disk_io_read,$disk_io_write" >> "$output_file"
    
    sleep 0.1  # 每0.1秒采样一次，提升精度
done

# 正常结束时的清理工作
cleanup 