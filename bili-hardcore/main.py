from scripts.login import auth
from scripts.start_senior import start
from scripts.validate import validate
from tools.logger import logger

try:
    auth()
    validate()
    start()
except Exception as e:
    logger.error(f"程序运行出错: {str(e)}")
    input("按回车键退出程序...")