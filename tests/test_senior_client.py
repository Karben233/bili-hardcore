"""
Unit tests for client/senior.py module.
"""
import os
import sys
from unittest.mock import Mock, patch

import pytest

# Add the bili-hardcore directory to the Python path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', 'bili-hardcore'))

from client import senior


class TestSeniorClient:
    """Test senior client functionality."""

    def test_category_get_success(self, mock_requests_session, mock_category_data):
        """Test successful category retrieval."""
        mock_requests_session.get.return_value.json.return_value = mock_category_data
        mock_requests_session.get.return_value.raise_for_status.return_value = None
        
        with patch('config.config') as mock_config:
            mock_config.access_token = 'test_token'
            mock_config.csrf = 'test_csrf'
            
            result = senior.category_get()
            
            assert result == mock_category_data['data']
            mock_requests_session.get.assert_called_once()

    def test_category_get_41099_error(self, mock_requests_session):
        """Test category retrieval with 41099 error (daily limit)."""
        mock_response_data = {
            'code': 41099,
            'message': 'Daily limit reached'
        }
        mock_requests_session.get.return_value.json.return_value = mock_response_data
        
        with patch('config.config') as mock_config:
            mock_config.access_token = 'test_token'
            mock_config.csrf = 'test_csrf'
            
            with pytest.raises(Exception, match="获取分类失败，可能是已经达到答题限制"):
                senior.category_get()

    def test_category_get_other_error(self, mock_requests_session):
        """Test category retrieval with other error."""
        mock_response_data = {
            'code': -1,
            'message': 'Other error'
        }
        mock_requests_session.get.return_value.json.return_value = mock_response_data
        
        with patch('config.config') as mock_config:
            mock_config.access_token = 'test_token'
            mock_config.csrf = 'test_csrf'
            
            with pytest.raises(Exception, match="获取分类失败，请前往B站APP确认"):
                senior.category_get()

    def test_captcha_get_success(self, mock_requests_session, mock_captcha_data):
        """Test successful captcha retrieval."""
        mock_requests_session.get.return_value.json.return_value = mock_captcha_data
        mock_requests_session.get.return_value.raise_for_status.return_value = None
        
        with patch('config.config') as mock_config:
            mock_config.access_token = 'test_token'
            mock_config.csrf = 'test_csrf'
            
            result = senior.captcha_get()
            
            assert result == mock_captcha_data['data']
            mock_requests_session.get.assert_called_once()

    def test_captcha_get_failure(self, mock_requests_session):
        """Test captcha retrieval failure."""
        mock_response_data = {
            'code': -1,
            'message': 'Captcha failed'
        }
        mock_requests_session.get.return_value.json.return_value = mock_response_data
        
        with patch('config.config') as mock_config:
            mock_config.access_token = 'test_token'
            mock_config.csrf = 'test_csrf'
            
            with pytest.raises(Exception, match="获取验证码失败"):
                senior.captcha_get()

    def test_captcha_submit_success(self, mock_requests_session):
        """Test successful captcha submission."""
        mock_response_data = {
            'code': 0,
            'message': 'Success'
        }
        mock_requests_session.post.return_value.json.return_value = mock_response_data
        mock_requests_session.post.return_value.raise_for_status.return_value = None
        
        with patch('config.config') as mock_config:
            mock_config.access_token = 'test_token'
            mock_config.csrf = 'test_csrf'
            
            result = senior.captcha_submit('test_code', 'test_token', '1,2,3')
            
            assert result is True
            mock_requests_session.post.assert_called_once()

    def test_captcha_submit_failure(self, mock_requests_session):
        """Test captcha submission failure."""
        mock_response_data = {
            'code': -1,
            'message': 'Submission failed'
        }
        mock_requests_session.post.return_value.json.return_value = mock_response_data
        
        with patch('config.config') as mock_config:
            mock_config.access_token = 'test_token'
            mock_config.csrf = 'test_csrf'
            
            with pytest.raises(Exception, match="提交验证码失败"):
                senior.captcha_submit('test_code', 'test_token', '1,2,3')

    def test_question_get_success(self, mock_requests_session, mock_question_data):
        """Test successful question retrieval."""
        mock_requests_session.get.return_value.json.return_value = mock_question_data
        mock_requests_session.get.return_value.raise_for_status.return_value = None
        
        with patch('config.config') as mock_config:
            mock_config.access_token = 'test_token'
            mock_config.csrf = 'test_csrf'
            
            result = senior.question_get()
            
            assert result == mock_question_data
            mock_requests_session.get.assert_called_once()

    def test_question_submit_success(self, mock_requests_session):
        """Test successful question submission."""
        mock_response_data = {
            'code': 0,
            'message': 'Success'
        }
        mock_requests_session.post.return_value.json.return_value = mock_response_data
        mock_requests_session.post.return_value.raise_for_status.return_value = None
        
        with patch('config.config') as mock_config:
            mock_config.access_token = 'test_token'
            mock_config.csrf = 'test_csrf'
            
            result = senior.question_submit('test_id', 'test_hash', 'test_answer')
            
            assert result == mock_response_data
            mock_requests_session.post.assert_called_once()

    def test_question_result_success(self, mock_requests_session, mock_result_data):
        """Test successful result retrieval."""
        mock_requests_session.get.return_value.json.return_value = mock_result_data
        mock_requests_session.get.return_value.raise_for_status.return_value = None
        
        with patch('config.config') as mock_config:
            mock_config.access_token = 'test_token'
            mock_config.csrf = 'test_csrf'
            
            result = senior.question_result()
            
            assert result == mock_result_data['data']
            mock_requests_session.get.assert_called_once()

    def test_question_result_failure(self, mock_requests_session):
        """Test result retrieval failure."""
        mock_response_data = {
            'code': -1,
            'message': 'Result failed'
        }
        mock_requests_session.get.return_value.json.return_value = mock_response_data
        
        with patch('config.config') as mock_config:
            mock_config.access_token = 'test_token'
            mock_config.csrf = 'test_csrf'
            
            with pytest.raises(Exception, match="答题结果获取失败"):
                senior.question_result()


