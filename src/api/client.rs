use crate::config::AuthData;
use crate::crypto::{appsign, gen_ticket_params};
use crate::error::AppError;
use reqwest::Client;
use reqwest::header::{ACCEPT, ACCEPT_LANGUAGE, CONTENT_TYPE, HeaderMap, HeaderValue, USER_AGENT};
use serde_json::Value;
use std::collections::HashMap;

const BASE_API: &str = "https://api.bilibili.com";

pub struct BiliClient {
    http: Client,
    pub access_token: String,
    pub csrf: String,
    pub extra_headers: HashMap<String, String>,
}

impl BiliClient {
    pub fn new() -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static("Mozilla/5.0 BiliDroid/1.12.0 (bbcallen@gmail.com)"),
        );
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/x-www-form-urlencoded"),
        );
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        headers.insert(
            ACCEPT_LANGUAGE,
            HeaderValue::from_static("zh-CN,zh;q=0.9,en;q=0.8"),
        );
        headers.insert(
            "x-bili-metadata-legal-region",
            HeaderValue::from_static("CN"),
        );
        headers.insert("x-bili-aurora-eid", HeaderValue::from_static(""));
        headers.insert("x-bili-aurora-zone", HeaderValue::from_static(""));

        let http = Client::builder()
            .default_headers(headers)
            .build()
            .expect("创建 HTTP 客户端失败");
        Self {
            http,
            access_token: String::new(),
            csrf: String::new(),
            extra_headers: HashMap::new(),
        }
    }

    pub fn set_auth(&mut self, auth: &AuthData) {
        self.access_token = auth.access_token.clone();
        self.csrf = auth.csrf.clone();
        self.extra_headers
            .insert("x-bili-mid".into(), auth.mid.clone());
        self.extra_headers
            .insert("cookie".into(), auth.cookie.clone());
    }

    pub fn set_ticket(&mut self, ticket: &str) {
        self.extra_headers
            .insert("x-bili-ticket".into(), ticket.into());
    }

    pub fn clone_for_async(&self) -> Self {
        Self {
            http: self.http.clone(),
            access_token: self.access_token.clone(),
            csrf: self.csrf.clone(),
            extra_headers: self.extra_headers.clone(),
        }
    }

    pub async fn fetch_ticket(&self) -> Result<String, AppError> {
        let params = gen_ticket_params();
        let resp = self
            .http
            .post("https://api.bilibili.com/bapis/bilibili.api.ticket.v1.Ticket/GenWebTicket")
            .header("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 Edg/120.0.0.0")
            .query(&params)
            .send()
            .await?;
        let text = resp.text().await?;
        tracing::info!("GenWebTicket => {}", truncate_str(&text, 500));
        let json: Value = serde_json::from_str(&text).map_err(|e| {
            AppError::other(format!(
                "JSON解析失败: {} | body: {}",
                e,
                truncate_str(&text, 200)
            ))
        })?;
        json["data"]["ticket"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| {
                AppError::other(format!(
                    "获取 ticket 失败: {}",
                    &text[..text.len().min(200)]
                ))
            })
    }

    fn common_params(&self) -> Vec<(String, String)> {
        vec![
            ("access_key".into(), self.access_token.clone()),
            ("csrf".into(), self.csrf.clone()),
            ("disable_rcmd".into(), "0".into()),
            ("mobi_app".into(), "android".into()),
            ("platform".into(), "android".into()),
            (
                "statistics".into(),
                r#"{"appId":1,"platform":3,"version":"8.40.0","abtest":""}"#.into(),
            ),
        ]
    }

    fn common_params_with_location(&self) -> Vec<(String, String)> {
        let mut params = self.common_params();
        params.push(("web_location".into(), "333.790".into()));
        params
    }

    async fn signed_get(
        &self,
        url: &str,
        params: Vec<(String, String)>,
    ) -> Result<Value, AppError> {
        let mut params = params;
        appsign(&mut params);

        let mut req = self.http.get(url).query(&params);
        for (k, v) in &self.extra_headers {
            req = req.header(k.as_str(), v.as_str());
        }
        let resp = req.send().await?;
        let text = resp.text().await?;
        tracing::info!("GET {} => {}", url, truncate_str(&text, 500));
        serde_json::from_str(&text).map_err(|e| {
            AppError::other(format!(
                "JSON解析失败: {} | body: {}",
                e,
                truncate_str(&text, 200)
            ))
        })
    }

    async fn signed_post(
        &self,
        url: &str,
        params: Vec<(String, String)>,
    ) -> Result<Value, AppError> {
        let mut params = params;
        appsign(&mut params);

        let mut req = self.http.post(url);
        for (k, v) in &self.extra_headers {
            req = req.header(k.as_str(), v.as_str());
        }
        let resp = req.form(&params).send().await?;
        let text = resp.text().await?;
        tracing::info!("POST {} => {}", url, truncate_str(&text, 500));
        serde_json::from_str(&text).map_err(|e| {
            AppError::other(format!(
                "JSON解析失败: {} | body: {}",
                e,
                truncate_str(&text, 200)
            ))
        })
    }

    // --- QR Code Login ---

    pub async fn qrcode_get(&self) -> Result<Value, AppError> {
        let resp = self
            .signed_post(
                "https://passport.bilibili.com/x/passport-tv-login/qrcode/auth_code",
                vec![("local_id".into(), "0".into())],
            )
            .await?;
        if resp["code"].as_i64() == Some(0) {
            Ok(resp["data"].clone())
        } else {
            Err(AppError::other(format!("获取二维码失败: {}", resp)))
        }
    }

    pub async fn qrcode_poll(&self, auth_code: &str) -> Result<Value, AppError> {
        self.signed_post(
            "https://passport.bilibili.com/x/passport-tv-login/qrcode/poll",
            vec![
                ("auth_code".into(), auth_code.into()),
                ("local_id".into(), "0".into()),
            ],
        )
        .await
    }

    // --- User Info ---

    pub async fn get_account_info(&self) -> Result<Value, AppError> {
        let resp = self
            .signed_get(
                "https://app.bilibili.com/x/v2/account/myinfo",
                vec![("access_key".into(), self.access_token.clone())],
            )
            .await?;
        if resp["code"].as_i64() == Some(0) {
            Ok(resp["data"].clone())
        } else {
            Err(AppError::Api {
                code: resp["code"].as_i64().unwrap_or(-1),
                message: format!("获取用户信息失败: {}", resp),
            })
        }
    }

    // --- Senior Quiz ---

    pub async fn category_get(&self) -> Result<Value, AppError> {
        let resp = self
            .signed_get(
                &format!("{}/x/senior/v1/category", BASE_API),
                self.common_params_with_location(),
            )
            .await?;
        if resp["code"].as_i64() == Some(0) {
            Ok(resp["data"].clone())
        } else if resp["code"].as_i64() == Some(41099) {
            Err(AppError::Api {
                code: 41099,
                message: format!("获取分类失败，可能已达到答题限制(每日3次): {}", resp),
            })
        } else {
            Err(AppError::Api {
                code: resp["code"].as_i64().unwrap_or(-1),
                message: format!("获取分类失败: {}", resp),
            })
        }
    }

    pub async fn captcha_get(&self) -> Result<Value, AppError> {
        let resp = self
            .signed_get(
                &format!("{}/x/senior/v1/captcha", BASE_API),
                self.common_params_with_location(),
            )
            .await?;
        if resp["code"].as_i64() == Some(0) {
            Ok(resp["data"].clone())
        } else {
            Err(AppError::Api {
                code: resp["code"].as_i64().unwrap_or(-1),
                message: format!("获取验证码失败: {}", resp),
            })
        }
    }

    pub async fn captcha_submit(
        &self,
        code: &str,
        captcha_token: &str,
        ids: &str,
    ) -> Result<bool, AppError> {
        let params = vec![
            ("access_key".into(), self.access_token.clone()),
            ("csrf".into(), self.csrf.clone()),
            ("bili_code".into(), code.into()),
            ("bili_token".into(), captcha_token.into()),
            ("disable_rcmd".into(), "0".into()),
            ("gt_challenge".into(), String::new()),
            ("gt_seccode".into(), String::new()),
            ("gt_validate".into(), String::new()),
            ("ids".into(), ids.into()),
            ("mobi_app".into(), "android".into()),
            ("platform".into(), "android".into()),
            (
                "statistics".into(),
                r#"{"appId":1,"platform":3,"version":"8.40.0","abtest":""}"#.into(),
            ),
            ("type".into(), "bilibili".into()),
        ];
        let resp = self
            .signed_post(&format!("{}/x/senior/v1/captcha/submit", BASE_API), params)
            .await?;
        Ok(resp["code"].as_i64() == Some(0))
    }

    pub async fn question_get(&self) -> Result<Value, AppError> {
        self.signed_get(
            &format!("{}/x/senior/v1/question", BASE_API),
            self.common_params_with_location(),
        )
        .await
    }

    pub async fn question_submit(
        &self,
        id: i64,
        ans_hash: &str,
        ans_text: &str,
    ) -> Result<Value, AppError> {
        let mut params = self.common_params_with_location();
        params.push(("id".into(), id.to_string()));
        params.push(("ans_hash".into(), ans_hash.into()));
        params.push(("ans_text".into(), ans_text.into()));
        self.signed_post(&format!("{}/x/senior/v1/answer/submit", BASE_API), params)
            .await
    }

    pub async fn question_result(&self) -> Result<Value, AppError> {
        let resp = self
            .signed_get(
                &format!("{}/x/senior/v1/answer/result", BASE_API),
                self.common_params_with_location(),
            )
            .await?;
        if resp["code"].as_i64() == Some(0) {
            Ok(resp["data"].clone())
        } else {
            Err(AppError::Api {
                code: resp["code"].as_i64().unwrap_or(-1),
                message: format!("获取答题结果失败: {}", resp),
            })
        }
    }
}

fn truncate_str(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        let mut end = max;
        while end > 0 && !s.is_char_boundary(end) {
            end -= 1;
        }
        &s[..end]
    }
}
