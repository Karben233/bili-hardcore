from config.config import API_KEY_GEMINI
from tools.logger import logger
def check():
    
    if not API_KEY_GEMINI:
        logger.error("程序终止: 请在config.py中配置GEMINI API KEY")
        exit()