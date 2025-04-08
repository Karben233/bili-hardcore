from tools.request_b import get, post

access_token = None;
csrf = None;

def category_get():
    '''
    获取分类
    '''
    res = get('https://api.bilibili.com/x/senior/v1/category', {
        'access_key':access_token,
        'csrf':csrf,
        'disable_rcmd':0,
        'mobi_app':'android',
        'platform':'android',
        'statistics':'{"appId":1,"platform":3,"version":"8.40.0","abtest":""}',
        'web_location':'333.790'
    });
    if res and res.get('code') == 0:
        return res.get('data')
    else:
        print('获取分类失败，可能是已开始答题或答题限制{}'.format(res))
        exit()

def captcha_get():
    '''
    获取验证码
    '''
    res = get('https://api.bilibili.com/x/senior/v1/captcha', {
        'access_key':access_token,
        'csrf':csrf,
        'disable_rcmd':0,
        'mobi_app':'android',
        'platform':'android',
        'statistics':'{"appId":1,"platform":3,"version":"8.40.0","abtest":""}',
        'web_location':'333.790'
    });
    if res and res.get('code') == 0:
        return res.get('data')
    else:
        print('获取验证码失败，可能是已开始答题或答题限制{}'.format(res))
        exit()

def captcha_submit(code,captcha_token,ids):
    '''
    提交验证码
    '''
    res = post('https://api.bilibili.com/x/senior/v1/captcha/submit', {
        "access_key": access_token,
        "bili_code": code,
        "bili_token": captcha_token,
        "csrf": csrf,
        "disable_rcmd": "0",
        "gt_challenge": "",
        "gt_seccode": "",
        "gt_validate": "",
        "ids": ids,
        "mobi_app": "android",
        "platform": "android",
        "statistics": "{\"appId\":1,\"platform\":3,\"version\":\"8.40.0\",\"abtest\":\"\"}",
        "type": "bilibili",
    });
    if res and res.get('code') == 0:
        return True
    else:
        raise Exception('提交验证码失败{}'.format(res))

def question_get():
    '''
    获取题目
    '''
    return get('https://api.bilibili.com/x/senior/v1/question', {
        "access_key": access_token,
        "csrf": csrf,
        "disable_rcmd": "0",
        "mobi_app": "android",
        "platform": "android",
        "statistics": "{\"appId\":1,\"platform\":3,\"version\":\"8.40.0\",\"abtest\":\"\"}",
        "web_location": "333.790",
    });

def question_submit(id,ans_hash,ans_text):
    '''
    提交答案
    '''
    return post('https://api.bilibili.com/x/senior/v1/answer/submit', {
        "access_key": access_token,
        "csrf": csrf,
        "id": id,
        "ans_hash": ans_hash,
        "ans_text": ans_text,
        "disable_rcmd": "0",
        "mobi_app": "android",
        "platform": "android",
        "statistics": "{\"appId\":1,\"platform\":3,\"version\":\"8.40.0\",\"abtest\":\"\"}",
        "web_location": "333.790",
    });

def question_result():
    '''
    获取答题结果
    '''
    res = get('https://api.bilibili.com/x/senior/v1/answer/result', {
        "access_key": access_token,
        "csrf": csrf,
        "disable_rcmd": "0",
        "mobi_app": "android",
        "platform": "android",
        "statistics": "{\"appId\":1,\"platform\":3,\"version\":\"8.40.0\",\"abtest\":\"\"}",
        "web_location": "333.790",
    });
    if res and res.get('code') == 0:
        return res.get('data')
        '''
        {
            "score": 28,// 总分数
            // 分数详情
            "scores": [
            {
                "category": "鬼畜",
                "score": 13,
                "total": 50
            },
            {
                "category": "游戏",
                "score": 15,
                "total": 50
            }
            ]
        }
        '''
    else:
        raise Exception('答题结果获取失败{}'.format(res))