"""
Test utilities and helper functions.
"""
import os
import json
import tempfile
import shutil
from unittest.mock import patch, mock_open


def create_temp_config_file(temp_dir, filename, content):
    """Create a temporary config file with given content."""
    file_path = os.path.join(temp_dir, filename)
    with open(file_path, 'w') as f:
        if isinstance(content, dict):
            json.dump(content, f)
        else:
            f.write(content)
    return file_path


def mock_file_system():
    """Context manager to mock file system operations."""
    return patch.multiple(
        'os.path',
        exists=patch('os.path.exists'),
        join=patch('os.path.join'),
        expanduser=patch('os.path.expanduser')
    )


def create_mock_auth_data():
    """Create mock authentication data."""
    return {
        'access_token': 'mock_access_token_123',
        'csrf': 'mock_csrf_token_456',
        'mid': '12345678',
        'cookie': 'mock_cookie=value; another_cookie=another_value'
    }


def create_mock_api_response(code=0, data=None, message='success'):
    """Create mock API response."""
    return {
        'code': code,
        'message': message,
        'data': data or {}
    }


def create_mock_question_data():
    """Create mock question data."""
    return {
        'code': 0,
        'data': {
            'id': 'mock_question_id_123',
            'question': 'Mock test question?',
            'question_num': 1,
            'answers': [
                {'ans_text': 'Option A', 'ans_hash': 'hash_a'},
                {'ans_text': 'Option B', 'ans_hash': 'hash_b'},
                {'ans_text': 'Option C', 'ans_hash': 'hash_c'},
                {'ans_text': 'Option D', 'ans_hash': 'hash_d'}
            ]
        }
    }


def create_mock_category_data():
    """Create mock category data."""
    return {
        'code': 0,
        'data': {
            'categories': [
                {'id': 1, 'name': '知识区'},
                {'id': 2, 'name': '历史区'},
                {'id': 3, 'name': '游戏区'},
                {'id': 4, 'name': '科技区'},
                {'id': 5, 'name': '生活区'}
            ]
        }
    }


def create_mock_captcha_data():
    """Create mock captcha data."""
    return {
        'code': 0,
        'data': {
            'url': 'https://mock-captcha.example.com/captcha.png',
            'token': 'mock_captcha_token_789'
        }
    }


def create_mock_result_data():
    """Create mock quiz result data."""
    return {
        'code': 0,
        'data': {
            'score': 85,
            'scores': [
                {'category': '知识区', 'score': 45, 'total': 50},
                {'category': '历史区', 'score': 40, 'total': 50}
            ]
        }
    }


def mock_requests_response(json_data=None, status_code=200, text=''):
    """Create a mock requests response."""
    response = mock_open()
    response.json.return_value = json_data or {}
    response.status_code = status_code
    response.text = text
    response.raise_for_status.return_value = None
    return response


def mock_llm_response(answer_choice=1):
    """Create a mock LLM response."""
    return str(answer_choice)


class MockLogger:
    """Mock logger for testing."""
    
    def __init__(self):
        self.messages = []
    
    def info(self, message):
        self.messages.append(('INFO', message))
    
    def warning(self, message):
        self.messages.append(('WARNING', message))
    
    def error(self, message):
        self.messages.append(('ERROR', message))
    
    def debug(self, message):
        self.messages.append(('DEBUG', message))
    
    def get_messages(self, level=None):
        """Get messages, optionally filtered by level."""
        if level:
            return [msg for msg in self.messages if msg[0] == level]
        return self.messages
    
    def clear(self):
        """Clear all messages."""
        self.messages = []


def setup_mock_environment():
    """Setup mock environment for testing."""
    env_patches = {
        'HOME': '/tmp/test_home',
        'USER': 'test_user',
        'PATH': '/usr/bin:/bin'
    }
    return patch.dict('os.environ', env_patches)


def create_temp_config_dir():
    """Create a temporary config directory structure."""
    temp_dir = tempfile.mkdtemp()
    config_dir = os.path.join(temp_dir, '.bili-hardcore')
    os.makedirs(config_dir, exist_ok=True)
    
    # Create subdirectories
    logs_dir = os.path.join(temp_dir, 'logs')
    os.makedirs(logs_dir, exist_ok=True)
    
    return temp_dir, config_dir, logs_dir


def cleanup_temp_dir(temp_dir):
    """Clean up temporary directory."""
    if os.path.exists(temp_dir):
        shutil.rmtree(temp_dir)
