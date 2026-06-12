use crate::api::BiliClient;
use crate::config::{self, AuthData, OpenAiConfig};
use crate::llm::LlmChunk;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

// --- Pages ---

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Page {
    Home,
    Config,
    Quiz,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HomeSelection {
    StartQuiz,
    Config,
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConfigFocus {
    BaseUrl,
    Model,
    ApiKey,
    ThinkingToggle,
    FastModeToggle,
    SaveBtn,
    ResetBtn,
}

#[derive(Debug, Clone)]
pub enum QuizPhase {
    NotConfigured,
    LoggingIn,
    WaitingScan {
        url: String,
        qr: String,
        auth_code: String,
        countdown: u32,
    },
    LoginTimeout {
        retry: bool,
    },
    CheckingLevel,
    FetchingQuestion,
    WaitingLlm,
    Submitting,
    ShowingResult {
        correct: bool,
        countdown: u8,
    },
    Captcha(CaptchaState),
    Finished {
        score: i64,
        scores: Vec<ScoreItem>,
    },
    Error(String),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CaptchaFocus {
    Categories,
    Input,
    Submit,
}

#[derive(Debug, Clone)]
pub struct CaptchaState {
    pub categories: Vec<CategoryItem>,
    pub cat_focus: usize,
    pub captcha_url: String,
    pub captcha_token: String,
    pub input: String,
    pub focus: CaptchaFocus,
    pub error: String,
}

#[derive(Debug, Clone)]
pub struct CategoryItem {
    pub id: i64,
    pub name: String,
    pub selected: bool,
}

#[derive(Debug, Clone)]
pub struct ScoreItem {
    pub category: String,
    pub score: i64,
    pub total: i64,
}

// --- Async events from background tasks ---

#[derive(Debug)]
pub enum AppEvent {
    TicketReady(String),
    QrReady {
        url: String,
        qr: String,
        auth_code: String,
    },
    LoginOk(AuthData),
    LoginPending,
    LevelOk,
    LevelFail(i64),
    QuestionReady {
        num: u32,
        question: String,
        answers: Vec<AnswerItem>,
        id: i64,
    },
    NeedCaptcha,
    CaptchaData {
        categories: Vec<CategoryItem>,
        url: String,
        token: String,
        image_bytes: Option<Vec<u8>>,
    },
    LlmChunk(LlmChunk),
    LlmErr(String),
    LlmRetry,
    SubmitOk {
        score: i64,
    },
    SubmitFail(String),
    QuizDone {
        score: i64,
        scores: Vec<ScoreItem>,
    },
    Fail(String),
}

#[derive(Debug, Clone)]
pub struct AnswerItem {
    pub text: String,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryItem {
    pub num: u32,
    pub question: String,
    pub options: Vec<String>,
    pub chosen_idx: usize,
    pub correct: bool,
    #[serde(default)]
    pub correct_idx: Option<usize>,
}

// --- Main App State ---

const SPINNER: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

pub struct App {
    pub quit: bool,
    pub page: Page,
    pub prev_page: Vec<Page>,

    // Home
    pub home_sel: HomeSelection,

    // Config page
    pub cfg_fields: [String; 3],
    pub cfg_focus: ConfigFocus,
    pub cfg_cursors: [usize; 3],
    pub cfg_thinking: bool,
    pub cfg_fast_mode: bool,
    pub config_confirm_reset: bool,
    pub config_reset_choice: u8,

    // Quiz state
    pub phase: QuizPhase,
    pub score: i64,
    pub question_id: i64,
    pub question_num: u32,
    pub answers: Vec<AnswerItem>,
    pub question_text: String,
    pub spinner: usize,
    pub history: Vec<HistoryItem>,
    pub history_scroll: usize,
    pub chosen_answer_idx: usize,

    // Streaming LLM state
    pub thinking_text: String,
    pub answer_text: String,

    // Shared
    pub config: Option<OpenAiConfig>,
    pub auth: Option<AuthData>,
    pub tx: mpsc::UnboundedSender<AppEvent>,
    pub rx: mpsc::UnboundedReceiver<AppEvent>,
    pub bili: BiliClient,
    // QR polling state
    pub qr_auth_code: Option<String>,
    pub qr_poll_tick: u32,

    // Captcha image rendering
    pub captcha_picker: Option<ratatui_image::picker::Picker>,
    pub captcha_image: Option<image::DynamicImage>,

    // Captcha refresh: preserve selections and focus
    pub captcha_preserve: Option<(Vec<bool>, usize, CaptchaFocus, String)>,

    // Selected category names for LLM prompt
    pub selected_categories: Vec<String>,

    // LLM retry counter
    pub llm_retries: u32,
}

impl App {
    pub fn new(cli_config: Option<OpenAiConfig>, captcha_picker: Option<ratatui_image::picker::Picker>) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        let config = cli_config
            .as_ref()
            .map(|c| {
                if let Err(e) = config::save_openai_config(c) {
                    tracing::error!("保存命令行配置失败: {}", e);
                }
                c.clone()
            })
            .or_else(|| {
                config::load_openai_config().unwrap_or_else(|e| {
                    tracing::error!("加载配置失败: {}", e);
                    None
                })
            });

        let auth = config::load_auth().unwrap_or_else(|e| {
            tracing::error!("加载认证失败: {}", e);
            None
        });

        let mut bili = BiliClient::new();
        if let Some(a) = &auth {
            bili.set_auth(a);
        }

        let cfg_fields = if let Some(c) = &config {
            [c.base_url.clone(), c.model.clone(), c.api_key.clone()]
        } else {
            [String::new(), String::new(), String::new()]
        };

        Self {
            quit: false,
            page: Page::Home,
            prev_page: vec![],
            home_sel: HomeSelection::StartQuiz,
            cfg_cursors: [
                cfg_fields[0].len(),
                cfg_fields[1].len(),
                cfg_fields[2].len(),
            ],
            cfg_focus: ConfigFocus::BaseUrl,
            cfg_fields,
            cfg_thinking: config.as_ref().is_some_and(|c| c.enable_thinking),
            cfg_fast_mode: config.as_ref().is_some_and(|c| c.enable_fast_mode),
            config_confirm_reset: false,
            config_reset_choice: 0,
            phase: QuizPhase::NotConfigured,
            score: 0,
            question_id: 0,
            question_num: 0,
            answers: vec![],
            question_text: String::new(),
            spinner: 0,
            history: config::load_history(),
            history_scroll: 0,
            chosen_answer_idx: 0,
            thinking_text: String::new(),
            answer_text: String::new(),
            config,
            auth,
            tx,
            rx,
            bili,
            qr_auth_code: None,
            qr_poll_tick: 0,
            captcha_picker,
            captcha_image: None,
            captcha_preserve: None,
            selected_categories: config::load_categories(),
            llm_retries: 0,
        }
    }

    pub fn go(&mut self, page: Page) {
        self.prev_page.push(self.page);
        self.page = page;
    }

    pub fn back(&mut self) {
        if let Some(p) = self.prev_page.pop() {
            self.page = p;
        }
    }

    pub fn reset_all(&mut self) {
        let _ = config::delete_openai_config();
        let _ = config::delete_auth();
        self.config = None;
        self.auth = None;
        self.bili = BiliClient::new();
        self.cfg_fields = [String::new(), String::new(), String::new()];
        self.cfg_cursors = [0, 0, 0];
        self.cfg_thinking = false;
        self.config_confirm_reset = false;
        self.config_reset_choice = 0;
        self.back();
    }

    pub fn logout_only(&mut self) {
        let _ = config::delete_auth();
        self.auth = None;
        self.bili = BiliClient::new();
        self.config_confirm_reset = false;
        self.config_reset_choice = 0;
        self.back();
    }

    pub fn spin_char(&self) -> char {
        SPINNER[self.spinner % SPINNER.len()]
    }

    pub fn tick(&mut self) {
        self.spinner = (self.spinner + 1) % SPINNER.len();

        // ShowingResult countdown (~100ms/tick, 5 ticks = 0.5s)
        if let QuizPhase::ShowingResult { correct, countdown } = self.phase {
            if countdown > 1 {
                self.phase = QuizPhase::ShowingResult {
                    correct,
                    countdown: countdown - 1,
                };
            } else {
                // countdown reached 0 → proceed to next question
                let num = self.question_num;
                self.history.push(HistoryItem {
                    num: self.question_num,
                    question: self.question_text.clone(),
                    options: self.answers.iter().map(|a| a.text.clone()).collect(),
                    chosen_idx: self.chosen_answer_idx,
                    correct,
                    correct_idx: None,
                });
                let _ = config::save_history(&self.history);
                if num < 100 {
                    self.phase = QuizPhase::FetchingQuestion;
                    self.spawn_fetch_question();
                } else {
                    self.phase = QuizPhase::Submitting;
                    self.fetch_final();
                }
            }
        }

        // QR 轮询 countdown
        if let QuizPhase::WaitingScan {
            countdown,
            auth_code,
            ..
        } = &self.phase
        {
            if *countdown > 0 {
                let ac = auth_code.clone();
                let url = match &self.phase {
                    QuizPhase::WaitingScan { url, qr, .. } => (url.clone(), qr.clone()),
                    _ => unreachable!(),
                };

                self.qr_poll_tick += 1;
                if self.qr_poll_tick >= 10 {
                    // 每 ~1秒递减 countdown 并轮询 (tick_rate 100ms)
                    self.qr_poll_tick = 0;
                    let new_cd = *countdown - 1;
                    self.phase = QuizPhase::WaitingScan {
                        url: url.0,
                        qr: url.1,
                        auth_code: ac.clone(),
                        countdown: new_cd,
                    };
                    self.poll_qr(&ac);
                }
            } else {
                self.phase = QuizPhase::LoginTimeout { retry: true };
            }
        }
    }

    // --- Async dispatchers ---

    pub fn spawn_login(&mut self) {
        if self.auth.is_some() {
            self.phase = QuizPhase::CheckingLevel;
            self.spawn_level_check();
            return;
        }
        self.phase = QuizPhase::LoggingIn;
        let tx = self.tx.clone();
        let bili = self.bili.async_clone();

        tokio::spawn(async move {
            match bili.fetch_ticket().await {
                Ok(ticket) => {
                    let mut bili = bili;
                    bili.set_ticket(&ticket);
                    let _ = tx.send(AppEvent::TicketReady(ticket));
                    match bili.qrcode_get().await {
                        Ok(data) => {
                            let url = data["url"].as_str().unwrap_or("").to_string();
                            let auth_code = data["auth_code"].as_str().unwrap_or("").to_string();
                            let qr = make_qr(&url);
                            let _ = tx.send(AppEvent::QrReady { url, qr, auth_code });
                        }
                        Err(e) => {
                            let _ = tx.send(AppEvent::Fail(format!("获取二维码失败: {}", e)));
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(AppEvent::Fail(format!("获取 ticket 失败: {}", e)));
                }
            }
        });
    }

    fn poll_qr(&self, auth_code: &str) {
        let tx = self.tx.clone();
        let bili = self.bili.async_clone();
        let code = auth_code.to_string();
        tokio::spawn(async move {
            match bili.qrcode_poll(&code).await {
                Ok(data) if data["code"].as_i64() == Some(0) => {
                    let d = &data["data"];
                    let access_token = d["access_token"].as_str().unwrap_or("").to_string();
                    let mid = d["mid"].as_i64().unwrap_or(0).to_string();
                    let cookies = d["cookie_info"]["cookies"].as_array();
                    let mut csrf = String::new();
                    let mut parts = Vec::new();
                    if let Some(arr) = cookies {
                        for c in arr {
                            let n = c["name"].as_str().unwrap_or("");
                            let v = c["value"].as_str().unwrap_or("");
                            parts.push(format!("{}={}", n, v));
                            if n == "bili_jct" {
                                csrf = v.to_string();
                            }
                        }
                    }
                    let auth = AuthData {
                        access_token,
                        csrf,
                        mid,
                        cookie: parts.join(";"),
                    };
                    let _ = config::save_auth(&auth)
                        .map_err(|e| tracing::error!("保存登录信息失败: {}", e));
                    let _ = tx.send(AppEvent::LoginOk(auth));
                }
                Ok(_) => {
                    let _ = tx.send(AppEvent::LoginPending);
                }
                Err(_) => {
                    let _ = tx.send(AppEvent::LoginPending);
                }
            }
        });
    }

    fn spawn_level_check(&self) {
        let tx = self.tx.clone();
        let bili = self.bili.async_clone();
        tokio::spawn(async move {
            match bili.get_account_info().await {
                Ok(info) => {
                    let lv = info["level"].as_i64().unwrap_or(0);
                    if lv == 6 {
                        let _ = tx.send(AppEvent::LevelOk);
                    } else {
                        let _ = tx.send(AppEvent::LevelFail(lv));
                    }
                }
                Err(e) => {
                    let _ = tx.send(AppEvent::Fail(e.to_string()));
                }
            }
        });
    }

    pub fn spawn_fetch_question(&self) {
        let tx = self.tx.clone();
        let bili = self.bili.async_clone();
        tokio::spawn(async move {
            match bili.question_get().await {
                Ok(data) if data["code"].as_i64() == Some(0) => {
                    let d = &data["data"];
                    let _ = tx.send(AppEvent::QuestionReady {
                        num: d["question_num"].as_u64().unwrap_or(0) as u32,
                        question: d["question"].as_str().unwrap_or("").to_string(),
                        answers: d["answers"]
                            .as_array()
                            .map(|a| {
                                a.iter()
                                    .filter_map(|v| {
                                        Some(AnswerItem {
                                            text: v["ans_text"].as_str()?.to_string(),
                                            hash: v["ans_hash"].as_str()?.to_string(),
                                        })
                                    })
                                    .collect::<Vec<_>>()
                            })
                            .unwrap_or_default(),
                        id: d["id"].as_i64().unwrap_or(0),
                    });
                }
                Ok(_) => {
                    let _ = tx.send(AppEvent::NeedCaptcha);
                }
                Err(e) => {
                    let _ = tx.send(AppEvent::Fail(e.to_string()));
                }
            }
        });
    }

    pub fn spawn_fetch_captcha(&self) {
        let tx = self.tx.clone();
        let bili = self.bili.async_clone();
        tokio::spawn(async move {
            let cats = match bili.category_get().await {
                Ok(data) => data["categories"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|c| {
                                Some(CategoryItem {
                                    id: c["id"].as_i64()?,
                                    name: c["name"].as_str()?.to_string(),
                                    selected: false,
                                })
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default(),
                Err(e) => {
                    let _ = tx.send(AppEvent::Fail(e.to_string()));
                    return;
                }
            };
            match bili.captcha_get().await {
                Ok(data) => {
                    let url = data["url"].as_str().unwrap_or("").to_string();
                    let token = data["token"].as_str().unwrap_or("").to_string();

                    let image_bytes = match reqwest::get(&url).await {
                        Ok(resp) if resp.status().is_success() => {
                            resp.bytes().await.ok().map(|b| b.to_vec())
                        }
                        _ => None,
                    };

                    let _ = tx.send(AppEvent::CaptchaData {
                        categories: cats,
                        url,
                        token,
                        image_bytes,
                    });
                }
                Err(e) => {
                    let _ = tx.send(AppEvent::Fail(e.to_string()));
                }
            }
        });
    }

    pub fn spawn_llm(&self) {
        if let Some(ref cfg) = self.config {
            let client = crate::llm::OpenAiClient::new(cfg);
            let prompt = format!(
                "题目:{}\n答案:{:?}",
                self.question_text,
                self.answers.iter().map(|a| &a.text).collect::<Vec<_>>()
            );
            let (llm_tx, mut llm_rx) = mpsc::unbounded_channel::<LlmChunk>();
            let tx = self.tx.clone();
            let current_retries = self.llm_retries;
            let max_retries = 3u32;

            let full_prompt = crate::config::build_quiz_prompt(
                &self.selected_categories,
                &prompt,
                cfg.enable_thinking,
            );
            tracing::info!("LLM prompt:\n{}", full_prompt);

            client.ask_stream(&prompt, self.selected_categories.clone(), llm_tx);

            tokio::spawn(async move {
                #[allow(unused_assignments)]
                let mut last_err = String::new();

                while let Some(chunk) = llm_rx.recv().await {
                    match chunk {
                        LlmChunk::Thinking(_) | LlmChunk::Content(_) => {
                            let _ = tx.send(AppEvent::LlmChunk(chunk));
                        }
                        LlmChunk::Done(text) => {
                            if text.is_empty() {
                                if current_retries + 1 >= max_retries {
                                    tracing::warn!("LLM 返回空内容，已达到最大重试次数");
                                    let _ = tx.send(AppEvent::LlmErr("LLM 返回空内容".into()));
                                    return;
                                }
                                tracing::warn!(
                                    "LLM 返回空内容，将重试 ({}/{})",
                                    current_retries + 1,
                                    max_retries
                                );
                                let _ = tx.send(AppEvent::LlmRetry);
                                return;
                            }
                            let _ = tx.send(AppEvent::LlmChunk(LlmChunk::Done(text)));
                            return;
                        }
                        LlmChunk::Error(msg) => {
                            last_err = msg;
                            if current_retries + 1 >= max_retries {
                                let _ = tx.send(AppEvent::LlmErr(last_err));
                                return;
                            }
                            tracing::warn!(
                                "LLM 请求失败，将重试 ({}/{}): {}",
                                current_retries + 1,
                                max_retries,
                                last_err
                            );
                            let _ = tx.send(AppEvent::LlmRetry);
                            return;
                        }
                    }
                }
            });
        }
    }

    pub fn spawn_submit(&self, ans_idx: usize) {
        if ans_idx == 0 || ans_idx > self.answers.len() {
            return;
        }
        let ans = &self.answers[ans_idx - 1];
        let tx = self.tx.clone();
        let bili = self.bili.async_clone();
        let qid = self.question_id;
        let hash = ans.hash.clone();
        let text = ans.text.clone();
        tokio::spawn(async move {
            match bili.question_submit(qid, &hash, &text).await {
                Ok(resp) if resp["code"].as_i64() == Some(0) => {
                    match bili.question_result().await {
                        Ok(r) => {
                            let s = r["score"].as_i64().unwrap_or(0);
                            let _ = tx.send(AppEvent::SubmitOk { score: s });
                        }
                        Err(_) => {
                            let _ = tx.send(AppEvent::SubmitOk { score: 0 });
                        }
                    }
                }
                Ok(resp) if resp["code"].as_i64() == Some(41103) => {
                    let _ = tx.send(AppEvent::SubmitFail("请检查是否已经是硬核会员".into()));
                }
                Ok(resp) => {
                    let _ = tx.send(AppEvent::SubmitFail(format!("提交失败: {}", resp)));
                }
                Err(e) => {
                    let _ = tx.send(AppEvent::Fail(e.to_string()));
                }
            }
        });
    }

    pub fn spawn_captcha_submit(&self, code: &str, token: &str, ids: &str) {
        let tx = self.tx.clone();
        let bili = self.bili.async_clone();
        let c = code.to_string();
        let t = token.to_string();
        let i = ids.to_string();
        tokio::spawn(async move {
            match bili.captcha_submit(&c, &t, &i).await {
                Ok(true) => {
                    // 验证通过，重新获取题目
                    match bili.question_get().await {
                        Ok(data) if data["code"].as_i64() == Some(0) => {
                            let d = &data["data"];
                            let _ = tx.send(AppEvent::QuestionReady {
                                num: d["question_num"].as_u64().unwrap_or(0) as u32,
                                question: d["question"].as_str().unwrap_or("").to_string(),
                                answers: d["answers"]
                                    .as_array()
                                    .map(|a| {
                                        a.iter()
                                            .filter_map(|v| {
                                                Some(AnswerItem {
                                                    text: v["ans_text"].as_str()?.to_string(),
                                                    hash: v["ans_hash"].as_str()?.to_string(),
                                                })
                                            })
                                            .collect::<Vec<_>>()
                                    })
                                    .unwrap_or_default(),
                                id: d["id"].as_i64().unwrap_or(0),
                            });
                        }
                        Ok(_) => {
                            let _ = tx.send(AppEvent::NeedCaptcha);
                        }
                        Err(e) => {
                            let _ = tx.send(AppEvent::Fail(e.to_string()));
                        }
                    }
                }
                _ => {
                    let _ = tx.send(AppEvent::Fail("验证码验证失败".into()));
                }
            }
        });
    }

    fn fetch_final(&self) {
        let tx = self.tx.clone();
        let bili = self.bili.async_clone();
        tokio::spawn(async move {
            match bili.question_result().await {
                Ok(data) => {
                    let score = data["score"].as_i64().unwrap_or(0);
                    let scores = data["scores"]
                        .as_array()
                        .map(|a| {
                            a.iter()
                                .filter_map(|s| {
                                    Some(ScoreItem {
                                        category: s["category"].as_str()?.to_string(),
                                        score: s["score"].as_i64()?,
                                        total: s["total"].as_i64()?,
                                    })
                                })
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    let _ = tx.send(AppEvent::QuizDone { score, scores });
                }
                Err(e) => {
                    let _ = tx.send(AppEvent::Fail(e.to_string()));
                }
            }
        });
    }

    // --- Event processing ---

    pub fn process(&mut self, ev: AppEvent) {
        // 答题相关事件：不在答题页面时丢弃，防止 ESC 退出后后台继续答题
        if self.page != Page::Quiz {
            match ev {
                AppEvent::TicketReady(_)
                | AppEvent::QrReady { .. }
                | AppEvent::LoginOk(_)
                | AppEvent::LoginPending
                | AppEvent::LevelOk
                | AppEvent::LevelFail(_)
                | AppEvent::QuestionReady { .. }
                | AppEvent::NeedCaptcha
                | AppEvent::CaptchaData { .. }
                | AppEvent::LlmChunk(_)
                | AppEvent::LlmErr(_)
                | AppEvent::LlmRetry
                | AppEvent::SubmitOk { .. }
                | AppEvent::SubmitFail(_)
                | AppEvent::QuizDone { .. }
                | AppEvent::Fail(_) => return,
            }
        }
        match ev {
            AppEvent::TicketReady(ticket) => {
                self.bili.set_ticket(&ticket);
            }
            AppEvent::QrReady { url, qr, auth_code } => {
                self.qr_auth_code = Some(auth_code.clone());
                self.qr_poll_tick = 0;
                self.phase = QuizPhase::WaitingScan {
                    url,
                    qr,
                    auth_code,
                    countdown: 60,
                };
            }
            AppEvent::LoginOk(auth) => {
                self.auth = Some(auth.clone());
                self.bili.set_auth(&auth);
                self.qr_auth_code = None;
                self.phase = QuizPhase::CheckingLevel;
                self.spawn_level_check();
            }
            AppEvent::LoginPending => {}
            AppEvent::LevelOk => {
                self.score = 0;
                self.phase = QuizPhase::FetchingQuestion;
                self.spawn_fetch_question();
            }
            AppEvent::LevelFail(lv) => {
                self.phase = QuizPhase::Error(format!("当前用户等级 {}，需满6级才能参与答题", lv));
            }
            AppEvent::QuestionReady {
                num,
                question,
                answers,
                id,
            } => {
                self.question_num = num;
                self.question_text = question;
                self.answers = answers;
                self.question_id = id;
                self.thinking_text.clear();
                self.answer_text.clear();
                self.llm_retries = 0;
                self.spawn_llm();
                self.phase = QuizPhase::WaitingLlm;
            }
            AppEvent::NeedCaptcha => {
                if !self.history.is_empty() {
                    self.history.clear();
                    let _ = config::save_history(&self.history);
                }
                self.phase = QuizPhase::FetchingQuestion;
                self.spawn_fetch_captcha();
            }
            AppEvent::CaptchaData {
                categories,
                url,
                token,
                image_bytes,
            } => {
                self.captcha_image = image_bytes.and_then(|b| image::load_from_memory(&b).ok());
                let (selected, cat_focus, focus, input) = self
                    .captcha_preserve
                    .take()
                    .unwrap_or((vec![], 0, CaptchaFocus::Categories, String::new()));
                let categories = categories
                    .into_iter()
                    .enumerate()
                    .map(|(i, mut c)| {
                        c.selected = selected.get(i).copied().unwrap_or(false);
                        c
                    })
                    .collect();
                self.phase = QuizPhase::Captcha(CaptchaState {
                    categories,
                    cat_focus,
                    captcha_url: url,
                    captcha_token: token,
                    input,
                    focus,
                    error: String::new(),
                });
            }
            AppEvent::LlmChunk(chunk) => match chunk {
                LlmChunk::Thinking(text) => {
                    self.thinking_text.push_str(&text);
                }
                LlmChunk::Content(text) => {
                    self.answer_text.push_str(&text);
                }
                LlmChunk::Done(full_text) => {
                    match parse_answer(&full_text) {
                        Some(idx) => {
                            self.chosen_answer_idx = idx;
                            self.phase = QuizPhase::Submitting;
                            self.spawn_submit(idx);
                        }
                        None => {
                            tracing::warn!("AI 回复无法解析: {}", full_text);
                            if self.llm_retries + 1 < 3 {
                                self.llm_retries += 1;
                                tracing::warn!(
                                    "将重试 LLM ({}/3)",
                                    self.llm_retries
                                );
                                self.thinking_text.clear();
                                self.answer_text.clear();
                                self.spawn_llm();
                            } else {
                                self.phase = QuizPhase::Error(format!(
                                    "AI 回复无法解析: {}",
                                    full_text
                                ));
                            }
                        }
                    }
                }
                LlmChunk::Error(msg) => {
                    self.phase = QuizPhase::Error(format!("AI 回答错误: {}", msg));
                }
            },
            AppEvent::LlmErr(msg) => {
                self.phase = QuizPhase::Error(format!("AI 回答错误: {}", msg));
            }
            AppEvent::LlmRetry => {
                self.llm_retries += 1;
                self.thinking_text.clear();
                self.answer_text.clear();
                self.spawn_llm();
            }
            AppEvent::SubmitOk { score } => {
                let correct = score > self.score;
                self.score = score;
                self.phase = QuizPhase::ShowingResult {
                    correct,
                    countdown: if self.cfg_fast_mode { 1 } else { 10 },
                };
            }
            AppEvent::SubmitFail(msg) => {
                self.phase = QuizPhase::Error(msg);
            }
            AppEvent::QuizDone { score, scores } => {
                self.phase = QuizPhase::Finished { score, scores };
            }
            AppEvent::Fail(msg) => {
                self.phase = QuizPhase::Error(msg);
            }
        }
    }
}

fn parse_answer(s: &str) -> Option<usize> {
    let s = s.trim();
    if let Ok(n) = s.parse::<usize>()
        && (1..=4).contains(&n)
    {
        return Some(n);
    }
    // "回答：3" or "回答:3"
    for prefix in &["回答：", "回答:"] {
        if let Some(rest) = s.strip_prefix(prefix)
            && let Ok(n) = rest.trim().parse::<usize>()
            && (1..=4).contains(&n)
        {
            return Some(n);
        }
    }
    // find any digit 1-4 in the string
    for c in s.chars() {
        if let Ok(n) = c.to_string().parse::<usize>()
            && (1..=4).contains(&n)
        {
            return Some(n);
        }
    }
    None
}

fn make_qr(url: &str) -> String {
    use qrcode::QrCode;
    use qrcode::render::unicode::Dense1x2;
    match QrCode::new(url.as_bytes()) {
        Ok(code) => code
            .render::<Dense1x2>()
            .quiet_zone(false)
            .module_dimensions(1, 1)
            .build(),
        Err(_) => "QR generation failed".into(),
    }
}

impl BiliClient {
    pub fn async_clone(&self) -> Self {
        self.clone_for_async()
    }
}
