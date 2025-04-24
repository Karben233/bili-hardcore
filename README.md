# Bili-Hardcore

B 站硬核会员自动答题工具，利用 LLM 实现智能答题功能。

**可用的模型：**
- DeepSeek (V3)
- Gemini (gemini-2.0-flash)(为了防止风控，答题间隔5秒，速度较慢，建议使用其他模型)
- OpenAI 风格的其他 API（OpenAI、火山引擎、硅基流动等，可自定义 url、模型名称）
  
不建议使用 R1 等思考模型，思维链过长可能会导致超时请求失败

## 使用前须知
1. 请确保您的 B 站账号已满 6 级，根据 B 站规则，6 级用户才可以进行硬核会员试炼
2. 硬核会员试炼每天有 3 次答题机会（指的是100道题全部提交，显示答题结果后，或在 B 站 APP 手动结束答题），达到限制后需要 24 小时后才能重新答题

## 使用说明

### 方式一：从 release 下载可执行文件
#### Windows
1. 下载 `bili-hardcore-windows-*.exe`
2. 双击 exe 运行或在命令行中执行 `.\bili-hardcore-windows-*.exe`

#### Mac
1. 下载 `bili-hardcore-mac-*`
2. 命令行中执行 `chmod +x bili-hardcore-mac-* && ./bili-hardcore-mac-*`

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
1. 选择回答模型
2. 输入自己的 API Key
3. 扫描二维码登录
4. 输入要进行答题的分类
5. 查看并输入图形验证码
6. 程序会自动开始答题流程

## 常见问题
1. **二维码乱码**：请尝试在 Windows Terminal 中使用命令运行 exe，或手动生成二维码进行扫码
2. **答题不及格**：尝试使用历史分区答题，历史分区的准确率较高
3. **AI 卡在一个问题一直过不去，回复类似于“无法确认、我不清楚”**：去 B 站 APP 手动把卡住的题目过了，切记不要在 B 站答题页面点击左上角返回按钮退出，会结束答题
4. **DeepSeek 官方模型在答题过程中报 400 错误**：检查一下题目里是否有敏感词，如果有敏感词，需要去 B 站 APP 手动过这一题，或者换个模型回答
5. **获取分类失败，41099 错误**：请前往 B 站 APP 答题页面确认是否已达到答题限制

## Gemini 模型使用问题及解决办法
1. 答题触发 429 错误：应该是触发了 Gemini 每分钟调用限制或触发了风控，依次尝试以下操作：
    1. 可以稍等一下重新运行，会接着中断的题目继续回答
    2. 如果还不行，尝试切换节点（修改IP）
    3. 再不行就需要手动修改一下代码里的 prompt
    4. 终极解决办法：别用 Gemini 模型了，用 DeepSeek 模型
2. 开始答题直接之后软件直接退出：需要切换到大陆及香港以外的节点进行答题

## 注意事项
- 使用前请确保已配置正确的 API Key，没有 API Key 的可以自己去免费申请一个火山引擎或者 Gemini，或者 DeepSeek 充值一元
- 程序仅调用 B 站接口和 LLM API，不会上传任何个人信息
- 首次输入 API Key 和登录后，会将信息保存到 `~/.bili-hardcore`，下次运行时会自动读取。如遇到奇怪问题，请先清空此文件夹重新运行软件
- 如果使用Gemini，注意需要切换至 Gemini 允许的地区运行，否则会被 Gemini API 拦截
- 请合理使用，遵守 B 站相关规则

## 运行截图
![PixPin_2025-04-08_15-45-29](https://github.com/user-attachments/assets/70b3930c-c60f-43f7-8d82-c5225997ebc5)

