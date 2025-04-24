from tools.request_b import get, post
from config import config

def get_account_info():
    '''
    查询用户信息
    :return:
    '''
    res = get('https://app.bilibili.com/x/v2/account/myinfo',{
         'access_key':config.access_token
    });
    if res and res.get('code') == 0:
        return res.get('data')
    else:
        raise Exception('获取用户信息失败{}'.format(res))