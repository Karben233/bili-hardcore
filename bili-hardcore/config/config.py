import os

# GEMINI
import json

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
    """保存GEMINI API密钥到用户目录
    
    Args:
        api_key (str): API密钥
    """
    key_file = os.path.join(os.path.expanduser('~'), '.bili-hardcore', 'gemini_key.json')
    try:
        os.makedirs(os.path.dirname(key_file), exist_ok=True)
        with open(key_file, 'w') as f:
            json.dump({'api_key': api_key}, f)
        print('GEMINI API密钥已保存')
    except Exception as e:
        print(f'保存GEMINI API密钥失败: {str(e)}')

# 从用户目录加载API密钥，如果不存在则提示用户输入
API_KEY_GEMINI = load_gemini_key()
if not API_KEY_GEMINI:
    API_KEY_GEMINI = input('请输入GEMINI API密钥: ').strip()
    if API_KEY_GEMINI:
        save_gemini_key(API_KEY_GEMINI)

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