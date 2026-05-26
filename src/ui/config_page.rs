use crate::app::{App, ConfigFocus};
use ratatui::style::{Color, Modifier, Style};

const LABELS: [&str; 3] = ["API URL", "模型名称", "API Key"];

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

    let focus = match app.cfg_focus {
        ConfigFocus::BaseUrl => 0,
        ConfigFocus::Model => 1,
        ConfigFocus::ApiKey => 2,
        ConfigFocus::SaveBtn => 3,
        ConfigFocus::ResetBtn => 4,
    };

    let chunks = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(1),
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
            let pos = app.cfg_cursors[i].min(text.len());
            text.insert(pos, '|');
        }
        f.render_widget(
            Paragraph::new(text).style(Style::default().fg(Color::White)),
            fi,
        );
    }

    // Buttons row: save and reset side by side
    let btn_layout = Layout::horizontal([
        Constraint::Percentage(50),
        Constraint::Percentage(50),
    ])
    .split(chunks[5]);

    let save_color = if app.cfg_focus == ConfigFocus::SaveBtn {
        selected_style(Color::Green)
    } else {
        dim_style(Color::DarkGray)
    };
    f.render_widget(
        Paragraph::new("[ 保存 ]")
            .style(save_color)
            .alignment(Alignment::Center),
        btn_layout[0],
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
        btn_layout[1],
    );

    f.render_widget(
        Paragraph::new("Tab/↑↓ 切换  Enter 确认  ESC 返回")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center),
        chunks[6],
    );
}

fn draw_reset_confirm(
    f: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    confirm: bool,
) {
    use ratatui::{
        layout::{Alignment, Constraint, Layout},
        style::{Color, Modifier, Style},
        widgets::Paragraph,
    };

    let chunks = Layout::vertical([
        Constraint::Percentage(30),
        Constraint::Length(2),
        Constraint::Length(1),
        Constraint::Length(2),
        Constraint::Length(2),
        Constraint::Length(1),
        Constraint::Percentage(30),
    ])
    .split(area);

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

    // Buttons row: cancel and confirm side by side
    let btn_layout = Layout::horizontal([
        Constraint::Percentage(50),
        Constraint::Percentage(50),
    ])
    .split(chunks[3]);

    f.render_widget(
        Paragraph::new("[ 取消 ]")
            .style(if !confirm {
                selected_style(Color::Green)
            } else {
                dim_style(Color::DarkGray)
            })
            .alignment(Alignment::Center),
        btn_layout[0],
    );

    f.render_widget(
        Paragraph::new("[ 确认重置 ]")
            .style(if confirm {
                selected_style(Color::Red)
            } else {
                dim_style(Color::DarkGray)
            })
            .alignment(Alignment::Center),
        btn_layout[1],
    );

    f.render_widget(
        Paragraph::new("←→ 选择  Enter 确认  ESC 取消")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center),
        chunks[5],
    );
}
