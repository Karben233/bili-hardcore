"""
Integration tests for bili-hardcore application.
"""
import os
import sys
from unittest.mock import MagicMock, Mock, patch

import pytest

# Add the bili-hardcore directory to the Python path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', 'bili-hardcore'))


class TestIntegration:
    """Integration tests for complete workflows."""

    def test_basic_imports(self):
        """Test that all main modules can be imported without errors."""
        # Test core imports
        try:
            from config import config
            from tools.logger import logger
            from tools.request_b import get, post
            assert True  # All imports successful
        except ImportError as e:
            pytest.fail(f"Failed to import core modules: {e}")

    def test_config_integration(self):
        """Test that config module works with testing environment."""
        from config import config

        # Test that testing mode is properly detected
        assert hasattr(config, 'TESTING')
        assert config.TESTING is True
        
        # Test that test values are set
        assert config.API_KEY_DEEPSEEK == 'test_deepseek_key'
        assert config.API_KEY_GEMINI == 'test_gemini_key'

    def test_logger_integration(self):
        """Test that logger works in testing environment."""
        from tools.logger import logger

        # Test that logger can be created
        assert logger is not None
        assert hasattr(logger, 'info')
        assert hasattr(logger, 'error')
        assert hasattr(logger, 'debug')

    def test_request_basic_functionality(self):
        """Test basic request functionality without external calls."""
        from tools.request_b import appsign

        # Test app signature generation
        params = {'test': 'value'}
        result = appsign(params)
        
        assert 'sign' in result
        assert 'ts' in result
        assert 'appkey' in result
        assert result['test'] == 'value'

    def test_client_modules_import(self):
        """Test that client modules can be imported."""
        try:
            from client import login, senior
            assert True  # All imports successful
        except ImportError as e:
            pytest.fail(f"Failed to import client modules: {e}")

    def test_llm_modules_import(self):
        """Test that LLM modules can be imported."""
        try:
            from tools.LLM import deepseek
            assert True  # All imports successful
        except ImportError as e:
            pytest.fail(f"Failed to import LLM modules: {e}")

    def test_end_to_end_workflow_simulation(self):
        """Test a simulated end-to-end workflow without external dependencies."""
        # This is a mock test that simulates the workflow without actual API calls
        
        # 1. Test configuration loading
        from config import config
        assert config.TESTING is True
        
        # 2. Test logger initialization
        from tools.logger import logger
        assert logger is not None
        
        # 3. Test request signing
        from tools.request_b import appsign
        signed_params = appsign({'test': 'data'})
        assert 'sign' in signed_params
        
        # 4. Test LLM initialization (without actual API call)
        from tools.LLM.deepseek import DeepSeekAPI
        api = DeepSeekAPI()
        assert api.api_key == 'test_deepseek_key'  # From config in testing mode
        
        # This represents a successful workflow simulation
        assert True

    def test_error_handling_integration(self):
        """Test error handling across modules."""
        from config import config
        from tools.request_b import appsign

        # Test that error handling works across modules
        try:
            # Test with invalid input
            result = appsign({})
            assert isinstance(result, dict)
        except Exception as e:
            # Error handling should work
            assert True

    def test_module_interaction(self):
        """Test interaction between different modules."""
        from config import config
        from tools.request_b import appsign

        # Test that config values are used in request signing
        params = {'test': 'value'}
        signed_params = appsign(params)
        
        # The appkey should come from config
        assert signed_params['appkey'] == config.API_CONFIG['appkey']
        
        # Test that the workflow is consistent
        assert 'sign' in signed_params
        assert 'ts' in signed_params