# -*- mode: python ; coding: utf-8 -*-
import os
import base64
import zlib

block_cipher = None

a = Analysis(
    ['bili-hardcore/main.py'],
    binaries=[],
    datas=[
        ('bili-hardcore/scripts', 'scripts'),  # 添加 scripts 目录
        ('bili-hardcore/config', 'config'),    # 添加 config 目录
        ('bili-hardcore/tools', 'tools'),    # 添加 tools 目录
        ('bili-hardcore/client', 'client'),  # 添加 client 目录
    ],
    hiddenimports=[
        'scripts.login',
        'scripts.check',
        'scripts.start_senior',
        'tools',
        'tools.bili_ticket',
        'tools.logger',
        'tools.request_b',
        'hmac',
        'requests',
        'requests.packages.urllib3',
        'urllib3',
        'certifi',
        'charset_normalizer',
        'idna',
        'client',
        'client.login',
        'client.senior',
        'qrcode',
        'qrcode.main',
        'config.config'
    ],
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=[],
    win_no_prefer_redirects=False,
    win_private_assemblies=False,
    cipher=block_cipher,
    noarchive=False,
)
pyz = PYZ(a.pure, a.zipped_data, cipher=block_cipher)

exe = EXE(
    pyz,
    a.scripts,
    a.binaries,
    a.zipfiles,
    a.datas,
    [],
    name='bili-hardcore-bin',
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=True,
    upx_exclude=[],
    runtime_tmpdir=None,
    console=True,
    disable_windowed_traceback=False,
    argv_emulation=False,
    target_arch=None,
    codesign_identity=None,
    entitlements_file=None,
)

# 创建一个自解压的shell脚本
print("生成自解压shell脚本...")

# 读取二进制可执行文件
with open('dist/bili-hardcore-bin', 'rb') as f:
    binary_data = f.read()

# 压缩并编码
compressed_data = zlib.compress(binary_data)
encoded_data = base64.b64encode(compressed_data).decode('ascii')

# 创建shell脚本头部
header = '''#!/bin/bash
# bili-hardcore 自解压可执行脚本
# 该脚本包含一个编码的二进制可执行文件，会在运行时自动解压

# 创建临时文件
TEMP_FILE=$(mktemp /tmp/bili-hardcore.XXXXXX)
trap 'rm -f "$TEMP_FILE"' EXIT

# 解码并解压二进制数据
echo "正在准备bili-hardcore..."
'''

# 创建shell脚本的解码部分
decoder = f'''
# 从这里开始是编码的二进制数据
BINARY_DATA="{encoded_data}"
echo "$BINARY_DATA" | base64 -d | python3 -c "import sys, zlib; sys.stdout.buffer.write(zlib.decompress(sys.stdin.buffer.read()))" > "$TEMP_FILE"
chmod +x "$TEMP_FILE"

# 执行解压后的二进制文件
"$TEMP_FILE" "$@"
'''

# 写入自解压shell脚本
with open('dist/bili-hardcore.sh', 'w') as f:
    f.write(header + decoder)

# 设置执行权限
os.chmod('dist/bili-hardcore.sh', 0o755)

print("自解压shell脚本已生成：dist/bili-hardcore.sh") 