use crate::app::{App, CaptchaState, Page, QuizPhase, SessionHistory, SessionStatus};
use crate::config::{self, OpenAiConfig};
use eframe::egui::{
    self, Color32, ColorImage, FontFamily, FontId, RichText, Stroke, TextureHandle, TextureOptions,
    WidgetText,
};
use lucide_icons::Icon;
use std::time::{Duration, Instant};

const CANVAS: Color32 = Color32::from_rgb(247, 248, 250);
const TOPBAR: Color32 = Color32::from_rgb(255, 255, 255);
const SURFACE: Color32 = Color32::from_rgb(255, 255, 255);
const SURFACE_RAISED: Color32 = Color32::from_rgb(241, 244, 247);
const BORDER: Color32 = Color32::from_rgb(227, 231, 236);
const ACCENT: Color32 = Color32::from_rgb(232, 112, 135);
const ACCENT_HOVER: Color32 = Color32::from_rgb(217, 95, 120);
const TEXT: Color32 = Color32::from_rgb(32, 36, 43);
const MUTED: Color32 = Color32::from_rgb(102, 112, 125);
const SUBTLE: Color32 = Color32::from_rgb(146, 154, 165);
const TEAL: Color32 = Color32::from_rgb(58, 159, 152);
const SUCCESS: Color32 = Color32::from_rgb(52, 152, 117);
const WARNING: Color32 = Color32::from_rgb(197, 138, 50);
const DANGER: Color32 = Color32::from_rgb(212, 95, 104);

fn medium(text: impl Into<String>, size: f32, color: Color32) -> RichText {
    RichText::new(text)
        .size(size)
        .family(FontFamily::Name("noto-medium".into()))
        .color(color)
}

fn bold(text: impl Into<String>, size: f32, color: Color32) -> RichText {
    RichText::new(text)
        .size(size)
        .family(FontFamily::Name("noto-bold".into()))
        .color(color)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GuiView {
    Home,
    History,
    HistoryDetail,
    Settings,
}

pub fn run(config: Option<OpenAiConfig>) -> Result<(), Box<dyn std::error::Error>> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Bili-Hardcore")
            .with_icon(app_icon())
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([900.0, 650.0])
            .with_decorations(false),
        ..Default::default()
    };
    eframe::run_native(
        "Bili-Hardcore",
        options,
        Box::new(move |cc| Ok(Box::new(GuiApp::new(cc, config)))),
    )
    .map_err(|error| error.to_string().into())
}

fn app_icon() -> egui::IconData {
    let image = image::load_from_memory(include_bytes!("../assets/app-icon.png"))
        .expect("embedded application icon must be a valid PNG")
        .into_rgba8();
    let (width, height) = image.dimensions();
    egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    }
}

struct GuiApp {
    app: App,
    last_tick: Instant,
    config_error: Option<String>,
    captcha_texture: Option<(String, TextureHandle)>,
    qr_texture: Option<(String, TextureHandle)>,
    view: GuiView,
    selected_session: Option<usize>,
    exit_quiz_confirm: bool,
    logout_confirm: bool,
    config_saved_until: Option<Instant>,
    scene_key: u64,
    scene_started: Instant,
}

impl GuiApp {
    fn new(cc: &eframe::CreationContext<'_>, config: Option<OpenAiConfig>) -> Self {
        install_fonts(&cc.egui_ctx);
        configure_style(&cc.egui_ctx);
        let mut result = Self {
            app: App::new(config, None),
            last_tick: Instant::now(),
            config_error: None,
            captcha_texture: None,
            qr_texture: None,
            view: GuiView::Home,
            selected_session: None,
            exit_quiz_confirm: false,
            logout_confirm: false,
            config_saved_until: None,
            scene_key: 0,
            scene_started: Instant::now(),
        };
        result.scene_key = result.current_scene_key();
        result
    }

    fn pump(&mut self) {
        while self.last_tick.elapsed() >= Duration::from_millis(100) {
            self.app.tick();
            self.last_tick += Duration::from_millis(100);
        }
        while let Ok(event) = self.app.rx.try_recv() {
            self.app.process(event);
        }
        if self.view == GuiView::Settings && self.app.page == Page::Home {
            self.view = GuiView::Home;
        }
        let scene_key = self.current_scene_key();
        if scene_key != self.scene_key {
            self.scene_key = scene_key;
            self.scene_started = Instant::now();
        }
    }

    fn current_scene_key(&self) -> u64 {
        let view = match self.view {
            GuiView::Home => 1,
            GuiView::History => 2,
            GuiView::HistoryDetail => 3,
            GuiView::Settings => 4,
        };
        let page = match self.app.page {
            Page::Home => 10,
            Page::Config => 20,
            Page::Quiz => 30,
        };
        let phase = match &self.app.phase {
            QuizPhase::NotConfigured => 1,
            QuizPhase::LoggingIn | QuizPhase::WaitingScan { .. } => 2,
            QuizPhase::LoginTimeout { .. } => 3,
            QuizPhase::CheckingLevel => 4,
            QuizPhase::LevelVerified { .. } => 5,
            QuizPhase::LevelInsufficient { .. } | QuizPhase::LevelCheckFailed(_) => 6,
            QuizPhase::FetchingQuestion => 7,
            QuizPhase::Captcha(_) => 8,
            QuizPhase::WaitingLlm | QuizPhase::WaitingRetry { .. } => 9,
            QuizPhase::Submitting | QuizPhase::ShowingResult { .. } => 10,
            QuizPhase::Finished { .. } => 11,
            QuizPhase::Error(_) => 12,
        };
        page + view * 100 + phase * 1_000
    }

    fn shell(&mut self, root: &mut egui::Ui) {
        let ctx = root.ctx().clone();
        let answering = self.app.page == Page::Quiz
            && matches!(
                self.app.phase,
                QuizPhase::WaitingLlm
                    | QuizPhase::WaitingRetry { .. }
                    | QuizPhase::Submitting
                    | QuizPhase::ShowingResult { .. }
            );
        let root_item_spacing = root.spacing().item_spacing;
        root.spacing_mut().item_spacing.y = 0.0;
        egui::Frame::NONE
            .fill(TOPBAR)
            .inner_margin(egui::Margin::symmetric(16, 8))
            .stroke(Stroke::new(1.0, BORDER))
            .show(root, |ui| {
                ui.set_min_height(43.0);
                ui.horizontal(|ui| {
                    egui::Frame::NONE
                        .fill(ACCENT.gamma_multiply(0.12))
                        .stroke(Stroke::new(1.0, Color32::from_rgb(244, 192, 201)))
                        .corner_radius(9)
                        .show(ui, |ui| {
                            ui.add_sized([30.0, 30.0], egui::Label::new(bold("B", 16.0, ACCENT)));
                        });
                    ui.add_space(2.0);
                    ui.vertical(|ui| {
                        ui.label(bold("Bili-Hardcore", 14.0, TEXT));
                        ui.label(RichText::new("硬核会员助手").size(10.0).color(SUBTLE));
                    });
                    if !answering {
                        ui.add_space(24.0);
                        if nav_button(
                            ui,
                            Icon::House,
                            "首页",
                            self.app.page == Page::Home && self.view == GuiView::Home,
                        )
                        .clicked()
                        {
                            self.navigate(GuiView::Home);
                        }
                        if nav_button(
                            ui,
                            Icon::History,
                            "历史记录",
                            self.app.page == Page::Home
                                && matches!(self.view, GuiView::History | GuiView::HistoryDetail),
                        )
                        .clicked()
                        {
                            self.navigate(GuiView::History);
                        }
                        if nav_button(
                            ui,
                            Icon::Settings,
                            "设置",
                            self.app.page == Page::Config || self.view == GuiView::Settings,
                        )
                        .clicked()
                        {
                            self.navigate(GuiView::Settings);
                        }
                    } else {
                        ui.add_space(18.0);
                        status_pill(ui, phase_label(&self.app.phase), ACCENT);
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if window_control(ui, Icon::X, "关闭", true).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        let maximized =
                            ui.input(|input| input.viewport().maximized.unwrap_or(false));
                        if window_control(
                            ui,
                            if maximized {
                                Icon::Minimize2
                            } else {
                                Icon::Maximize2
                            },
                            if maximized { "还原" } else { "最大化" },
                            false,
                        )
                        .clicked()
                        {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
                        }
                        if window_control(ui, Icon::Minus, "最小化", false).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                        }
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                                .size(11.0)
                                .color(SUBTLE),
                        );
                        ui.separator();
                        let ready = self.app.config.is_some();
                        let color = if ready { TEAL } else { WARNING };
                        let label = if ready {
                            "模型已配置"
                        } else {
                            "点击配置模型"
                        };
                        if ui
                            .add(
                                egui::Button::new(icon_text(
                                    if ready {
                                        Icon::CircleCheck
                                    } else {
                                        Icon::CircleAlert
                                    },
                                    label,
                                    12.0,
                                    color,
                                ))
                                .fill(Color32::TRANSPARENT)
                                .stroke(Stroke::NONE),
                            )
                            .clicked()
                        {
                            self.navigate(GuiView::Settings);
                        }
                        let drag_width = ui.available_width().max(0.0);
                        if drag_width > 8.0 {
                            let (_, titlebar) = ui.allocate_exact_size(
                                egui::vec2(drag_width, 36.0),
                                egui::Sense::click_and_drag(),
                            );
                            if titlebar.double_clicked() {
                                let maximized =
                                    ui.input(|input| input.viewport().maximized.unwrap_or(false));
                                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
                            } else if titlebar.drag_started_by(egui::PointerButton::Primary) {
                                ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                            }
                        }
                    });
                });
            });
        let remaining = root.available_size();
        egui::Frame::NONE
            .fill(CANVAS)
            .inner_margin(egui::Margin::ZERO)
            .show(root, |ui| {
                ui.set_min_size(remaining);
                let progress = (self.scene_started.elapsed().as_secs_f32() / 0.18).clamp(0.0, 1.0);
                let eased = 1.0 - (1.0 - progress).powi(3);
                ui.set_opacity(0.58 + eased * 0.42);
                ui.add_space((1.0 - eased) * 8.0);
                match self.app.page {
                    Page::Quiz => self.quiz(ui, &ctx),
                    Page::Config => self.config(ui),
                    Page::Home => match self.view {
                        GuiView::Home => self.home(ui, &ctx),
                        GuiView::History => self.history(ui),
                        GuiView::HistoryDetail => self.history_detail(ui),
                        GuiView::Settings => self.config(ui),
                    },
                }
            });
        root.spacing_mut().item_spacing = root_item_spacing;
        borderless_resize_handles(&ctx, root.max_rect());
        self.exit_quiz_dialog(&ctx);
    }

    fn navigate(&mut self, view: GuiView) {
        self.selected_session = if view == GuiView::HistoryDetail {
            self.selected_session
        } else {
            None
        };
        if self.app.page == Page::Quiz {
            self.app.back();
        }
        self.view = view;
        match view {
            GuiView::Settings => {
                if self.app.page != Page::Config {
                    self.app.enter_config();
                }
            }
            _ => {
                if self.app.page == Page::Config {
                    self.app.back();
                }
                self.app.page = Page::Home;
            }
        }
    }

    fn home(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context) {
        let compact = ui.available_width() < 980.0;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let width = (ui.available_width() - 48.0).min(1080.0);
                centered_width(ui, width, |ui| {
                    ui.add_space(32.0);
                    ui.label(bold("准备开始硬核会员试炼", 24.0, TEXT));
                    ui.label(
                        RichText::new("配置大语言模型后，自动完成 100 道题的答题试炼")
                            .size(14.0)
                            .color(MUTED),
                    );
                    ui.add_space(26.0);
                    if compact {
                        home_left_column(ui, &mut self.app);
                        ui.add_space(16.0);
                        if home_history_column(ui, &self.app) {
                            self.navigate(GuiView::History);
                        }
                    } else {
                        const RIGHT_WIDTH: f32 = 360.0;
                        const GAP: f32 = 24.0;
                        let left_width = (width - RIGHT_WIDTH - GAP).max(420.0);
                        ui.horizontal_top(|ui| {
                            ui.allocate_ui_with_layout(
                                egui::vec2(left_width, 0.0),
                                egui::Layout::top_down(egui::Align::Min),
                                |ui| {
                                    ui.set_width(left_width);
                                    home_left_column(ui, &mut self.app);
                                },
                            );
                            ui.add_space(GAP);
                            ui.allocate_ui_with_layout(
                                egui::vec2(RIGHT_WIDTH, 0.0),
                                egui::Layout::top_down(egui::Align::Min),
                                |ui| {
                                    ui.set_width(RIGHT_WIDTH);
                                    if home_history_column(ui, &self.app) {
                                        self.navigate(GuiView::History);
                                    }
                                },
                            );
                        });
                    }
                });
            });
    }

    fn config(&mut self, ui: &mut egui::Ui) {
        let compact = ui.available_width() < 980.0;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let width = (ui.available_width() - 48.0).min(1080.0);
                centered_width(ui, width, |ui| {
                    ui.add_space(32.0);
                    ui.label(bold("设置", 22.0, TEXT));
                    ui.label(
                        RichText::new("配置大语言模型连接凭证与答题策略")
                            .size(14.0)
                            .color(MUTED),
                    );
                    ui.add_space(24.0);
                    if compact {
                        config_credentials(
                            ui,
                            &mut self.app,
                            &mut self.config_error,
                            &mut self.config_saved_until,
                        );
                        ui.add_space(20.0);
                        config_behavior(ui, &mut self.app);
                        ui.add_space(20.0);
                        config_account(ui, &mut self.app, &mut self.logout_confirm);
                    } else {
                        const RIGHT_WIDTH: f32 = 360.0;
                        const GAP: f32 = 24.0;
                        let left_width = width - RIGHT_WIDTH - GAP;
                        ui.horizontal_top(|ui| {
                            ui.allocate_ui_with_layout(
                                egui::vec2(left_width, 0.0),
                                egui::Layout::top_down(egui::Align::Min),
                                |ui| {
                                    config_credentials(
                                        ui,
                                        &mut self.app,
                                        &mut self.config_error,
                                        &mut self.config_saved_until,
                                    )
                                },
                            );
                            ui.add_space(GAP);
                            ui.allocate_ui_with_layout(
                                egui::vec2(RIGHT_WIDTH, 0.0),
                                egui::Layout::top_down(egui::Align::Min),
                                |ui| {
                                    config_behavior(ui, &mut self.app);
                                    ui.add_space(20.0);
                                    config_account(ui, &mut self.app, &mut self.logout_confirm);
                                },
                            );
                        });
                    }
                });
            });
        if self.app.config_confirm_reset {
            egui::Window::new("确认重置")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ui.ctx(), |ui| {
                    ui.set_min_width(360.0);
                    ui.horizontal(|ui| {
                        ui.label(icon_only(Icon::CircleAlert, 22.0, DANGER));
                        ui.vertical(|ui| {
                            ui.label(RichText::new("删除全部本地数据？").strong().color(TEXT));
                            ui.label(
                                RichText::new(
                                    "API 配置、B 站登录信息和本地历史将被清除，此操作不可撤销。",
                                )
                                .color(MUTED),
                            );
                        });
                    });
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        if secondary_button(ui, Icon::ArrowLeft, "取消", [100.0, 36.0]).clicked()
                        {
                            self.app.config_confirm_reset = false;
                        }
                        if ui
                            .add_sized(
                                [120.0, 36.0],
                                egui::Button::new("确认删除").fill(DANGER).corner_radius(6),
                            )
                            .clicked()
                        {
                            self.app.reset_all();
                        }
                    });
                });
        }
        if self.logout_confirm {
            egui::Window::new("退出登录")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ui.ctx(), |ui| {
                    ui.set_min_width(380.0);
                    ui.label(
                        RichText::new("退出后需要重新扫码登录才能进行答题试炼。")
                            .size(14.0)
                            .color(MUTED),
                    );
                    ui.add_space(16.0);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if secondary_button(ui, Icon::LogOut, "退出登录", [120.0, 36.0]).clicked()
                        {
                            self.app.logout_only();
                            self.logout_confirm = false;
                        }
                        if secondary_button(ui, Icon::X, "取消", [92.0, 36.0]).clicked() {
                            self.logout_confirm = false;
                        }
                    });
                });
        }
    }

    fn quiz(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let phase = self.app.phase.clone();
        if matches!(
            phase,
            QuizPhase::WaitingLlm
                | QuizPhase::WaitingRetry { .. }
                | QuizPhase::Submitting
                | QuizPhase::ShowingResult { .. }
        ) {
            self.question(ui);
            return;
        }
        egui::Frame::NONE
            .inner_margin(egui::Margin::symmetric(24, 24))
            .show(ui, |ui| match phase {
                QuizPhase::NotConfigured => self.not_configured(ui),
                QuizPhase::LoggingIn => status(ui, "正在准备登录…"),
                QuizPhase::CheckingLevel => status(ui, "正在验证用户等级…"),
                QuizPhase::LevelVerified { level, .. } => self.level_verified(ui, level),
                QuizPhase::LevelInsufficient { level } => self.level_insufficient(ui, level),
                QuizPhase::LevelCheckFailed(message) => self.level_check_failed(ui, &message),
                QuizPhase::FetchingQuestion => status(ui, "正在获取题目…"),
                QuizPhase::WaitingScan { url, countdown, .. } => {
                    self.login(ui, ctx, &url, countdown)
                }
                QuizPhase::LoginTimeout { .. } => self.login_timeout(ui),
                QuizPhase::Captcha(state) => self.captcha(ui, ctx, &state),
                QuizPhase::WaitingLlm
                | QuizPhase::WaitingRetry { .. }
                | QuizPhase::Submitting
                | QuizPhase::ShowingResult { .. } => unreachable!(),
                QuizPhase::Finished { score, scores } => self.finished(ui, score, &scores),
                QuizPhase::Error(message) => self.error(ui, &message),
            });
    }

    fn level_verified(&mut self, ui: &mut egui::Ui, level: i64) {
        state_panel(
            ui,
            Icon::BadgeCheck,
            "账号等级验证通过",
            &format!("当前等级 {level}，符合硬核会员试炼要求"),
            SUCCESS,
            |ui| {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(RichText::new("正在进入下一步…").color(MUTED));
                });
            },
        );
    }

    fn level_insufficient(&mut self, ui: &mut egui::Ui, level: i64) {
        state_panel(
            ui,
            Icon::ShieldAlert,
            "账号等级不足",
            &format!("当前等级 {level}，参与试炼需要达到 6 级"),
            WARNING,
            |ui| {
                if secondary_button(ui, Icon::House, "返回首页", [132.0, 40.0]).clicked() {
                    self.app.back();
                }
                if primary_button(ui, Icon::LogIn, "重新登录", [132.0, 40.0]).clicked() {
                    self.app.relogin_for_quiz();
                }
            },
        );
    }

    fn level_check_failed(&mut self, ui: &mut egui::Ui, message: &str) {
        state_panel(
            ui,
            Icon::CircleAlert,
            "账号等级验证失败",
            message,
            DANGER,
            |ui| {
                if primary_button(ui, Icon::RefreshCw, "重新验证", [132.0, 40.0]).clicked() {
                    self.app.retry_level_check();
                }
                if secondary_button(ui, Icon::LogIn, "重新登录", [132.0, 40.0]).clicked() {
                    self.app.relogin_for_quiz();
                }
            },
        );
    }

    fn not_configured(&mut self, ui: &mut egui::Ui) {
        state_panel(
            ui,
            Icon::Settings2,
            "尚未配置大语言模型",
            "完成 API 配置后即可开始答题。",
            WARNING,
            |ui| {
                if primary_button(ui, Icon::Settings, "前往配置", [140.0, 40.0]).clicked() {
                    self.app.enter_config();
                }
            },
        );
    }

    fn login(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, url: &str, countdown: u32) {
        let width = ui.available_width().min(800.0);
        centered_width(ui, width, |ui| {
            ui.label(bold("扫码登录", 22.0, TEXT));
            ui.label(
                RichText::new("使用哔哩哔哩 App 扫描二维码完成授权")
                    .size(14.0)
                    .color(MUTED),
            );
            ui.add_space(24.0);
            ui.horizontal_top(|ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(280.0, 0.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        surface().show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                if self.qr_texture.as_ref().map(|(key, _)| key.as_str())
                                    != Some(url)
                                {
                                    self.qr_texture = qr_image(url).map(|image| {
                                        (
                                            url.to_owned(),
                                            ctx.load_texture(
                                                "login-qr",
                                                image,
                                                TextureOptions::NEAREST,
                                            ),
                                        )
                                    });
                                }
                                if let Some((_, texture)) = &self.qr_texture {
                                    egui::Frame::NONE
                                        .fill(Color32::WHITE)
                                        .stroke(Stroke::new(1.0, BORDER))
                                        .corner_radius(10)
                                        .show(ui, |ui| {
                                            ui.image((texture.id(), egui::vec2(240.0, 240.0)));
                                        });
                                }
                                ui.add_space(12.0);
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(format!("{countdown} 秒后过期"))
                                            .size(12.0)
                                            .color(if countdown < 15 { WARNING } else { SUBTLE }),
                                    );
                                    if ui
                                        .button(icon_text(Icon::RefreshCw, "刷新", 12.0, MUTED))
                                        .clicked()
                                    {
                                        self.app.spawn_login();
                                    }
                                    if ui
                                        .button(icon_text(
                                            Icon::ExternalLink,
                                            "浏览器打开",
                                            12.0,
                                            MUTED,
                                        ))
                                        .clicked()
                                    {
                                        let qr_url = format!(
                                            "https://api.cl2wm.cn/api/qrcode/code?text={}",
                                            urlencoding::encode(url)
                                        );
                                        let _ = webbrowser::open(&qr_url);
                                    }
                                });
                            });
                        })
                    },
                );
                ui.add_space(24.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(width - 304.0, 0.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        surface().show(ui, |ui| {
                            ui.label(RichText::new("登录步骤").size(15.0).strong().color(TEXT));
                            ui.add_space(20.0);
                            login_step(ui, "1", "打开哔哩哔哩 App", "");
                            login_step(ui, "2", "扫描二维码并确认授权", "等待扫码中…");
                            login_step(ui, "3", "保持窗口开启，授权后自动继续", "");
                        });
                        ui.add_space(16.0);
                        egui::Frame::NONE
                            .fill(SURFACE_RAISED)
                            .stroke(Stroke::new(1.0, BORDER))
                            .corner_radius(10)
                            .inner_margin(egui::Margin::symmetric(16, 14))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(icon_only(Icon::CircleAlert, 15.0, WARNING));
                                    ui.label(
                                        RichText::new("账号等级要求：参与试炼需要达到 6 级")
                                            .size(13.0)
                                            .color(TEXT),
                                    );
                                });
                            });
                    },
                );
            });
        });
    }

    fn login_timeout(&mut self, ui: &mut egui::Ui) {
        state_panel(
            ui,
            Icon::CircleAlert,
            "登录二维码已过期",
            "重新获取二维码后再次扫码。",
            WARNING,
            |ui| {
                if primary_button(ui, Icon::RefreshCw, "重新获取", [140.0, 40.0]).clicked() {
                    self.app.spawn_login();
                }
                if secondary_button(ui, Icon::ArrowLeft, "返回首页", [130.0, 40.0]).clicked() {
                    self.app.back();
                }
            },
        );
    }

    fn captcha(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, state: &CaptchaState) {
        let image_key = state.captcha_url.clone();
        if self.captcha_texture.as_ref().map(|(key, _)| key.as_str()) != Some(image_key.as_str()) {
            self.captcha_texture = self.app.captcha_image.as_ref().map(|image| {
                let image = dynamic_image(image);
                (
                    image_key.clone(),
                    ctx.load_texture("captcha", image, TextureOptions::LINEAR),
                )
            });
        }
        let mut toggled = None;
        let mut refresh = false;
        let mut submit = false;
        let mut open = false;
        let selected_count = state.categories.iter().filter(|item| item.selected).count();
        let width = ui.available_width().min(900.0);
        centered_width(ui, width, |ui| {
            ui.label(
                RichText::new("选择答题分类")
                    .size(22.0)
                    .strong()
                    .color(TEXT),
            );
            ui.label(
                RichText::new("选择最多三个分类，并完成验证码验证")
                    .size(14.0)
                    .color(MUTED),
            );
            ui.add_space(24.0);
            let compact = width < 760.0;
            if compact {
                surface().show(ui, |ui| {
                    captcha_categories(ui, state, selected_count, &mut toggled);
                });
                ui.add_space(16.0);
                surface().show(ui, |ui| {
                    captcha_form(
                        ui,
                        ctx,
                        &mut self.app,
                        state,
                        &mut self.captcha_texture,
                        &mut submit,
                        &mut refresh,
                        &mut open,
                    );
                });
            } else {
                ui.horizontal_top(|ui| {
                    ui.allocate_ui_with_layout(
                        egui::vec2(width - 324.0, 0.0),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            surface().show(ui, |ui| {
                                captcha_categories(ui, state, selected_count, &mut toggled);
                            })
                        },
                    );
                    ui.add_space(24.0);
                    ui.allocate_ui_with_layout(
                        egui::vec2(300.0, 0.0),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            surface().show(ui, |ui| {
                                captcha_form(
                                    ui,
                                    ctx,
                                    &mut self.app,
                                    state,
                                    &mut self.captcha_texture,
                                    &mut submit,
                                    &mut refresh,
                                    &mut open,
                                );
                            })
                        },
                    );
                });
            }
        });
        if let Some(index) = toggled {
            self.app.toggle_captcha_category(index);
        }
        if refresh {
            self.app.refresh_captcha();
        }
        if open {
            let _ = webbrowser::open(&state.captcha_url);
        }
        if submit
            && let Err(error) = self.app.submit_captcha()
            && let QuizPhase::Captcha(current) = &mut self.app.phase
        {
            current.error = error;
        }
    }

    fn question(&mut self, ui: &mut egui::Ui) {
        let answered = matches!(self.app.phase, QuizPhase::ShowingResult { .. });
        let progress_target =
            ((self.app.question_num.saturating_sub(1) + u32::from(answered)) as f32 / 100.0)
                .clamp(0.0, 1.0);
        let progress = ui.ctx().animate_value_with_time(
            ui.make_persistent_id("quiz-progress"),
            progress_target,
            0.22,
        );
        egui::Frame::NONE
            .fill(SURFACE)
            .stroke(Stroke::new(1.0, BORDER))
            .inner_margin(egui::Margin::symmetric(24, 12))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(self.app.question_num.to_string())
                                .size(22.0)
                                .strong()
                                .color(TEXT),
                        );
                        ui.label(RichText::new("/ 100").size(14.0).color(SUBTLE));
                    });
                    ui.add_space(10.0);
                    ui.add(
                        egui::ProgressBar::new(progress)
                            .desired_width(200.0)
                            .desired_height(6.0)
                            .fill(ACCENT),
                    );
                    ui.add_space(20.0);
                    metric(ui, "当前得分", &self.app.score.to_string());
                    let accuracy = if self.app.question_num > 1 {
                        self.app.score * 100 / i64::from(self.app.question_num - 1)
                    } else {
                        0
                    };
                    ui.add_space(18.0);
                    metric(ui, "正确率", &format!("{accuracy}%"));
                    ui.add_space(18.0);
                    metric(ui, "已完成", &self.app.history.len().to_string());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        status_pill(ui, phase_label(&self.app.phase), ACCENT);
                    });
                });
            });
        let wide = ui.available_width() >= 900.0;
        if wide {
            let width = ui.available_width();
            ui.horizontal_top(|ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(width - 320.0 - 24.0, 0.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        egui::Frame::NONE
                            .fill(CANVAS)
                            .inner_margin(egui::Margin::same(24))
                            .show(ui, |ui| question_panel(ui, &self.app));
                    },
                );
                ui.allocate_ui_with_layout(
                    egui::vec2(320.0, 0.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        egui::Frame::NONE
                            .fill(SURFACE)
                            .stroke(Stroke::new(1.0, BORDER))
                            .inner_margin(egui::Margin::symmetric(16, 20))
                            .show(ui, |ui| history_panel(ui, &self.app));
                    },
                );
            });
        } else {
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Frame::NONE
                    .fill(CANVAS)
                    .inner_margin(egui::Margin::same(20))
                    .show(ui, |ui| question_panel(ui, &self.app));
                egui::Frame::NONE
                    .fill(SURFACE)
                    .stroke(Stroke::new(1.0, BORDER))
                    .inner_margin(egui::Margin::same(16))
                    .show(ui, |ui| history_panel(ui, &self.app));
            });
        }
    }

    fn finished(&mut self, ui: &mut egui::Ui, score: i64, scores: &[crate::app::ScoreItem]) {
        let passed = score >= 60;
        let color = if passed { TEAL } else { DANGER };
        let latest = self.app.sessions.first().cloned();
        let width = ui.available_width().min(800.0);
        centered_width(ui, width, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                result_header(ui, passed, score, color);
                ui.add_space(20.0);
                let column = (width - 20.0) / 2.0;
                ui.horizontal_top(|ui| {
                    ui.allocate_ui_with_layout(
                        egui::vec2(column, 0.0),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| result_scores(ui, scores),
                    );
                    ui.add_space(20.0);
                    ui.allocate_ui_with_layout(
                        egui::vec2(column, 0.0),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| result_session_info(ui, latest.as_ref()),
                    );
                });
                ui.add_space(20.0);
                result_question_summary(ui, latest.as_ref());
                ui.add_space(20.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if primary_button(ui, Icon::Home, "返回首页", [132.0, 40.0]).clicked() {
                        self.app.back();
                        self.view = GuiView::Home;
                    }
                    if secondary_button(ui, Icon::List, "查看本次详情", [152.0, 40.0]).clicked()
                    {
                        self.app.back();
                        self.selected_session = Some(0);
                        self.view = GuiView::HistoryDetail;
                    }
                });
            });
        });
    }

    fn history(&mut self, ui: &mut egui::Ui) {
        let mut open = None;
        let width = (ui.available_width() - 48.0).min(900.0);
        centered_width(ui, width, |ui| {
            ui.add_space(32.0);
            ui.label(bold("历史记录", 22.0, TEXT));
            ui.label(
                RichText::new("查看每次答题试炼的结果与详情")
                    .size(14.0)
                    .color(MUTED),
            );
            ui.add_space(24.0);
            egui::ScrollArea::vertical().show(ui, |ui| {
                if !self.app.history.is_empty() {
                    inline_alert(
                        ui,
                        Icon::Archive,
                        &format!(
                            "检测到旧版逐题记录：已完成 {}/100。该数据继续用于恢复当前答题，不作为独立场次。",
                            self.app.history.len()
                        ),
                        WARNING,
                    );
                    ui.add_space(12.0);
                }
                if self.app.sessions.is_empty() {
                    ui.add_space(80.0);
                    ui.vertical_centered(|ui| {
                        ui.label(icon_only(Icon::History, 28.0, SUBTLE));
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new("还没有场次记录")
                                .size(17.0)
                                .strong()
                                .color(TEXT),
                        );
                        ui.label(
                            RichText::new("完成或中途退出一次答题后，记录会显示在这里。")
                                .size(13.0)
                                .color(MUTED),
                        );
                    });
                    return;
                }
                for (index, session) in self.app.sessions.iter().enumerate() {
                    if session_row(ui, session).clicked() {
                        open = Some(index);
                    }
                    ui.add_space(10.0);
                }
            });
        });
        if let Some(index) = open {
            self.selected_session = Some(index);
            self.view = GuiView::HistoryDetail;
        }
    }

    fn history_detail(&mut self, ui: &mut egui::Ui) {
        let Some(index) = self.selected_session else {
            self.view = GuiView::History;
            return;
        };
        let Some(session) = self.app.sessions.get(index).cloned() else {
            self.selected_session = None;
            self.view = GuiView::History;
            return;
        };
        let width = (ui.available_width() - 48.0).min(900.0);
        centered_width(ui, width, |ui| {
            ui.add_space(24.0);
            if ui
                .button(icon_text(Icon::ArrowLeft, "返回历史记录", 13.0, MUTED))
                .clicked()
            {
                self.selected_session = None;
                self.view = GuiView::History;
                return;
            }
            ui.add_space(12.0);
            ui.label(bold("场次详情", 20.0, TEXT));
            ui.label(
                RichText::new(format!(
                    "{} · {}",
                    format_time(session.started_at),
                    session.model
                ))
                .size(13.0)
                .color(MUTED),
            );
            ui.add_space(20.0);
            session_summary(ui, &session);
            ui.add_space(12.0);
            surface().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(icon_only(Icon::ListChecks, 19.0, TEAL));
                    ui.label(
                        RichText::new("逐题答题摘要")
                            .size(16.0)
                            .strong()
                            .color(TEXT),
                    );
                });
                ui.add_space(12.0);
                if session.questions.is_empty() {
                    ui.label(RichText::new("此记录没有逐题数据").color(MUTED));
                }
                egui::ScrollArea::vertical()
                    .max_height(360.0)
                    .show(ui, |ui| {
                        for question in &session.questions {
                            question_history_row(ui, question);
                            ui.add_space(8.0);
                        }
                    });
            });
        });
    }

    fn exit_quiz_dialog(&mut self, ctx: &egui::Context) {
        if !self.exit_quiz_confirm {
            return;
        }
        egui::Window::new("退出答题")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_min_width(380.0);
                ui.label(
                    RichText::new("确定停止当前答题任务？")
                        .size(17.0)
                        .strong()
                        .color(TEXT),
                );
                ui.label(
                    RichText::new("后台请求会立即取消，已经完成的题目将保存为异常终止场次。")
                        .size(13.0)
                        .color(MUTED),
                );
                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    if secondary_button(ui, Icon::X, "继续答题", [124.0, 40.0]).clicked() {
                        self.exit_quiz_confirm = false;
                    }
                    if ui
                        .add_sized(
                            [124.0, 40.0],
                            egui::Button::new(icon_text(
                                Icon::LogOut,
                                "退出答题",
                                14.0,
                                Color32::WHITE,
                            ))
                            .fill(DANGER)
                            .corner_radius(8),
                        )
                        .clicked()
                    {
                        self.app.back();
                        self.exit_quiz_confirm = false;
                        self.view = GuiView::Home;
                    }
                });
            });
    }

    fn error(&mut self, ui: &mut egui::Ui, message: &str) {
        state_panel(
            ui,
            Icon::CircleAlert,
            "流程中断",
            message,
            DANGER,
            |ui| {
                if primary_button(ui, Icon::Home, "返回首页", [140.0, 40.0]).clicked() {
                    self.app.back();
                }
            },
        );
    }
}

impl eframe::App for GuiApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.pump();
        self.shell(ui);
        let animating = self.scene_started.elapsed() < Duration::from_millis(220)
            || matches!(
                self.app.phase,
                QuizPhase::LoggingIn
                    | QuizPhase::WaitingScan { .. }
                    | QuizPhase::CheckingLevel
                    | QuizPhase::LevelVerified { .. }
                    | QuizPhase::FetchingQuestion
                    | QuizPhase::WaitingLlm
                    | QuizPhase::WaitingRetry { .. }
                    | QuizPhase::Submitting
            );
        ui.ctx().request_repaint_after(if animating {
            Duration::from_millis(16)
        } else {
            Duration::from_millis(100)
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.app.stop_quiz("应用窗口已关闭", "window_close");
    }
}

fn surface() -> egui::Frame {
    egui::Frame::NONE
        .fill(SURFACE)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(12)
        .inner_margin(egui::Margin::same(20))
}

fn inset_surface() -> egui::Frame {
    egui::Frame::NONE
        .fill(SURFACE_RAISED)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(8)
        .inner_margin(egui::Margin::same(14))
}

fn icon_char(value: Icon) -> char {
    char::from(value)
}

fn icon_only(value: Icon, size: f32, color: Color32) -> RichText {
    RichText::new(icon_char(value).to_string())
        .font(FontId::new(size, FontFamily::Name("lucide".into())))
        .color(color)
}

fn icon_text(value: Icon, label: &str, size: f32, color: Color32) -> WidgetText {
    icon_text_with_family(value, label, size, color, FontFamily::Proportional)
}

fn icon_text_medium(value: Icon, label: &str, size: f32, color: Color32) -> WidgetText {
    icon_text_with_family(
        value,
        label,
        size,
        color,
        FontFamily::Name("noto-medium".into()),
    )
}

fn icon_text_with_family(
    value: Icon,
    label: &str,
    size: f32,
    color: Color32,
    family: FontFamily,
) -> WidgetText {
    let mut job = egui::text::LayoutJob::default();
    job.append(
        &icon_char(value).to_string(),
        0.0,
        egui::TextFormat {
            font_id: FontId::new(size, FontFamily::Name("lucide".into())),
            color,
            ..Default::default()
        },
    );
    job.append(
        &format!("  {label}"),
        0.0,
        egui::TextFormat {
            font_id: FontId::new(size, family),
            color,
            ..Default::default()
        },
    );
    job.into()
}

fn primary_button(ui: &mut egui::Ui, icon: Icon, label: &str, size: [f32; 2]) -> egui::Response {
    let response = ui.add_sized(
        size,
        egui::Button::new(icon_text_medium(icon, label, 14.0, Color32::WHITE))
            .fill(ACCENT)
            .stroke(Stroke::NONE)
            .corner_radius(8),
    );
    animated_button_feedback(ui, response, ACCENT_HOVER)
}

fn secondary_button(ui: &mut egui::Ui, icon: Icon, label: &str, size: [f32; 2]) -> egui::Response {
    let response = ui.add_sized(
        size,
        egui::Button::new(icon_text_medium(icon, label, 13.0, TEXT))
            .fill(SURFACE_RAISED)
            .stroke(Stroke::new(1.0, BORDER))
            .corner_radius(8),
    );
    animated_button_feedback(ui, response, ACCENT.gamma_multiply(0.55))
}

fn nav_button(ui: &mut egui::Ui, icon: Icon, label: &str, selected: bool) -> egui::Response {
    let selected_t =
        ui.ctx()
            .animate_bool_with_time(ui.make_persistent_id(("nav", label)), selected, 0.16);
    let text_color = mix_color(MUTED, ACCENT_HOVER, selected_t);
    let response = ui.add_sized(
        [96.0, 34.0],
        egui::Button::new(icon_text_medium(icon, label, 13.0, text_color))
            .fill(mix_color(
                Color32::TRANSPARENT,
                Color32::from_rgb(253, 238, 241),
                selected_t,
            ))
            .stroke(Stroke::new(selected_t, ACCENT.gamma_multiply(0.35)))
            .corner_radius(8),
    );
    animated_button_feedback(ui, response, ACCENT.gamma_multiply(0.42))
}

fn animated_button_feedback(
    ui: &mut egui::Ui,
    response: egui::Response,
    color: Color32,
) -> egui::Response {
    let hover =
        ui.ctx()
            .animate_bool_with_time(response.id.with("hover"), response.hovered(), 0.12);
    if hover > 0.0 {
        ui.painter().rect_stroke(
            response.rect.shrink(0.5),
            8.0,
            Stroke::new(hover, color),
            egui::StrokeKind::Inside,
        );
    }
    response
}

fn window_control(ui: &mut egui::Ui, icon: Icon, tooltip: &str, danger: bool) -> egui::Response {
    let response = ui
        .scope(|ui| {
            ui.visuals_mut().widgets.inactive.bg_fill = Color32::TRANSPARENT;
            ui.visuals_mut().widgets.inactive.bg_stroke = Stroke::NONE;
            ui.visuals_mut().widgets.hovered.bg_fill = if danger {
                Color32::from_rgb(232, 82, 92)
            } else {
                SURFACE_RAISED
            };
            ui.visuals_mut().widgets.hovered.bg_stroke = Stroke::NONE;
            ui.visuals_mut().widgets.hovered.fg_stroke =
                Stroke::new(1.0, if danger { Color32::WHITE } else { TEXT });
            ui.add_sized(
                [40.0, 36.0],
                egui::Button::new(icon_only(
                    icon,
                    15.0,
                    if danger && ui.ui_contains_pointer() {
                        Color32::WHITE
                    } else {
                        MUTED
                    },
                ))
                .fill(Color32::TRANSPARENT)
                .stroke(Stroke::NONE)
                .corner_radius(7),
            )
        })
        .inner;
    response.on_hover_text(tooltip)
}

fn borderless_resize_handles(ctx: &egui::Context, rect: egui::Rect) {
    if ctx.input(|input| input.viewport().maximized.unwrap_or(false)) {
        return;
    }
    let edge = 6.0;
    let corner = 14.0;
    let handles = [
        (
            egui::Rect::from_min_max(rect.min, rect.min + egui::vec2(corner, corner)),
            egui::ResizeDirection::NorthWest,
            egui::CursorIcon::ResizeNorthWest,
        ),
        (
            egui::Rect::from_min_max(
                egui::pos2(rect.right() - corner, rect.top()),
                egui::pos2(rect.right(), rect.top() + corner),
            ),
            egui::ResizeDirection::NorthEast,
            egui::CursorIcon::ResizeNorthEast,
        ),
        (
            egui::Rect::from_min_max(
                egui::pos2(rect.left(), rect.bottom() - corner),
                egui::pos2(rect.left() + corner, rect.bottom()),
            ),
            egui::ResizeDirection::SouthWest,
            egui::CursorIcon::ResizeSouthWest,
        ),
        (
            egui::Rect::from_min_max(rect.max - egui::vec2(corner, corner), rect.max),
            egui::ResizeDirection::SouthEast,
            egui::CursorIcon::ResizeSouthEast,
        ),
        (
            egui::Rect::from_min_max(
                egui::pos2(rect.left() + corner, rect.top()),
                egui::pos2(rect.right() - corner, rect.top() + edge),
            ),
            egui::ResizeDirection::North,
            egui::CursorIcon::ResizeNorth,
        ),
        (
            egui::Rect::from_min_max(
                egui::pos2(rect.left() + corner, rect.bottom() - edge),
                egui::pos2(rect.right() - corner, rect.bottom()),
            ),
            egui::ResizeDirection::South,
            egui::CursorIcon::ResizeSouth,
        ),
        (
            egui::Rect::from_min_max(
                egui::pos2(rect.left(), rect.top() + corner),
                egui::pos2(rect.left() + edge, rect.bottom() - corner),
            ),
            egui::ResizeDirection::West,
            egui::CursorIcon::ResizeWest,
        ),
        (
            egui::Rect::from_min_max(
                egui::pos2(rect.right() - edge, rect.top() + corner),
                egui::pos2(rect.right(), rect.bottom() - corner),
            ),
            egui::ResizeDirection::East,
            egui::CursorIcon::ResizeEast,
        ),
    ];
    egui::Area::new("borderless-resize-handles".into())
        .order(egui::Order::Foreground)
        .fixed_pos(rect.min)
        .show(ctx, |ui| {
            for (index, (handle, direction, cursor)) in handles.into_iter().enumerate() {
                let local = handle.translate(-rect.min.to_vec2());
                let response = ui
                    .interact(local, ui.id().with(index), egui::Sense::drag())
                    .on_hover_cursor(cursor);
                if response.drag_started() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(direction));
                }
            }
        });
}

fn mix_color(from: Color32, to: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let mix = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
    Color32::from_rgba_unmultiplied(
        mix(from.r(), to.r()),
        mix(from.g(), to.g()),
        mix(from.b(), to.b()),
        mix(from.a(), to.a()),
    )
}

fn centered_width(ui: &mut egui::Ui, width: f32, add: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal_top(|ui| {
        ui.add_space(((ui.available_width() - width) / 2.0).max(0.0));
        ui.allocate_ui_with_layout(
            egui::vec2(width, 0.0),
            egui::Layout::top_down(egui::Align::Min),
            add,
        );
    });
}

fn format_time(timestamp: i64) -> String {
    let Ok(time) = time::OffsetDateTime::from_unix_timestamp(timestamp) else {
        return "未记录".into();
    };
    let offset = time::UtcOffset::from_hms(8, 0, 0).unwrap_or(time::UtcOffset::UTC);
    time.to_offset(offset)
        .format(&time::macros::format_description!(
            "[year]-[month]-[day] [hour]:[minute]"
        ))
        .unwrap_or_else(|_| "未记录".into())
}

fn session_status(status: SessionStatus) -> (&'static str, Color32, Icon) {
    match status {
        SessionStatus::Passed => ("已通过", SUCCESS, Icon::CircleCheck),
        SessionStatus::Failed => ("未通过", DANGER, Icon::CircleX),
        SessionStatus::Interrupted => ("异常终止", WARNING, Icon::TriangleAlert),
    }
}

fn session_row(ui: &mut egui::Ui, session: &SessionHistory) -> egui::Response {
    let (label, color, _icon) = session_status(session.status);
    let response = egui::Frame::NONE
        .fill(SURFACE)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(12)
        .inner_margin(egui::Margin::symmetric(20, 16))
        .show(ui, |ui| {
            ui.set_min_height(64.0);
            ui.horizontal(|ui| {
                egui::Frame::NONE
                    .fill(color.gamma_multiply(0.09))
                    .stroke(Stroke::new(1.0, color.gamma_multiply(0.35)))
                    .corner_radius(10)
                    .show(ui, |ui| {
                        ui.set_min_size(egui::vec2(62.0, 62.0));
                        ui.vertical_centered(|ui| {
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new(session.score.to_string())
                                    .size(22.0)
                                    .strong()
                                    .color(color),
                            );
                            ui.label(RichText::new("分").size(11.0).color(SUBTLE));
                        });
                    });
                ui.add_space(6.0);
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        status_pill(ui, label, color);
                        if let Some(message) = &session.failure_message {
                            ui.label(RichText::new(message).size(12.0).color(SUBTLE))
                                .on_hover_text(message);
                        }
                    });
                    ui.add_space(5.0);
                    ui.label(icon_text(
                        Icon::Clock,
                        &format_time(session.started_at),
                        13.0,
                        MUTED,
                    ))
                    .on_hover_text(format_time(session.started_at));
                    let categories = if session.categories.is_empty() {
                        "分类未记录".into()
                    } else {
                        session.categories.join(" · ")
                    };
                    ui.horizontal_wrapped(|ui| {
                        ui.label(icon_text(Icon::Bot, &session.model, 13.0, MUTED))
                            .on_hover_text(&session.model);
                        ui.add_space(10.0);
                        ui.label(icon_text(Icon::Tag, &categories, 13.0, MUTED));
                        if session.status == SessionStatus::Interrupted {
                            ui.label(
                                RichText::new(format!(
                                    "已完成 {}/100 题",
                                    session.completed_questions
                                ))
                                .size(12.0)
                                .color(SUBTLE),
                            );
                        }
                    });
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(icon_only(Icon::ChevronRight, 17.0, SUBTLE));
                });
            });
        })
        .response
        .interact(egui::Sense::click());
    if let Some(message) = &session.failure_message {
        response.on_hover_text(message)
    } else {
        response
    }
}

fn session_summary(ui: &mut egui::Ui, session: &SessionHistory) {
    let (label, color, icon) = session_status(session.status);
    surface().show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(icon_only(icon, 24.0, color));
            ui.vertical(|ui| {
                ui.label(RichText::new(label).size(20.0).strong().color(TEXT));
                let categories = if session.categories.is_empty() {
                    "分类未记录".into()
                } else {
                    session.categories.join(" / ")
                };
                ui.label(
                    RichText::new(format!(
                        "{} 分 · 完成 {}/100 · {}",
                        session.score, session.completed_questions, categories
                    ))
                    .color(MUTED),
                );
            });
        });
        if !session.category_scores.is_empty() {
            ui.add_space(12.0);
            ui.horizontal_wrapped(|ui| {
                for score in &session.category_scores {
                    status_pill(
                        ui,
                        &format!("{} {}/{}", score.category, score.score, score.total),
                        TEAL,
                    );
                }
            });
        }
        if let Some(message) = &session.failure_message {
            ui.add_space(12.0);
            inline_alert(ui, Icon::CircleAlert, message, WARNING);
        }
    });
}

fn result_header(ui: &mut egui::Ui, passed: bool, score: i64, color: Color32) {
    egui::Frame::NONE
        .fill(color.gamma_multiply(0.09))
        .stroke(Stroke::new(1.0, color.gamma_multiply(0.38)))
        .corner_radius(12)
        .inner_margin(egui::Margin::symmetric(32, 28))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(icon_only(
                    if passed {
                        Icon::CircleCheck
                    } else {
                        Icon::CircleX
                    },
                    40.0,
                    color,
                ));
                ui.add_space(8.0);
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(if passed {
                            "试炼通过"
                        } else {
                            "本次未通过"
                        })
                        .size(22.0)
                        .strong()
                        .color(color),
                    );
                    if !passed {
                        ui.label(
                            RichText::new("可以调整模型或分类后重新尝试。")
                                .size(13.0)
                                .color(MUTED),
                        );
                    }
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(
                            RichText::new(score.to_string())
                                .size(48.0)
                                .strong()
                                .color(color),
                        );
                        ui.label(RichText::new("总分").size(12.0).color(SUBTLE));
                    });
                });
            });
        });
}

fn result_scores(ui: &mut egui::Ui, scores: &[crate::app::ScoreItem]) {
    surface().show(ui, |ui| {
        ui.set_min_height(150.0);
        ui.label(RichText::new("分类得分").size(14.0).strong().color(TEXT));
        ui.add_space(14.0);
        if scores.is_empty() {
            ui.label(RichText::new("分类得分未记录").size(13.0).color(SUBTLE));
        }
        for item in scores {
            ui.horizontal(|ui| {
                ui.label(RichText::new(&item.category).size(13.0).color(MUTED));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!("{} / {}", item.score, item.total))
                            .size(13.0)
                            .strong()
                            .color(TEXT),
                    );
                });
            });
            ui.add(
                egui::ProgressBar::new(if item.total > 0 {
                    item.score as f32 / item.total as f32
                } else {
                    0.0
                })
                .desired_width(ui.available_width())
                .desired_height(5.0)
                .fill(TEAL),
            );
            ui.add_space(8.0);
        }
    });
}

fn result_session_info(ui: &mut egui::Ui, session: Option<&SessionHistory>) {
    surface().show(ui, |ui| {
        ui.set_min_height(150.0);
        ui.label(RichText::new("本次试炼").size(14.0).strong().color(TEXT));
        ui.add_space(14.0);
        let Some(session) = session else {
            ui.label(RichText::new("场次信息未记录").size(13.0).color(SUBTLE));
            return;
        };
        result_info_row(ui, "使用模型", &session.model);
        result_info_row(
            ui,
            "所选分类",
            &if session.categories.is_empty() {
                "未记录".into()
            } else {
                session.categories.join(" · ")
            },
        );
        result_info_row(ui, "完成时间", &format_time(session.updated_at));
        result_info_row(
            ui,
            "完成题数",
            &format!("{} / 100", session.completed_questions),
        );
    });
}

fn result_info_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(13.0).color(MUTED));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(value).size(13.0).strong().color(TEXT))
                .on_hover_text(value);
        });
    });
    ui.add_space(8.0);
}

fn result_question_summary(ui: &mut egui::Ui, session: Option<&SessionHistory>) {
    surface().show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(icon_only(Icon::ListChecks, 16.0, MUTED));
            ui.label(
                RichText::new("逐题答题摘要")
                    .size(14.0)
                    .strong()
                    .color(TEXT),
            );
        });
        ui.add_space(12.0);
        match session {
            Some(session) if !session.questions.is_empty() => {
                for question in session.questions.iter().take(5) {
                    question_history_row(ui, question);
                    ui.add_space(6.0);
                }
                if session.questions.len() > 5 {
                    ui.label(
                        RichText::new(format!(
                            "另有 {} 道题，请在历史详情中查看",
                            session.questions.len() - 5
                        ))
                        .size(12.0)
                        .color(SUBTLE),
                    );
                }
            }
            _ => {
                ui.label(RichText::new("暂无逐题摘要").size(13.0).color(SUBTLE));
            }
        }
    });
}

fn question_history_row(ui: &mut egui::Ui, question: &crate::app::QuestionHistory) {
    let color = if question.correct { SUCCESS } else { DANGER };
    egui::Frame::NONE
        .fill(color.gamma_multiply(0.07))
        .stroke(Stroke::new(1.0, color.gamma_multiply(0.28)))
        .corner_radius(8)
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(icon_only(
                    if question.correct {
                        Icon::Check
                    } else {
                        Icon::X
                    },
                    16.0,
                    color,
                ));
                ui.label(
                    RichText::new(format!(
                        "Q{}. {}",
                        question.question_number, question.question
                    ))
                    .size(13.0)
                    .strong()
                    .color(TEXT),
                );
            });
            let answer = question
                .chosen_index
                .checked_sub(1)
                .and_then(|index| question.options.get(index))
                .map(|text| {
                    format!(
                        "{} · {}",
                        (b'A' + (question.chosen_index - 1) as u8) as char,
                        text
                    )
                })
                .unwrap_or_else(|| "未记录".into());
            ui.label(
                RichText::new(format!("提交选项：{answer}"))
                    .size(12.0)
                    .color(MUTED),
            );
        });
}

fn inline_alert(ui: &mut egui::Ui, icon: Icon, message: &str, color: Color32) {
    egui::Frame::NONE
        .fill(color.gamma_multiply(0.08))
        .stroke(Stroke::new(1.0, color.gamma_multiply(0.3)))
        .corner_radius(8)
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(icon_only(icon, 16.0, color));
                ui.label(RichText::new(message).size(12.0).color(TEXT));
            });
        });
}

fn status_pill(ui: &mut egui::Ui, label: &str, color: Color32) {
    egui::Frame::NONE
        .fill(color.gamma_multiply(0.13))
        .stroke(Stroke::new(1.0, color.gamma_multiply(0.45)))
        .corner_radius(20)
        .inner_margin(egui::Margin::symmetric(9, 4))
        .show(ui, |ui| {
            let pulse = 0.72
                + 0.28
                    * (ui.input(|input| input.time) as f32 * std::f32::consts::TAU / 2.0)
                        .sin()
                        .abs();
            ui.horizontal(|ui| {
                let (dot, _) = ui.allocate_exact_size(egui::vec2(7.0, 7.0), egui::Sense::hover());
                ui.painter()
                    .circle_filled(dot.center(), 3.0, color.gamma_multiply(pulse));
                ui.label(
                    RichText::new(label)
                        .size(12.0)
                        .family(FontFamily::Name("noto-medium".into()))
                        .color(color),
                );
            });
        });
}

fn home_left_column(ui: &mut egui::Ui, app: &mut App) {
    let available = ui.available_width();
    let gap = 12.0;
    let card_width = (available - gap) / 2.0;
    ui.horizontal_top(|ui| {
        ui.allocate_ui_with_layout(
            egui::vec2(card_width, 0.0),
            egui::Layout::top_down(egui::Align::Min),
            |ui| home_status_card(ui, app, true),
        );
        ui.add_space(gap);
        ui.allocate_ui_with_layout(
            egui::vec2(card_width, 0.0),
            egui::Layout::top_down(egui::Align::Min),
            |ui| home_status_card(ui, app, false),
        );
    });
    ui.add_space(16.0);
    surface().show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(medium("自动答题", 17.0, TEXT));
                ui.add_space(2.0);
                let description = if app.auth.is_none() {
                    "扫码登录 B 站账号，验证等级后继续准备试炼。"
                } else if app.config.is_none() {
                    "账号已就绪，完成模型配置后即可开始答题。"
                } else {
                    "模型与账号均已就绪，可以开始答题试炼。"
                };
                ui.label(RichText::new(description).size(13.0).color(MUTED));
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let (icon, label) = if app.auth.is_none() {
                    (Icon::LogIn, "登录")
                } else if app.config.is_none() {
                    (Icon::Settings, "配置模型")
                } else {
                    (Icon::Play, "开始答题")
                };
                if primary_button(ui, icon, label, [132.0, 40.0]).clicked() {
                    if app.auth.is_none() {
                        app.enter_login();
                    } else if app.config.is_some() {
                        app.enter_quiz();
                    } else {
                        app.enter_config();
                    }
                }
            });
        });
        if app.config.is_none() || app.auth.is_none() {
            ui.add_space(16.0);
            ui.separator();
            ui.add_space(8.0);
            let (icon, label) = if app.config.is_none() {
                (Icon::Settings, "模型设置")
            } else {
                (Icon::LogIn, "扫码登录")
            };
            if secondary_button(ui, icon, label, [100.0, 32.0]).clicked() {
                if app.config.is_none() {
                    app.enter_config();
                } else {
                    app.enter_quiz();
                }
            }
        }
    });
    ui.add_space(16.0);
    egui::Frame::NONE
        .fill(SURFACE_RAISED)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(10)
        .inner_margin(egui::Margin::symmetric(16, 14))
        .show(ui, |ui| {
            ui.label(
                RichText::new("试炼说明 · 共 100 道题，分 3 个分类 · 及格分 60 分 · 模型自动作答，无需手动操作 · 认证信息仅保存在本机")
                    .size(12.0)
                    .color(MUTED),
            );
        });
}

fn home_status_card(ui: &mut egui::Ui, app: &App, model: bool) {
    surface().show(ui, |ui| {
        ui.set_min_height(82.0);
        let ready = if model {
            app.config.is_some()
        } else {
            app.auth.is_some()
        };
        ui.horizontal(|ui| {
            ui.label(icon_only(
                if model { Icon::Bot } else { Icon::Zap },
                18.0,
                MUTED,
            ));
            ui.label(
                RichText::new(if model {
                    "模型配置"
                } else {
                    "账号状态"
                })
                .size(13.0)
                .color(MUTED),
            );
        });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(icon_only(
                if ready {
                    Icon::CircleCheck
                } else {
                    Icon::CircleAlert
                },
                16.0,
                if ready { TEAL } else { WARNING },
            ));
            ui.label(
                RichText::new(if ready {
                    if model { "已配置" } else { "已登录" }
                } else if model {
                    "未配置"
                } else {
                    "需要扫码"
                })
                .size(14.0)
                .strong()
                .color(TEXT),
            );
        });
        let detail = if model {
            app.config
                .as_ref()
                .map(|config| config.model.clone())
                .unwrap_or_else(|| "需要先完成模型设置".into())
        } else if let Some(auth) = &app.auth {
            masked_uid(&auth.mid)
        } else {
            "扫描二维码完成授权".to_string()
        };
        ui.label(RichText::new(detail).size(13.0).color(MUTED));
    });
}

fn home_history_column(ui: &mut egui::Ui, app: &App) -> bool {
    let mut open_history = false;
    surface().show(ui, |ui| {
        ui.set_min_height(204.0);
        ui.horizontal(|ui| {
            ui.label(icon_only(Icon::History, 16.0, MUTED));
            ui.label(medium("历史记录", 15.0, TEXT));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("{} 次试炼", app.sessions.len()))
                        .size(12.0)
                        .color(SUBTLE),
                );
            });
        });
        ui.add_space(16.0);
        if let Some(session) = app.sessions.first() {
            egui::Frame::NONE
                .fill(SURFACE_RAISED)
                .corner_radius(8)
                .inner_margin(egui::Margin::symmetric(14, 12))
                .show(ui, |ui| {
                    let (label, color, _) = session_status(session.status);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("最近一次").size(12.0).color(SUBTLE));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            status_pill(ui, label, color);
                        });
                    });
                    ui.label(
                        RichText::new(format!("{} 分", session.score))
                            .size(22.0)
                            .strong()
                            .color(TEXT),
                    );
                    let categories = if session.categories.is_empty() {
                        "分类未记录".into()
                    } else {
                        session.categories.join(" · ")
                    };
                    ui.label(
                        RichText::new(format!(
                            "{} · {}",
                            categories,
                            format_time(session.started_at)
                        ))
                        .size(12.0)
                        .color(MUTED),
                    );
                });
            ui.add_space(8.0);
            if ui
                .add(
                    egui::Button::new(icon_text(
                        Icon::ChevronRight,
                        "查看全部历史记录",
                        13.0,
                        ACCENT_HOVER,
                    ))
                    .fill(Color32::TRANSPARENT)
                    .stroke(Stroke::NONE),
                )
                .clicked()
            {
                open_history = true;
            }
        } else {
            ui.vertical_centered(|ui| {
                ui.add_space(24.0);
                ui.label(icon_only(Icon::History, 28.0, SUBTLE));
                ui.label(RichText::new("暂无历史记录").size(13.0).color(MUTED));
                ui.label(
                    RichText::new("完成第一次试炼后将在此显示")
                        .size(12.0)
                        .color(SUBTLE),
                );
            });
        }
    });
    ui.add_space(12.0);
    let passed = app
        .sessions
        .iter()
        .filter(|s| s.status == SessionStatus::Passed)
        .count();
    let average = if app.sessions.is_empty() {
        "--".to_string()
    } else {
        (app.sessions.iter().map(|s| s.score).sum::<i64>() / app.sessions.len() as i64).to_string()
    };
    ui.columns(2, |columns| {
        home_stat(&mut columns[0], &passed.to_string(), "已通过", TEAL);
        home_stat(&mut columns[1], &average, "平均分", TEXT);
    });
    open_history
}

fn home_stat(ui: &mut egui::Ui, value: &str, label: &str, color: Color32) {
    egui::Frame::NONE
        .fill(SURFACE)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(10)
        .inner_margin(egui::Margin::symmetric(16, 14))
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.label(RichText::new(value).size(20.0).strong().color(color));
                ui.label(RichText::new(label).size(12.0).color(MUTED));
            });
        });
}

fn masked_uid(uid: &str) -> String {
    if uid.len() <= 4 {
        return format!("UID {uid}");
    }
    let prefix = &uid[..2];
    let suffix = &uid[uid.len() - 2..];
    format!("UID {prefix}****{suffix}")
}

fn field(ui: &mut egui::Ui, label: &str, value: &mut String, password: bool) {
    ui.label(RichText::new(label).size(13.0).strong().color(TEXT));
    ui.add_sized(
        [ui.available_width(), 36.0],
        egui::TextEdit::singleline(value)
            .password(password)
            .margin(egui::Margin::symmetric(12, 7))
            .desired_width(f32::INFINITY),
    );
    ui.add_space(10.0);
}

fn config_credentials(
    ui: &mut egui::Ui,
    app: &mut App,
    error: &mut Option<String>,
    saved_until: &mut Option<Instant>,
) {
    surface().show(ui, |ui| {
        ui.label(medium("连接凭证", 16.0, TEXT));
        ui.add_space(16.0);
        ui.label(RichText::new("服务商").size(13.0).strong().color(TEXT));
        let presets = config::load_presets();
        let selected = presets
            .iter()
            .position(|preset| preset.config.base_url == app.cfg_fields[0])
            .unwrap_or(0);
        egui::ComboBox::from_id_salt("provider")
            .width(ui.available_width())
            .selected_text(
                presets
                    .get(selected)
                    .map(|preset| preset.provider_name.as_str())
                    .unwrap_or("自定义"),
            )
            .show_ui(ui, |ui| {
                for (index, preset) in presets.iter().enumerate() {
                    if ui
                        .selectable_label(index == selected, &preset.provider_name)
                        .clicked()
                    {
                        app.apply_preset(index);
                    }
                }
            });
        ui.add_space(14.0);
        field(ui, "API 地址  *", &mut app.cfg_fields[0], false);
        field(ui, "模型名称  *", &mut app.cfg_fields[1], false);
        field(ui, "API Key  *", &mut app.cfg_fields[2], true);
        ui.horizontal(|ui| {
            ui.label(icon_only(Icon::LockKeyhole, 13.0, SUBTLE));
            ui.label(
                RichText::new("仅保存在本机，不会上传到任何服务器")
                    .size(12.0)
                    .color(SUBTLE),
            );
        });
        if let Some(message) = error {
            ui.add_space(12.0);
            egui::Frame::NONE
                .fill(DANGER.gamma_multiply(0.12))
                .stroke(Stroke::new(1.0, DANGER.gamma_multiply(0.5)))
                .corner_radius(6)
                .inner_margin(egui::Margin::same(10))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(icon_only(Icon::CircleAlert, 16.0, DANGER));
                        ui.label(RichText::new(message.as_str()).size(12.0).color(DANGER));
                    });
                });
        }
        if saved_until.is_some_and(|until| until > Instant::now()) {
            ui.add_space(12.0);
            inline_alert(ui, Icon::CircleCheck, "配置已保存", SUCCESS);
        }
        ui.add_space(16.0);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if primary_button(ui, Icon::Save, "保存配置", [124.0, 36.0]).clicked() {
                *error = app.persist_config().err();
                if error.is_none() {
                    *saved_until = Some(Instant::now() + Duration::from_secs(2));
                }
            }
        });
    });
}

fn config_behavior(ui: &mut egui::Ui, app: &mut App) {
    surface().show(ui, |ui| {
        ui.label(medium("答题策略", 16.0, TEXT));
        ui.add_space(20.0);

        setting_row(
            ui,
            Icon::Brain,
            "思考模式",
            "开启后模型在回答前进行分析，准确率更高，速度较慢",
            |ui| {
                toggle_switch(ui, &mut app.cfg_thinking);
            },
        );
        ui.add_space(16.0);
        if app.cfg_thinking {
            ui.vertical(|ui| {
                ui.label(RichText::new("思考强度").size(13.0).strong().color(TEXT));
                segmented_control(ui, &mut app.cfg_effort, &["低", "高", "最大"]);
                ui.label(
                    RichText::new(match app.cfg_effort {
                        0 => "较少 Token 消耗，适合简单题目",
                        1 => "平衡准确率与速度",
                        _ => "最高准确率，Token 消耗较多",
                    })
                    .size(12.0)
                    .color(SUBTLE),
                );
            });
        }
        ui.add_space(16.0);
        ui.separator();
        ui.add_space(12.0);
        setting_row(
            ui,
            Icon::Zap,
            "快速模式",
            "跳过 AI 分析摘要展示，直接提交答案，每题耗时更短",
            |ui| {
                toggle_switch(ui, &mut app.cfg_fast_mode);
            },
        );
    });
}

fn config_account(ui: &mut egui::Ui, app: &mut App, logout_confirm: &mut bool) {
    surface().show(ui, |ui| {
        ui.label(medium("账号与数据", 16.0, TEXT));
        ui.add_space(16.0);
        inset_surface().show(ui, |ui| {
            ui.label(
                RichText::new(if app.auth.is_some() {
                    "当前登录账号已验证，认证信息仅保存在本机"
                } else {
                    "未登录 B 站账号"
                })
                .size(13.0)
                .color(MUTED),
            );
        });
        if app.auth.is_some() {
            ui.add_space(12.0);
            if secondary_button(ui, Icon::LogOut, "退出登录", [ui.available_width(), 36.0])
                .clicked()
            {
                *logout_confirm = true;
            }
        }
        ui.add_space(12.0);
        ui.separator();
        ui.add_space(12.0);
        if ui
            .add_sized(
                [ui.available_width(), 36.0],
                egui::Button::new(icon_text(Icon::Trash2, "重置全部本地数据", 13.0, DANGER))
                    .fill(DANGER.gamma_multiply(0.08))
                    .stroke(Stroke::new(1.0, DANGER.gamma_multiply(0.3)))
                    .corner_radius(8),
            )
            .clicked()
        {
            app.config_confirm_reset = true;
        }
    });
}

fn setting_row(
    ui: &mut egui::Ui,
    icon: Icon,
    title: &str,
    description: &str,
    action: impl FnOnce(&mut egui::Ui),
) {
    ui.horizontal(|ui| {
        ui.label(icon_only(icon, 18.0, MUTED));
        ui.label(medium(title, 14.0, TEXT));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), action);
    });
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        ui.add_space(28.0);
        ui.add(egui::Label::new(RichText::new(description).size(12.0).color(MUTED)).wrap());
    });
}

fn toggle_switch(ui: &mut egui::Ui, value: &mut bool) -> egui::Response {
    let (rect, mut response) = ui.allocate_exact_size(egui::vec2(40.0, 22.0), egui::Sense::click());
    if response.clicked() {
        *value = !*value;
        response.mark_changed();
    }
    let animation = ui
        .ctx()
        .animate_bool_with_time(response.id.with("toggle"), *value, 0.16);
    let fill = mix_color(Color32::from_rgb(196, 202, 210), ACCENT, animation);
    ui.painter().rect_filled(rect, 11.0, fill);
    let center_x = egui::lerp((rect.left() + 11.0)..=(rect.right() - 11.0), animation);
    ui.painter()
        .circle_filled(egui::pos2(center_x, rect.center().y), 8.0, Color32::WHITE);
    response
}

fn segmented_control(ui: &mut egui::Ui, selected: &mut usize, labels: &[&str]) {
    egui::Frame::NONE
        .fill(SURFACE_RAISED)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(8)
        .inner_margin(egui::Margin::same(3))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 3.0;
            ui.horizontal(|ui| {
                let width = ((ui.available_width() - 6.0) / labels.len() as f32).max(60.0);
                for (index, label) in labels.iter().enumerate() {
                    let active = *selected == index;
                    let active_t = ui.ctx().animate_bool_with_time(
                        ui.make_persistent_id(("segment", *label)),
                        active,
                        0.14,
                    );
                    if ui
                        .add_sized(
                            [width, 30.0],
                            egui::Button::new(RichText::new(*label).size(13.0).color(mix_color(
                                MUTED,
                                ACCENT_HOVER,
                                active_t,
                            )))
                            .fill(mix_color(Color32::TRANSPARENT, SURFACE, active_t))
                            .stroke(Stroke::new(active_t, BORDER))
                            .corner_radius(6),
                        )
                        .clicked()
                    {
                        *selected = index;
                    }
                }
            });
        });
}

fn status(ui: &mut egui::Ui, message: &str) {
    state_panel(
        ui,
        Icon::RefreshCw,
        message,
        "请保持窗口开启，流程完成后会自动继续。",
        ACCENT,
        |ui| {
            ui.spinner();
        },
    );
}

fn state_panel(
    ui: &mut egui::Ui,
    icon: Icon,
    title: &str,
    description: &str,
    color: Color32,
    add: impl FnOnce(&mut egui::Ui),
) {
    let width = ui.available_width().min(480.0);
    ui.add_space((ui.available_height() * 0.12).max(32.0));
    centered_width(ui, width, |ui| {
        surface()
            .inner_margin(egui::Margin::same(40))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(72.0, 72.0), egui::Sense::hover());
                    let pulse = 0.5
                        + 0.5
                            * (ui.input(|input| input.time) as f32 * std::f32::consts::TAU / 1.8)
                                .sin();
                    ui.painter().circle_filled(
                        rect.center(),
                        36.0,
                        color.gamma_multiply(0.075 + pulse * 0.035),
                    );
                    ui.painter().circle_stroke(
                        rect.center(),
                        35.0,
                        Stroke::new(2.0, color.gamma_multiply(0.42)),
                    );
                    ui.put(
                        rect,
                        egui::Label::new(icon_only(icon, 32.0, color))
                            .selectable(false)
                            .halign(egui::Align::Center),
                    );
                    ui.add_space(24.0);
                    ui.label(medium(title, 18.0, TEXT));
                    ui.add_space(6.0);
                    ui.label(RichText::new(description).size(14.0).color(MUTED));
                    ui.add_space(24.0);
                    ui.horizontal(|ui| add(ui));
                });
            });
    });
}

fn login_step(ui: &mut egui::Ui, number: &str, title: &str, description: &str) {
    ui.horizontal(|ui| {
        egui::Frame::NONE
            .fill(SURFACE_RAISED)
            .stroke(Stroke::new(1.0, BORDER))
            .corner_radius(20)
            .inner_margin(egui::Margin::symmetric(10, 6))
            .show(ui, |ui| {
                ui.label(RichText::new(number).size(12.0).strong().color(ACCENT));
            });
        ui.vertical(|ui| {
            ui.label(RichText::new(title).size(13.0).strong().color(TEXT));
            ui.label(RichText::new(description).size(12.0).color(MUTED));
        });
    });
    ui.add_space(12.0);
}

fn metric(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.vertical(|ui| {
        ui.label(RichText::new(label).size(11.0).color(SUBTLE));
        ui.label(RichText::new(value).size(16.0).strong().color(TEXT));
    });
}

fn captcha_categories(
    ui: &mut egui::Ui,
    state: &CaptchaState,
    selected_count: usize,
    toggled: &mut Option<usize>,
) {
    ui.horizontal(|ui| {
        ui.label(medium("答题分类", 16.0, TEXT));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            status_pill(ui, &format!("已选 {selected_count}/3"), ACCENT);
        });
    });
    ui.add_space(16.0);
    let columns = if ui.available_width() >= 480.0 { 4 } else { 2 };
    egui::Grid::new("captcha-categories")
        .num_columns(columns)
        .spacing([8.0, 8.0])
        .show(ui, |ui| {
            for (index, category) in state.categories.iter().enumerate() {
                let selected = category.selected;
                let disabled = selected_count >= 3 && !selected;
                let selected_t = ui.ctx().animate_bool_with_time(
                    ui.make_persistent_id(("category", category.id)),
                    selected,
                    0.16,
                );
                let color = mix_color(BORDER, ACCENT, selected_t);
                let response = ui.add_enabled(
                    !disabled,
                    egui::Button::new(icon_text(
                        if selected { Icon::Check } else { Icon::Circle },
                        &category.name,
                        13.0,
                        mix_color(TEXT, ACCENT, selected_t),
                    ))
                    .fill(mix_color(
                        SURFACE,
                        Color32::from_rgb(253, 238, 241),
                        selected_t,
                    ))
                    .stroke(Stroke::new(1.0, color))
                    .corner_radius(8)
                    .min_size(egui::vec2(104.0, 40.0)),
                );
                if response.clicked() {
                    *toggled = Some(index);
                }
                if (index + 1) % columns == 0 {
                    ui.end_row();
                }
            }
        });
}

#[allow(clippy::too_many_arguments)]
fn captcha_form(
    ui: &mut egui::Ui,
    _ctx: &egui::Context,
    app: &mut App,
    state: &CaptchaState,
    texture: &mut Option<(String, TextureHandle)>,
    submit: &mut bool,
    refresh: &mut bool,
    open: &mut bool,
) {
    ui.label(medium("验证码验证", 16.0, TEXT));
    ui.add_space(14.0);
    if let Some((_, texture)) = texture {
        let width = ui.available_width().min(252.0);
        let ratio = texture.size()[1] as f32 / texture.size()[0] as f32;
        egui::Frame::NONE
            .fill(SURFACE_RAISED)
            .stroke(Stroke::new(1.0, BORDER))
            .corner_radius(8)
            .inner_margin(egui::Margin::same(8))
            .show(ui, |ui| {
                ui.image((
                    texture.id(),
                    egui::vec2(width - 16.0, (width - 16.0) * ratio),
                ));
            });
    } else {
        egui::Frame::NONE
            .fill(SURFACE_RAISED)
            .stroke(Stroke::new(1.0, BORDER))
            .corner_radius(8)
            .show(ui, |ui| {
                ui.set_min_size(egui::vec2(ui.available_width(), 110.0));
                ui.centered_and_justified(|ui| {
                    ui.label(
                        RichText::new("验证码图片加载失败")
                            .size(13.0)
                            .color(WARNING),
                    );
                });
            });
    }
    ui.add_space(12.0);
    ui.horizontal(|ui| {
        if ui
            .button(icon_text(Icon::RefreshCw, "刷新", 12.0, MUTED))
            .clicked()
        {
            *refresh = true;
        }
        if ui
            .button(icon_text(Icon::ExternalLink, "浏览器打开", 12.0, MUTED))
            .clicked()
        {
            *open = true;
        }
    });
    ui.add_space(10.0);
    ui.label(RichText::new("验证码").size(13.0).strong().color(TEXT));
    if let QuizPhase::Captcha(current) = &mut app.phase {
        ui.add_sized(
            [ui.available_width(), 36.0],
            egui::TextEdit::singleline(&mut current.input)
                .hint_text("请输入图片中的字符")
                .margin(egui::Margin::symmetric(12, 7)),
        );
    }
    if !state.error.is_empty() {
        ui.label(RichText::new(&state.error).size(12.0).color(DANGER));
    }
    ui.add_space(12.0);
    *submit = primary_button(ui, Icon::Send, "提交验证", [ui.available_width(), 40.0]).clicked();
}

fn phase_label(phase: &QuizPhase) -> &'static str {
    match phase {
        QuizPhase::NotConfigured => "等待模型配置",
        QuizPhase::LoggingIn | QuizPhase::WaitingScan { .. } | QuizPhase::LoginTimeout { .. } => {
            "B 站账号登录"
        }
        QuizPhase::CheckingLevel => "验证账号资格",
        QuizPhase::LevelVerified { .. } => "账号资格已通过",
        QuizPhase::LevelInsufficient { .. } | QuizPhase::LevelCheckFailed(_) => "账号资格需处理",
        QuizPhase::FetchingQuestion | QuizPhase::Captcha(_) => "准备答题内容",
        QuizPhase::WaitingLlm | QuizPhase::WaitingRetry { .. } => "AI 分析题目",
        QuizPhase::Submitting | QuizPhase::ShowingResult { .. } => "提交与验证答案",
        QuizPhase::Finished { .. } => "答题完成",
        QuizPhase::Error(_) => "需要处理",
    }
}

fn question_panel(ui: &mut egui::Ui, app: &App) {
    // The panel is placed in a fixed-width column. Keep its children at that
    // width so short/empty streaming text cannot make the layout jump.
    let panel_width = ui.available_width();
    ui.set_width(panel_width);
    ui.set_min_height(ui.available_height().max(420.0));
    ui.label(
        RichText::new(format!("第 {} 题", app.question_num))
            .size(12.0)
            .color(SUBTLE),
    );
    ui.add_space(8.0);
    ui.label(medium(&app.question_text, 17.0, TEXT));
    ui.add_space(18.0);
    for (index, answer) in app.answers.iter().enumerate() {
        let chosen = index + 1 == app.chosen_answer_idx;
        let result = match app.phase {
            QuizPhase::ShowingResult { correct, .. } if chosen => Some(correct),
            _ if chosen => Some(true),
            _ => None,
        };
        let target_color = match result {
            Some(false) => DANGER,
            Some(true) if matches!(app.phase, QuizPhase::ShowingResult { .. }) => TEAL,
            Some(true) => ACCENT,
            None => BORDER,
        };
        let selected_t = ui.ctx().animate_bool_with_time(
            ui.make_persistent_id(("answer", app.question_num, index)),
            result.is_some(),
            0.18,
        );
        let color = mix_color(BORDER, target_color, selected_t);
        egui::Frame::NONE
            .fill(mix_color(
                SURFACE,
                target_color.gamma_multiply(0.09),
                selected_t,
            ))
            .stroke(Stroke::new(1.0, color))
            .corner_radius(8)
            .inner_margin(egui::Margin::symmetric(16, 12))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    egui::Frame::NONE
                        .fill(if result.is_some() { color } else { SURFACE })
                        .stroke(Stroke::new(1.5, color))
                        .corner_radius(20)
                        .show(ui, |ui| {
                            ui.add_sized(
                                [22.0, 22.0],
                                egui::Label::new(
                                    RichText::new(((b'A' + index as u8) as char).to_string())
                                        .size(12.0)
                                        .strong()
                                        .color(if result.is_some() {
                                            Color32::WHITE
                                        } else {
                                            TEXT
                                        }),
                                ),
                            );
                        });
                    ui.add_space(4.0);
                    ui.label(RichText::new(&answer.text).size(14.0).color(TEXT));
                });
            });
        ui.add_space(8.0);
    }
    ui.add_space(10.0);
    match &app.phase {
        QuizPhase::WaitingLlm => {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(RichText::new("AI 正在分析…").color(MUTED));
            });
        }
        QuizPhase::WaitingRetry { attempt, deadline } => {
            inline_alert(
                ui,
                Icon::Clock,
                &format!(
                    "模型请求超时，{} 秒后进行第 {attempt}/{} 次重试",
                    deadline.saturating_duration_since(Instant::now()).as_secs(),
                    App::MAX_LLM_RETRIES
                ),
                WARNING,
            );
        }
        QuizPhase::Submitting => {
            ui.label(RichText::new("正在提交答案…").color(MUTED));
        }
        QuizPhase::ShowingResult { correct, .. } => {
            inline_alert(
                ui,
                if *correct {
                    Icon::CircleCheck
                } else {
                    Icon::CircleX
                },
                if *correct {
                    "回答正确，已获得 1 分"
                } else {
                    "回答错误，未获得分数"
                },
                if *correct { TEAL } else { DANGER },
            );
        }
        _ => {}
    }
    if !app.thinking_text.is_empty() || matches!(app.phase, QuizPhase::WaitingLlm) {
        ui.add_space(14.0);
        surface()
            .inner_margin(egui::Margin::same(16))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(RichText::new("●").size(11.0).color(ACCENT));
                    ui.label(
                        RichText::new("AI 分析摘要")
                            .size(13.0)
                            .strong()
                            .color(MUTED),
                    );
                    if matches!(app.phase, QuizPhase::WaitingLlm) {
                        ui.spinner();
                    }
                });
                ui.add_space(8.0);
                egui::ScrollArea::vertical()
                    .stick_to_bottom(true)
                    .max_height(120.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        if app.thinking_text.is_empty() {
                            ui.label(
                                RichText::new("等待模型返回分析摘要…")
                                    .size(13.0)
                                    .color(SUBTLE),
                            );
                        } else {
                            ui.label(RichText::new(&app.thinking_text).size(13.0).color(TEXT));
                        }
                    });
            });
    }
}

fn history_panel(ui: &mut egui::Ui, app: &App) {
    ui.set_min_height(ui.available_height().max(420.0));
    ui.label(medium("本次答题记录", 13.0, MUTED));
    ui.add_space(12.0);
    egui::ScrollArea::vertical().show(ui, |ui| {
        if app.history.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(70.0);
                ui.label(icon_only(Icon::History, 26.0, SUBTLE));
                ui.label(RichText::new("暂无答题记录").color(MUTED));
            });
        }
        for item in app.history.iter().rev() {
            let color = if item.correct { SUCCESS } else { DANGER };
            egui::Frame::NONE
                .fill(color.gamma_multiply(0.07))
                .stroke(Stroke::new(1.0, color.gamma_multiply(0.3)))
                .corner_radius(8)
                .inner_margin(egui::Margin::symmetric(12, 10))
                .show(ui, |ui| {
                    ui.horizontal_top(|ui| {
                        ui.label(icon_only(
                            if item.correct {
                                Icon::CircleCheck
                            } else {
                                Icon::CircleX
                            },
                            15.0,
                            color,
                        ));
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new(format!("第 {} 题", item.num))
                                    .size(12.0)
                                    .color(SUBTLE),
                            );
                            ui.label(RichText::new(&item.question).size(13.0).color(TEXT));
                            if let Some(answer) = item
                                .chosen_idx
                                .checked_sub(1)
                                .and_then(|index| item.options.get(index))
                            {
                                ui.label(RichText::new(answer).size(12.0).strong().color(color));
                            }
                        });
                    });
                });
            ui.add_space(6.0);
        }
    });
}

fn dynamic_image(image: &image::DynamicImage) -> ColorImage {
    let rgba = image.to_rgba8();
    ColorImage::from_rgba_unmultiplied(
        [rgba.width() as usize, rgba.height() as usize],
        rgba.as_raw(),
    )
}

fn qr_image(url: &str) -> Option<ColorImage> {
    use qrcode::Color;
    let code = qrcode::QrCode::new(url.as_bytes()).ok()?;
    let quiet = 4usize;
    let modules = code.width() + quiet * 2;
    let scale = 6usize;
    let size = modules * scale;
    let mut pixels = vec![Color32::WHITE; size * size];
    for y in 0..code.width() {
        for x in 0..code.width() {
            if code[(x, y)] == Color::Dark {
                for py in 0..scale {
                    for px in 0..scale {
                        let output_x = (x + quiet) * scale + px;
                        let output_y = (y + quiet) * scale + py;
                        pixels[output_y * size + output_x] = Color32::BLACK;
                    }
                }
            }
        }
    }
    Some(ColorImage::new([size, size], pixels))
}

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "noto-sc".into(),
        egui::FontData::from_static(include_bytes!("../assets/NotoSansSC-Regular.otf")).into(),
    );
    fonts.font_data.insert(
        "noto-medium".into(),
        egui::FontData::from_static(include_bytes!("../assets/NotoSansSC-Medium.otf")).into(),
    );
    fonts.font_data.insert(
        "noto-bold".into(),
        egui::FontData::from_static(include_bytes!("../assets/NotoSansSC-Bold.otf")).into(),
    );
    fonts.font_data.insert(
        "lucide".into(),
        egui::FontData::from_static(lucide_icons::LUCIDE_FONT_BYTES).into(),
    );
    fonts
        .families
        .insert(FontFamily::Name("lucide".into()), vec!["lucide".into()]);
    fonts.families.insert(
        FontFamily::Name("noto-medium".into()),
        vec!["noto-medium".into(), "noto-sc".into()],
    );
    fonts.families.insert(
        FontFamily::Name("noto-bold".into()),
        vec!["noto-bold".into(), "noto-medium".into(), "noto-sc".into()],
    );
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .insert(0, "noto-sc".into());
    }
    ctx.set_fonts(fonts);
}

fn configure_style(ctx: &egui::Context) {
    let mut style = (*ctx.style_of(egui::Theme::Light)).clone();
    style.animation_time = 0.16;
    style.override_font_id = Some(FontId::new(14.0, FontFamily::Proportional));
    style.spacing.item_spacing = egui::vec2(10.0, 8.0);
    style.spacing.button_padding = egui::vec2(12.0, 8.0);
    style.spacing.interact_size.y = 34.0;
    style.visuals = egui::Visuals::light();
    style.visuals.selection.bg_fill = ACCENT;
    style.visuals.selection.stroke = Stroke::new(1.0, Color32::WHITE);
    style.visuals.window_fill = SURFACE;
    style.visuals.panel_fill = CANVAS;
    style.visuals.faint_bg_color = SURFACE_RAISED;
    style.visuals.extreme_bg_color = Color32::WHITE;
    style.visuals.code_bg_color = SURFACE_RAISED;
    style.visuals.widgets.noninteractive.bg_fill = SURFACE;
    style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
    style.visuals.widgets.inactive.bg_fill = SURFACE_RAISED;
    style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER);
    style.visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, MUTED);
    style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(252, 239, 242);
    style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT_HOVER);
    style.visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, TEXT);
    style.visuals.widgets.active.bg_fill = ACCENT;
    style.visuals.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT_HOVER);
    style.visuals.widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    style.visuals.widgets.open.bg_fill = SURFACE_RAISED;
    style.visuals.widgets.open.bg_stroke = Stroke::new(1.0, ACCENT);
    style.visuals.window_corner_radius = 12.into();
    style.visuals.menu_corner_radius = 8.into();
    style.visuals.widgets.noninteractive.corner_radius = 8.into();
    style.visuals.widgets.inactive.corner_radius = 8.into();
    style.visuals.widgets.hovered.corner_radius = 8.into();
    style.visuals.widgets.active.corner_radius = 8.into();
    style.visuals.widgets.open.corner_radius = 8.into();
    style.text_styles.insert(
        egui::TextStyle::Heading,
        FontId::new(24.0, FontFamily::Name("noto-bold".into())),
    );
    style.text_styles.insert(
        egui::TextStyle::Body,
        FontId::new(14.0, FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Button,
        FontId::new(14.0, FontFamily::Name("noto-medium".into())),
    );
    style.text_styles.insert(
        egui::TextStyle::Small,
        FontId::new(12.0, FontFamily::Proportional),
    );
    ctx.set_style_of(egui::Theme::Light, style.clone());
    ctx.set_style_of(egui::Theme::Dark, style);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qr_texture_is_square_and_nonempty() {
        let image = qr_image("https://example.com").unwrap();
        assert_eq!(image.size[0], image.size[1]);
        assert!(image.pixels.contains(&Color32::BLACK));
    }
}
