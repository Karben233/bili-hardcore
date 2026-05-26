use crate::app::{App, ConfigFocus};

const LABELS: [&str; 3] = ["API URL", "模型名称", "API Key"];

pub fn draw(f: &mut ratatui::Frame, app: &App) {
    use ratatui::{
        layout::{Alignment, Constraint, Layout},
        style::{Color, Modifier, Style},
        widgets::{Block, Borders, Paragraph},
    };

    let size = f.area();
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" API 配置 ")
        .title_alignment(Alignment::Center);
    let inner = block.inner(size);
    f.render_widget(block, size);

    let focus = match app.cfg_focus {
        ConfigFocus::BaseUrl => 0,
        ConfigFocus::Model => 1,
        ConfigFocus::ApiKey => 2,
        ConfigFocus::SaveBtn => 3,
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
            text.insert(pos, '▎');
        }
        f.render_widget(
            Paragraph::new(text).style(Style::default().fg(Color::White)),
            fi,
        );
    }

    let save_color = if app.cfg_focus == ConfigFocus::SaveBtn {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    f.render_widget(
        Paragraph::new("  [ 保存 ]  ")
            .style(save_color)
            .alignment(Alignment::Center),
        chunks[5],
    );

    f.render_widget(
        Paragraph::new("Tab/↑↓ 切换  Enter 保存  ESC 返回")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center),
        chunks[6],
    );
}
