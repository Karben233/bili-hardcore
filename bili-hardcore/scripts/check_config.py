import os
import shutil
from tools.logger import logger

def check():
    # 检查配置文件
    if check_config():
        logger.info('发现已保存的配置文件')
        choice = input('是否使用已保存的配置？[1]加载已保存配置 [2]重新开始: ')
        if choice == '2':
            clear_config()

def check_config():
    """检查配置文件"""
    config_dir = os.path.join(os.path.expanduser('~'), '.bili-hardcore')
    return os.path.exists(config_dir) and len(os.listdir(config_dir)) > 0

def clear_config():
    """清空配置文件"""
    config_dir = os.path.join(os.path.expanduser('~'), '.bili-hardcore')
    if os.path.exists(config_dir):
        shutil.rmtree(config_dir)
        logger.info('配置文件已清空')