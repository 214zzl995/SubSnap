#!/usr/bin/env python3
"""
生成CPU和内存使用热力图的Python脚本
"""

import pandas as pd
import matplotlib.pyplot as plt
import seaborn as sns
import numpy as np
import argparse
import os
from datetime import datetime
import glob

def load_monitoring_data(file_pattern):
    """加载监控数据文件"""
    files = glob.glob(file_pattern)
    if not files:
        print(f"❌ 未找到匹配的数据文件: {file_pattern}")
        return None
    
    all_data = []
    for file in files:
        try:
            df = pd.read_csv(file)
            # 确保数据类型正确
            numeric_columns = ['timestamp_ms', 'cpu_user', 'cpu_sys', 'cpu_idle', 'memory_mb', 
                             'memory_free_mb', 'process_cpu', 'process_memory', 'load_avg_1m', 
                             'disk_io_read', 'disk_io_write']
            
            for col in numeric_columns:
                if col in df.columns:
                    df[col] = pd.to_numeric(df[col], errors='coerce').fillna(0)
            
            # 提取模式名称（从文件路径）
            path_parts = file.split('/')
            if len(path_parts) >= 2:
                mode_name = path_parts[-3] if 'run_' in path_parts[-2] else path_parts[-2]
            else:
                mode_name = os.path.basename(file).replace('_monitor.csv', '').replace('monitor_', '').replace('.csv', '')
            df['mode'] = mode_name
            all_data.append(df)
            print(f"✅ 加载数据文件: {file} ({len(df)} 条记录)")
        except Exception as e:
            print(f"⚠️  加载文件 {file} 失败: {e}")
    
    if not all_data:
        return None
    
    return pd.concat(all_data, ignore_index=True)

def create_heatmap(df, output_dir="performance_charts"):
    """创建CPU和内存使用热力图"""
    if not os.path.exists(output_dir):
        os.makedirs(output_dir)
    
    # 设置中文字体
    plt.rcParams['font.sans-serif'] = ['Arial Unicode MS', 'DejaVu Sans']
    plt.rcParams['axes.unicode_minus'] = False
    
    # 1. 系统资源热力图
    plt.figure(figsize=(15, 10))
    
    # 准备数据 - 按模式和时间创建矩阵
    modes = df['mode'].unique()
    
    # 创建时间序列数据
    heatmap_data = []
    mode_labels = []
    
    for mode in modes:
        mode_data = df[df['mode'] == mode].copy()
        if len(mode_data) == 0:
            continue
            
        # 标准化时间戳（相对于开始时间，转换为秒）
        mode_data['timestamp_sec'] = mode_data['timestamp_ms'] / 1000.0
        mode_data['relative_time'] = mode_data['timestamp_sec'] - mode_data['timestamp_sec'].min()
        
        # 创建固定长度的时间序列（取样）
        max_time = mode_data['relative_time'].max()
        time_points = np.linspace(0, max_time, 100)  # 100个时间点，提升精度
        
        # 计算综合CPU使用率
        mode_data['cpu_total'] = mode_data['cpu_user'] + mode_data['cpu_sys']
        cpu_interp = np.interp(time_points, mode_data['relative_time'], mode_data['cpu_total'])
        memory_interp = np.interp(time_points, mode_data['relative_time'], mode_data['memory_mb'])
        
        # 合并CPU和内存数据
        combined_data = np.column_stack([cpu_interp, memory_interp / 100])  # 内存数据缩放
        heatmap_data.append(combined_data)
        mode_labels.append(mode)
    
    if heatmap_data:
        # 创建子图
        fig, ((ax1, ax2), (ax3, ax4)) = plt.subplots(2, 2, figsize=(20, 12))
        
        # 1. CPU使用率热力图
        cpu_data = np.array([data[:, 0] for data in heatmap_data])
        sns.heatmap(cpu_data, 
                   yticklabels=mode_labels,
                   xticklabels=[f"{i:.0f}s" for i in range(0, 100, 20)],
                   cmap='YlOrRd',
                   cbar_kws={'label': 'CPU使用率 (%)'},
                   ax=ax1)
        ax1.set_title('CPU使用率热力图', fontsize=14, fontweight='bold')
        ax1.set_xlabel('时间')
        ax1.set_ylabel('转换模式')
        
        # 2. 内存使用热力图
        memory_data = np.array([data[:, 1] for data in heatmap_data]) * 100  # 恢复缩放
        sns.heatmap(memory_data,
                   yticklabels=mode_labels,
                   xticklabels=[f"{i:.0f}s" for i in range(0, 100, 20)],
                   cmap='Blues',
                   cbar_kws={'label': '内存使用 (MB)'},
                   ax=ax2)
        ax2.set_title('内存使用热力图', fontsize=14, fontweight='bold')
        ax2.set_xlabel('时间')
        ax2.set_ylabel('转换模式')
        
        # 3. 平均资源使用对比（使用新的字段）
        df['cpu_total'] = df['cpu_user'] + df['cpu_sys']
        avg_stats = df.groupby('mode')[['cpu_total', 'memory_mb']].mean()
        avg_stats.plot(kind='bar', ax=ax3)
        ax3.set_title('平均资源使用对比', fontsize=14, fontweight='bold')
        ax3.set_xlabel('转换模式')
        ax3.set_ylabel('使用量')
        ax3.legend(['CPU (%)', '内存 (MB)'])
        ax3.tick_params(axis='x', rotation=45)
        
        # 4. 资源使用分布箱线图（使用新的字段）
        df_melted = pd.melt(df, id_vars=['mode'], value_vars=['cpu_total', 'memory_mb'], 
                           var_name='resource_type', value_name='usage')
        
        # 分别绘制CPU和内存
        cpu_df = df_melted[df_melted['resource_type'] == 'cpu_total']
        memory_df = df_melted[df_melted['resource_type'] == 'memory_mb']
        
        # 在同一个子图中绘制两个箱线图
        positions1 = np.arange(len(modes))
        positions2 = positions1 + 0.4
        
        cpu_data_by_mode = [df[df['mode'] == mode]['cpu_total'].values for mode in modes]
        memory_data_by_mode = [df[df['mode'] == mode]['memory_mb'].values for mode in modes]
        
        bp1 = ax4.boxplot(cpu_data_by_mode, positions=positions1, widths=0.3, 
                         patch_artist=True, boxprops=dict(facecolor='lightcoral'))
        bp2 = ax4.boxplot(memory_data_by_mode, positions=positions2, widths=0.3,
                         patch_artist=True, boxprops=dict(facecolor='lightblue'))
        
        ax4.set_title('资源使用分布', fontsize=14, fontweight='bold')
        ax4.set_xlabel('转换模式')
        ax4.set_ylabel('使用量')
        ax4.set_xticks(positions1 + 0.2)
        ax4.set_xticklabels(modes)
        ax4.legend([bp1["boxes"][0], bp2["boxes"][0]], ['CPU (%)', '内存 (MB)'])
        
        plt.tight_layout()
        heatmap_file = f"{output_dir}/resource_heatmap.png"
        plt.savefig(heatmap_file, dpi=300, bbox_inches='tight')
        print(f"🎨 热力图已保存: {heatmap_file}")
        
        # 5. 详细的时间序列图
        plt.figure(figsize=(15, 8))
        
        plt.subplot(2, 1, 1)
        for mode in modes:
            mode_data = df[df['mode'] == mode]
            if len(mode_data) > 0:
                mode_data['timestamp_sec'] = mode_data['timestamp_ms'] / 1000.0
                time_normalized = (mode_data['timestamp_sec'] - mode_data['timestamp_sec'].min())
                mode_data['cpu_total'] = mode_data['cpu_user'] + mode_data['cpu_sys']
                plt.plot(time_normalized, mode_data['cpu_total'], 
                        label=f'{mode}', linewidth=2, marker='o', markersize=2)
        
        plt.title('CPU使用率时间序列', fontsize=14, fontweight='bold')
        plt.xlabel('时间 (秒)')
        plt.ylabel('CPU使用率 (%)')
        plt.legend()
        plt.grid(True, alpha=0.3)
        
        plt.subplot(2, 1, 2)
        for mode in modes:
            mode_data = df[df['mode'] == mode]
            if len(mode_data) > 0:
                mode_data['timestamp_sec'] = mode_data['timestamp_ms'] / 1000.0
                time_normalized = (mode_data['timestamp_sec'] - mode_data['timestamp_sec'].min())
                plt.plot(time_normalized, mode_data['memory_mb'], 
                        label=f'{mode}', linewidth=2, marker='s', markersize=2)
        
        plt.title('内存使用时间序列', fontsize=14, fontweight='bold')
        plt.xlabel('时间 (秒)')
        plt.ylabel('内存使用 (MB)')
        plt.legend()
        plt.grid(True, alpha=0.3)
        
        plt.tight_layout()
        timeseries_file = f"{output_dir}/resource_timeseries.png"
        plt.savefig(timeseries_file, dpi=300, bbox_inches='tight')
        print(f"📈 时间序列图已保存: {timeseries_file}")
        
        # 生成汇总报告
        generate_summary_report(df, output_dir)
        
    else:
        print("❌ 没有有效的数据用于生成热力图")

def generate_summary_report(df, output_dir):
    """生成汇总报告"""
    report_file = f"{output_dir}/performance_report.txt"
    
    with open(report_file, 'w', encoding='utf-8') as f:
        f.write("📊 性能测试资源使用报告\n")
        f.write("=" * 50 + "\n\n")
        f.write(f"生成时间: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n\n")
        
        # 按模式统计
        f.write("📈 各模式平均资源使用:\n")
        f.write("-" * 30 + "\n")
        for mode in df['mode'].unique():
            mode_data = df[df['mode'] == mode]
            mode_data['cpu_total'] = mode_data['cpu_user'] + mode_data['cpu_sys']
            avg_cpu = mode_data['cpu_total'].mean()
            avg_memory = mode_data['memory_mb'].mean()
            max_cpu = mode_data['cpu_total'].max()
            max_memory = mode_data['memory_mb'].max()
            avg_load = mode_data['load_avg_1m'].mean()
            
            f.write(f"\n🔧 {mode.upper()} 模式:\n")
            f.write(f"  • 平均CPU使用率: {avg_cpu:.2f}%\n")
            f.write(f"  • 峰值CPU使用率: {max_cpu:.2f}%\n")
            f.write(f"  • 平均内存使用: {avg_memory:.2f} MB\n")
            f.write(f"  • 峰值内存使用: {max_memory:.2f} MB\n")
            f.write(f"  • 平均系统负载: {avg_load:.2f}\n")
        
        # 性能排名
        f.write(f"\n🏆 性能排名:\n")
        f.write("-" * 20 + "\n")
        df['cpu_total'] = df['cpu_user'] + df['cpu_sys']
        avg_by_mode = df.groupby('mode')[['cpu_total', 'memory_mb']].mean()
        
        cpu_ranking = avg_by_mode.sort_values('cpu_total')
        f.write("\n💻 CPU使用率排名（从低到高）:\n")
        for i, (mode, data) in enumerate(cpu_ranking.iterrows(), 1):
            f.write(f"  {i}. {mode}: {data['cpu_total']:.2f}%\n")
        
        memory_ranking = avg_by_mode.sort_values('memory_mb')
        f.write("\n🧠 内存使用排名（从低到高）:\n")
        for i, (mode, data) in enumerate(memory_ranking.iterrows(), 1):
            f.write(f"  {i}. {mode}: {data['memory_mb']:.2f} MB\n")
    
    print(f"📄 性能报告已保存: {report_file}")

def main():
    parser = argparse.ArgumentParser(description='生成性能测试资源使用热力图')
    parser.add_argument('--data', '-d', default='*_monitor.csv', 
                       help='监控数据文件模式 (默认: *_monitor.csv)')
    parser.add_argument('--output', '-o', default='performance_charts',
                       help='输出目录 (默认: performance_charts)')
    
    args = parser.parse_args()
    
    print("🎨 开始生成性能测试热力图...")
    
    # 加载数据
    df = load_monitoring_data(args.data)
    if df is None or len(df) == 0:
        print("❌ 无法加载数据或数据为空")
        return
    
    print(f"📊 加载了 {len(df)} 条监控记录，涵盖 {len(df['mode'].unique())} 个模式")
    
    # 生成图表
    create_heatmap(df, args.output)
    
    print("🎉 热力图生成完成！")

if __name__ == "__main__":
    main() 