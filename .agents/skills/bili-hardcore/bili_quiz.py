#!/usr/bin/env python3
"""bili-hardcore skill: B 站硬核会员答题 API CLI.

移植自原 Rust 项目 src/crypto.rs、src/api/client.rs、src/config.rs（auth 部分）。
仅依赖 Python 标准库。供 opencode agent 通过 bash 调用。

子命令:
  status                          检查本地登录态（含 7 天过期判断）
  ticket                          获取 web ticket（HMAC-SHA256 签名）
  qrcode                          获取 TV 登录二维码 url + auth_code
  poll <auth_code>                轮询二维码登录状态，成功则落地 auth.json
  level                           查询账号等级（需 6 级才能答题）
  category                        获取分区分类列表
  captcha                         获取验证码 token 并下载图片到本地
  captcha-submit <code> <token> <ids>   提交验证码
  question                        获取一道题（题目 + 选项 text/hash）
  submit <id> <hash> <text>       提交答案并返回是否正确 + 累计得分
  result                          查询最终分类得分
  logout                          删除本地 auth.json

所有子命令以 JSON 输出到 stdout（便于 agent 解析）；诊断信息走 stderr。
"""

from __future__ import annotations

import argparse
import hashlib
import hmac
import json
import sys
import time
import urllib.parse
import urllib.request
from pathlib import Path
from typing import Any

# --- 常量（移植自 crypto.rs / api/client.rs）---------------------------

APPKEY = "783bbb7264451d82"
APPSEC = "2653583c8873dea268ab9386918b1d65"
TICKET_HMAC_KEY = "XgwSnGZ1p"

BASE_API = "https://api.bilibili.com"
CONFIG_DIR = Path.home() / ".bili-hardcore"
AUTH_PATH = CONFIG_DIR / "auth.json"
CAPTCHA_IMG_PATH = CONFIG_DIR / "captcha.jpg"

# 模拟客户端 HTTP 头（client.rs:20-39）
APP_HEADERS = {
    "User-Agent": "Mozilla/5.0 BiliDroid/1.12.0 (bbcallen@gmail.com)",
    "Content-Type": "application/x-www-form-urlencoded",
    "Accept": "application/json",
    "Accept-Language": "zh-CN,zh;q=0.9,en;q=0.8",
    "x-bili-metadata-legal-region": "CN",
    "x-bili-aurora-eid": "",
    "x-bili-aurora-zone": "",
}

BROWSER_UA = (
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 "
    "(KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 Edg/120.0.0.0"
)

AUTH_MAX_AGE_SECONDS = 7 * 24 * 3600  # 7 天，对齐 config.rs:108


# --- 签名（移植自 crypto.rs）-------------------------------------------


def appsign(params: list[tuple[str, str]]) -> None:
    """B 站 API appsign：追加 ts+appkey → 排序 → urlencode → MD5(query+appsec)。

    与 Rust 版逐字节一致：sign 不参与 MD5（在 urlencode 之后才 append）。
    """
    ts = str(int(time.time()))
    params.append(("ts", ts))
    params.append(("appkey", APPKEY))
    params.sort(key=lambda kv: kv[0])
    query = "&".join(
        f"{urllib.parse.quote(k, safe='')}={urllib.parse.quote(v, safe='')}"
        for k, v in params
    )
    sign = hashlib.md5((query + APPSEC).encode("utf-8")).hexdigest()
    params.append(("sign", sign))


def gen_ticket_params() -> list[tuple[str, str]]:
    """生成 web ticket 签名参数（crypto.rs:54）。

    HMAC-SHA256(key=XgwSnGZ1p, msg="ts"+ts) → hexsign。
    """
    ts = str(int(time.time()))
    hexsign = hmac.new(
        TICKET_HMAC_KEY.encode("utf-8"),
        f"ts{ts}".encode("utf-8"),
        hashlib.sha256,
    ).hexdigest()
    return [
        ("key_id", "ec02"),
        ("hexsign", hexsign),
        ("context[ts]", ts),
        ("csrf", ""),
    ]


# --- 认证持久化（移植自 config.rs auth 部分）---------------------------


def ensure_config_dir() -> None:
    CONFIG_DIR.mkdir(parents=True, exist_ok=True)


def load_auth() -> dict[str, str] | None:
    """加载本地登录态，超过 7 天视为过期返回 None。"""
    if not AUTH_PATH.exists():
        return None
    try:
        mtime = AUTH_PATH.stat().st_mtime
        if time.time() - mtime > AUTH_MAX_AGE_SECONDS:
            return None
        data = json.loads(AUTH_PATH.read_text(encoding="utf-8"))
        if not data.get("access_token"):
            return None
        return data
    except (OSError, json.JSONDecodeError):
        return None


def save_auth(auth: dict[str, str]) -> None:
    ensure_config_dir()
    AUTH_PATH.write_text(
        json.dumps(auth, ensure_ascii=False, indent=2), encoding="utf-8"
    )


def delete_auth() -> None:
    if AUTH_PATH.exists():
        AUTH_PATH.unlink()


# --- HTTP（移植自 api/client.rs）---------------------------------------


def build_common_params(auth: dict | None) -> list[tuple[str, str]]:
    """common_params（client.rs:105）。"""
    a = auth or {}
    return [
        ("access_key", a.get("access_token", "")),
        ("csrf", a.get("csrf", "")),
        ("disable_rcmd", "0"),
        ("mobi_app", "android"),
        ("platform", "android"),
        ("statistics", '{"appId":1,"platform":3,"version":"8.40.0","abtest":""}'),
    ]


def build_common_params_with_location(auth: dict | None) -> list[tuple[str, str]]:
    """common_params_with_location（client.rs:119）。"""
    params = build_common_params(auth)
    params.append(("web_location", "333.790"))
    return params


def _request(
    method: str,
    url: str,
    *,
    query: list[tuple[str, str]] | None = None,
    form: list[tuple[str, str]] | None = None,
    auth: dict | None = None,
    browser_ua: bool = False,
    extra_headers: dict[str, str] | None = None,
) -> dict[str, Any]:
    """底层 HTTP 请求，返回解析后的 JSON。失败抛异常。

    注意：signed_get/signed_post 会在调用 appsign 之前传入完整 params，
    本函数不负责签名。ticket 端点走 query 不签名；其它端点走 form 签名后提交。
    """
    headers = dict(APP_HEADERS)
    if browser_ua:
        headers["User-Agent"] = BROWSER_UA
    if auth:
        if auth.get("mid"):
            headers["x-bili-mid"] = auth["mid"]
        if auth.get("cookie"):
            headers["cookie"] = auth["cookie"]
        if auth.get("_ticket"):
            headers["x-bili-ticket"] = auth["_ticket"]
    if extra_headers:
        headers.update(extra_headers)

    final_url = url
    data: bytes | None = None
    if query:
        qs = urllib.parse.urlencode(query)
        final_url = f"{url}?{qs}"
    if form:
        data = urllib.parse.urlencode(form).encode("utf-8")

    req = urllib.request.Request(final_url, data=data, headers=headers, method=method)
    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            body = resp.read().decode("utf-8", errors="replace")
    except urllib.error.HTTPError as e:
        body = e.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"HTTP {e.code} {e.reason}: {body[:500]}") from None
    except urllib.error.URLError as e:
        raise RuntimeError(f"网络请求失败: {e}") from None

    try:
        return json.loads(body)
    except json.JSONDecodeError:
        raise RuntimeError(f"JSON 解析失败: {body[:500]}") from None


def signed_get(
    url: str, params: list[tuple[str, str]], auth: dict | None
) -> dict[str, Any]:
    """signed_get（client.rs:125）：appsign 后 GET。"""
    appsign(params)
    return _request("GET", url, query=params, auth=auth)


def signed_post(
    url: str, params: list[tuple[str, str]], auth: dict | None
) -> dict[str, Any]:
    """signed_post（client.rs:149）：appsign 后 POST form。"""
    appsign(params)
    return _request("POST", url, form=params, auth=auth)


# --- 子命令实现 --------------------------------------------------------


def cmd_status(_args) -> dict[str, Any]:
    auth = load_auth()
    if not auth:
        return {"logged_in": False}
    return {
        "logged_in": True,
        "mid": auth.get("mid", ""),
        "has_csrf": bool(auth.get("csrf")),
        "auth_path": str(AUTH_PATH),
    }


def cmd_ticket(_args) -> dict[str, Any]:
    """fetch_ticket（client.rs:76）：浏览器 UA，query 带 gen_ticket_params。"""
    params = gen_ticket_params()
    url = (
        "https://api.bilibili.com/bapis/bilibili.api.ticket.v1.Ticket/GenWebTicket"
    )
    resp = _request("POST", url, query=params, browser_ua=True)
    ticket = resp.get("data", {}).get("ticket")
    if not ticket:
        return {"ok": False, "raw": resp}
    return {"ok": True, "ticket": ticket}


def _pre_auth() -> dict[str, str]:
    """登录前上下文：仅带 web ticket（无 access_key）。

    对齐 Rust 版 spawn_login：fetch_ticket → set_ticket → qrcode_get/poll。
    """
    try:
        t = cmd_ticket(None)
        if t.get("ok"):
            return {"_ticket": t["ticket"]}
    except RuntimeError:
        pass
    return {}


def cmd_qrcode(_args) -> dict[str, Any]:
    """qrcode_get（client.rs:175）：登录前需先取 ticket 带上。"""
    resp = signed_post(
        "https://passport.bilibili.com/x/passport-tv-login/qrcode/auth_code",
        [("local_id", "0")],
        auth=_pre_auth(),
    )
    if resp.get("code") != 0:
        return {"ok": False, "raw": resp}
    data = resp.get("data", {})
    return {
        "ok": True,
        "url": data.get("url", ""),
        "auth_code": data.get("auth_code", ""),
    }


def cmd_poll(args) -> dict[str, Any]:
    """qrcode_poll（client.rs:189）：成功落地 auth.json。登录前需带 ticket。"""
    resp = signed_post(
        "https://passport.bilibili.com/x/passport-tv-login/qrcode/poll",
        [("auth_code", args.auth_code), ("local_id", "0")],
        auth=_pre_auth(),
    )
    if resp.get("code") != 0:
        return {"ok": False, "pending": True, "code": resp.get("code"), "raw": resp}
    data = resp.get("data", {})
    access_token = data.get("access_token", "")
    mid = str(data.get("mid", ""))
    cookies = data.get("cookie_info", {}).get("cookies", [])
    csrf = ""
    parts: list[str] = []
    for c in cookies:
        n = c.get("name", "")
        v = c.get("value", "")
        if n:
            parts.append(f"{n}={v}")
            if n == "bili_jct":
                csrf = v
    auth = {
        "access_token": access_token,
        "csrf": csrf,
        "mid": mid,
        "cookie": "; ".join(parts),
    }
    save_auth(auth)
    return {"ok": True, "mid": mid}


def _auth_with_ticket() -> dict | None:
    """加载登录态，并尝试附上 ticket（level 等端点需要）。"""
    auth = load_auth()
    if not auth:
        return None
    try:
        t = cmd_ticket(None)
        if t.get("ok"):
            auth["_ticket"] = t["ticket"]
    except RuntimeError:
        pass
    return auth


def cmd_level(_args) -> dict[str, Any]:
    """get_account_info（client.rs:202）。"""
    auth = _auth_with_ticket()
    if not auth:
        return {"ok": False, "error": "未登录，请先扫码登录"}
    resp = signed_get(
        "https://app.bilibili.com/x/v2/account/myinfo",
        [("access_key", auth.get("access_token", ""))],
        auth=auth,
    )
    if resp.get("code") != 0:
        return {"ok": False, "raw": resp}
    data = resp.get("data", {})
    return {
        "ok": True,
        "level": data.get("level", 0),
        "mid": data.get("mid"),
        "name": data.get("name", ""),
    }


def cmd_category(_args) -> dict[str, Any]:
    """category_get（client.rs:221）。"""
    auth = _auth_with_ticket()
    if not auth:
        return {"ok": False, "error": "未登录"}
    resp = signed_get(
        f"{BASE_API}/x/senior/v1/category",
        build_common_params_with_location(auth),
        auth=auth,
    )
    if resp.get("code") != 0:
        return {"ok": False, "code": resp.get("code"), "raw": resp}
    cats = []
    for c in resp.get("data", {}).get("categories", []) or []:
        cats.append({"id": c.get("id"), "name": c.get("name", "")})
    return {"ok": True, "categories": cats}


def cmd_captcha(_args) -> dict[str, Any]:
    """captcha_get（client.rs:243）：下载图片到本地供 agent 识别。"""
    auth = _auth_with_ticket()
    if not auth:
        return {"ok": False, "error": "未登录"}
    resp = signed_get(
        f"{BASE_API}/x/senior/v1/captcha",
        build_common_params_with_location(auth),
        auth=auth,
    )
    if resp.get("code") != 0:
        return {"ok": False, "raw": resp}
    data = resp.get("data", {})
    url = data.get("url", "")
    token = data.get("token", "")
    img_path = None
    if url:
        try:
            ensure_config_dir()
            req = urllib.request.Request(url, headers={"User-Agent": BROWSER_UA})
            with urllib.request.urlopen(req, timeout=15) as r:
                CAPTCHA_IMG_PATH.write_bytes(r.read())
            img_path = str(CAPTCHA_IMG_PATH)
        except Exception as e:
            print(f"[warn] 下载验证码图片失败: {e}", file=sys.stderr)
    return {"ok": True, "url": url, "token": token, "image_path": img_path}


def cmd_captcha_submit(args) -> dict[str, Any]:
    """captcha_submit（client.rs:260）。成功后再取一题返回。"""
    auth = _auth_with_ticket()
    if not auth:
        return {"ok": False, "error": "未登录"}
    params = [
        ("access_key", auth.get("access_token", "")),
        ("csrf", auth.get("csrf", "")),
        ("bili_code", args.code),
        ("bili_token", args.token),
        ("disable_rcmd", "0"),
        ("gt_challenge", ""),
        ("gt_seccode", ""),
        ("gt_validate", ""),
        ("ids", args.ids),
        ("mobi_app", "android"),
        ("platform", "android"),
        ("statistics", '{"appId":1,"platform":3,"version":"8.40.0","abtest":""}'),
        ("type", "bilibili"),
    ]
    resp = signed_post(f"{BASE_API}/x/senior/v1/captcha/submit", params, auth=auth)
    if resp.get("code") != 0:
        return {"ok": False, "raw": resp}
    # 验证通过，取下一题
    return _fetch_question(auth)


def _fetch_question(auth: dict) -> dict[str, Any]:
    """question_get（client.rs:290）：返回题目或提示需要验证码。"""
    resp = signed_get(
        f"{BASE_API}/x/senior/v1/question",
        build_common_params_with_location(auth),
        auth=auth,
    )
    if resp.get("code") == 0:
        d = resp.get("data", {})
        answers = []
        for a in d.get("answers", []) or []:
            answers.append({"text": a.get("ans_text", ""), "hash": a.get("ans_hash", "")})
        return {
            "ok": True,
            "need_captcha": False,
            "id": d.get("id", 0),
            "question_num": d.get("question_num", 0),
            "question": d.get("question", ""),
            "answers": answers,
        }
    # 非 0 通常表示需要验证码或达到答题限制
    return {
        "ok": False,
        "need_captcha": True,
        "code": resp.get("code"),
        "raw": resp,
    }


def cmd_question(_args) -> dict[str, Any]:
    auth = _auth_with_ticket()
    if not auth:
        return {"ok": False, "error": "未登录"}
    return _fetch_question(auth)


def cmd_submit(args) -> dict[str, Any]:
    """question_submit + question_result（client.rs:298 / 312）。"""
    auth = _auth_with_ticket()
    if not auth:
        return {"ok": False, "error": "未登录"}
    params = build_common_params_with_location(auth)
    params.append(("id", str(args.id)))
    params.append(("ans_hash", args.hash))
    params.append(("ans_text", args.text))
    resp = signed_post(f"{BASE_API}/x/senior/v1/answer/submit", params, auth=auth)
    if resp.get("code") != 0:
        if resp.get("code") == 41103:
            return {"ok": False, "error": "请检查是否已经是硬核会员", "raw": resp}
        return {"ok": False, "raw": resp}
    # 取结果：score 即累计答对数
    r = signed_get(
        f"{BASE_API}/x/senior/v1/answer/result",
        build_common_params_with_location(auth),
        auth=auth,
    )
    if r.get("code") != 0:
        return {"ok": True, "correct": None, "score": 0, "raw": r}
    data = r.get("data", {})
    return {"ok": True, "score": data.get("score", 0)}


def cmd_result(_args) -> dict[str, Any]:
    """question_result（client.rs:312）：最终分类得分。"""
    auth = _auth_with_ticket()
    if not auth:
        return {"ok": False, "error": "未登录"}
    resp = signed_get(
        f"{BASE_API}/x/senior/v1/answer/result",
        build_common_params_with_location(auth),
        auth=auth,
    )
    if resp.get("code") != 0:
        return {"ok": False, "raw": resp}
    data = resp.get("data", {})
    scores = []
    for s in data.get("scores", []) or []:
        scores.append(
            {
                "category": s.get("category", ""),
                "score": s.get("score", 0),
                "total": s.get("total", 0),
            }
        )
    return {"ok": True, "score": data.get("score", 0), "scores": scores}


def cmd_logout(_args) -> dict[str, Any]:
    delete_auth()
    return {"ok": True}


# --- CLI ---------------------------------------------------------------


def main() -> int:
    parser = argparse.ArgumentParser(
        prog="bili_quiz.py",
        description="B 站硬核会员答题 API CLI（bili-hardcore skill）",
    )
    sub = parser.add_subparsers(dest="cmd", required=True)

    sub.add_parser("status", help="检查本地登录态")
    sub.add_parser("ticket", help="获取 web ticket")
    sub.add_parser("qrcode", help="获取登录二维码")
    p_poll = sub.add_parser("poll", help="轮询二维码登录")
    p_poll.add_argument("auth_code")
    sub.add_parser("level", help="查询账号等级")
    sub.add_parser("category", help="获取分区分类")
    sub.add_parser("captcha", help="获取验证码")
    p_cs = sub.add_parser("captcha-submit", help="提交验证码")
    p_cs.add_argument("code")
    p_cs.add_argument("token")
    p_cs.add_argument("ids")
    sub.add_parser("question", help="获取一道题")
    p_sub = sub.add_parser("submit", help="提交答案")
    p_sub.add_argument("id", type=int)
    p_sub.add_argument("hash")
    p_sub.add_argument("text")
    sub.add_parser("result", help="查询最终得分")
    sub.add_parser("logout", help="删除本地登录态")

    args = parser.parse_args()

    dispatch = {
        "status": cmd_status,
        "ticket": cmd_ticket,
        "qrcode": cmd_qrcode,
        "poll": cmd_poll,
        "level": cmd_level,
        "category": cmd_category,
        "captcha": cmd_captcha,
        "captcha-submit": cmd_captcha_submit,
        "question": cmd_question,
        "submit": cmd_submit,
        "result": cmd_result,
        "logout": cmd_logout,
    }
    try:
        result = dispatch[args.cmd](args)
        print(json.dumps(result, ensure_ascii=False, indent=2))
        return 0 if (isinstance(result, dict) and result.get("ok", True)) else 1
    except Exception as e:
        print(json.dumps({"ok": False, "error": str(e)}, ensure_ascii=False))
        return 1


if __name__ == "__main__":
    sys.exit(main())
