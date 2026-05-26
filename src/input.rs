use crate::app::*;
use crate::config::OpenAiConfig;
use crossterm::event::KeyCode;

use crate::app::CaptchaFocus;

impl App {
    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) {
        match self.page {
            Page::Home => self.key_home(key.code),
            Page::Config => self.key_config(key.code),
            Page::Quiz => self.key_quiz(key.code),
        }
    }

    fn key_home(&mut self, code: KeyCode) {
        match code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.home_sel = match self.home_sel {
                    HomeSelection::StartQuiz => HomeSelection::Quit,
                    HomeSelection::Config => HomeSelection::StartQuiz,
                    HomeSelection::Quit => HomeSelection::Config,
                };
            }
            KeyCode::Down | KeyCode::Char('j') => {
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
        // Handle confirmation dialog when active
        if self.config_confirm_reset {
            match code {
                KeyCode::Left | KeyCode::Up => {
                    self.config_reset_choice = false;
                }
                KeyCode::Right | KeyCode::Down => {
                    self.config_reset_choice = true;
                }
                KeyCode::Enter => {
                    if self.config_reset_choice {
                        self.reset_all();
                    } else {
                        self.config_confirm_reset = false;
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
            ConfigFocus::SaveBtn | ConfigFocus::ResetBtn => None,
        };

        match code {
            KeyCode::Esc => self.back(),
            KeyCode::Enter => match self.cfg_focus {
                ConfigFocus::SaveBtn => self.save_config(),
                ConfigFocus::ResetBtn => {
                    self.config_confirm_reset = true;
                    self.config_reset_choice = false;
                }
                _ => {}
            },
            KeyCode::Tab => {
                self.cfg_focus = match self.cfg_focus {
                    ConfigFocus::BaseUrl => ConfigFocus::Model,
                    ConfigFocus::Model => ConfigFocus::ApiKey,
                    ConfigFocus::ApiKey => ConfigFocus::SaveBtn,
                    ConfigFocus::SaveBtn => ConfigFocus::ResetBtn,
                    ConfigFocus::ResetBtn => ConfigFocus::BaseUrl,
                };
            }
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
                    ConfigFocus::ApiKey => ConfigFocus::SaveBtn,
                    ConfigFocus::SaveBtn => ConfigFocus::ResetBtn,
                    ConfigFocus::ResetBtn => ConfigFocus::BaseUrl,
                };
            }
            KeyCode::Up => {
                self.cfg_focus = match self.cfg_focus {
                    ConfigFocus::BaseUrl => ConfigFocus::ResetBtn,
                    ConfigFocus::Model => ConfigFocus::BaseUrl,
                    ConfigFocus::ApiKey => ConfigFocus::Model,
                    ConfigFocus::SaveBtn => ConfigFocus::ApiKey,
                    ConfigFocus::ResetBtn => ConfigFocus::SaveBtn,
                };
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

    fn key_quiz(&mut self, code: KeyCode) {
        match &self.phase {
            QuizPhase::NotConfigured => match code {
                KeyCode::Enter => self.enter_config(),
                KeyCode::Esc => self.back(),
                _ => {}
            },
            QuizPhase::LoginTimeout { retry } => match code {
                KeyCode::Left | KeyCode::Up | KeyCode::Char('h') | KeyCode::Char('k') => {
                    self.phase = QuizPhase::LoginTimeout { retry: true };
                }
                KeyCode::Right | KeyCode::Down | KeyCode::Char('l') | KeyCode::Char('j') => {
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
            QuizPhase::Captcha(_) => self.key_captcha(code),
            QuizPhase::Finished { .. } | QuizPhase::Error(_) => {
                if matches!(code, KeyCode::Enter | KeyCode::Esc) {
                    self.back();
                }
            }
            _ => {
                if matches!(code, KeyCode::Esc) {
                    self.back();
                }
            }
        }
    }

    fn key_captcha(&mut self, code: KeyCode) {
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
            KeyCode::Tab => CaptchaState {
                focus: match cs.focus {
                    CaptchaFocus::Categories => CaptchaFocus::Input,
                    CaptchaFocus::Input => CaptchaFocus::Submit,
                    CaptchaFocus::Submit => CaptchaFocus::Categories,
                },
                error: String::new(),
                ..cs
            },
            // Up arrow navigation
            KeyCode::Up if matches!(cs.focus, CaptchaFocus::Categories) && cs.cat_focus > 0 => {
                CaptchaState {
                    cat_focus: cs.cat_focus - 1,
                    error: String::new(),
                    ..cs
                }
            }
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
            // Down on Submit: stay on Submit
            KeyCode::Down if matches!(cs.focus, CaptchaFocus::Submit) => cs,
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
            // Refresh captcha (only when NOT in Input focus)
            KeyCode::Char('r') if !matches!(cs.focus, CaptchaFocus::Input) => {
                self.captcha_image = None;
                self.spawn_fetch_captcha();
                self.phase = QuizPhase::FetchingQuestion;
                return;
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
        if let Some(ref c) = self.config {
            self.cfg_fields = [c.base_url.clone(), c.model.clone(), c.api_key.clone()];
        }
        self.cfg_cursors = [
            self.cfg_fields[0].len(),
            self.cfg_fields[1].len(),
            self.cfg_fields[2].len(),
        ];
        self.cfg_focus = ConfigFocus::BaseUrl;
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
