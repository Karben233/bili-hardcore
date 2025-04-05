# -*- mode: python ; coding: utf-8 -*-

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
    name='bili-hardcore',
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