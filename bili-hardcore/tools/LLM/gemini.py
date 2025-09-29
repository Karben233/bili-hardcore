from time import sleep, time
from typing import Any, Dict, Optional

import requests
from config.config import API_KEY_GEMINI, PROMPT
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
            response = requests.post(
                url,
                headers=headers,
                params=params,
                json=data,
                timeout=timeout
            )
            if response.status_code == 429:
                logger.error("😭触发了 gemini 风控, 请等待自动重试")

            response.raise_for_status()
            return response.json()["candidates"][0]["content"]["parts"][0]["text"]
        except requests.exceptions.RequestException as e:
            raise Exception(f"Gemini API request failed: {str(e)}")