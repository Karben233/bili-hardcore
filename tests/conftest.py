"""
Pytest configuration and fixtures for bili-hardcore tests.
"""
import pytest
import os
import json
import tempfile
import shutil
from unittest.mock import Mock, patch
from typing import Dict, Any


@pytest.fixture
def temp_dir():
    """Create a temporary directory for test files."""
    temp_dir = tempfile.mkdtemp()
    yield temp_dir
    shutil.rmtree(temp_dir)


@pytest.fixture
def mock_config_dir(temp_dir):
    """Mock the config directory for testing."""
    config_dir = os.path.join(temp_dir, '.bili-hardcore')
    os.makedirs(config_dir, exist_ok=True)
    
    with patch('config.config.AUTH_FILE', os.path.join(config_dir, 'auth.json')):
        yield config_dir


@pytest.fixture
def mock_auth_data():
    """Mock authentication data."""
    return {
        'access_token': 'test_access_token',
        'csrf': 'test_csrf',
        'mid': '12345678',
        'cookie': 'test_cookie=value'
    }


@pytest.fixture
def mock_api_response():
    """Mock API response structure."""
    return {
        'code': 0,
        'message': 'success',
        'data': {}
    }


@pytest.fixture
def mock_question_data():
    """Mock question data structure."""
    return {
        'code': 0,
        'data': {
            'id': 'test_question_id',
            'question': 'What is the capital of China?',
            'question_num': 1,
            'answers': [
                {'ans_text': 'Beijing', 'ans_hash': 'hash1'},
                {'ans_text': 'Shanghai', 'ans_hash': 'hash2'},
                {'ans_text': 'Guangzhou', 'ans_hash': 'hash3'},
                {'ans_text': 'Shenzhen', 'ans_hash': 'hash4'}
            ]
        }
    }


@pytest.fixture
def mock_category_data():
    """Mock category data structure."""
    return {
        'code': 0,
        'data': {
            'categories': [
                {'id': 1, 'name': '知识区'},
                {'id': 2, 'name': '历史区'},
                {'id': 3, 'name': '游戏区'}
            ]
        }
    }


@pytest.fixture
def mock_captcha_data():
    """Mock captcha data structure."""
    return {
        'code': 0,
        'data': {
            'url': 'https://example.com/captcha.png',
            'token': 'test_captcha_token'
        }
    }


@pytest.fixture
def mock_result_data():
    """Mock quiz result data structure."""
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


@pytest.fixture
def mock_llm_response():
    """Mock LLM API response."""
    return "2"  # Answer choice


@pytest.fixture
def mock_requests_session():
    """Mock requests session for HTTP calls."""
    with patch('tools.request_b.session') as mock_session:
        mock_response = Mock()
        mock_response.json.return_value = {'code': 0, 'data': {}}
        mock_response.raise_for_status.return_value = None
        mock_session.get.return_value = mock_response
        mock_session.post.return_value = mock_response
        yield mock_session


@pytest.fixture(autouse=True)
def setup_test_env():
    """Setup test environment variables and paths."""
    # Mock home directory for config files
    with patch.dict(os.environ, {'HOME': '/tmp/test_home'}):
        yield


@pytest.fixture
def sample_config():
    """Sample configuration for testing."""
    return {
        'API_KEY_DEEPSEEK': 'test_deepseek_key',
        'API_KEY_GEMINI': 'test_gemini_key',
        'BASE_URL_OPENAI': 'https://api.openai.com/v1',
        'MODEL_OPENAI': 'gpt-3.5-turbo',
        'API_KEY_OPENAI': 'test_openai_key'
    }
