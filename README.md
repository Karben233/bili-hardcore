# Bili-Hardcore

B 站硬核会员自动答题工具，利用 LLM 实现智能答题功能。

**支持 OpenAI 兼容格式的 API**（OpenAI、火山引擎、硅基流动、阿里云百炼等，可自定义 URL、模型名称、API Key）

不建议思考模型，思维链过长可能会导致超时请求失败

## 使用前须知
- 程序仅在本地调用 B 站 API 和 LLM API，不会上传您的登录信息和 API Key，请放心使用
- 请确保您的 B 站账号已满 6 级，根据 B 站规则，6 级用户才可以进行硬核会员试炼
- 硬核会员试炼每天有 3 次答题机会（指的是100道题全部提交，显示答题结果后，或在 B 站 APP 手动结束答题），达到限制后需要 24 小时后才能重新答题，具体时间可以前往 B 站 APP 答题页面查看
- 首次输入模型配置和登录后，会将信息保存到 `~/.bili-hardcore`，下次运行时会自动读取。如配置错误或遇到奇怪问题，运行脚本后请选择”重新开始”
- 没有 API Key 的可以自己去免费去硅基流动注册一个账号，会送 14 元免费额度，这是我的[邀请链接](https://cloud.siliconflow.cn/i/9Fur0aVC)
- 请合理使用，遵守 B 站相关规则

## 使用说明

### 方式一：从 release 下载可执行文件
#### Windows
1. 下载 `bili-hardcore-windows-*.exe`
2. 双击 exe 运行或在命令行中执行 `.\bili-hardcore-windows-*.exe`

#### Mac
1. 下载 `bili-hardcore-macos-*`
2. 命令行中执行 `chmod +x bili-hardcore-macos-* && ./bili-hardcore-macos-*`
>如果碰到"Apple could not verify xxx is free of malware that may harm your Mac"问题,可以在**系统设置 > 隐私与安全性**中点击**仍要打开**,或者命令行执行 `xattr -d com.apple.quarantine ./bili-hardcore-macos-*`

#### Ubuntu
1. 下载 `bili-hardcore-ubuntu-*`
2. 命令行中执行 `chmod +x bili-hardcore-ubuntu-* &&./bili-hardcore-ubuntu-*`

### 方式二：从源码运行
请使用 Python 3.9 及以上版本运行

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
1. 输入 API 基础 URL、模型名称和 API Key
2. 扫描二维码登录
3. 输入要进行答题的分类
4. 查看并输入图形验证码
5. 程序会自动开始答题流程

## 常见问题
1. **二维码乱码**：请尝试使用其他命令行工具运行，或手动生成二维码进行扫码
2. **答题不及格**：尝试使用历史分区答题，历史分区的准确率较高
3. **AI 卡在一个问题一直过不去，回复类似于“无法确认、我不清楚”**：换一个模型，或者去 B 站 APP 手动把卡住的题目过了，切记不要在 B 站答题页面点击左上角返回按钮退出，会结束答题
4. **获取分类失败，41099 错误**：请前往 B 站 APP 答题页面确认是否已达到答题限制

## 运行截图
![PixPin_2025-04-08_15-45-29](https://github.com/user-attachments/assets/70b3930c-c60f-43f7-8d82-c5225997ebc5)

