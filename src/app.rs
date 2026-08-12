use crate::api::BiliClient;
use crate::config::{self, AuthData, OpenAiConfig};
use crate::llm::LlmChunk;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

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
    ThinkingEffort,
    FastModeToggle,
    SaveBtn,
    TemplateBtn,
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
    LevelVerified {
        level: i64,
        countdown: u8,
    },
    LevelInsufficient {
        level: i64,
    },
    LevelCheckFailed(String),
    FetchingQuestion,
    WaitingLlm,
    WaitingRetry {
        attempt: u32,
        deadline: std::time::Instant,
    },
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuizIntent {
    Answer,
    LoginOnly,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreItem {
    pub category: String,
    pub score: i64,
    pub total: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Passed,
    Failed,
    Interrupted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionHistory {
    pub question_number: u32,
    pub question: String,
    pub options: Vec<String>,
    pub chosen_index: usize,
    pub correct: bool,
    #[serde(default)]
    pub correct_index: Option<usize>,
}

impl From<&HistoryItem> for QuestionHistory {
    fn from(item: &HistoryItem) -> Self {
        Self {
            question_number: item.num,
            question: item.question.clone(),
            options: item.options.clone(),
            chosen_index: item.chosen_idx,
            correct: item.correct,
            correct_index: item.correct_idx,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionHistory {
    pub id: String,
    pub started_at: i64,
    #[serde(default)]
    pub finished_at: Option<i64>,
    pub updated_at: i64,
    pub model: String,
    #[serde(default)]
    pub categories: Vec<String>,
    pub status: SessionStatus,
    pub completed_questions: u32,
    pub score: i64,
    #[serde(default)]
    pub category_scores: Vec<ScoreItem>,
    #[serde(default)]
    pub failure_stage: Option<String>,
    #[serde(default)]
    pub failure_message: Option<String>,
    #[serde(default)]
    pub questions: Vec<QuestionHistory>,
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
    LevelOk(i64),
    LevelFail(i64),
    LevelCheckFailed(String),
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
    CaptchaRejected,
    LlmChunk(LlmChunk),
    LlmRetry {
        reason: String,
    },
    LlmRetryFire,
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
    /// 思考强度档位索引：0=low 1=high 2=max
    pub cfg_effort: usize,
    pub cfg_fast_mode: bool,
    pub config_confirm_reset: bool,
    pub config_reset_choice: u8,
    pub cfg_preset_open: bool,
    pub cfg_preset_sel: usize,

    // Quiz state
    pub phase: QuizPhase,
    pub score: i64,
    pub question_id: i64,
    pub question_num: u32,
    pub answers: Vec<AnswerItem>,
    pub question_text: String,
    pub spinner: usize,
    pub history: Vec<HistoryItem>,
    pub sessions: Vec<SessionHistory>,
    pub active_session: Option<SessionHistory>,
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
    pub captcha_error: Option<String>,

    // Selected category names for LLM prompt
    pub selected_categories: Vec<String>,

    // LLM retry counter
    pub llm_retries: u32,

    // Cancellation token for quiz background tasks
    pub quiz_token: CancellationToken,
    pub quiz_intent: QuizIntent,
}

impl App {
    /// LLM 单题最大重试次数（不含首次请求）。
    pub const MAX_LLM_RETRIES: u32 = 3;

    pub fn new(
        cli_config: Option<OpenAiConfig>,
        captcha_picker: Option<ratatui_image::picker::Picker>,
    ) -> Self {
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
            cfg_thinking: config.as_ref().is_none_or(|c| c.enable_thinking),
            cfg_effort: config
                .as_ref()
                .and_then(|c| match c.reasoning_effort.as_str() {
                    "low" => Some(0),
                    "high" => Some(1),
                    "max" => Some(2),
                    _ => None,
                })
                .unwrap_or(1),
            cfg_fast_mode: config.as_ref().is_some_and(|c| c.enable_fast_mode),
            config_confirm_reset: false,
            config_reset_choice: 0,
            cfg_preset_open: false,
            cfg_preset_sel: 0,
            phase: QuizPhase::NotConfigured,
            score: 0,
            question_id: 0,
            question_num: 0,
            answers: vec![],
            question_text: String::new(),
            spinner: 0,
            history: config::load_history(),
            sessions: config::load_sessions(),
            active_session: None,
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
            captcha_error: None,
            selected_categories: config::load_categories(),
            llm_retries: 0,
            quiz_token: CancellationToken::new(),
            quiz_intent: QuizIntent::Answer,
        }
    }

    pub fn go(&mut self, page: Page) {
        self.prev_page.push(self.page);
        self.page = page;
        if page == Page::Quiz {
            self.quiz_token = CancellationToken::new();
        }
    }

    pub fn back(&mut self) {
        let leaving_quiz = self.page == Page::Quiz;
        if let Some(p) = self.prev_page.pop() {
            self.page = p;
        }
        if leaving_quiz {
            self.stop_quiz("用户退出答题", "quiz_exit");
        }
    }

    pub(crate) fn stop_quiz(&mut self, message: &str, stage: &str) {
        self.interrupt_session(message, stage);
        self.quiz_token.cancel();
        self.quiz_token = CancellationToken::new();
    }

    pub(crate) fn enter_config(&mut self) {
        let is_first_time = self.config.is_none();
        if let Some(ref config) = self.config {
            self.cfg_fields = [
                config.base_url.clone(),
                config.model.clone(),
                config.api_key.clone(),
            ];
        }
        self.cfg_cursors = [
            self.cfg_fields[0].len(),
            self.cfg_fields[1].len(),
            self.cfg_fields[2].len(),
        ];
        self.cfg_focus = ConfigFocus::BaseUrl;
        self.cfg_preset_open = is_first_time;
        self.cfg_preset_sel = 0;
        self.go(Page::Config);
    }

    pub(crate) fn apply_preset(&mut self, index: usize) {
        if let Some(preset) = config::load_presets().get(index) {
            self.cfg_fields[0] = preset.config.base_url.clone();
            self.cfg_fields[1] = preset.config.model.clone();
            self.cfg_cursors[0] = self.cfg_fields[0].len();
            self.cfg_cursors[1] = self.cfg_fields[1].len();
        }
    }

    pub(crate) fn save_config(&mut self) -> Result<(), String> {
        self.persist_config()?;
        self.back();
        if self.page == Page::Quiz {
            self.spawn_login();
        }
        Ok(())
    }

    pub(crate) fn persist_config(&mut self) -> Result<(), String> {
        let base_url = self.cfg_fields[0].trim().trim_end_matches('/').to_string();
        let model = self.cfg_fields[1].trim().to_string();
        let api_key = self.cfg_fields[2].trim().to_string();
        if base_url.is_empty() || model.is_empty() || api_key.is_empty() {
            return Err("请完整填写 API URL、模型名称和 API Key".into());
        }
        if !(base_url.starts_with("https://") || base_url.starts_with("http://")) {
            return Err("API URL 必须以 http:// 或 https:// 开头".into());
        }

        let config = OpenAiConfig {
            base_url,
            model,
            api_key,
            enable_thinking: self.cfg_thinking,
            reasoning_effort: ["low", "high", "max"][self.cfg_effort].to_string(),
            enable_fast_mode: self.cfg_fast_mode,
        };
        config::save_openai_config(&config).map_err(|error| error.to_string())?;
        self.config = Some(config);
        Ok(())
    }

    pub(crate) fn enter_quiz(&mut self) {
        self.quiz_intent = QuizIntent::Answer;
        self.go(Page::Quiz);
        if self.config.is_none() {
            self.phase = QuizPhase::NotConfigured;
        } else {
            self.spawn_login();
        }
    }

    #[cfg_attr(not(feature = "gui"), allow(dead_code))]
    pub(crate) fn enter_login(&mut self) {
        self.quiz_intent = QuizIntent::LoginOnly;
        self.go(Page::Quiz);
        self.spawn_login();
    }

    #[cfg_attr(not(feature = "gui"), allow(dead_code))]
    pub(crate) fn retry_level_check(&mut self) {
        if self.auth.is_some() {
            self.phase = QuizPhase::CheckingLevel;
            self.spawn_level_check();
        } else {
            self.spawn_login();
        }
    }

    #[cfg_attr(not(feature = "gui"), allow(dead_code))]
    pub(crate) fn relogin_for_quiz(&mut self) {
        let _ = config::delete_auth();
        self.auth = None;
        self.bili = BiliClient::new();
        self.spawn_login();
    }

    pub(crate) fn refresh_captcha(&mut self) {
        if let QuizPhase::Captcha(state) = &self.phase {
            let selected = state.categories.iter().map(|item| item.selected).collect();
            self.captcha_preserve = Some((selected, state.cat_focus, state.focus, String::new()));
            self.captcha_error = None;
            self.captcha_image = None;
            self.phase = QuizPhase::FetchingQuestion;
            self.spawn_fetch_captcha();
        }
    }

    pub(crate) fn toggle_captcha_category(&mut self, index: usize) {
        let QuizPhase::Captcha(state) = &mut self.phase else {
            return;
        };
        let selected_count = state.categories.iter().filter(|item| item.selected).count();
        if let Some(item) = state.categories.get_mut(index)
            && (item.selected || selected_count < 3)
        {
            item.selected = !item.selected;
            state.error.clear();
        }
    }

    pub(crate) fn submit_captcha(&mut self) -> Result<(), String> {
        let QuizPhase::Captcha(state) = &self.phase else {
            return Err("验证码尚未加载".into());
        };
        let ids = state
            .categories
            .iter()
            .filter(|item| item.selected)
            .map(|item| item.id.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let input = state.input.trim().to_string();
        let token = state.captcha_token.clone();
        let categories = state
            .categories
            .iter()
            .filter(|item| item.selected)
            .map(|item| item.name.clone())
            .collect::<Vec<_>>();
        if ids.is_empty() || input.is_empty() {
            return Err(match (ids.is_empty(), input.is_empty()) {
                (true, true) => "请选择分类并输入验证码",
                (true, false) => "请选择分类",
                (false, true) => "请输入验证码",
                (false, false) => unreachable!(),
            }
            .into());
        }

        self.selected_categories = categories;
        self.captcha_error = None;
        config::save_categories(&self.selected_categories).map_err(|error| error.to_string())?;
        let selected = state.categories.iter().map(|item| item.selected).collect();
        self.captcha_preserve = Some((selected, state.cat_focus, state.focus, input.clone()));
        self.spawn_captcha_submit(&input, &token, &ids);
        self.phase = QuizPhase::FetchingQuestion;
        Ok(())
    }

    pub fn reset_all(&mut self) {
        let _ = config::delete_openai_config();
        let _ = config::delete_auth();
        let _ = config::delete_local_history();
        self.config = None;
        self.auth = None;
        self.bili = BiliClient::new();
        self.cfg_fields = [String::new(), String::new(), String::new()];
        self.cfg_cursors = [0, 0, 0];
        self.cfg_thinking = true;
        self.cfg_effort = 1;
        self.config_confirm_reset = false;
        self.config_reset_choice = 0;
        self.cfg_preset_open = false;
        self.cfg_preset_sel = 0;
        self.history.clear();
        self.sessions.clear();
        self.active_session = None;
        self.selected_categories.clear();
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
        if self.quiz_token.is_cancelled() {
            return;
        }
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
                self.record_session_question(correct);
                if num < 100 {
                    self.phase = QuizPhase::FetchingQuestion;
                    self.spawn_fetch_question();
                } else {
                    self.phase = QuizPhase::Submitting;
                    self.fetch_final();
                }
            }
        }

        if let QuizPhase::LevelVerified { level, countdown } = self.phase {
            if countdown > 1 {
                self.phase = QuizPhase::LevelVerified {
                    level,
                    countdown: countdown - 1,
                };
            } else {
                if self.quiz_intent == QuizIntent::LoginOnly {
                    self.phase = QuizPhase::NotConfigured;
                    self.back();
                    return;
                }
                self.score = self.history.iter().filter(|item| item.correct).count() as i64;
                self.phase = QuizPhase::FetchingQuestion;
                self.spawn_fetch_question();
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
                    if lv >= 6 {
                        let _ = tx.send(AppEvent::LevelOk(lv));
                    } else {
                        let _ = tx.send(AppEvent::LevelFail(lv));
                    }
                }
                Err(e) => {
                    let _ = tx.send(AppEvent::LevelCheckFailed(e.to_string()));
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
            let token = self.quiz_token.clone();
            let client = crate::llm::OpenAiClient::new(cfg);
            let prompt = format!(
                "题目:{}\n答案:{:?}",
                self.question_text,
                self.answers.iter().map(|a| &a.text).collect::<Vec<_>>()
            );
            let (llm_tx, mut llm_rx) = mpsc::unbounded_channel::<LlmChunk>();
            let tx = self.tx.clone();

            let full_prompt = crate::config::build_quiz_prompt(
                &self.selected_categories,
                &prompt,
                cfg.enable_thinking,
            );
            tracing::info!("LLM prompt:\n{}", full_prompt);

            client.ask_stream(
                &prompt,
                self.selected_categories.clone(),
                llm_tx,
                token.clone(),
            );

            tokio::spawn(async move {
                while let Some(chunk) = llm_rx.recv().await {
                    if token.is_cancelled() {
                        return;
                    }
                    match chunk {
                        LlmChunk::Thinking(_) | LlmChunk::Content(_) => {
                            let _ = tx.send(AppEvent::LlmChunk(chunk));
                        }
                        LlmChunk::Done(text) => {
                            let _ = tx.send(AppEvent::LlmChunk(LlmChunk::Done(text)));
                            return;
                        }
                        LlmChunk::Error(msg) => {
                            // 传输/API/解析失败：不自动重试。此时请求可能已到达上游
                            // 并计费，叠加多个长任务会耗尽中转并发槽。仅当模型给出无效
                            // 答案（Done 但 parse_answer 失败）时才走重试路径。
                            tracing::warn!("LLM 请求失败，停止自动重试: {}", msg);
                            let _ = tx.send(AppEvent::LlmChunk(LlmChunk::Error(msg)));
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
                    let _ = tx.send(AppEvent::CaptchaRejected);
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
                | AppEvent::LevelOk(_)
                | AppEvent::LevelFail(_)
                | AppEvent::LevelCheckFailed(_)
                | AppEvent::QuestionReady { .. }
                | AppEvent::NeedCaptcha
                | AppEvent::CaptchaData { .. }
                | AppEvent::CaptchaRejected
                | AppEvent::LlmChunk(_)
                | AppEvent::LlmRetry { .. }
                | AppEvent::LlmRetryFire
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
            AppEvent::LevelOk(level) => {
                self.phase = QuizPhase::LevelVerified {
                    level,
                    countdown: 8,
                };
            }
            AppEvent::LevelFail(lv) => {
                self.phase = QuizPhase::LevelInsufficient { level: lv };
            }
            AppEvent::LevelCheckFailed(message) => {
                self.phase = QuizPhase::LevelCheckFailed(message);
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
                self.ensure_active_session();
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
                let (selected, cat_focus, focus, input) = self.captcha_preserve.take().unwrap_or((
                    vec![],
                    0,
                    CaptchaFocus::Categories,
                    String::new(),
                ));
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
                    error: self.captcha_error.take().unwrap_or_default(),
                });
            }
            AppEvent::CaptchaRejected => {
                if let Some(state) = &mut self.captcha_preserve {
                    state.3.clear();
                }
                self.captcha_error = Some("验证码错误，请重新输入".into());
                self.phase = QuizPhase::FetchingQuestion;
                self.spawn_fetch_captcha();
            }
            AppEvent::LlmChunk(chunk) => match chunk {
                LlmChunk::Thinking(text) => {
                    self.thinking_text.push_str(&text);
                }
                LlmChunk::Content(text) => {
                    self.answer_text.push_str(&text);
                }
                LlmChunk::Done(full_text) => match parse_answer(&full_text) {
                    Some(idx) => {
                        self.chosen_answer_idx = idx;
                        self.phase = QuizPhase::Submitting;
                        self.spawn_submit(idx);
                    }
                    None => {
                        tracing::warn!("AI 回复无法解析: {}", full_text);
                        let _ = self.tx.send(AppEvent::LlmRetry {
                            reason: format!("AI 回复无法解析: {}", full_text),
                        });
                    }
                },
                LlmChunk::Error(msg) => {
                    self.phase = QuizPhase::Error(format!(
                        "AI 请求失败: {msg}（为避免重复计费，未自动重试）"
                    ));
                }
            },
            AppEvent::LlmRetry { reason } => {
                let attempt = self.llm_retries + 1;
                if attempt > Self::MAX_LLM_RETRIES {
                    tracing::warn!(
                        "LLM 已达最大重试次数 {}，放弃: {}",
                        Self::MAX_LLM_RETRIES,
                        reason
                    );
                    self.phase = QuizPhase::Error(format!(
                        "AI 回答错误: {}（已重试 {} 次）",
                        reason,
                        Self::MAX_LLM_RETRIES
                    ));
                } else {
                    self.llm_retries = attempt;
                    let secs = 2u64 << (attempt - 1);
                    tracing::warn!(
                        "LLM 将第 {}/{} 次重试，{}s 后重试: {}",
                        attempt,
                        Self::MAX_LLM_RETRIES,
                        secs,
                        reason
                    );
                    self.thinking_text.clear();
                    self.answer_text.clear();
                    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(secs);
                    self.phase = QuizPhase::WaitingRetry { attempt, deadline };
                    let ct = self.quiz_token.clone();
                    let tx = self.tx.clone();
                    tokio::spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_secs(secs)).await;
                        if ct.is_cancelled() {
                            return;
                        }
                        let _ = tx.send(AppEvent::LlmRetryFire);
                    });
                }
            }
            AppEvent::LlmRetryFire => {
                if matches!(self.phase, QuizPhase::WaitingRetry { .. }) {
                    self.spawn_llm();
                    self.phase = QuizPhase::WaitingLlm;
                } else {
                    tracing::warn!("忽略过期的 LlmRetryFire（当前 phase 已变更）");
                }
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
                self.interrupt_session(&msg, "answer_submit");
                self.phase = QuizPhase::Error(msg);
            }
            AppEvent::QuizDone { score, scores } => {
                self.finish_session(score, &scores);
                self.phase = QuizPhase::Finished { score, scores };
            }
            AppEvent::Fail(msg) => {
                self.interrupt_session(&msg, "workflow");
                self.phase = QuizPhase::Error(msg);
            }
        }
    }

    fn ensure_active_session(&mut self) {
        if self.active_session.is_some() {
            return;
        }
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        let questions = self
            .history
            .iter()
            .map(QuestionHistory::from)
            .collect::<Vec<_>>();
        self.active_session = Some(SessionHistory {
            id: format!("{now}-{}", self.sessions.len() + 1),
            started_at: now,
            finished_at: None,
            updated_at: now,
            model: self
                .config
                .as_ref()
                .map(|config| config.model.clone())
                .unwrap_or_else(|| "未记录".into()),
            categories: self.selected_categories.clone(),
            status: SessionStatus::Interrupted,
            completed_questions: questions.len() as u32,
            score: self.score,
            category_scores: vec![],
            failure_stage: None,
            failure_message: None,
            questions,
        });
    }

    fn record_session_question(&mut self, correct: bool) {
        self.ensure_active_session();
        let Some(session) = &mut self.active_session else {
            return;
        };
        let item = HistoryItem {
            num: self.question_num,
            question: self.question_text.clone(),
            options: self
                .answers
                .iter()
                .map(|answer| answer.text.clone())
                .collect(),
            chosen_idx: self.chosen_answer_idx,
            correct,
            correct_idx: None,
        };
        if !session
            .questions
            .iter()
            .any(|question| question.question_number == item.num)
        {
            session.questions.push(QuestionHistory::from(&item));
        }
        session.completed_questions = session.questions.len() as u32;
        session.score = self.score;
        session.updated_at = time::OffsetDateTime::now_utc().unix_timestamp();
    }

    fn finish_session(&mut self, score: i64, scores: &[ScoreItem]) {
        self.ensure_active_session();
        let Some(mut session) = self.active_session.take() else {
            return;
        };
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        session.updated_at = now;
        session.finished_at = Some(now);
        session.score = score;
        session.status = if score >= 60 {
            SessionStatus::Passed
        } else {
            SessionStatus::Failed
        };
        session.category_scores = scores.to_vec();
        session.failure_stage = None;
        session.failure_message = None;
        self.sessions.insert(0, session);
        #[cfg(not(test))]
        let _ = config::save_sessions(&self.sessions);
    }

    fn interrupt_session(&mut self, message: &str, stage: &str) {
        let Some(mut session) = self.active_session.take() else {
            return;
        };
        if session.completed_questions == 0 {
            return;
        }
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        session.updated_at = now;
        session.finished_at = Some(now);
        session.status = SessionStatus::Interrupted;
        session.failure_stage = Some(stage.to_string());
        session.failure_message = Some(redact_error(message));
        self.sessions.insert(0, session);
        #[cfg(not(test))]
        let _ = config::save_sessions(&self.sessions);
    }
}

fn redact_error(message: &str) -> String {
    let lower = message.to_ascii_lowercase();
    if lower.contains("api key") || lower.contains("token") || lower.contains("cookie") {
        "请求失败，敏感信息已隐藏".into()
    } else {
        message.chars().take(240).collect()
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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_app() -> App {
        let mut app = App::new(None, None);
        app.history.clear();
        app.sessions.clear();
        app.active_session = None;
        app.page = Page::Quiz;
        app.phase = QuizPhase::Captcha(CaptchaState {
            categories: (1..=4)
                .map(|id| CategoryItem {
                    id,
                    name: format!("分类{id}"),
                    selected: false,
                })
                .collect(),
            cat_focus: 0,
            captcha_url: "https://example.com/captcha.jpg".into(),
            captcha_token: "token".into(),
            input: String::new(),
            focus: CaptchaFocus::Categories,
            error: String::new(),
        });
        app
    }

    #[test]
    fn captcha_selection_is_limited_to_three() {
        let mut app = test_app();
        for index in 0..4 {
            app.toggle_captcha_category(index);
        }
        let QuizPhase::Captcha(state) = &app.phase else {
            panic!()
        };
        assert_eq!(
            state.categories.iter().filter(|item| item.selected).count(),
            3
        );
        assert!(!state.categories[3].selected);
    }

    #[test]
    fn captcha_submission_validates_required_values() {
        let mut app = test_app();
        assert_eq!(app.submit_captcha().unwrap_err(), "请选择分类并输入验证码");
        app.toggle_captcha_category(0);
        assert_eq!(app.submit_captcha().unwrap_err(), "请输入验证码");
    }

    #[test]
    fn leaving_quiz_cancels_background_work() {
        let mut app = test_app();
        app.prev_page.push(Page::Home);
        let token = app.quiz_token.clone();
        app.back();
        assert!(token.is_cancelled());
        assert_eq!(app.page, Page::Home);
    }

    #[test]
    fn process_ignores_quiz_events_outside_quiz_page() {
        let mut app = test_app();
        app.page = Page::Home;
        app.process(AppEvent::Fail("ignored".into()));
        assert!(matches!(app.phase, QuizPhase::Captcha(_)));
    }

    #[test]
    fn level_events_keep_contextual_states() {
        let mut app = test_app();
        app.process(AppEvent::LevelOk(6));
        assert!(matches!(
            app.phase,
            QuizPhase::LevelVerified { level: 6, .. }
        ));
        app.process(AppEvent::LevelFail(5));
        assert!(matches!(
            app.phase,
            QuizPhase::LevelInsufficient { level: 5 }
        ));
        app.process(AppEvent::LevelCheckFailed("网络超时".into()));
        assert!(matches!(app.phase, QuizPhase::LevelCheckFailed(_)));
    }

    #[tokio::test]
    async fn login_only_flow_does_not_require_model_configuration() {
        let mut app = test_app();
        app.page = Page::Home;
        app.prev_page.clear();
        app.auth = None;
        app.config = None;

        app.enter_login();

        assert_eq!(app.page, Page::Quiz);
        assert_eq!(app.quiz_intent, QuizIntent::LoginOnly);
        assert!(matches!(app.phase, QuizPhase::LoggingIn));
    }

    #[test]
    fn login_only_level_success_returns_home_without_fetching_questions() {
        let mut app = test_app();
        app.page = Page::Quiz;
        app.prev_page = vec![Page::Home];
        app.quiz_intent = QuizIntent::LoginOnly;
        app.phase = QuizPhase::LevelVerified {
            level: 6,
            countdown: 1,
        };

        app.tick();

        assert_eq!(app.page, Page::Home);
        assert!(!matches!(app.phase, QuizPhase::FetchingQuestion));
    }

    #[test]
    fn quiz_done_finalizes_session() {
        let mut app = test_app();
        app.config = Some(OpenAiConfig {
            base_url: "https://example.com".into(),
            model: "test-model".into(),
            api_key: "secret".into(),
            enable_thinking: false,
            reasoning_effort: "high".into(),
            enable_fast_mode: false,
        });
        app.question_num = 1;
        app.question_text = "测试题目".into();
        app.answers = vec![AnswerItem {
            text: "答案".into(),
            hash: "hash".into(),
        }];
        app.chosen_answer_idx = 1;
        app.ensure_active_session();
        app.record_session_question(true);
        app.process(AppEvent::QuizDone {
            score: 80,
            scores: vec![],
        });
        assert_eq!(app.sessions[0].status, SessionStatus::Passed);
        assert_eq!(app.sessions[0].completed_questions, 1);
        assert!(app.sessions[0].finished_at.is_some());
    }

    #[test]
    fn leaving_quiz_records_meaningful_interruption() {
        let mut app = test_app();
        app.question_num = 1;
        app.question_text = "测试题目".into();
        app.answers = vec![AnswerItem {
            text: "答案".into(),
            hash: "hash".into(),
        }];
        app.chosen_answer_idx = 1;
        app.ensure_active_session();
        app.record_session_question(false);
        app.prev_page.push(Page::Home);
        app.back();
        assert_eq!(app.sessions[0].status, SessionStatus::Interrupted);
        assert_eq!(app.sessions[0].failure_stage.as_deref(), Some("quiz_exit"));
    }
}
