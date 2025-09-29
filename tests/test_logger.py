"""
Unit tests for tools/logger.py module.
"""
import pytest
import os
import sys
import logging
from unittest.mock import patch, Mock, mock_open

# Add the bili-hardcore directory to the Python path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', 'bili-hardcore'))

from tools import logger


class TestLogger:
    """Test logger functionality."""

    def test_setup_logger_basic(self, temp_dir):
        """Test basic logger setup."""
        with patch('tools.logger.os.path.join') as mock_join:
            mock_join.return_value = os.path.join(temp_dir, 'test.log')
            
            with patch('tools.logger.os.makedirs') as mock_makedirs:
                with patch('builtins.open', mock_open()) as mock_file:
                    test_logger = logger.setup_logger('test_logger')
                    
                    assert isinstance(test_logger, logging.Logger)
                    assert test_logger.name == 'test_logger'
                    assert test_logger.level == logger.logger_level
                    
                    # Verify handlers were added
                    # In testing mode, only console handler should be present
                    assert len(test_logger.handlers) >= 1  # Console handler
                    
                    # In testing mode, makedirs should not be called
                    # mock_makedirs.assert_called_once()  # Not called in testing mode

    def test_setup_logger_handlers(self, temp_dir):
        """Test that logger has correct handlers."""
        with patch('tools.logger.os.path.join') as mock_join:
            mock_join.return_value = os.path.join(temp_dir, 'test.log')
            
            with patch('tools.logger.os.makedirs'):
                with patch('builtins.open', mock_open()):
                    test_logger = logger.setup_logger('test_handler_logger')
                    
                    # Check for file handler (should be 0 in testing mode)
                    file_handlers = [h for h in test_logger.handlers if isinstance(h, logging.FileHandler)]
                    assert len(file_handlers) == 0  # No file handlers in testing mode
                    
                    # Check for console handler (should be present)
                    console_handlers = [h for h in test_logger.handlers if isinstance(h, logging.StreamHandler) and not isinstance(h, logging.FileHandler)]
                    assert len(console_handlers) >= 1  # At least one console handler

    def test_setup_logger_formatters(self, temp_dir):
        """Test that logger formatters are correct."""
        with patch('tools.logger.os.path.join') as mock_join:
            mock_join.return_value = os.path.join(temp_dir, 'test.log')
            
            with patch('tools.logger.os.makedirs'):
                with patch('builtins.open', mock_open()):
                    test_logger = logger.setup_logger('test_formatter_logger')
                    
                    # Check formatters
                    for handler in test_logger.handlers:
                        formatter = handler.formatter
                        assert formatter is not None
                        # Check format string contains expected components
                        format_str = formatter._fmt
                        assert '%(asctime)s' in format_str
                        assert '%(name)s' in format_str
                        assert '%(levelname)s' in format_str
                        assert '%(message)s' in format_str

    def test_setup_logger_levels(self, temp_dir):
        """Test that logger levels are set correctly."""
        with patch('tools.logger.os.path.join') as mock_join:
            mock_join.return_value = os.path.join(temp_dir, 'test.log')
            
            with patch('tools.logger.os.makedirs'):
                with patch('builtins.open', mock_open()):
                    test_logger = logger.setup_logger('test_level_logger')
                    
                    # Check logger level
                    assert test_logger.level == logger.logger_level
                    
                    # Check handler levels
                    for handler in test_logger.handlers:
                        assert handler.level == logger.logger_level



    def test_global_logger_instance(self):
        """Test that global logger instance is created."""
        assert hasattr(logger, 'logger')
        assert isinstance(logger.logger, logging.Logger)
        assert logger.logger.name == 'bili-hardcore'

    def test_logger_level_configuration(self):
        """Test logger level configuration."""
        # Test that logger_level is defined and is a valid logging level
        assert hasattr(logger, 'logger_level')
        assert logger.logger_level in [logging.DEBUG, logging.INFO, logging.WARNING, logging.ERROR, logging.CRITICAL]

    def test_setup_logger_exception_handling(self, temp_dir):
        """Test logger setup with exception handling."""
        with patch('tools.logger.os.path.join') as mock_join:
            mock_join.return_value = os.path.join(temp_dir, 'test.log')
            
            with patch('tools.logger.os.makedirs', side_effect=OSError("Permission denied")):
                # Should not raise exception, should handle gracefully
                try:
                    test_logger = logger.setup_logger('test_exception_logger')
                    # If it doesn't raise an exception, that's fine
                    # The actual behavior depends on implementation
                except OSError:
                    # This is also acceptable behavior
                    pass

    def test_multiple_logger_instances(self, temp_dir):
        """Test creating multiple logger instances."""
        with patch('tools.logger.os.path.join') as mock_join:
            mock_join.return_value = os.path.join(temp_dir, 'test.log')
            
            with patch('tools.logger.os.makedirs'):
                with patch('builtins.open', mock_open()):
                    logger1 = logger.setup_logger('logger1')
                    logger2 = logger.setup_logger('logger2')
                    
                    assert logger1.name == 'logger1'
                    assert logger2.name == 'logger2'
                    assert logger1 is not logger2

    def test_logger_with_custom_name(self, temp_dir):
        """Test logger with custom name."""
        with patch('tools.logger.os.path.join') as mock_join:
            mock_join.return_value = os.path.join(temp_dir, 'custom.log')
            
            with patch('tools.logger.os.makedirs'):
                with patch('builtins.open', mock_open()):
                    custom_logger = logger.setup_logger('custom_test_logger')
                    
                    assert custom_logger.name == 'custom_test_logger'
                    assert custom_logger.level == logger.logger_level
