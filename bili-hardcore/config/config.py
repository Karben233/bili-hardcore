import os
from scripts.check_config import check
# API Keys
import json
from tools.logger import logger

# OpenAI默认配置
BASE_URL_OPENAI = ''
MODEL_OPENAI = ''
API_KEY_OPENAI = ''

access_token = None;
csrf = None;

def load_api_key(key_type):
    """从用户目录加载API密钥
    
    Args:
        key_type (str): API类型 (gemini 或 deepseek)
    
    Returns:
        str: API密钥
    """
    key_file = os.path.join(os.path.expanduser('~'), '.bili-hardcore', f'{key_type}_key.json')
    if os.path.exists(key_file):
        try:
            with open(key_file, 'r') as f:
                data = json.load(f)
                return data.get('api_key', '')
        except Exception as e:
            print(f'读取{key_type.upper()} API密钥失败: {str(e)}')
    return ''

def save_api_key(key_type, api_key):
    """保存API密钥到用户目录
    
    Args:
        key_type (str): API类型 (gemini 或 deepseek)
        api_key (str): API密钥
    """
    key_file = os.path.join(os.path.expanduser('~'), '.bili-hardcore', f'{key_type}_key.json')
    try:
        os.makedirs(os.path.dirname(key_file), exist_ok=True)
        with open(key_file, 'w') as f:
            json.dump({'api_key': api_key}, f)
        print(f'{key_type.upper()} API密钥已保存')
    except Exception as e:
        print(f'保存{key_type.upper()} API密钥失败: {str(e)}')

# 从用户目录加载API密钥，如果不存在则提示用户输入
def load_gemini_key():
    """从用户目录加载GEMINI API密钥
    
    Returns:
        str: API密钥
    """
    key_file = os.path.join(os.path.expanduser('~'), '.bili-hardcore', 'gemini_key.json')
    if os.path.exists(key_file):
        try:
            with open(key_file, 'r') as f:
                data = json.load(f)
                return data.get('api_key', '')
        except Exception as e:
            print(f'读取GEMINI API密钥失败: {str(e)}')
    return ''

def save_gemini_key(api_key):
    save_api_key('gemini', api_key)

def save_openai_config(base_url, model, api_key):
    """保存OpenAI配置到用户目录
    
    Args:
        base_url (str): OpenAI API基础URL
        model (str): 模型名称
        api_key (str): API密钥
    """
    config_file = os.path.join(os.path.expanduser('~'), '.bili-hardcore', 'openai_config.json')
    try:
        os.makedirs(os.path.dirname(config_file), exist_ok=True)
        with open(config_file, 'w') as f:
            json.dump({
                'base_url': base_url,
                'model': model,
                'api_key': api_key
            }, f)
        print('OpenAI配置已保存')
    except Exception as e:
        print(f'保存OpenAI配置失败: {str(e)}')

def load_openai_config():
    """从用户目录加载OpenAI配置
    
    Returns:
        tuple: (base_url, model, api_key)
    """
    config_file = os.path.join(os.path.expanduser('~'), '.bili-hardcore', 'openai_config.json')
    if os.path.exists(config_file):
        try:
            with open(config_file, 'r') as f:
                data = json.load(f)
                return data.get('base_url', ''), data.get('model', ''), data.get('api_key', '')
        except Exception as e:
            print(f'读取OpenAI配置失败: {str(e)}')
    return '', '', ''

logger.info("哔哩哔哩硬核会员自动答题脚本")
logger.info("本软件免费且代码开源")
logger.info("源码&问题反馈: https://github.com/Karben233/bili-hardcore")

check()
# 选择使用的LLM模型
print("请选择使用的LLM模型:")
print("1. DeepSeek(V3)")
print("2. Gemini(2.0-flash)(免费版可能会触发 Gemini 风控 429 报错)")
print("3. 自定义 OpenAI 格式的 API 及模型 (OpenAI, 火山引擎, 硅基流动等)")
model_choice = input("请输入数字(1,2,3): ").strip()

API_KEY_GEMINI = ''
API_KEY_DEEPSEEK = ''

if model_choice == '2':
    API_KEY_GEMINI = load_api_key('gemini')
    if not API_KEY_GEMINI:
        API_KEY_GEMINI = input('请输入GEMINI API密钥: ').strip()
        if API_KEY_GEMINI:
            save_api_key('gemini', API_KEY_GEMINI)

elif model_choice == '1':
    API_KEY_DEEPSEEK = load_api_key('deepseek')
    if not API_KEY_DEEPSEEK:
        API_KEY_DEEPSEEK = input('请输入DEEPSEEK API密钥: ').strip()
        if API_KEY_DEEPSEEK:
            save_api_key('deepseek', API_KEY_DEEPSEEK)
            
elif model_choice == '3':
    BASE_URL_OPENAI, MODEL_OPENAI, API_KEY_OPENAI = load_openai_config()
    if not all([BASE_URL_OPENAI, MODEL_OPENAI, API_KEY_OPENAI]):
        BASE_URL_OPENAI = input('请输入API基础URL (例如: https://ark.cn-beijing.volces.com/api/v3): ').strip()
        if BASE_URL_OPENAI.endswith('/'):
            BASE_URL_OPENAI = BASE_URL_OPENAI.rstrip('/')
        MODEL_OPENAI = input('请输入模型名称 (例如: deepseek-v3-250324, 不建议使用思考模型，可能产生意想不到的问题): ').strip()
        API_KEY_OPENAI = input('请输入API密钥: ').strip()
        if all([BASE_URL_OPENAI, MODEL_OPENAI, API_KEY_OPENAI]):
            save_openai_config(BASE_URL_OPENAI, MODEL_OPENAI, API_KEY_OPENAI)
else:
    print("无效的选择，默认使用deepseek")
    API_KEY_DEEPSEEK = load_api_key('deepseek')
    if not API_KEY_DEEPSEEK:
        API_KEY_DEEPSEEK = input('请输入DEEPSEEK API密钥:').strip()
        if API_KEY_DEEPSEEK:
            save_api_key('deepseek', API_KEY_DEEPSEEK)

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

# 此处的"当前时间"没有实际用途, 只是为了防止重复的prompt在短时间内大量请求, 被gemini或其他厂商检测到, 触发风控 (类似于沉浸式翻译)
PROMPT = '''
当前时间：{}
你是一个高效精准的答题专家，面对选择题时，直接根据问题和选项判断正确答案，并返回对应选项的序号（1, 2, 3, 4）。示例：
问题：大的反义词是什么？
选项：['长', '宽', '小', '热']
回答：3
如果不确定正确答案，选择最接近的选项序号返回，不提供额外解释或超出 1-4 的内容。
---
请回答我的问题：{}
'''