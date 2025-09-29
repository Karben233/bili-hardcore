"""
Unit tests for client/login.py module.
"""
import os
import sys
from unittest.mock import Mock, patch

import pytest

# Add the bili-hardcore directory to the Python path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', 'bili-hardcore'))

from client import login


class TestLoginClient:
    """Test login client functionality."""

    def test_qrcode_get_success(self, mock_requests_session, mock_api_response):
        """Test successful QR code generation."""
        mock_response_data = {
            'code': 0,
            'data': {
                'url': 'https://example.com/qr.png',
                'auth_code': 'test_auth_code_123'
            }
        }
        mock_requests_session.post.return_value.json.return_value = mock_response_data
        mock_requests_session.post.return_value.raise_for_status.return_value = None
        
        result = login.qrcode_get()
        
        assert result == mock_response_data['data']
        mock_requests_session.post.assert_called_once()
        
        # Verify the correct endpoint was called
        call_args = mock_requests_session.post.call_args
        assert 'https://passport.bilibili.com/x/passport-tv-login/qrcode/auth_code' in call_args[0]

    def test_qrcode_get_failure(self, mock_requests_session):
        """Test QR code generation failure."""
        mock_response_data = {
            'code': -1,
            'message': 'Failed to generate QR code'
        }
        mock_requests_session.post.return_value.json.return_value = mock_response_data
        
        with pytest.raises(Exception, match="获取二维码失败"):
            login.qrcode_get()

    def test_qrcode_get_no_response(self, mock_requests_session):
        """Test QR code generation with no response."""
        mock_requests_session.post.return_value.json.return_value = None
        
        with pytest.raises(Exception, match="获取二维码失败"):
            login.qrcode_get()

    def test_qrcode_poll_success(self, mock_requests_session, mock_api_response):
        """Test successful QR code polling."""
        mock_response_data = {
            'code': 0,
            'data': {
                'access_token': 'test_access_token',
                'mid': 12345678,
                'cookie_info': {
                    'cookies': [
                        {'name': 'bili_jct', 'value': 'test_csrf_token'},
                        {'name': 'other_cookie', 'value': 'other_value'}
                    ]
                }
            }
        }
        mock_requests_session.post.return_value.json.return_value = mock_response_data
        mock_requests_session.post.return_value.raise_for_status.return_value = None
        
        result = login.qrcode_poll('test_auth_code')
        
        assert result == mock_response_data
        mock_requests_session.post.assert_called_once()
        
        # Verify the correct endpoint was called
        call_args = mock_requests_session.post.call_args
        assert 'https://passport.bilibili.com/x/passport-tv-login/qrcode/poll' in call_args[0]

    def test_qrcode_poll_failure(self, mock_requests_session):
        """Test QR code polling failure."""
        mock_response_data = {
            'code': -1,
            'message': 'Polling failed'
        }
        mock_requests_session.post.return_value.json.return_value = mock_response_data
        
        result = login.qrcode_poll('test_auth_code')
        
        assert result == mock_response_data

    def test_qrcode_poll_request_exception(self, mock_requests_session):
        """Test QR code polling with request exception."""
        mock_requests_session.post.side_effect = Exception("Network error")
        
        with pytest.raises(Exception, match="Network error"):
            login.qrcode_poll('test_auth_code')

    def test_qrcode_get_request_exception(self, mock_requests_session):
        """Test QR code generation with request exception."""
        mock_requests_session.post.side_effect = Exception("Network error")
        
        with pytest.raises(Exception, match="Network error"):
            login.qrcode_get()

    def test_qrcode_get_parameters(self, mock_requests_session):
        """Test that QR code generation uses correct parameters."""
        mock_response_data = {
            'code': 0,
            'data': {'url': 'test', 'auth_code': 'test'}
        }
        mock_requests_session.post.return_value.json.return_value = mock_response_data
        mock_requests_session.post.return_value.raise_for_status.return_value = None
        
        login.qrcode_get()
        
        # Verify the request was made with correct data
        call_args = mock_requests_session.post.call_args
        assert 'data' in call_args[1]
        request_data = call_args[1]['data']
        assert 'local_id' in request_data
        assert request_data['local_id'] == 0

    def test_qrcode_poll_parameters(self, mock_requests_session):
        """Test that QR code polling uses correct parameters."""
        mock_response_data = {
            'code': 0,
            'data': {}
        }
        mock_requests_session.post.return_value.json.return_value = mock_response_data
        mock_requests_session.post.return_value.raise_for_status.return_value = None
        
        login.qrcode_poll('test_auth_code_123')
        
        # Verify the request was made with correct data
        call_args = mock_requests_session.post.call_args
        assert 'data' in call_args[1]
        request_data = call_args[1]['data']
        assert 'auth_code' in request_data
        assert 'local_id' in request_data
        assert request_data['auth_code'] == 'test_auth_code_123'
        assert request_data['local_id'] == 0
