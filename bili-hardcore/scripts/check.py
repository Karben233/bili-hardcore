from config.config import API_KEY_GEMINI
from tools.logger import logger
def check():
    from config.config import model_choice, API_KEY_GEMINI, API_KEY_DEEPSEEK
    
    if model_choice == '1' and not API_KEY_GEMINI:
        logger.error("程序终止: 请配置GEMINI API KEY")
        exit()
    elif model_choice == '2' and not API_KEY_DEEPSEEK:
        logger.error("程序终止: 请配置DEEPSEEK API KEY")
        exit()