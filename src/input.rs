use crate::app::*;
use crate::config::{self, OpenAiConfig};
use crossterm::event::{KeyCode, KeyModifiers};

const PRESET_COUNT: usize = 4; // must match presets.json length

use crate::app::CaptchaFocus;

impl App {
    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) {
        match self.page {
            Page::Home => self.key_home(key.code),
            Page::Config => self.key_config(key.code),
            Page::Quiz => self.key_quiz(key),
        }
    }

    fn key_home(&mut self, code: KeyCode) {
        match code {
            KeyCode::Up => {
                self.home_sel = match self.home_sel {
                    HomeSelection::StartQuiz => HomeSelection::Quit,
                    HomeSelection::Config => HomeSelection::StartQuiz,
                    HomeSelection::Quit => HomeSelection::Config,
                };
            }
            KeyCode::Down => {
                self.home_sel = match self.home_sel {
                    HomeSelection::StartQuiz => HomeSelection::Config,
                    HomeSelection::Config => HomeSelection::Quit,
                    HomeSelection::Quit => HomeSelection::StartQuiz,
                };
            }
            KeyCode::Enter => match self.home_sel {
                HomeSelection::StartQuiz => self.enter_quiz(),
                HomeSelection::Config => self.enter_config(),
                HomeSelection::Quit => self.quit = true,
            },
            _ => {}
        }
    }

    fn key_config(&mut self, code: KeyCode) {
        // Handle preset selection overlay when active
        if self.cfg_preset_open {
            match code {
                KeyCode::Up => {
                    self.cfg_preset_sel = self.cfg_preset_sel.saturating_sub(1);
                }
                KeyCode::Down => {
                    self.cfg_preset_sel = (self.cfg_preset_sel + 1).min(PRESET_COUNT - 1);
                }
                KeyCode::Enter => {
                    let presets = config::load_presets();
                    if let Some(preset) = presets.get(self.cfg_preset_sel) {
                        self.cfg_fields[0] = preset.config.base_url.clone();
                        self.cfg_fields[1] = preset.config.model.clone();
                        self.cfg_cursors[0] = self.cfg_fields[0].len();
                        self.cfg_cursors[1] = self.cfg_fields[1].len();
                    }
                    self.cfg_preset_open = false;
                }
                KeyCode::Esc => {
                    self.cfg_preset_open = false;
                }
                _ => {}
            }
            return;
        }

        // Handle confirmation dialog when active
        if self.config_confirm_reset {
            match code {
                KeyCode::Up => {
                    self.config_reset_choice = self.config_reset_choice.saturating_sub(1);
                }
                KeyCode::Down => {
                    self.config_reset_choice = (self.config_reset_choice + 1).min(2);
                }
                KeyCode::Enter => {
                    match self.config_reset_choice {
                        0 => self.config_confirm_reset = false,
                        1 => self.logout_only(),
                        _ => self.reset_all(),
                    }
                }
                KeyCode::Esc => {
                    self.config_confirm_reset = false;
                }
                _ => {}
            }
            return;
        }

        let field_idx = match self.cfg_focus {
            ConfigFocus::BaseUrl => Some(0),
            ConfigFocus::Model => Some(1),
            ConfigFocus::ApiKey => Some(2),
            ConfigFocus::ThinkingToggle
            | ConfigFocus::FastModeToggle
            | ConfigFocus::SaveBtn
            | ConfigFocus::TemplateBtn
            | ConfigFocus::ResetBtn => None,
        };

        match code {
            KeyCode::Esc => self.back(),
            KeyCode::Enter => match self.cfg_focus {
                ConfigFocus::SaveBtn => self.save_config(),
                ConfigFocus::ResetBtn => {
                    self.config_confirm_reset = true;
                    self.config_reset_choice = 0;
                }
                ConfigFocus::ThinkingToggle => self.cfg_thinking = !self.cfg_thinking,
                ConfigFocus::FastModeToggle => self.cfg_fast_mode = !self.cfg_fast_mode,
                ConfigFocus::TemplateBtn => {
                    self.cfg_preset_open = true;
                    self.cfg_preset_sel = 0;
                }
                _ => {}
            },
            KeyCode::Backspace => {
                if let Some(idx) = field_idx {
                    let pos = self.cfg_cursors[idx];
                    if pos > 0 {
                        self.cfg_fields[idx].remove(pos - 1);
                        self.cfg_cursors[idx] -= 1;
                    }
                }
            }
            KeyCode::Left => {
                if let Some(idx) = field_idx
                    && self.cfg_cursors[idx] > 0
                {
                    self.cfg_cursors[idx] -= 1;
                }
            }
            KeyCode::Right => {
                if let Some(idx) = field_idx
                    && self.cfg_cursors[idx] < self.cfg_fields[idx].len()
                {
                    self.cfg_cursors[idx] += 1;
                }
            }
            KeyCode::Down => {
                self.cfg_focus = match self.cfg_focus {
                    ConfigFocus::BaseUrl => ConfigFocus::Model,
                    ConfigFocus::Model => ConfigFocus::ApiKey,
                    ConfigFocus::ApiKey => ConfigFocus::ThinkingToggle,
                    ConfigFocus::ThinkingToggle => ConfigFocus::FastModeToggle,
                    ConfigFocus::FastModeToggle => ConfigFocus::SaveBtn,
                    ConfigFocus::SaveBtn => ConfigFocus::TemplateBtn,
                    ConfigFocus::TemplateBtn => ConfigFocus::ResetBtn,
                    ConfigFocus::ResetBtn => ConfigFocus::BaseUrl,
                };
            }
            KeyCode::Up => {
                self.cfg_focus = match self.cfg_focus {
                    ConfigFocus::BaseUrl => ConfigFocus::ResetBtn,
                    ConfigFocus::Model => ConfigFocus::BaseUrl,
                    ConfigFocus::ApiKey => ConfigFocus::Model,
                    ConfigFocus::FastModeToggle => ConfigFocus::ThinkingToggle,
                    ConfigFocus::ThinkingToggle => ConfigFocus::ApiKey,
                    ConfigFocus::SaveBtn => ConfigFocus::FastModeToggle,
                    ConfigFocus::TemplateBtn => ConfigFocus::SaveBtn,
                    ConfigFocus::ResetBtn => ConfigFocus::TemplateBtn,
                };
            }
            KeyCode::Char(' ')
                if self.cfg_focus == ConfigFocus::ThinkingToggle
                    || self.cfg_focus == ConfigFocus::FastModeToggle =>
            {
                match self.cfg_focus {
                    ConfigFocus::ThinkingToggle => self.cfg_thinking = !self.cfg_thinking,
                    ConfigFocus::FastModeToggle => self.cfg_fast_mode = !self.cfg_fast_mode,
                    _ => {}
                }
            }
            KeyCode::Char(c) => {
                if let Some(idx) = field_idx {
                    let pos = self.cfg_cursors[idx];
                    self.cfg_fields[idx].insert(pos, c);
                    self.cfg_cursors[idx] += 1;
                }
            }
            _ => {}
        }
    }

    fn key_quiz(&mut self, key: crossterm::event::KeyEvent) {
        let code = key.code;
        match &self.phase {
            QuizPhase::NotConfigured => match code {
                KeyCode::Enter => self.enter_config(),
                KeyCode::Esc => self.back(),
                _ => {}
            },
            QuizPhase::LoginTimeout { retry } => match code {
                KeyCode::Up => {
                    self.phase = QuizPhase::LoginTimeout { retry: true };
                }
                KeyCode::Down => {
                    self.phase = QuizPhase::LoginTimeout { retry: false };
                }
                KeyCode::Enter => {
                    if *retry {
                        self.spawn_login();
                    } else {
                        self.back();
                    }
                }
                _ => {}
            },
            QuizPhase::WaitingScan { url, .. } => match code {
                KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.spawn_login();
                }
                KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    let qr_url = format!(
                        "https://api.cl2wm.cn/api/qrcode/code?text={}",
                        urlencoding::encode(url)
                    );
                    let _ = webbrowser::open(&qr_url);
                }
                KeyCode::Esc => self.back(),
                _ => {}
            },
            QuizPhase::Captcha(_) => self.key_captcha(key),
            QuizPhase::Finished { .. } | QuizPhase::Error(_) => {
                if matches!(code, KeyCode::Enter | KeyCode::Esc) {
                    self.back();
                }
            }
            _ => match code {
                KeyCode::Esc => self.back(),
                KeyCode::Down => {
                    self.history_scroll = self.history_scroll.saturating_add(1);
                }
                KeyCode::Up => {
                    self.history_scroll = self.history_scroll.saturating_sub(1);
                }
                _ => {}
            },
        }
    }

    fn key_captcha(&mut self, key: crossterm::event::KeyEvent) {
        let code = key.code;
        let cs = match std::mem::replace(&mut self.phase, QuizPhase::NotConfigured) {
            QuizPhase::Captcha(cs) => cs,
            other => {
                self.phase = other;
                return;
            }
        };

        let cs = match code {
            KeyCode::Esc => {
                self.back();
                self.phase = QuizPhase::NotConfigured;
                return;
            }
            // Up arrow navigation
            KeyCode::Up if matches!(cs.focus, CaptchaFocus::Categories) && cs.cat_focus > 0 => {
                CaptchaState {
                    cat_focus: cs.cat_focus - 1,
                    error: String::new(),
                    ..cs
                }
            }
            KeyCode::Up if matches!(cs.focus, CaptchaFocus::Categories) => CaptchaState {
                focus: CaptchaFocus::Submit,
                error: String::new(),
                ..cs
            },
            KeyCode::Up if matches!(cs.focus, CaptchaFocus::Input) => CaptchaState {
                focus: CaptchaFocus::Categories,
                error: String::new(),
                ..cs
            },
            KeyCode::Up if matches!(cs.focus, CaptchaFocus::Submit) => CaptchaState {
                focus: CaptchaFocus::Input,
                error: String::new(),
                ..cs
            },
            // Down arrow navigation
            KeyCode::Down
                if matches!(cs.focus, CaptchaFocus::Categories)
                    && cs.cat_focus < cs.categories.len().saturating_sub(1) =>
            {
                CaptchaState {
                    cat_focus: cs.cat_focus + 1,
                    error: String::new(),
                    ..cs
                }
            }
            KeyCode::Down if matches!(cs.focus, CaptchaFocus::Categories) => CaptchaState {
                focus: CaptchaFocus::Input,
                error: String::new(),
                ..cs
            },
            KeyCode::Down if matches!(cs.focus, CaptchaFocus::Input) => CaptchaState {
                focus: CaptchaFocus::Submit,
                error: String::new(),
                ..cs
            },
            KeyCode::Down if matches!(cs.focus, CaptchaFocus::Submit) => CaptchaState {
                focus: CaptchaFocus::Categories,
                cat_focus: 0,
                error: String::new(),
                ..cs
            },
            // Space toggles category selection (only in Categories focus)
            KeyCode::Char(' ') if matches!(cs.focus, CaptchaFocus::Categories) => {
                let count = cs.categories.iter().filter(|c| c.selected).count();
                let mut cats = cs.categories;
                if cs.cat_focus < cats.len() {
                    if cats[cs.cat_focus].selected {
                        cats[cs.cat_focus].selected = false;
                    } else if count < 3 {
                        cats[cs.cat_focus].selected = true;
                    }
                }
                CaptchaState {
                    categories: cats,
                    error: String::new(),
                    ..cs
                }
            }
            // Refresh captcha (Ctrl+R works everywhere, preserves selections)
            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                let selected: Vec<bool> = cs.categories.iter().map(|c| c.selected).collect();
                self.captcha_preserve = Some((selected, cs.cat_focus, cs.focus, String::new()));
                self.captcha_image = None;
                self.spawn_fetch_captcha();
                self.phase = QuizPhase::FetchingQuestion;
                return;
            }
            // Ctrl+B: open captcha in browser (must be before generic Char(c))
            KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                let _ = webbrowser::open(&cs.captcha_url);
                cs
            }
            // Character input (only in Input focus)
            KeyCode::Char(c) if matches!(cs.focus, CaptchaFocus::Input) => {
                let mut input = cs.input;
                input.push(c);
                CaptchaState { input, error: String::new(), ..cs }
            }
            // Backspace (only in Input focus)
            KeyCode::Backspace if matches!(cs.focus, CaptchaFocus::Input) => {
                let mut input = cs.input;
                input.pop();
                CaptchaState { input, error: String::new(), ..cs }
            }
            // Enter on Submit: try to submit with error feedback
            KeyCode::Enter if matches!(cs.focus, CaptchaFocus::Submit) => {
                let ids: String = cs
                    .categories
                    .iter()
                    .filter(|c| c.selected)
                    .map(|c| c.id.to_string())
                    .collect::<Vec<_>>()
                    .join(",");
                if cs.input.is_empty() && ids.is_empty() {
                    CaptchaState { error: "请选择分类并输入验证码".into(), ..cs }
                } else if cs.input.is_empty() {
                    CaptchaState { error: "请输入验证码".into(), ..cs }
                } else if ids.is_empty() {
                    CaptchaState { error: "请选择分类".into(), ..cs }
                } else {
                    self.selected_categories = cs
                        .categories
                        .iter()
                        .filter(|c| c.selected)
                        .map(|c| c.name.clone())
                        .collect();
                    let _ = config::save_categories(&self.selected_categories);
                    self.spawn_captcha_submit(&cs.input, &cs.captcha_token, &ids);
                    self.phase = QuizPhase::FetchingQuestion;
                    return;
                }
            }
            // Enter on non-Submit: do nothing
            KeyCode::Enter => cs,
            _ => cs,
        };

        self.phase = QuizPhase::Captcha(cs);
    }

    // --- Navigation actions ---

    fn enter_config(&mut self) {
        let is_first_time = self.config.is_none();
        if let Some(ref c) = self.config {
            self.cfg_fields = [c.base_url.clone(), c.model.clone(), c.api_key.clone()];
        }
        self.cfg_cursors = [
            self.cfg_fields[0].len(),
            self.cfg_fields[1].len(),
            self.cfg_fields[2].len(),
        ];
        self.cfg_focus = ConfigFocus::BaseUrl;
        // Auto-open preset selection for first-time users (no existing config)
        if is_first_time {
            self.cfg_preset_open = true;
            self.cfg_preset_sel = 0;
        }
        self.go(Page::Config);
    }

    fn save_config(&mut self) {
        let base = self.cfg_fields[0].trim_end_matches('/').to_string();
        let model = self.cfg_fields[1].clone();
        let key = self.cfg_fields[2].clone();
        if base.is_empty() || model.is_empty() || key.is_empty() {
            return;
        }
        let cfg = OpenAiConfig {
            base_url: base,
            model,
            api_key: key,
            enable_thinking: self.cfg_thinking,
            enable_fast_mode: self.cfg_fast_mode,
        };
        let _ = crate::config::save_openai_config(&cfg).map_err(|e| tracing::error!("{}", e));
        self.config = Some(cfg);
        self.back();
        // 保存配置后如果回到答题页，重新启动答题流程
        if self.page == Page::Quiz {
            self.spawn_login();
        }
    }

    fn enter_quiz(&mut self) {
        self.go(Page::Quiz);
        if self.config.is_none() {
            self.phase = QuizPhase::NotConfigured;
            return;
        }
        self.spawn_login();
    }
}
