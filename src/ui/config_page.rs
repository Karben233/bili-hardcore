use crate::app::{App, ConfigFocus};
use crate::config;
use ratatui::style::{Color, Modifier, Style};

const LABELS: [&str; 3] = ["API URL(Completions)", "模型名称", "API Key"];

fn focus_index(focus: ConfigFocus) -> usize {
    match focus {
        ConfigFocus::BaseUrl => 0,
        ConfigFocus::Model => 1,
        ConfigFocus::ApiKey => 2,
        ConfigFocus::ThinkingToggle => 3,
        ConfigFocus::FastModeToggle => 4,
        ConfigFocus::SaveBtn => 5,
        ConfigFocus::TemplateBtn => 6,
        ConfigFocus::ResetBtn => 7,
    }
}

fn selected_style(color: Color) -> Style {
    Style::default().fg(color).add_modifier(Modifier::BOLD)
}

fn dim_style(color: Color) -> Style {
    Style::default().fg(color)
}

pub fn draw(f: &mut ratatui::Frame, app: &App) {
    use ratatui::{
        layout::{Alignment, Constraint, Layout},
        style::{Color, Style},
        widgets::{Block, Borders, Paragraph},
    };

    let size = f.area();
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" API 配置 ")
        .title_alignment(Alignment::Center);
    let inner = block.inner(size);
    f.render_widget(block, size);

    // Preset selection overlay
    if app.cfg_preset_open {
        draw_preset_select(f, inner, app.cfg_preset_sel);
        return;
    }

    // Confirmation dialog overlay
    if app.config_confirm_reset {
        draw_reset_confirm(f, inner, app.config_reset_choice);
        return;
    }

    let focus = focus_index(app.cfg_focus);

    let chunks = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(2),
        Constraint::Length(2),
        Constraint::Length(2),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .split(inner);

    f.render_widget(
        Paragraph::new("请输入 OpenAI 兼容 API 配置信息")
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Center),
        chunks[0],
    );

    for (i, label) in LABELS.iter().enumerate() {
        let is_focused = focus == i;
        let border_color = if is_focused {
            Color::Cyan
        } else {
            Color::DarkGray
        };
        let field_block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} ", label))
            .style(Style::default().fg(border_color));
        let fi = field_block.inner(chunks[i + 1]);
        f.render_widget(field_block, chunks[i + 1]);

        let val = &app.cfg_fields[i];
        let display = if i == 2 && !val.is_empty() && !is_focused {
            "*".repeat(val.len().min(20))
        } else {
            val.clone()
        };

        let mut text = display;
        if is_focused {
            let char_count = text.chars().count();
            let pos = app.cfg_cursors[i].min(char_count);
            let byte_pos = text.char_indices().nth(pos).map_or(text.len(), |(i, _)| i);
            text.insert(byte_pos, '|');
        }
        f.render_widget(
            Paragraph::new(text).style(Style::default().fg(Color::White)),
            fi,
        );
    }

    // Thinking toggle (chunks[4])
    let thinking_focused = app.cfg_focus == ConfigFocus::ThinkingToggle;
    let toggle_border_color = if thinking_focused {
        Color::Cyan
    } else {
        Color::DarkGray
    };
    let toggle_block = Block::default()
        .borders(Borders::ALL)
        .title(" 思考模式 ")
        .style(Style::default().fg(toggle_border_color));
    let toggle_inner = toggle_block.inner(chunks[4]);
    f.render_widget(toggle_block, chunks[4]);

    let toggle_text = if app.cfg_thinking {
        "[✓] 开启 - 准确率高，速度慢"
    } else {
        "[ ] 关闭 - 准确率低，速度快"
    };
    let toggle_color = if thinking_focused {
        Color::White
    } else {
        Color::DarkGray
    };
    f.render_widget(
        Paragraph::new(toggle_text).style(Style::default().fg(toggle_color)),
        toggle_inner,
    );

    // Fast mode toggle (chunks[5])
    let fast_focused = app.cfg_focus == ConfigFocus::FastModeToggle;
    let fast_border_color = if fast_focused {
        Color::Cyan
    } else {
        Color::DarkGray
    };
    let fast_block = Block::default()
        .borders(Borders::ALL)
        .title(" 快速模式 ")
        .style(Style::default().fg(fast_border_color));
    let fast_inner = fast_block.inner(chunks[5]);
    f.render_widget(fast_block, chunks[5]);

    let fast_text = if app.cfg_fast_mode {
        "[✓] 开启 - 取消答题间隔"
    } else {
        "[ ] 关闭 - 每题间隔约1秒"
    };
    let fast_color = if fast_focused {
        Color::White
    } else {
        Color::DarkGray
    };
    f.render_widget(
        Paragraph::new(fast_text).style(Style::default().fg(fast_color)),
        fast_inner,
    );

    // Save button (chunks[6])
    let save_focused = app.cfg_focus == ConfigFocus::SaveBtn;
    let save_style = if save_focused {
        selected_style(Color::Green)
    } else {
        dim_style(Color::DarkGray)
    };
    let save_text = if save_focused { "[ 保存 ]" } else { "  保存  " };
    f.render_widget(
        Paragraph::new(save_text)
            .style(save_style)
            .alignment(Alignment::Center),
        chunks[6],
    );

    // Template button (chunks[7])
    let tpl_focused = app.cfg_focus == ConfigFocus::TemplateBtn;
    let tpl_style = if tpl_focused {
        selected_style(Color::Cyan)
    } else {
        dim_style(Color::DarkGray)
    };
    let tpl_text = if tpl_focused {
        "[ 选择预设模板 ]"
    } else {
        "  选择预设模板  "
    };
    f.render_widget(
        Paragraph::new(tpl_text)
            .style(tpl_style)
            .alignment(Alignment::Center),
        chunks[7],
    );

    // Reset button (chunks[8])
    let reset_focused = app.cfg_focus == ConfigFocus::ResetBtn;
    let reset_style = if reset_focused {
        selected_style(Color::Red)
    } else {
        dim_style(Color::DarkGray)
    };
    let reset_text = if reset_focused { "[ 重置 ]" } else { "  重置  " };
    f.render_widget(
        Paragraph::new(reset_text)
            .style(reset_style)
            .alignment(Alignment::Center),
        chunks[8],
    );

    f.render_widget(
        Paragraph::new("↑↓ 切换  Space 勾选  Enter 确认  ESC 返回")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center),
        chunks[10],
    );
}

fn draw_preset_select(
    f: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    sel: usize,
) {
    use ratatui::{
        layout::{Alignment, Constraint, Layout},
        style::{Color, Modifier, Style},
        widgets::Paragraph,
    };

    let presets = config::load_presets();
    let _count = presets.len();

    let outer = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .split(area);

    // title + separator + presets (each 2 rows for breathing room)
    let preset_rows: Vec<Constraint> = std::iter::once(Constraint::Length(1)) // title
        .chain(std::iter::once(Constraint::Length(1))) // separator
        .chain(presets.iter().map(|_| Constraint::Length(2))) // each preset (taller)
        .chain(std::iter::once(Constraint::Min(1))) // flexible spacer
        .collect();

    let chunks = Layout::vertical(preset_rows).split(outer[0]);

    // Title
    f.render_widget(
        Paragraph::new("选择预设模板")
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center),
        chunks[0],
    );

    // Separator
    f.render_widget(
        Paragraph::new("─".repeat(chunks[1].width as usize))
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center),
        chunks[1],
    );

    // Preset items
    for (i, preset) in presets.iter().enumerate() {
        let is_sel = i == sel;
        let text_color = if is_sel {
            Color::Yellow
        } else {
            Color::White
        };
        let style = if is_sel {
            Style::default().fg(text_color).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(text_color)
        };

        let text = if is_sel {
            format!("[ {} ]", preset.provider_name)
        } else {
            format!("  {}  ", preset.provider_name)
        };
        f.render_widget(
            Paragraph::new(text)
                .style(style)
                .alignment(Alignment::Center),
            chunks[2 + i],
        );
    }

    // Help text at the absolute bottom
    f.render_widget(
        Paragraph::new("↑↓ 选择  Enter 确认  ESC 取消")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center),
        outer[1],
    );
}

fn draw_reset_confirm(
    f: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    choice: u8,
) {
    use ratatui::{
        layout::{Alignment, Constraint, Layout},
        style::{Color, Modifier, Style},
        widgets::Paragraph,
    };

    let outer = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .split(area);

    let chunks = Layout::vertical([
        Constraint::Percentage(30),
        Constraint::Length(2),
        Constraint::Length(1),
        Constraint::Length(2),
        Constraint::Length(2),
        Constraint::Length(2),
        Constraint::Percentage(30),
    ])
    .split(outer[0]);

    f.render_widget(
        Paragraph::new("确认重置")
            .style(
                Style::default()
                    .fg(Color::Red)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center),
        chunks[1],
    );

    f.render_widget(
        Paragraph::new("将会重置所有配置项以及登录状态")
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Center),
        chunks[2],
    );

    f.render_widget(
        Paragraph::new(if choice == 0 { "[ 取消 ]" } else { "  取消  " })
            .style(if choice == 0 {
                selected_style(Color::Green)
            } else {
                dim_style(Color::DarkGray)
            })
            .alignment(Alignment::Center),
        chunks[3],
    );

    f.render_widget(
        Paragraph::new(if choice == 1 {
            "[ 仅退出登录 ]"
        } else {
            "  仅退出登录  "
        })
        .style(if choice == 1 {
            selected_style(Color::Yellow)
        } else {
            dim_style(Color::DarkGray)
        })
        .alignment(Alignment::Center),
        chunks[4],
    );

    f.render_widget(
        Paragraph::new(if choice == 2 { "[ 确认重置 ]" } else { "  确认重置  " })
            .style(if choice == 2 {
                selected_style(Color::Red)
            } else {
                dim_style(Color::DarkGray)
            })
            .alignment(Alignment::Center),
        chunks[5],
    );

    f.render_widget(
        Paragraph::new("↑↓ 选择  Enter 确认  ESC 取消")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center),
        outer[1],
    );
}
