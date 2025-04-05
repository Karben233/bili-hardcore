import os

# GEMINI
API_KEY_GEMINI=''# 填写自己的GEMINI API KEY

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
你是一个全知全能的答题专家，现在我要问你一个问题，答案一共有四个选项，请告诉我第几个答案是正确的。比如：
```
问题：大的反义词是什么？
答案：['长','宽','小','热']
```
你的回答应该是：3
如果你不确定正确的答案是什么，就回答我一个你认为最接近的正确答案，不要回复`1,2,3,4`以外的内容
---
下面，请回答我的问题：{}
'''