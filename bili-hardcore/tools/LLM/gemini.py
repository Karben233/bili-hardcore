import requests
from typing import Dict, Any, Optional
from config.config import PROMPT,API_KEY_GEMINI
from datetime import datetime
from time import sleep


class GeminiAPI:
    def __init__(self):
        self.base_url = "https://generativelanguage.googleapis.com/v1beta"
        self.model = "gemini-2.0-flash"
        self.api_key = API_KEY_GEMINI

    def ask(self, question: str, timeout: Optional[int] = 30) -> Dict[str, Any]:
        url = f"{self.base_url}/models/{self.model}:generateContent"
        
        headers = {
            "Content-Type": "application/json"
        }
        
        current_time = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
        
        data = {
            "contents": [
                {
                    "parts": [
                        {
                            "text": PROMPT.format(current_time, question)
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
            response.raise_for_status()
            return response.json()["candidates"][0]["content"]["parts"][0]["text"]
        except requests.exceptions.RequestException as e:
            raise Exception(f"Gemini API request failed: {str(e)}")