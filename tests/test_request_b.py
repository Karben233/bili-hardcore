"""
Unit tests for request_b module.
"""
import hashlib
import os
import sys
import time
from unittest.mock import Mock, call, patch

import pytest

# Add the bili-hardcore directory to the Python path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', 'bili-hardcore'))

from tools import request_b


class TestRequestB:
    """Test HTTP request utilities."""

    def test_appsign_success(self):
        """Test successful app signature generation."""
        params = {'param1': 'value1', 'param2': 'value2'}
        
        with patch('tools.request_b.time.time', return_value=1234567890):
            result = request_b.appsign(params)
            
            # Check that timestamp and appkey were added
            assert 'ts' in result
            assert 'appkey' in result
            assert 'sign' in result
            assert result['ts'] == '1234567890'
            assert result['appkey'] == request_b.appkey
            
            # Verify signature generation
            expected_params = dict(sorted(params.items()))
            expected_params.update({'ts': '1234567890', 'appkey': request_b.appkey})
            expected_query = '&'.join([f"{k}={v}" for k, v in expected_params.items()])
            expected_sign = hashlib.md5((expected_query + request_b.appsec).encode()).hexdigest()
            
            assert result['sign'] == expected_sign


    def test_get_request_success(self, mock_requests_session, mock_api_response):
        """Test successful GET request."""
        mock_requests_session.get.return_value.json.return_value = mock_api_response
        mock_requests_session.get.return_value.raise_for_status.return_value = None
        
        result = request_b.get('https://api.test.com/endpoint', {'param': 'value'})
        
        assert result == mock_api_response
        mock_requests_session.get.assert_called_once()
        
        # Verify the call was made with signed parameters
        call_args = mock_requests_session.get.call_args
        assert 'https://api.test.com/endpoint' in call_args[0]
        assert 'params' in call_args[1]


    def test_get_request_connection_error(self, mock_requests_session):
        """Test GET request with connection error."""
        mock_requests_session.get.side_effect = request_b.requests.exceptions.ConnectionError("Connection failed")
        
        with pytest.raises(request_b.requests.exceptions.ConnectionError):
            request_b.get('https://api.test.com/endpoint', {'param': 'value'})


    def test_post_request_success(self, mock_requests_session, mock_api_response):
        """Test successful POST request."""
        mock_requests_session.post.return_value.json.return_value = mock_api_response
        mock_requests_session.post.return_value.raise_for_status.return_value = None
        
        result = request_b.post('https://api.test.com/endpoint', {'param': 'value'})
        
        assert result == mock_api_response
        mock_requests_session.post.assert_called_once()
        
        # Verify the call was made with signed parameters
        call_args = mock_requests_session.post.call_args
        assert 'https://api.test.com/endpoint' in call_args[0]
        assert 'data' in call_args[1]



    def test_session_configuration(self):
        """Test that session is properly configured."""
        session = request_b.session

        # Check that session has adapters configured
        adapters = session.adapters
        assert 'http://' in adapters
        assert 'https://' in adapters
        
        # Basic session configuration test
        assert hasattr(session, 'adapters')
        assert hasattr(session, 'headers')

    def test_headers_configuration(self):
        """Test that headers are properly configured."""
        headers = request_b.headers
        
        assert 'User-Agent' in headers
        assert 'Content-Type' in headers
        assert 'Accept' in headers
        assert 'Accept-Language' in headers
        
        # Verify specific header values
        assert headers['Content-Type'] == 'application/x-www-form-urlencoded'
        assert headers['Accept'] == 'application/json'

    def test_appkey_appsec_configuration(self):
        """Test that appkey and appsec are properly configured."""
        from config.config import API_CONFIG
        
        assert request_b.appkey == API_CONFIG['appkey']
        assert request_b.appsec == API_CONFIG['appsec']
        assert request_b.appkey is not None
        assert request_b.appsec is not None
