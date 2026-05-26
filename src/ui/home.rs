use crate::app::{App, HomeSelection};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, Wrap},
};

pub fn draw(f: &mut Frame, app: &App) {
    let size = f.area();
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Bili-Hardcore ")
        .title_alignment(Alignment::Center)
        .style(Style::default().fg(Color::Cyan));
    let inner = block.inner(size);
    f.render_widget(block, size);

    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(3),
        Constraint::Length(8),
        Constraint::Length(1),
    ])
    .split(inner);

    let title = Paragraph::new("B站硬核会员自动答题工具")
        .style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center);
    f.render_widget(title, chunks[0]);

    let intro = Paragraph::new(
        "本软件免费且代码开源\n源码 & 问题反馈: https://github.com/Karben233/bili-hardcore",
    )
    .style(Style::default().fg(Color::DarkGray))
    .alignment(Alignment::Center)
    .wrap(Wrap { trim: true });
    f.render_widget(intro, chunks[1]);

    let menu = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(2),
        Constraint::Length(2),
    ])
    .split(chunks[2]);

    let styles = match app.home_sel {
        HomeSelection::StartQuiz => (
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            Style::default().fg(Color::DarkGray),
            Style::default().fg(Color::DarkGray),
        ),
        HomeSelection::Config => (
            Style::default().fg(Color::DarkGray),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            Style::default().fg(Color::DarkGray),
        ),
        HomeSelection::Quit => (
            Style::default().fg(Color::DarkGray),
            Style::default().fg(Color::DarkGray),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
    };

    f.render_widget(
        Paragraph::new("  ▶  开始答题  ")
            .style(styles.0)
            .alignment(Alignment::Center),
        menu[0],
    );
    f.render_widget(
        Paragraph::new("  ⚙  配置  ")
            .style(styles.1)
            .alignment(Alignment::Center),
        menu[1],
    );
    f.render_widget(
        Paragraph::new("  ✕  退出  ")
            .style(styles.2)
            .alignment(Alignment::Center),
        menu[2],
    );

    f.render_widget(
        Paragraph::new("↑↓ 选择  Enter 确认")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center),
        chunks[3],
    );
}
