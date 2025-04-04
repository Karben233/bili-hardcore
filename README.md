# Bili-Hardcore

一个基于 Python 开发的B站自动答题工具，利用 Gemini API 实现智能答题功能。

## 安装说明

1. 克隆项目到本地

```bash
git clone [项目地址]
cd bili-hardcore
```

2. 安装依赖

```bash
pip install -r requirements.txt
```

3. 配置文件

在 `config/config.py` 中配置以下信息：
- `API_KEY_GEMINI` 填写自己的GEMINI API KEY

## 使用方法

1. 运行主程序

```bash
python bili-hardcore/main.py
```

2. 扫描二维码登录
3. 选择要进行答题的分类
4. 查看并输入图形验证码
5. 程序会自动开始答题流程

## 注意事项

- 使用前请确保已配置正确的 Gemini API Key
- 请合理使用，遵守B站相关规则