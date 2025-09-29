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

## Code Quality | 代码质量

### Standards | 标准

**English:**
- Follow existing code style and patterns
- Add tests for new functionality
- Update documentation as needed
- Ensure no log files are created during testing
- Fix any security warnings (use `bandit` for scanning)
- Keep commits focused and well-documented

**中文:**
- 遵循现有的代码风格和模式
- 为新功能添加测试
- 根据需要更新文档
- 确保测试期间不创建日志文件
- 修复任何安全警告（使用 `bandit` 进行扫描）
- 保持提交内容专注且文档完善

### Testing Environment | 测试环境

**English:**
The project includes a testing environment detection system:
- Tests automatically detect when running in test mode
- Interactive prompts are disabled during testing
- Log files are not created during test runs
- External API calls are mocked

**中文:**
项目包含测试环境检测系统：
- 测试会自动检测是否在测试模式下运行
- 测试期间禁用交互式提示
- 测试运行期间不创建日志文件
- 外部 API 调用被模拟

## Submitting Changes | 提交更改

### Process | 流程

**English:**
1. **Create an Issue first** (recommended) to discuss your changes
2. **Create a feature branch** from `main`:
   ```bash
   git checkout -b feature/your-feature-name
   ```
3. **Make your changes** with appropriate tests
4. **Run the test suite** to ensure everything passes
5. **Run code quality checks**:
   ```bash
   python run_tests.py --type lint
   python run_tests.py --type format-check
   ```
6. **Create a Pull Request** with a clear description

**中文:**
1. **首先创建 Issue**（推荐）来讨论你的更改
2. **从 `main` 创建功能分支**：
   ```bash
   git checkout -b feature/your-feature-name
   ```
3. **进行更改**并添加适当的测试
4. **运行测试套件**确保所有测试通过
5. **运行代码质量检查**：
   ```bash
   python run_tests.py --type lint
   python run_tests.py --type format-check
   ```
6. **创建 Pull Request** 并提供清晰的描述

### PR Guidelines | PR 指南

**English:**
- Use clear, descriptive titles
- Provide detailed descriptions of changes
- Include test results and verification
- Reference related issues if applicable
- Be responsive to feedback and questions

**中文:**
- 使用清晰、描述性的标题
- 提供更改的详细描述
- 包含测试结果和验证信息
- 如果适用，引用相关 Issue
- 对反馈和问题及时回应

## Available Test Commands | 可用的测试命令

**English:**
```bash
# Run all tests
pytest tests/ -v

# Run specific test modules
pytest tests/test_config.py -v
pytest tests/test_client/ -v

# Generate coverage report
pytest tests/ --cov=bili-hardcore --cov-report=term-missing

# Run different test types
python run_tests.py --type unit
python run_tests.py --type integration
python run_tests.py --type all

# Code quality checks
python run_tests.py --type lint
python run_tests.py --type format-check
python run_tests.py --type security
```

**中文:**
```bash
# 运行所有测试
pytest tests/ -v

# 运行特定测试模块
pytest tests/test_config.py -v
pytest tests/test_client/ -v

# 生成覆盖率报告
pytest tests/ --cov=bili-hardcore --cov-report=term-missing

# 运行不同类型的测试
python run_tests.py --type unit
python run_tests.py --type integration
python run_tests.py --type all

# 代码质量检查
python run_tests.py --type lint
python run_tests.py --type format-check
python run_tests.py --type security
```

## Communication | 沟通

**English:**
- **Language**: Use Chinese for communication with maintainers (they are native Chinese speakers)
- **Issues**: Create issues in Chinese to discuss features or report bugs
- **Pull Requests**: Use Chinese for PR titles and descriptions
- **Comments**: Respond to feedback in Chinese when possible

**中文:**
- **语言**：与维护者沟通时使用中文（他们是中文母语者）
- **Issue**：用中文创建 Issue 来讨论功能或报告错误
- **Pull Request**：PR 标题和描述使用中文
- **评论**：尽可能用中文回复反馈

## Getting Help | 获取帮助

**English:**
- Create an issue for questions or discussions
- Check existing issues and PRs for similar topics
- Be patient - maintainers may be busy with other commitments

**中文:**
- 为问题或讨论创建 Issue
- 检查现有的 Issue 和 PR 以寻找类似主题
- 保持耐心 - 维护者可能忙于其他事务

---

Thank you for contributing to Bili-Hardcore! 🚀

感谢您为 Bili-Hardcore 做出贡献！🚀