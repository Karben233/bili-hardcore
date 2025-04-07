#!/bin/bash

echo "开始bili-hardcore项目的Shell打包..."

# 确保Python环境已安装
if ! command -v python3 &> /dev/null; then
    echo "错误：未找到Python3。请安装Python3后再试。"
    exit 1
fi

# 创建虚拟环境
echo "创建虚拟环境..."
python3 -m venv venv
source venv/bin/activate

# 安装依赖
echo "安装依赖..."
python -m pip install -r requirements.txt
python -m pip install pyinstaller

# 使用PyInstaller直接打包
echo "开始打包应用..."
pyinstaller --clean build_unix.spec

# 创建shell脚本
echo "创建shell启动脚本..."
cat > dist/bili-hardcore.sh << 'EOF'
#!/bin/bash
# bili-hardcore 启动脚本
SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" &> /dev/null && pwd)
"$SCRIPT_DIR/bili-hardcore_bin" "$@"
EOF

# 设置执行权限
chmod +x dist/bili-hardcore.sh

# 退出虚拟环境
deactivate

echo "构建完成！"
echo "您可以通过以下命令运行应用："
echo "  ./dist/bili-hardcore.sh" 