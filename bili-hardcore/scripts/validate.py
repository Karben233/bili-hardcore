from client.user_info import get_account_info
from tools.logger import logger

def validate():
    account_info = get_account_info();
    logger.debug('获取用户信息成功 {}'.format(account_info))
    if account_info and not account_info.get('level') == 6:
        raise Exception('当前用户未满6级，无法参与答题')