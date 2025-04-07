import requests
from typing import Dict, Any, Optional
from config.config import PROMPT, API_KEY_OPENAI, BASE_URL_OPENAI,MODEL_OPENAI
from time import time

class OpenAIAPI:
    def __init__(self, base_url: str = None, model: str = None, api_key: str = None):
        self.base_url = base_url or BASE_URL_OPENAI
        self.model = model or MODEL_OPENAI
        self.api_key = api_key or API_KEY_OPENAI
        
        if not all([self.base_url, self.model, self.api_key]):
            raise ValueError("OpenAI配置不完整，请先配置BASE_URL_OPENAI、MODEL_OPENAI和API_KEY_OPENAI")

    def ask(self, question: str, timeout: Optional[int] = 10) -> Dict[str, Any]:
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
        except requests.exceptions.RequestException as e:
            raise Exception(f"OpenAI API request failed: {str(e)}")