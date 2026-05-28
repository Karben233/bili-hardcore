use crate::app::{App, ConfigFocus};
use ratatui::style::{Color, Modifier, Style};

const LABELS: [&str; 3] = ["API URL", "模型名称", "API Key"];

fn focus_index(focus: ConfigFocus) -> usize {
    match focus {
        ConfigFocus::BaseUrl => 0,
        ConfigFocus::Model => 1,
        ConfigFocus::ApiKey => 2,
        ConfigFocus::ThinkingToggle => 3,
        ConfigFocus::FastModeToggle => 4,
        ConfigFocus::SaveBtn => 5,
        ConfigFocus::ResetBtn => 6,
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
        Constraint::Min(1),
        Constraint::Length(2),
        Constraint::Length(2),
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
        "[✓] 开启 - 准确率高，速度慢，可能存在兼容性问题"
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

    let save_color = if app.cfg_focus == ConfigFocus::SaveBtn {
        selected_style(Color::Green)
    } else {
        dim_style(Color::DarkGray)
    };
    f.render_widget(
        Paragraph::new("[ 保存 ]")
            .style(save_color)
            .alignment(Alignment::Center),
        chunks[6],
    );

    let reset_color = if app.cfg_focus == ConfigFocus::ResetBtn {
        selected_style(Color::Red)
    } else {
        dim_style(Color::DarkGray)
    };
    f.render_widget(
        Paragraph::new("[ 重置 ]")
            .style(reset_color)
            .alignment(Alignment::Center),
        chunks[7],
    );

    f.render_widget(
        Paragraph::new("↑↓ 切换  Space 勾选  Enter 确认  ESC 返回")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center),
        chunks[8],
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
        Paragraph::new("[ 取消 ]")
            .style(if choice == 0 {
                selected_style(Color::Green)
            } else {
                dim_style(Color::DarkGray)
            })
            .alignment(Alignment::Center),
        chunks[3],
    );

    f.render_widget(
        Paragraph::new("[ 仅退出登录 ]")
            .style(if choice == 1 {
                selected_style(Color::Yellow)
            } else {
                dim_style(Color::DarkGray)
            })
            .alignment(Alignment::Center),
        chunks[4],
    );

    f.render_widget(
        Paragraph::new("[ 确认重置 ]")
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
