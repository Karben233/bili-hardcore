# Bili-Hardcore

B 站硬核会员自动答题工具，利用 Gemini API 实现智能答题功能。

## 使用说明

### 方式一：从 release 下载 exe 文件
1. 下载 exe 文件
2. 双击 exe 运行或在命令行中执行 `.\bili-hardcore.exe`

### 方式二：从源码运行
1. 克隆项目到本地

```bash
git clone [项目地址]
cd bili-hardcore
```

2. 安装依赖

```bash
pip install -r requirements.txt
```
3. 运行主程序

```bash
python bili-hardcore/main.py
```
## 使用流程
1. 输入自己的 Gemini API Key
2. 扫描二维码登录
3. 输入要进行答题的分类
4. 查看并输入图形验证码
5. 程序会自动开始答题流程

## 注意事项
- 使用前请确保已配置正确的 Gemini API Key，没有 Gemini API Key 的可以自己去免费申请一个
- 程序仅调用 B 站接口和 Gemini API，不会上传任何个人信息
- 首次输入 API Key 和登录后，会将信息保存到 `~/.bili-hardcore`，下次运行时会自动读取。如遇到奇怪问题，请先清空此文件夹重新运行软件
- 注意需要切换至 Gemini 允许的地区运行，否则会被 Gemini API 拦截
- 请合理使用，遵守 B 站相关规则