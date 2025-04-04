import hashlib
import time
import urllib.parse
import requests
from requests.adapters import HTTPAdapter
from urllib3.util.retry import Retry
from config.config import API_CONFIG, HEADERS
from tools.logger import logger

# 创建Session对象并配置重试策略
session = requests.Session()
retry_strategy = Retry(
    total=3,
    backoff_factor=1,
    status_forcelist=[500, 502, 503, 504]
)
session.mount('http://', HTTPAdapter(max_retries=retry_strategy))
session.mount('https://', HTTPAdapter(max_retries=retry_strategy))

# 使用配置文件中的值
appkey = API_CONFIG['appkey']
appsec = API_CONFIG['appsec']
headers = HEADERS.copy()

def appsign(params):
    """为请求参数进行 APP 签名
    
    Args:
        params (dict): 请求参数
    
    Returns:
        dict: 添加签名后的参数
    """
    try:
        params.update({'ts': str(int(time.time()))})
        params.update({'appkey': appkey})
        params = dict(sorted(params.items()))
        query = urllib.parse.urlencode(params)
        sign = hashlib.md5((query+appsec).encode()).hexdigest()
        params.update({'sign':sign})
        return params
    except Exception as e:
        logger.error(f'生成签名失败: {str(e)}')
        raise

def get(url, params):
    """发送GET请求
    
    Args:
        url (str): 请求URL
        params (dict): 请求参数
    
    Returns:
        dict: 响应数据
    """
    try:
        signed_params = appsign(params)
        logger.debug(f'发送GET请求: {url}, 参数: {signed_params}')
        response = session.get(url, params=signed_params, headers=headers)
        response.raise_for_status()
        data = response.json()
        logger.debug(f'请求成功: {data}')
        return data
    except requests.exceptions.HTTPError as e:
        logger.error(f'HTTP错误: {e}\n响应内容: {e.response.text}')
        raise
    except requests.exceptions.RequestException as e:
        logger.error(f'请求失败: {e}')
        raise
    except ValueError as e:
        logger.error(f'解析响应JSON失败: {e}')
        raise

def post(url, params):
    """发送POST请求
    
    Args:
        url (str): 请求URL
        params (dict): 请求参数
    
    Returns:
        dict: 响应数据
    """
    try:
        signed_params = appsign(params)
        logger.debug(f'发送POST请求: {url}, 参数: {signed_params}')
        response = session.post(url, data=signed_params, headers=headers)
        response.raise_for_status()
        data = response.json()
        logger.debug(f'请求成功: {data}')
        return data
    except requests.exceptions.HTTPError as e:
        logger.error(f'HTTP错误: {e}\n响应内容: {e.response.text}')
        raise
    except requests.exceptions.RequestException as e:
        logger.error(f'请求失败: {e}')
        raise
    except ValueError as e:
        logger.error(f'解析响应JSON失败: {e}')
        raise