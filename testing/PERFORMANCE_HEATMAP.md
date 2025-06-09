# 📊 性能测试热力图功能

这个功能扩展了原有的性能测试，增加了CPU和内存使用的可视化热力图生成。

## 🚀 快速开始

### 1. 进入测试目录

```bash
cd testing
```

### 2. 安装Python依赖

```bash
./setup_heatmap.sh
```

这会安装所需的Python库：pandas, matplotlib, seaborn, numpy

### 3. 运行演示

```bash
./demo_heatmap.sh
```

快速演示功能，生成演示热力图。

### 4. 完整性能测试

```bash
./benchmark_test.sh
```

运行完整的性能测试，包括热力图生成。

## 📁 文件说明

| 文件 | 说明 |
|------|------|
| `monitor_system.sh` | 系统资源监控脚本 |
| `generate_heatmap.py` | 热力图生成Python脚本 |
| `setup_heatmap.sh` | 安装Python依赖 |
| `demo_heatmap.sh` | 快速演示脚本 |
| `benchmark_test.sh` | 主要性能测试脚本 |

**注意：所有测试文件现在都在 `testing/` 目录中**

## 🎨 生成的图表

运行测试后，会在 `testing/results/performance_charts/` 目录生成以下文件：

### 📊 热力图
- `resource_heatmap.png` - 包含4个子图的综合热力图：
  - CPU使用率热力图
  - 内存使用热力图  
  - 平均资源使用对比柱状图
  - 资源使用分布箱线图

### 📈 时间序列图
- `resource_timeseries.png` - CPU和内存使用的时间序列图

### 📄 报告
- `performance_report.txt` - 详细的性能分析报告

## 🔧 工作原理

1. **监控阶段**：
   - 每次运行测试时，后台启动 `monitor_system.sh`
   - 以0.5秒间隔采集系统CPU和内存数据
   - 同时监控特定进程（sub-snap）的资源使用
   - 数据保存为CSV格式（如 `results/ffmpeg_monitor.csv`）

2. **分析阶段**：
   - `generate_heatmap.py` 读取所有监控数据
   - 生成多种可视化图表
   - 计算统计数据和性能排名

3. **输出阶段**：
   - 生成高分辨率图片（300 DPI）
   - 创建详细的文本报告
   - 提供性能对比和排名

## 📊 数据格式

监控数据CSV文件格式：
```csv
timestamp,cpu_percent,memory_mb,process_cpu,process_memory
1703123456,25.3,2048,12.5,45.2
```

- `timestamp`: Unix时间戳
- `cpu_percent`: 系统整体CPU使用率(%)
- `memory_mb`: 系统内存使用量(MB)
- `process_cpu`: 目标进程CPU使用率(%)
- `process_memory`: 目标进程内存使用率(%)

## 🎯 自定义使用

### 单独监控

```bash
./monitor_system.sh output.csv 30 "进程名称"
```

- `output.csv`: 输出文件
- `30`: 监控时长（秒）
- `"进程名称"`: 要监控的进程名称模式（可选）

### 单独生成热力图

```bash
python3 generate_heatmap.py --data "results/*_monitor.csv" --output results/charts_dir
```

- `--data`: 数据文件模式（现在在results目录中）
- `--output`: 输出目录

## 🔍 故障排除

### Python库缺失
```bash
pip3 install pandas matplotlib seaborn numpy
```

### 监控脚本权限
```bash
chmod +x monitor_system.sh generate_heatmap.py
```

### macOS特定问题
- 确保有权限访问系统监控信息
- 可能需要在系统偏好设置中授权终端访问

## 💡 使用建议

1. **测试环境**：在相对稳定的系统环境下运行测试
2. **资源监控**：关闭不必要的应用以获得更准确的结果
3. **多次运行**：建议运行多次取平均值
4. **数据保存**：重要的测试结果建议备份CSV数据文件

## 🎉 示例输出

运行完成后，你会看到类似这样的热力图：

- **CPU热力图**：显示各转换模式在测试期间的CPU使用情况
- **内存热力图**：显示各转换模式的内存使用模式
- **性能对比**：直观比较不同模式的资源消耗
- **分布分析**：了解资源使用的稳定性和波动情况

这些可视化数据可以帮助你：
- 🔍 识别性能瓶颈
- ⚡ 优化资源使用
- 📊 对比不同转换方法
- 🎯 选择最适合的转换模式 