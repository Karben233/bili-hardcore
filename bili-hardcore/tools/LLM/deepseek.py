import requests
from typing import Dict, Any, Optional
from config.config import PROMPT, API_KEY_DEEPSEEK
from datetime import datetime

class DeepSeekAPI:
    def __init__(self):
        self.base_url = "https://api.deepseek.com/v1"
        self.model = "deepseek-chat"
        self.api_key = API_KEY_DEEPSEEK

    def ask(self, question: str, timeout: Optional[int] = 30) -> Dict[str, Any]:
        url = f"{self.base_url}/chat/completions"
        
        headers = {
            "Content-Type": "application/json",
            "Authorization": f"Bearer {self.api_key}"
        }
        
        current_time = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
        
        data = {
            "model": self.model,
            "messages": [
                {
                    "role": "user",
                    "content": PROMPT.format(current_time, question)
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
        except requests.exceptions.RequestException as e:
            raise Exception(f"DeepSeek API request failed: {str(e)}")