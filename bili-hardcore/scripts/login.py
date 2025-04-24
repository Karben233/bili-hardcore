import os
import json
import time
from time import sleep
from tools.bili_ticket import getTicket
from client.login import qrcode_get, qrcode_poll
from qrcode.main import QRCode
from qrcode.constants import ERROR_CORRECT_L
import tools.request_b
from config import config
from tools.logger import logger

def load_auth_data():
    """从缓存加载认证信息
    
    Returns:
        bool: 是否成功加载认证信息
    """
    if os.path.exists(config.AUTH_FILE):
        try:
            # 检查文件最后修改时间是否超过7天
            file_mtime = os.path.getmtime(config.AUTH_FILE)
            current_time = time.time()
            if (current_time - file_mtime) > 7 * 24 * 3600:  # 7天的秒数
                logger.info('认证信息已过期（超过7天），需要重新登录')
                return False
                
            with open(config.AUTH_FILE, 'r') as f:
                auth_data = json.load(f)
                if all(key in auth_data for key in ['access_token', 'csrf', 'mid', 'cookie']):
                    config.access_token = auth_data['access_token']
                    config.csrf = auth_data['csrf']
                    tools.request_b.headers.update({
                        'x-bili-mid': auth_data['mid'],
                        'cookie': auth_data['cookie']
                    })
                    logger.info('已从缓存加载登录信息')
                    return True
        except Exception as e:
            logger.error(f'读取认证信息失败: {str(e)}')
    return False

def save_auth_data(auth_data):
    """保存认证信息到缓存
    
    Args:
        auth_data (dict): 认证信息
    """
    try:
        os.makedirs(os.path.dirname(config.AUTH_FILE), exist_ok=True)
        with open(config.AUTH_FILE, 'w') as f:
            json.dump(auth_data, f, indent=4)
        logger.info('认证信息已保存到缓存')
    except Exception as e:
        logger.error(f'保存认证信息失败: {str(e)}')

def auth():
    """用户认证流程
    
    Returns:
        bool: 认证是否成功
    """
    if load_auth_data():
        return True

    try:
        # 初始化认证信息
        tools.request_b.headers.update({'x-bili-ticket': getTicket()})
        qrcode_data = qrcode_get()
        url = qrcode_data.get('url')
        
        
        # 创建QRCode实例
        qr = QRCode(
            version=1,
            error_correction=ERROR_CORRECT_L,
            box_size=2,
            border=1
        )

        # 添加数据
        qr.add_data(url)
        qr.make(fit=True)

        # 打印二维码
        qr.print_ascii()
        logger.info('请使用哔哩哔哩APP扫描二维码登录')
        logger.info(f"如果二维码不能正常显示，请使用 https://cli.im/ 手动生成此链接的二维码进行扫码：{url}")
        # 轮询二维码状态
        auth_code = qrcode_data.get('auth_code')
        retry_count = 0
        max_retries = 60  # 最大重试次数

        while retry_count < max_retries:
            try:
                poll_data = qrcode_poll(auth_code)
                if poll_data.get('code') == 0:
                    data = poll_data.get('data')
                    auth_data = {
                        'access_token': data.get('access_token'),
                        'mid': str(data.get('mid')),
                    }

                    cookies = data.get('cookie_info').get('cookies');
                    for cookie in cookies:
                        if cookie.get('name') == 'bili_jct':
                            auth_data.update({'csrf': cookie.get('value')})
                            break
                    cookie_str = ';'.join([f"{cookie.get('name')}={cookie.get('value')}" for cookie in cookies])
                    auth_data.update({'cookie': cookie_str})
                    # 更新认证信息
                    config.access_token = auth_data['access_token']
                    config.csrf = auth_data['csrf']
                    tools.request_b.headers.update({
                        'x-bili-mid': auth_data['mid'],
                        'cookie': auth_data['cookie']
                    })

                    # 保存认证信息
                    save_auth_data(auth_data)
                    logger.info('登录成功')
                    return True

            except Exception as e:
                logger.error(f'轮询二维码状态失败: {str(e)}')

            sleep(1)
            retry_count += 1

        logger.error('二维码登录超时')
        return False

    except Exception as e:
        logger.error(f'认证过程发生错误: {str(e)}')
        return False
