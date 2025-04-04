from tools.request_b import post


def qrcode_get():
    '''
    获取二维码
    :return: url,auth_code
    '''
    res = post('https://passport.bilibili.com/x/passport-tv-login/qrcode/auth_code', {
        'local_id':0
    });
    if res and res.get('code') == 0:
        return res.get('data')
    else:
        raise Exception('获取二维码失败{}'.format(res))

def qrcode_poll(auth_code):
    '''
    轮询二维码状态
    :param auth_code:
    :return:
    '''
    return post('https://passport.bilibili.com/x/passport-tv-login/qrcode/poll', {
        'auth_code': auth_code,
        'local_id':0
    });