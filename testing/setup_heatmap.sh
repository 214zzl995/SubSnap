#!/bin/bash

# 安装热力图生成所需的Python依赖

echo "🐍 设置热力图生成环境..."

# 检查Python3
if ! command -v python3 >/dev/null 2>&1; then
    echo "❌ 未找到Python3，请先安装Python3"
    echo "   可以通过 brew install python3 安装"
    exit 1
fi

echo "✅ 找到Python3: $(python3 --version)"

# 检查pip3
if ! command -v pip3 >/dev/null 2>&1; then
    echo "❌ 未找到pip3"
    exit 1
fi

echo "✅ 找到pip3"

# 检查Python环境管理方式并安装必要的库
echo "📦 安装Python依赖..."

libraries=("pandas" "matplotlib" "seaborn" "numpy")

# 首先尝试用--user方式安装
echo "尝试用户级安装..."
failed=false

for lib in "${libraries[@]}"; do
    echo -n "  安装 $lib... "
    if pip3 install --user "$lib" >/dev/null 2>&1; then
        echo "✅"
    else
        # 如果用户级安装失败，尝试用--break-system-packages
        if pip3 install --break-system-packages "$lib" >/dev/null 2>&1; then
            echo "✅ (系统级)"
        else
            echo "❌ 安装失败"
            failed=true
        fi
    fi
done

if [ "$failed" = true ]; then
    echo ""
    echo "⚠️  部分库安装失败。可能的解决方案："
    echo ""
    echo "方案1 - 用户级安装："
    echo "   pip3 install --user pandas matplotlib seaborn numpy"
    echo ""
    echo "方案2 - 使用虚拟环境："
    echo "   python3 -m venv venv"
    echo "   source venv/bin/activate"
    echo "   pip install pandas matplotlib seaborn numpy"
    echo ""
    echo "方案3 - 使用Homebrew："
    echo "   brew install python@3.13"
    echo "   # 然后重新运行此脚本"
else
    echo ""
    echo "🎉 所有依赖安装完成！"
fi

# 测试导入
echo "🧪 测试库导入..."
python3 -c "
import pandas as pd
import matplotlib.pyplot as plt
import seaborn as sns
import numpy as np
print('✅ 所有库导入成功！')
" 2>/dev/null

if [ $? -eq 0 ]; then
    echo "🎨 热力图环境已就绪！"
    echo ""
    echo "💡 使用方法："
    echo "  1. 运行: ./benchmark_test.sh"
    echo "  2. 热力图将自动生成在 results/performance_charts/ 目录"
else
    echo "❌ 库导入测试失败"
fi 