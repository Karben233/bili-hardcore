"""
Unit tests for tools/LLM/deepseek.py module.
"""
import pytest
import sys
import os
from unittest.mock import patch, Mock

# Add the bili-hardcore directory to the Python path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', 'bili-hardcore'))

from tools.LLM import deepseek


class TestDeepSeekAPI:
    """Test DeepSeek LLM API functionality."""

    def test_init(self):
        """Test DeepSeek API initialization."""
        with patch('tools.LLM.deepseek.API_KEY_DEEPSEEK', 'test_api_key'):
            api = deepseek.DeepSeekAPI()
            
            assert api.base_url == "https://api.deepseek.com/v1"
            assert api.model == "deepseek-chat"
            assert api.api_key == "test_api_key"

    def test_ask_success(self):
        """Test successful API request."""
        mock_response = Mock()
        mock_response.json.return_value = {
            "choices": [
                {"message": {"content": "2"}}
            ]
        }
        mock_response.raise_for_status.return_value = None
        
        with patch('tools.LLM.deepseek.API_KEY_DEEPSEEK', 'test_api_key'):
            with patch('tools.LLM.deepseek.requests.post', return_value=mock_response) as mock_post:
                api = deepseek.DeepSeekAPI()
                result = api.ask("What is 1+1?", timeout=30)
                
                assert result == "2"
                mock_post.assert_called_once()
                
                # Verify request parameters
                call_args = mock_post.call_args
                assert call_args[1]['timeout'] == 30
                
                # Verify request data
                request_data = call_args[1]['json']
                assert request_data['model'] == "deepseek-chat"
                assert len(request_data['messages']) == 1
                assert request_data['messages'][0]['role'] == "user"
                assert "What is 1+1?" in request_data['messages'][0]['content']





    def test_ask_default_timeout(self):
        """Test that default timeout is used when not specified."""
        mock_response = Mock()
        mock_response.json.return_value = {
            "choices": [
                {"message": {"content": "1"}}
            ]
        }
        mock_response.raise_for_status.return_value = None
        
        with patch('tools.LLM.deepseek.API_KEY_DEEPSEEK', 'test_api_key'):
            with patch('tools.LLM.deepseek.requests.post', return_value=mock_response) as mock_post:
                api = deepseek.DeepSeekAPI()
                result = api.ask("Test question")
                
                # Verify default timeout was used
                call_args = mock_post.call_args
                assert call_args[1]['timeout'] == 30

    def test_ask_custom_timeout(self):
        """Test API request with custom timeout."""
        mock_response = Mock()
        mock_response.json.return_value = {
            "choices": [
                {"message": {"content": "4"}}
            ]
        }
        mock_response.raise_for_status.return_value = None
        
        with patch('tools.LLM.deepseek.API_KEY_DEEPSEEK', 'test_api_key'):
            with patch('tools.LLM.deepseek.requests.post', return_value=mock_response) as mock_post:
                api = deepseek.DeepSeekAPI()
                result = api.ask("Test question", timeout=60)
                
                # Verify custom timeout was used
                call_args = mock_post.call_args
                assert call_args[1]['timeout'] == 60

    def test_ask_headers(self):
        """Test that correct headers are sent with request."""
        mock_response = Mock()
        mock_response.json.return_value = {
            "choices": [
                {"message": {"content": "2"}}
            ]
        }
        mock_response.raise_for_status.return_value = None
        
        with patch('tools.LLM.deepseek.API_KEY_DEEPSEEK', 'test_api_key'):
            with patch('tools.LLM.deepseek.requests.post', return_value=mock_response) as mock_post:
                api = deepseek.DeepSeekAPI()
                result = api.ask("Test question")
                
                # Verify headers
                call_args = mock_post.call_args
                headers = call_args[1]['headers']
                
                assert headers['Content-Type'] == "application/json"
                assert headers['Authorization'] == "Bearer test_api_key"

    def test_ask_url_construction(self):
        """Test that correct URL is constructed for API request."""
        mock_response = Mock()
        mock_response.json.return_value = {
            "choices": [
                {"message": {"content": "1"}}
            ]
        }
        mock_response.raise_for_status.return_value = None
        
        with patch('tools.LLM.deepseek.API_KEY_DEEPSEEK', 'test_api_key'):
            with patch('tools.LLM.deepseek.requests.post', return_value=mock_response) as mock_post:
                api = deepseek.DeepSeekAPI()
                result = api.ask("Test question")
                
                # Verify URL
                call_args = mock_post.call_args
                url = call_args[0][0]
                
                assert url == "https://api.deepseek.com/v1/chat/completions"
