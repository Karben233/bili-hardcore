use crate::app::{App, CaptchaFocus, CaptchaState, QuizPhase};
use ratatui::style::{Color, Modifier, Style};

/// 选中按钮样式：亮色文字 + 加粗
fn selected_style(color: Color) -> Style {
    Style::default().fg(color).add_modifier(Modifier::BOLD)
}

/// 未选中按钮样式：暗色文字
fn dim_style(color: Color) -> Style {
    Style::default().fg(color)
}

pub fn draw(f: &mut ratatui::Frame, app: &App) {
    use ratatui::{
        layout::{Alignment, Constraint, Layout},
        style::{Color, Modifier, Style},
        widgets::{Block, Borders, Gauge, Paragraph, Wrap},
    };

    let size = f.area();
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" 答题 ")
        .title_alignment(Alignment::Center);
    let inner = block.inner(size);
    f.render_widget(block, size);

    match &app.phase {
        QuizPhase::NotConfigured => {
            let chunks = Layout::vertical([
                Constraint::Percentage(40),
                Constraint::Length(2),
                Constraint::Length(2),
                Constraint::Percentage(40),
            ])
            .split(inner);
            f.render_widget(
                Paragraph::new("未配置 AI API，请先完成配置")
                    .style(
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )
                    .alignment(Alignment::Center),
                chunks[1],
            );
            f.render_widget(
                Paragraph::new("  [ 确认 - 前往配置 ]  ")
                    .style(selected_style(Color::Cyan))
                    .alignment(Alignment::Center),
                chunks[2],
            );
        }

        QuizPhase::LoggingIn | QuizPhase::CheckingLevel | QuizPhase::FetchingQuestion => {
            let msg = match &app.phase {
                QuizPhase::LoggingIn => "正在准备登录...",
                QuizPhase::CheckingLevel => "正在验证用户等级...",
                QuizPhase::FetchingQuestion => "正在获取题目...",
                _ => "",
            };
            center_text(f, inner, msg, Color::Cyan);
        }

        QuizPhase::WaitingScan {
            qr, countdown, ..
        } => {
            let chunks = Layout::vertical([
                Constraint::Length(2),
                Constraint::Min(6),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(inner);

            f.render_widget(
                Paragraph::new("请使用哔哩哔哩APP扫描二维码登录")
                    .style(
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )
                    .alignment(Alignment::Center),
                chunks[0],
            );
            f.render_widget(
                Paragraph::new(qr.as_str())
                    .style(Style::default().fg(Color::White))
                    .alignment(Alignment::Center),
                chunks[1],
            );
            f.render_widget(
                Paragraph::new(format!("等待扫码... ({}s)", countdown))
                    .style(Style::default().fg(Color::Cyan))
                    .alignment(Alignment::Center),
                chunks[2],
            );
            f.render_widget(
                Paragraph::new("B 浏览器打开二维码  ESC 返回")
                    .style(Style::default().fg(Color::DarkGray))
                    .alignment(Alignment::Center),
                chunks[3],
            );
        }

        QuizPhase::LoginTimeout { retry } => {
            let chunks = Layout::vertical([
                Constraint::Percentage(30),
                Constraint::Length(2),
                Constraint::Length(2),
                Constraint::Length(2),
                Constraint::Length(1),
                Constraint::Percentage(30),
            ])
            .split(inner);
            f.render_widget(
                Paragraph::new("二维码登录超时")
                    .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
                    .alignment(Alignment::Center),
                chunks[1],
            );
            f.render_widget(
                Paragraph::new("  [ 重试 ]  ")
                    .style(if *retry {
                        selected_style(Color::Green)
                    } else {
                        dim_style(Color::DarkGray)
                    })
                    .alignment(Alignment::Center),
                chunks[2],
            );
            f.render_widget(
                Paragraph::new("  [ 返回首页 ]  ")
                    .style(if *retry {
                        dim_style(Color::DarkGray)
                    } else {
                        selected_style(Color::Red)
                    })
                    .alignment(Alignment::Center),
                chunks[3],
            );
            f.render_widget(
                Paragraph::new("↑↓ 切换  Enter 确认")
                    .style(Style::default().fg(Color::DarkGray))
                    .alignment(Alignment::Center),
                chunks[4],
            );
        }

        QuizPhase::ShowingQuestion | QuizPhase::WaitingLlm | QuizPhase::Submitting => {
            let num = app.question_num;
            let accuracy = if num > 0 {
                (app.score as f64 / num as f64 * 100.0) as u32
            } else {
                0
            };
            let progress = if app.question_num > 0 {
                app.question_num as f64 / 100.0
            } else {
                0.0
            };

            let outer = Layout::vertical([
                Constraint::Length(1),
                Constraint::Min(3),
                Constraint::Length(1),
            ])
            .split(inner);

            f.render_widget(
                Gauge::default()
                    .gauge_style(Style::default().fg(Color::Cyan))
                    .ratio(progress.min(1.0))
                    .label(format!(
                        "第 {}/100 题 | 得分: {} | 正确率: {}%",
                        num, app.score, accuracy
                    )),
                outer[0],
            );

            // Two-column layout: left = current question, right = history
            let columns =
                Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)])
                    .split(outer[1]);

            // Left column: question + options/status
            let left =
                Layout::vertical([Constraint::Length(3), Constraint::Min(3)]).split(columns[0]);

            f.render_widget(
                Paragraph::new(format!("题目: {}", app.question_text))
                    .style(
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    )
                    .wrap(Wrap { trim: true }),
                left[0],
            );

            match &app.phase {
                QuizPhase::WaitingLlm => {
                    f.render_widget(
                        Paragraph::new(format!("{} AI 思考中...", app.spin_char()))
                            .style(Style::default().fg(Color::Cyan)),
                        left[1],
                    );
                }
                QuizPhase::Submitting => {
                    f.render_widget(
                        Paragraph::new(format!("正在提交第 {} 题答案...", num))
                            .style(Style::default().fg(Color::Cyan)),
                        left[1],
                    );
                }
                _ => {
                    let mut opts = String::new();
                    for (i, a) in app.answers.iter().enumerate() {
                        let label = (b'A' + i as u8) as char;
                        opts.push_str(&format!("{}. {}\n", label, a.text));
                    }
                    f.render_widget(
                        Paragraph::new(opts)
                            .style(Style::default().fg(Color::White))
                            .wrap(Wrap { trim: true }),
                        left[1],
                    );
                }
            }

            // Right column: history
            let history_block = Block::default()
                .borders(Borders::LEFT)
                .title(" 已答题目 ")
                .style(Style::default().fg(Color::DarkGray));
            let history_inner = history_block.inner(columns[1]);
            f.render_widget(history_block, columns[1]);

            if app.history.is_empty() {
                f.render_widget(
                    Paragraph::new("暂无答题记录")
                        .style(Style::default().fg(Color::DarkGray))
                        .alignment(Alignment::Center),
                    history_inner,
                );
            } else {
                use ratatui::text::{Line, Span};
                let mut lines: Vec<Line> = vec![];
                for item in app.history.iter().rev() {
                    let mark = if item.correct { "✓" } else { "✗" };
                    let mark_color = if item.correct {
                        Color::Green
                    } else {
                        Color::Red
                    };
                    // Question header line with result indicator
                    lines.push(Line::from(vec![
                        Span::styled(
                            format!("Q{}. ", item.num),
                            Style::default().fg(Color::Yellow),
                        ),
                        Span::styled(
                            mark.to_string(),
                            Style::default().fg(mark_color).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!(" {}", item.question),
                            Style::default().fg(Color::White),
                        ),
                    ]));
                    // Options with highlight on chosen one
                    for (i, opt) in item.options.iter().enumerate() {
                        let label = (b'A' + i as u8) as char;
                        if i + 1 == item.chosen_idx {
                            lines.push(Line::from(vec![
                                Span::styled(
                                    format!("  > {}. ", label),
                                    Style::default().fg(mark_color).add_modifier(Modifier::BOLD),
                                ),
                                Span::styled(opt.clone(), Style::default().fg(mark_color)),
                            ]));
                        } else {
                            lines.push(Line::from(Span::styled(
                                format!("    {}. {}", label, opt),
                                Style::default().fg(Color::DarkGray),
                            )));
                        }
                    }
                    lines.push(Line::from(""));
                }
                f.render_widget(
                    Paragraph::new(lines)
                        .style(Style::default().fg(Color::White))
                        .wrap(Wrap { trim: true })
                        .scroll((app.history_scroll as u16, 0)),
                    history_inner,
                );
            }

            f.render_widget(
                Paragraph::new("ESC 退出答题")
                    .style(Style::default().fg(Color::DarkGray))
                    .alignment(Alignment::Center),
                outer[2],
            );
        }

        QuizPhase::Captcha(cs) => {
            draw_captcha(
                f,
                inner,
                cs,
                app.captcha_picker.as_ref(),
                app.captcha_image.as_ref(),
            );
        }

        QuizPhase::Finished { score, scores } => {
            let mut lines = vec![format!("总分: {}", score)];
            if !scores.is_empty() {
                lines.push(String::new());
                lines.push("分类得分:".to_string());
                for s in scores {
                    lines.push(format!("  {}: {}/{}", s.category, s.score, s.total));
                }
            }
            lines.push(String::new());
            if *score >= 60 {
                lines.push("恭喜您通过了答题！".to_string());
            } else {
                lines.push("未能通过答题，请重新运行程序再次答题".to_string());
                lines.push("提示: 知识区和历史区的正确率会更高".to_string());
            }

            let chunks = Layout::vertical([
                Constraint::Length(2),
                Constraint::Min(5),
                Constraint::Length(2),
                Constraint::Length(1),
            ])
            .split(inner);

            f.render_widget(
                Paragraph::new("==========答题结果==========")
                    .style(
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )
                    .alignment(Alignment::Center),
                chunks[0],
            );
            f.render_widget(
                Paragraph::new(lines.join("\n"))
                    .style(if *score >= 60 {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default().fg(Color::Red)
                    })
                    .alignment(Alignment::Center)
                    .wrap(Wrap { trim: true }),
                chunks[1],
            );
            f.render_widget(
                Paragraph::new("  [ Enter 返回首页 ]  ")
                    .style(selected_style(Color::Cyan))
                    .alignment(Alignment::Center),
                chunks[2],
            );
        }

        QuizPhase::Error(msg) => {
            let chunks = Layout::vertical([
                Constraint::Percentage(35),
                Constraint::Length(3),
                Constraint::Length(2),
                Constraint::Percentage(35),
            ])
            .split(inner);
            f.render_widget(
                Paragraph::new(format!("错误: {}", msg))
                    .style(Style::default().fg(Color::Red))
                    .alignment(Alignment::Center)
                    .wrap(Wrap { trim: true }),
                chunks[1],
            );
            f.render_widget(
                Paragraph::new("  [ Enter 返回首页 ]  ")
                    .style(selected_style(Color::Cyan))
                    .alignment(Alignment::Center),
                chunks[2],
            );
        }
    }
}

fn draw_captcha(
    f: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    cs: &CaptchaState,
    picker: Option<&ratatui_image::picker::Picker>,
    img: Option<&image::DynamicImage>,
) {
    use ratatui::{
        layout::{Alignment, Constraint, Layout},
        style::{Color, Style},
        widgets::{Block, Borders, Paragraph},
    };

    let has_image = picker.is_some() && img.is_some();
    let image_height = if has_image { 12 } else { 2 };

    let chunks = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(8),
        Constraint::Length(image_height),
        Constraint::Min(3),
        Constraint::Length(2),
        Constraint::Length(1),
    ])
    .split(area);

    f.render_widget(
        Paragraph::new("需要验证码验证（空格键选择分类，最多3个）")
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Center),
        chunks[0],
    );

    let mut cat_text = String::new();
    for (i, cat) in cs.categories.iter().enumerate() {
        let check = if cat.selected { "☑" } else { "☐" };
        let marker = if matches!(cs.focus, CaptchaFocus::Categories) && i == cs.cat_focus {
            " >"
        } else {
            "  "
        };
        cat_text.push_str(&format!("{}{} {}. {}\n", marker, check, cat.id, cat.name));
    }
    let cat_style = if matches!(cs.focus, CaptchaFocus::Categories) {
        Style::default().fg(Color::White)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    f.render_widget(
        Paragraph::new(cat_text).style(cat_style),
        chunks[1],
    );

    match (picker, img) {
        (Some(p), Some(dyn_img)) => {
            // Two-part layout in image area: image on top (Min), URL text below (Length 2)
            let img_layout = Layout::vertical([
                Constraint::Min(1),
                Constraint::Length(2),
            ])
            .split(chunks[2]);

            // Try rendering the image; if it fails, just skip (URL is shown below anyway)
            if let Ok(protocol) = p.new_protocol(dyn_img.clone(), img_layout[0], ratatui_image::Resize::Fit(None)) {
                f.render_widget(ratatui_image::Image::new(&protocol), img_layout[0]);
            }

            let browser_style = if matches!(cs.focus, CaptchaFocus::OpenBrowser) {
                selected_style(Color::Cyan)
            } else {
                dim_style(Color::DarkGray)
            };
            f.render_widget(
                Paragraph::new("[ 使用浏览器打开验证码 ]")
                    .style(browser_style)
                    .alignment(Alignment::Center),
                img_layout[1],
            );
        }
        _ => {
            let browser_style = if matches!(cs.focus, CaptchaFocus::OpenBrowser) {
                selected_style(Color::Cyan)
            } else {
                dim_style(Color::DarkGray)
            };
            f.render_widget(
                Paragraph::new("[ 使用浏览器打开验证码 ]")
                    .style(browser_style)
                    .alignment(Alignment::Center),
                chunks[2],
            );
        }
    }

    let input_block = Block::default()
        .borders(Borders::ALL)
        .title(" 验证码 ")
        .style(if matches!(cs.focus, CaptchaFocus::Input) {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        });
    let input_inner = input_block.inner(chunks[3]);
    f.render_widget(input_block, chunks[3]);
    f.render_widget(
        Paragraph::new(format!("{}▎", cs.input)).style(Style::default().fg(Color::White)),
        input_inner,
    );

    if cs.error.is_empty() {
        let submit_style = if matches!(cs.focus, CaptchaFocus::Submit) {
            selected_style(Color::Green)
        } else {
            dim_style(Color::DarkGray)
        };
        f.render_widget(
            Paragraph::new("  [ 提交 ]  ")
                .style(submit_style)
                .alignment(Alignment::Center),
            chunks[4],
        );
    } else {
        f.render_widget(
            Paragraph::new(format!("  {}  ", cs.error))
                .style(Style::default().fg(Color::Red))
                .alignment(Alignment::Center),
            chunks[4],
        );
    }

    f.render_widget(
        Paragraph::new("↑↓ 选择分类  空格 勾选  Tab 切换  R 刷新  ESC 取消")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center),
        chunks[5],
    );
}

fn center_text(
    f: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    msg: &str,
    color: ratatui::style::Color,
) {
    use ratatui::{
        layout::{Alignment, Constraint, Layout},
        style::Style,
        widgets::Paragraph,
    };
    let chunks = Layout::vertical([
        Constraint::Percentage(40),
        Constraint::Length(1),
        Constraint::Percentage(40),
    ])
    .split(area);
    f.render_widget(
        Paragraph::new(msg)
            .style(Style::default().fg(color))
            .alignment(Alignment::Center),
        chunks[1],
    );
}
