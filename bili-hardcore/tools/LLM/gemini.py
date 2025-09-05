import requests
from typing import Dict, Any, Optional
from config.config import PROMPT,API_KEY_GEMINI
from time import time,sleep
from tools.logger import logger


class GeminiAPI:
    def __init__(self):
        self.base_url = "https://ai-proxy.chatwise.app/generativelanguage/v1beta"
        self.model = "gemini-2.5-flash"
        self.api_key = API_KEY_GEMINI

    def ask(self, question: str, timeout: Optional[int] = 30) -> Dict[str, Any]:
        url = f"{self.base_url}/models/{self.model}:generateContent"
        
        headers = {
            "Content-Type": "application/json"
        }
        
        data = {
            "contents": [
                {
                    "parts": [
                        {
                            "text": PROMPT.format(time(), question)
                        }
                    ]
                }
            ]
        }

        params = {
            "key": self.api_key
        }

        try:
            sleep(5) # 每次请求间隔5秒，这样可能会减少 429 错误
            response = requests.post(
                url,
                headers=headers,
                params=params,
                json=data,
                timeout=timeout
            )
            if response.status_code == 429:
                logger.error("😭触发了 gemini 风控, 尝试以下步骤重试")
                logger.error("1. 稍等一下重新运行")
                logger.error("2. 尝试切换代理节点（更换IP）")
                logger.error("3. 建议更换其他模型答题")

            response.raise_for_status()
            return response.json()["candidates"][0]["content"]["parts"][0]["text"]
        except requests.exceptions.RequestException as e:
            raise Exception(f"Gemini API request failed: {str(e)}")