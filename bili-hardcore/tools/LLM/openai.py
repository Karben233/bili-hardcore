import requests
from typing import Dict, Any, Optional
from config.config import PROMPT, API_KEY_OPENAI, BASE_URL_OPENAI,MODEL_OPENAI
from time import time
from tools.logger import logger

class OpenAIAPI:
    def __init__(self, base_url: str = None, model: str = None, api_key: str = None):
        self.base_url = base_url or BASE_URL_OPENAI
        self.model = model or MODEL_OPENAI
        self.api_key = api_key or API_KEY_OPENAI
        
        if not all([self.base_url, self.model]):
            raise ValueError("OpenAI配置不完整，请先配置BASE_URL_OPENAI、MODEL_OPENAI和API_KEY_OPENAI")

    def ask(self, question: str, timeout: Optional[int] = 30) -> Dict[str, Any]:
        url = f"{self.base_url}/chat/completions"
        
        headers = {
            "Content-Type": "application/json",
            "Authorization": f"Bearer {self.api_key}"
        }
        
        data = {
            "model": self.model,
            "messages": [
                {
                    "role": "user",
                    "content": PROMPT.format(time(), question)
                }
            ]
        }

        try:
            response = requests.post(
                url,
                headers=headers,
                json=data,
                timeout=timeout
            )
            response.raise_for_status()
            return response.json()["choices"][0]["message"]["content"]
        except requests.exceptions.SSLError as e:
            if 'dashscope.aliyuncs.com' in self.base_url:
                logger.error("😭使用阿里云百炼请关闭系统代理，否则可能会报错🚫✈️")
            raise Exception(f"OpenAI API request failed: {str(e)}")
        except requests.exceptions.RequestException as e:
            if 'deepseek-reasoner' in self.model or 'deepseek-r1' in self.model or 'thinking' in self.model or 'o1' in self.model or 'claude-3-7-sonnet' in self.model:
                logger.error("😭不建议使用思考模型，思维链过长可能导致超时")
            raise Exception(f"OpenAI API request failed: {str(e)}")