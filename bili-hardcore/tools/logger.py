import logging
import os
from datetime import datetime

logger_level = logging.INFO
# logger_level = logging.DEBUG

def setup_logger(name='bili-hardcore'):
    """设置日志系统
    
    Args:
        name (str): 日志器名称
    
    Returns:
        logging.Logger: 配置好的日志器实例
    """
    logger = logging.getLogger(name)
    logger.setLevel(logger_level)

    # 创建日志目录
    log_dir = os.path.join(os.path.dirname(os.path.dirname(__file__)), 'logs')
    os.makedirs(log_dir, exist_ok=True)

    # 文件处理器
    log_file = os.path.join(log_dir, f'{datetime.now().strftime("%Y-%m-%d_%H-%M-%S")}.log')
    file_handler = logging.FileHandler(log_file, encoding='utf-8')
    file_handler.setLevel(logger_level)

    # 控制台处理器
    console_handler = logging.StreamHandler()
    console_handler.setLevel(logger_level)

    # 设置日志格式
    formatter = logging.Formatter('%(asctime)s - %(name)s - %(levelname)s - %(message)s')
    file_handler.setFormatter(formatter)
    console_handler.setFormatter(formatter)

    # 添加处理器
    logger.addHandler(file_handler)
    logger.addHandler(console_handler)

    return logger

# 创建全局日志器实例
logger = setup_logger()