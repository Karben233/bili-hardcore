import os

# API Keys
import json

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

def load_base_url(key_type):
    """从用户目录加载API base_url
    
    Args:
        key_type (str): API类型 (gemini 或 deepseek)
    
    Returns:
        str: API base_url
    """
    key_file = os.path.join(os.path.expanduser('~'), '.bili-hardcore', f'{key_type}_key.json')
    if os.path.exists(key_file):
        try:
            with open(key_file, 'r') as f:
                data = json.load(f)
                return data.get('base_url', '')
        except Exception as e:
            print(f'读取{key_type.upper()} API base_url失败: {str(e)}')
    return ''

def save_api_key(key_type, api_key, base_url=''):
    """保存API密钥和base_url到用户目录
    
    Args:
        key_type (str): API类型 (gemini 或 deepseek)
        api_key (str): API密钥
        base_url (str, optional): API基础URL，默认为空
    """
    key_file = os.path.join(os.path.expanduser('~'), '.bili-hardcore', f'{key_type}_key.json')
    try:
        os.makedirs(os.path.dirname(key_file), exist_ok=True)
        data = {'api_key': api_key}
        if base_url:
            data['base_url'] = base_url
        with open(key_file, 'w') as f:
            json.dump(data, f)
        print(f'{key_type.upper()} API信息已保存')
    except Exception as e:
        print(f'保存{key_type.upper()} API信息失败: {str(e)}')

# 选择使用的LLM模型
print("请选择使用的LLM模型：")
print("1. DeepSeek")
print("2. Gemini")
model_choice = input("请输入数字(1或2): ").strip()

API_KEY_GEMINI = ''
API_KEY_DEEPSEEK = ''
BASE_URL_GEMINI = ''
BASE_URL_DEEPSEEK = ''

if model_choice == '2':
    API_KEY_GEMINI = load_api_key('gemini')
    BASE_URL_GEMINI = load_base_url('gemini') or "https://generativelanguage.googleapis.com/v1beta"
    if not API_KEY_GEMINI:
        API_KEY_GEMINI = input('请输入GEMINI API密钥: ').strip()
        if API_KEY_GEMINI:
            base_url_input = input('请输入GEMINI API基础URL(可选，直接回车使用默认值): ').strip()
            BASE_URL_GEMINI = base_url_input or BASE_URL_GEMINI
            save_api_key('gemini', API_KEY_GEMINI, BASE_URL_GEMINI)

elif model_choice == '1':
    API_KEY_DEEPSEEK = load_api_key('deepseek')
    BASE_URL_DEEPSEEK = load_base_url('deepseek') or "https://api.deepseek.com/v1"
    if not API_KEY_DEEPSEEK:
        API_KEY_DEEPSEEK = input('请输入DEEPSEEK API密钥: ').strip()
        if API_KEY_DEEPSEEK:
            base_url_input = input('请输入DEEPSEEK API基础URL(可选，直接回车使用默认值): ').strip()
            BASE_URL_DEEPSEEK = base_url_input or BASE_URL_DEEPSEEK
            save_api_key('deepseek', API_KEY_DEEPSEEK, BASE_URL_DEEPSEEK)
else:
    print("无效的选择，默认使用deepseek")
    API_KEY_DEEPSEEK = load_api_key('deepseek')
    BASE_URL_DEEPSEEK = load_base_url('deepseek') or "https://api.deepseek.com/v1"
    if not API_KEY_DEEPSEEK:
        API_KEY_DEEPSEEK = input('请输入DEEPSEEK API密钥:').strip()
        if API_KEY_DEEPSEEK:
            base_url_input = input('请输入DEEPSEEK API基础URL(可选，直接回车使用默认值): ').strip()
            BASE_URL_DEEPSEEK = base_url_input or BASE_URL_DEEPSEEK
            save_api_key('deepseek', API_KEY_DEEPSEEK, BASE_URL_DEEPSEEK)

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