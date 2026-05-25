import os
from scripts.check_config import check
import json
from tools.logger import logger

BASE_URL_OPENAI = ''
MODEL_OPENAI = ''
API_KEY_OPENAI = ''

access_token = None
csrf = None

def save_openai_config(base_url, model, api_key):
    config_file = os.path.join(os.path.expanduser('~'), '.bili-hardcore', 'openai_config.json')
    try:
        os.makedirs(os.path.dirname(config_file), exist_ok=True)
        with open(config_file, 'w') as f:
            json.dump({
                'base_url': base_url,
                'model': model,
                'api_key': api_key
            }, f)
        print('配置已保存')
    except Exception as e:
        print(f'保存配置失败: {str(e)}')

def load_openai_config():
    config_file = os.path.join(os.path.expanduser('~'), '.bili-hardcore', 'openai_config.json')
    if os.path.exists(config_file):
        try:
            with open(config_file, 'r') as f:
                data = json.load(f)
                return data.get('base_url', ''), data.get('model', ''), data.get('api_key', '')
        except Exception as e:
            print(f'读取配置失败: {str(e)}')
    return '', '', ''

logger.info("哔哩哔哩硬核会员自动答题脚本")
logger.info("本软件免费且代码开源")
logger.info("源码&问题反馈: https://github.com/Karben233/bili-hardcore")

check()

BASE_URL_OPENAI, MODEL_OPENAI, API_KEY_OPENAI = load_openai_config()
if not all([BASE_URL_OPENAI, MODEL_OPENAI, API_KEY_OPENAI]):
    BASE_URL_OPENAI = input('请输入API基础URL (例如: https://ark.cn-beijing.volces.com/api/v3): ').strip()
    if BASE_URL_OPENAI.endswith('/'):
        BASE_URL_OPENAI = BASE_URL_OPENAI.rstrip('/')
    MODEL_OPENAI = input('请输入模型名称 (例如: deepseek-v3-250324): ').strip()
    API_KEY_OPENAI = input('请输入API密钥: ').strip()
    if all([BASE_URL_OPENAI, MODEL_OPENAI, API_KEY_OPENAI]):
        save_openai_config(BASE_URL_OPENAI, MODEL_OPENAI, API_KEY_OPENAI)

# 项目根目录
BASE_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# 日志目录
LOG_DIR = os.path.join(BASE_DIR, 'logs')
os.makedirs(LOG_DIR, exist_ok=True)

# API配置
API_CONFIG = {
    'appkey': '783bbb7264451d82',
    'appsec': '2653583c8873dea268ab9386918b1d65',
    'user_agent': 'Mozilla/5.0 BiliDroid/1.12.0 (bbcallen@gmail.com)',
}

# 请求头配置
HEADERS = {
    'User-Agent': API_CONFIG['user_agent'],
    'Content-Type': 'application/x-www-form-urlencoded',
    'Accept': 'application/json',
    'Accept-Language': 'zh-CN,zh;q=0.9,en;q=0.8',
    'x-bili-metadata-legal-region': 'CN',
    'x-bili-aurora-eid': '',
    'x-bili-aurora-zone': '',
}

# 认证文件路径
AUTH_FILE = os.path.join(os.path.expanduser('~'), '.bili-hardcore', 'auth.json')

# 此处的"当前时间"没有实际用途, 只是为了防止重复的prompt在短时间内大量请求, 触发风控 (类似于沉浸式翻译)
PROMPT = '''
当前时间：{}
你是一个高效精准的答题专家，面对选择题时，直接根据问题和选项判断正确答案，并返回对应选项的序号（1, 2, 3, 4）。示例：
问题：大的反义词是什么？
选项：['长', '宽', '小', '热']
回答：3
如果不确定正确答案，选择最接近的选项序号返回，不提供额外解释或超出 1-4 的内容。
---
不要思考，直接回答我的问题：{}
'''