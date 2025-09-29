"""
Simple test to verify the testing framework works.
"""
import os

import pytest


def test_basic_functionality():
    """Test basic Python functionality."""
    assert 1 + 1 == 2
    assert "hello" == "hello"
    assert True is True


def test_file_structure():
    """Test that the project structure exists."""
    # Check that main directories exist
    assert os.path.exists("bili-hardcore")
    assert os.path.exists("tests")
    assert os.path.exists("requirements.txt")
    assert os.path.exists("requirements-test.txt")


def test_imports():
    """Test that we can import standard libraries."""
    import hashlib
    import json
    import time

    import requests

    # Test that imports work
    assert json is not None
    assert time is not None
    assert hashlib is not None
    assert requests is not None


@pytest.mark.unit
def test_unit_marker():
    """Test that pytest markers work."""
    assert True


@pytest.mark.integration
def test_integration_marker():
    """Test that integration markers work."""
    assert True


class TestSimpleClass:
    """Test class structure."""
    
    def test_class_method(self):
        """Test class method."""
        assert self.__class__.__name__ == "TestSimpleClass"
    
    def test_with_fixture(self, temp_dir):
        """Test using a fixture."""
        assert os.path.exists(temp_dir)
        assert os.path.isdir(temp_dir)


def test_mock_functionality():
    """Test mock functionality."""
    from unittest.mock import Mock

    # Create a mock object
    mock_obj = Mock()
    mock_obj.return_value = "test_value"
    
    # Test mock functionality
    result = mock_obj()
    assert result == "test_value"
    
    # Test mock was called
    mock_obj.assert_called_once()
