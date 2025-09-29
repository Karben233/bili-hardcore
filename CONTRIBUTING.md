# Contributing to Bili-Hardcore | 为 Bili-Hardcore 做贡献

Thank you for your interest in contributing to Bili-Hardcore! This document provides guidelines for contributing to the project.

感谢您对 Bili-Hardcore 项目的贡献兴趣！本文档提供了项目贡献指南。

## Development Setup | 开发环境设置

### 1. Fork and Clone | Fork 和克隆

**English:**
1. Fork the repository on GitHub
2. Clone your fork locally:
   ```bash
   git clone https://github.com/YOUR_USERNAME/bili-hardcore.git
   cd bili-hardcore
   ```

**中文:**
1. 在 GitHub 上 Fork 这个仓库
2. 本地克隆你的 Fork：
   ```bash
   git clone https://github.com/YOUR_USERNAME/bili-hardcore.git
   cd bili-hardcore
   ```

### 2. Install Dependencies | 安装依赖

**English:**
```bash
# Install core dependencies
pip install -r requirements.txt

# Install testing dependencies
pip install -r requirements-test.txt
```

**中文:**
```bash
# 安装核心依赖
pip install -r requirements.txt

# 安装测试依赖
pip install -r requirements-test.txt
```

### 3. Verify Setup | 验证设置

**English:**
```bash
# Run tests to ensure everything works
pytest tests/ -v
```

**中文:**
```bash
# 运行测试确保一切正常
pytest tests/ -v
```

## Testing Guidelines | 测试指南

### Test Requirements | 测试要求

**English:**
- **All tests must pass** before submitting a PR
- **Run the full test suite**: `pytest tests/ -v`
- **Check test coverage**: `pytest tests/ --cov=bili-hardcore --cov-report=term-missing`
- **70 tests are currently passing** - maintain this standard
- **No log files should be created during testing**

**中文:**
- **提交 PR 前所有测试必须通过**
- **运行完整测试套件**：`pytest tests/ -v`
- **检查测试覆盖率**：`pytest tests/ --cov=bili-hardcore --cov-report=term-missing`
- **目前有 70 个测试通过** - 请保持这个标准
- **测试期间不应创建日志文件**

### Test Structure | 测试结构

**English:**
The test suite includes:
- **Unit Tests**: Individual module testing (config, client, tools, etc.)
- **Integration Tests**: Module interaction testing
- **Mock Tests**: External dependency isolation
- **Security Tests**: Bandit security scanning

All tests are designed to run without external dependencies or side effects.

**中文:**
测试套件包括：
- **单元测试**：单个模块测试（config、client、tools 等）
- **集成测试**：模块交互测试
- **Mock 测试**：外部依赖隔离
- **安全测试**：Bandit 安全扫描

所有测试都设计为在没有外部依赖或副作用的情况下运行。

---

Thank you for contributing to Bili-Hardcore! 🚀

感谢您为 Bili-Hardcore 做出贡献！🚀