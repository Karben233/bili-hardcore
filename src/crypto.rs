use hmac::{Hmac, Mac};
use md5::{Digest, Md5};
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<sha2::Sha256>;

const APPKEY: &str = "783bbb7264451d82";
const APPSEC: &str = "2653583c8873dea268ab9386918b1d65";

/// 模拟 Python urllib.parse.urlencode 的行为
/// 使用 quote_plus 风格编码（空格变为 +），但我们的参数不含空格
fn urlencode_params(params: &[(String, String)]) -> String {
    params
        .iter()
        .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
        .collect::<Vec<_>>()
        .join("&")
}

/// B站 API 参数签名（对齐 Python 版本的 appsign 函数）
/// 1. 添加 ts 时间戳 + appkey
/// 2. 按 key 排序
/// 3. urlencode 拼接
/// 4. MD5(query + appsec) 生成 sign
pub fn appsign(params: &mut Vec<(String, String)>) {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    params.push(("ts".into(), ts.to_string()));
    params.push(("appkey".into(), APPKEY.into()));

    params.sort_by(|a, b| a.0.cmp(&b.0));

    let query = urlencode_params(params);

    let mut hasher = Md5::new();
    hasher.update(query.as_bytes());
    hasher.update(APPSEC.as_bytes());
    let sign = format!("{:x}", hasher.finalize());

    params.push(("sign".into(), sign));
}

/// HMAC-SHA256 签名（对齐 Python 版本的 hmac_sha256 函数）
pub fn hmac_sha256(key: &str, message: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(key.as_bytes()).expect("HMAC key length valid");
    mac.update(message.as_bytes());
    let code_bytes = mac.finalize().into_bytes();
    format!("{:x}", code_bytes)
}

/// 生成 B站 web ticket 的签名参数
pub fn gen_ticket_params() -> Vec<(String, String)> {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let hexsign = hmac_sha256("XgwSnGZ1p", &format!("ts{}", ts));
    vec![
        ("key_id".into(), "ec02".into()),
        ("hexsign".into(), hexsign),
        ("context[ts]".into(), ts.to_string()),
        ("csrf".into(), String::new()),
    ]
}
