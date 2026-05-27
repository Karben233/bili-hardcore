mod api;
mod app;
mod config;
mod crypto;
mod error;
mod input;
mod llm;
mod ui;

use app::App;
use clap::Parser;
use config::OpenAiConfig;
use crossterm::{
    event::{self, Event, EventStream},
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::StreamExt;
use ratatui::prelude::*;
use std::io;

#[derive(Parser)]
#[command(name = "bili-hardcore", about = "B站硬核会员自动答题工具")]
struct Cli {
    /// API 基础 URL
    url: Option<String>,
    /// 模型名称
    model: Option<String>,
    /// API 密钥
    #[arg(short = 'k', long = "api-key")]
    api_key: Option<String>,
}

fn setup_logging() -> Result<tracing_appender::non_blocking::WorkerGuard, Box<dyn std::error::Error>> {
    let log_dir = std::path::Path::new("./logs");
    let _ = std::fs::create_dir_all(log_dir);
    let file_appender = tracing_appender::rolling::never(log_dir, "bili-hardcore.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(false)
        .init();
    Ok(guard)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let cli_config = match (cli.url, cli.model, cli.api_key) {
        (Some(url), Some(model), Some(key)) => Some(OpenAiConfig {
            base_url: url.trim_end_matches('/').to_string(),
            model,
            api_key: key,
        }),
        _ => None,
    };

    let _log_guard = setup_logging()?;

    // TUI setup
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, EnterAlternateScreen)?;

    let captcha_picker = ratatui_image::picker::Picker::from_query_stdio().ok();

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(cli_config, captcha_picker);
    let result = run_app(&mut terminal, &mut app).await;

    // Restore terminal
    terminal::disable_raw_mode()?;
    crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<(), Box<dyn std::error::Error>> {
    let tick_rate = std::time::Duration::from_millis(100);
    let mut reader = EventStream::new();

    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        let mut event_fut = reader.next();
        let tick = tokio::time::sleep(tick_rate);

        tokio::select! {
            ev = &mut event_fut => {
                if let Some(Ok(ev)) = ev {
                    match ev {
                        Event::Key(key) if key.kind == event::KeyEventKind::Press => {
                            app.handle_key(key);
                        }
                        Event::Resize(_, _) => {}
                        _ => {}
                    }
                }
            }
            _ = tick => {
                app.tick();
            }
        }

        // Process async events
        while let Ok(ev) = app.rx.try_recv() {
            app.process(ev);
        }

        if app.quit {
            break;
        }
    }

    Ok(())
}
