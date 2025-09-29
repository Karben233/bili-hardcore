# Contributing to Bili-Hardcore

Thank you for your interest in contributing to Bili-Hardcore! This document provides guidelines for contributing to the project.

## Development Setup

1. **Fork and clone the repository**
2. **Install dependencies**:
   ```bash
   pip install -r requirements.txt
   pip install -r requirements-test.txt
   ```

3. **Run tests to ensure everything works**:
   ```bash
   pytest tests/ -v
   ```

## Testing Guidelines

- **All tests must pass** before submitting a PR
- **Run the full test suite**: `pytest tests/ -v`
- **Check test coverage**: `pytest tests/ --cov=bili-hardcore --cov-report=term-missing`
- **70 tests are currently passing** - maintain this standard

## Code Quality

- Follow existing code style and patterns
- Add tests for new functionality
- Update documentation as needed
- Ensure no log files are created during testing

## Submitting Changes

1. **Create a feature branch** from `main`
2. **Make your changes** with appropriate tests
3. **Run the test suite** to ensure everything passes
4. **Create a Pull Request** with a clear description

## Test Structure

The test suite includes:
- **Unit Tests**: Individual module testing
- **Integration Tests**: Module interaction testing
- **Mock Tests**: External dependency isolation

All tests are designed to run without external dependencies or side effects.
