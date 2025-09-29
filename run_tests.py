#!/usr/bin/env python3
"""
Test runner script for bili-hardcore project.
Provides an easy way to run different types of tests with various configurations.
"""

import argparse
import sys
import subprocess
import os
from pathlib import Path


def run_command(cmd, description=""):
    """Run a command and return the result."""
    print(f"\n{'='*60}")
    if description:
        print(f"Running: {description}")
    print(f"Command: {' '.join(cmd)}")
    print(f"{'='*60}")
    
    try:
        result = subprocess.run(cmd, check=True, capture_output=False)
        return True
    except subprocess.CalledProcessError as e:
        print(f"\n❌ Command failed with exit code {e.returncode}")
        return False
    except FileNotFoundError:
        print(f"\n❌ Command not found: {cmd[0]}")
        return False


def check_dependencies():
    """Check if required dependencies are installed."""
    required_packages = ['pytest', 'coverage', 'flake8']
    missing_packages = []
    
    for package in required_packages:
        try:
            __import__(package.replace('-', '_'))
        except ImportError:
            missing_packages.append(package)
    
    if missing_packages:
        print(f"❌ Missing required packages: {', '.join(missing_packages)}")
        print("Please install them with: pip install -r requirements-test.txt")
        return False
    
    return True


def run_unit_tests(verbose=False, coverage=False):
    """Run unit tests."""
    cmd = ['pytest', '-m', 'unit']
    
    if verbose:
        cmd.append('-v')
    
    if coverage:
        cmd.extend(['--cov=bili-hardcore', '--cov-report=term-missing'])
    
    return run_command(cmd, "Unit Tests")


def run_integration_tests(verbose=False, coverage=False):
    """Run integration tests."""
    cmd = ['pytest', '-m', 'integration']
    
    if verbose:
        cmd.append('-v')
    
    if coverage:
        cmd.extend(['--cov=bili-hardcore', '--cov-report=term-missing'])
    
    return run_command(cmd, "Integration Tests")


def run_all_tests(verbose=False, coverage=False):
    """Run all tests."""
    cmd = ['pytest']
    
    if verbose:
        cmd.append('-v')
    
    if coverage:
        cmd.extend(['--cov=bili-hardcore', '--cov-report=html', '--cov-report=term-missing'])
    
    return run_command(cmd, "All Tests")


def run_linting():
    """Run code linting checks."""
    commands = [
        (['flake8', 'bili-hardcore/', 'tests/'], "Flake8 Linting"),
        (['bandit', '-r', 'bili-hardcore/'], "Security Check (Bandit)"),
        (['safety', 'check'], "Dependency Security Check")
    ]
    
    all_passed = True
    for cmd, description in commands:
        if not run_command(cmd, description):
            all_passed = False
    
    return all_passed


def run_formatting():
    """Run code formatting checks."""
    commands = [
        (['black', '--check', 'bili-hardcore/', 'tests/'], "Black Formatting Check"),
        (['isort', '--check-only', 'bili-hardcore/', 'tests/'], "Import Sorting Check")
    ]
    
    all_passed = True
    for cmd, description in commands:
        if not run_command(cmd, description):
            all_passed = False
    
    return all_passed


def format_code():
    """Format code with black and isort."""
    commands = [
        (['black', 'bili-hardcore/', 'tests/'], "Formatting with Black"),
        (['isort', 'bili-hardcore/', 'tests/'], "Sorting Imports with isort")
    ]
    
    all_passed = True
    for cmd, description in commands:
        if not run_command(cmd, description):
            all_passed = False
    
    return all_passed


def generate_coverage_report():
    """Generate detailed coverage report."""
    cmd = ['pytest', '--cov=bili-hardcore', '--cov-report=html', '--cov-report=xml']
    success = run_command(cmd, "Generating Coverage Report")
    
    if success:
        print("\n📊 Coverage report generated:")
        print("   - HTML report: htmlcov/index.html")
        print("   - XML report: coverage.xml")
    
    return success


def main():
    """Main function to handle command line arguments and run tests."""
    parser = argparse.ArgumentParser(description="Test runner for bili-hardcore project")
    
    parser.add_argument(
        '--type', '-t',
        choices=['unit', 'integration', 'all', 'lint', 'format-check', 'format-fix', 'coverage'],
        default='all',
        help='Type of tests to run'
    )
    
    parser.add_argument(
        '--verbose', '-v',
        action='store_true',
        help='Verbose output'
    )
    
    parser.add_argument(
        '--coverage', '-c',
        action='store_true',
        help='Include coverage report'
    )
    
    parser.add_argument(
        '--check-deps',
        action='store_true',
        help='Check if required dependencies are installed'
    )
    
    args = parser.parse_args()
    
    # Check dependencies if requested
    if args.check_deps:
        if not check_dependencies():
            sys.exit(1)
        print("✅ All required dependencies are installed")
        return
    
    # Check dependencies before running tests
    if not check_dependencies():
        sys.exit(1)
    
    # Change to project root directory
    project_root = Path(__file__).parent
    os.chdir(project_root)
    
    success = True
    
    if args.type == 'unit':
        success = run_unit_tests(args.verbose, args.coverage)
    elif args.type == 'integration':
        success = run_integration_tests(args.verbose, args.coverage)
    elif args.type == 'all':
        success = run_all_tests(args.verbose, args.coverage)
    elif args.type == 'lint':
        success = run_linting()
    elif args.type == 'format-check':
        success = run_formatting()
    elif args.type == 'format-fix':
        success = format_code()
    elif args.type == 'coverage':
        success = generate_coverage_report()
    
    # Print summary
    print(f"\n{'='*60}")
    if success:
        print("✅ All checks passed!")
    else:
        print("❌ Some checks failed!")
    print(f"{'='*60}")
    
    sys.exit(0 if success else 1)


if __name__ == '__main__':
    main()
