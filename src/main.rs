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
#[command(
    name = "bili-hardcore",
    version,
    about = "B站硬核会员自动答题工具",
    help_template = "{about}\n\n用法: {usage}\n\n命令:\n{subcommands}\n参数:\n{positionals}\n选项:\n{options}",
    subcommand_help_heading = "命令",
    next_help_heading = "参数"
)]
struct Cli {
    /// API 基础 URL
    url: Option<String>,
    /// 模型名称
    model: Option<String>,
    /// API 密钥
    #[arg(short = 'k', long = "api-key", help_heading = "选项")]
    api_key: Option<String>,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(clap::Subcommand)]
enum Commands {
    /// 更新到最新版本
    Update,
    /// 卸载 bili-hardcore
    Uninstall,
}

#[cfg(debug_assertions)]
fn setup_logging() -> Result<tracing_appender::non_blocking::WorkerGuard, Box<dyn std::error::Error>> {
    let log_dir = std::path::Path::new("./logs");
    let _ = std::fs::create_dir_all(log_dir);
    let file_appender = tracing_appender::rolling::never(log_dir, "bili-hardcore.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(false)
        .with_timer(tracing_subscriber::fmt::time::LocalTime::new(time::macros::format_description!("[year]-[month]-[day] [hour]:[minute]:[second]")))
        .init();
    Ok(guard)
}

#[cfg(not(debug_assertions))]
fn setup_logging() -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    if let Some(cmd) = &cli.command {
        match cmd {
            Commands::Update => return run_update().await,
            Commands::Uninstall => return uninstall(),
        }
    }

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

fn uninstall() -> Result<(), Box<dyn std::error::Error>> {
    let exe_path = std::env::current_exe()?;
    let exe_display = exe_path.display();

    if !exe_path.exists() {
        eprintln!("未找到可执行文件: {exe_display}");
        std::process::exit(1);
    }

    print!("确认卸载 bili-hardcore ({exe_display})? [y/N] ");
    std::io::Write::flush(&mut std::io::stdout())?;
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    if !input.trim().eq_ignore_ascii_case("y") {
        println!("已取消");
        return Ok(());
    }

    #[cfg(unix)]
    {
        std::fs::remove_file(&exe_path)?;
        println!("已删除: {exe_display}");
        println!();
        println!("如果之前通过 install.sh 安装，可移除 PATH 配置:");
        println!("  ~/.local/bin 可能已为空，可直接删除");
    }

    #[cfg(windows)]
    {
        let script = format!(
            "@echo off\r\nping -n 2 127.0.0.1 >nul\r\ndel /f \"{}\"\r\necho 已删除: {}\r\npause\r\n",
            exe_display, exe_display
        );
        let tmp = std::env::temp_dir().join("bili-hardcore-uninstall.bat");
        std::fs::write(&tmp, script)?;
        std::process::Command::new("cmd")
            .args(["/C", "start", &tmp.to_string_lossy()])
            .spawn()?;
    }

    println!("卸载完成");
    Ok(())
}

const REPO: &str = "Karben233/bili-hardcore";

async fn run_update() -> Result<(), Box<dyn std::error::Error>> {
    let current_version = env!("CARGO_PKG_VERSION");
    println!("当前版本: v{current_version}");

    println!("正在检查最新版本...");
    let client = reqwest::Client::new();
    let release: serde_json::Value = client
        .get(format!("https://api.github.com/repos/{REPO}/releases/latest"))
        .header("User-Agent", "bili-hardcore")
        .send()
        .await?
        .json()
        .await?;

    let latest_tag = release["tag_name"].as_str().unwrap_or("unknown");
    let latest_version = latest_tag.trim_start_matches('v');
    println!("最新版本: {latest_tag}");

    if latest_version == current_version {
        println!("已是最新版本");
        return Ok(());
    }

    // Detect platform
    let (_os, arch) = detect_platform()?;

    // Build download filename
    let filename = if cfg!(windows) {
        format!("bili-hardcore-{latest_tag}-windows-x64.zip")
    } else if cfg!(target_os = "macos") {
        format!("bili-hardcore-{latest_tag}-darwin-universal.tar.gz")
    } else {
        let variant = "-musl";
        format!("bili-hardcore-{latest_tag}-linux-{arch}{variant}.tar.gz")
    };

    let url = format!("https://github.com/{REPO}/releases/download/{latest_tag}/{filename}");
    println!("正在下载 {filename}...");

    let resp = client.get(&url).send().await?;
    if !resp.status().is_success() {
        eprintln!("下载失败: HTTP {}", resp.status());
        std::process::exit(1);
    }
    let bytes = resp.bytes().await?;

    let tmp_dir = std::env::temp_dir().join("bili-hardcore-update");
    let _ = std::fs::remove_dir_all(&tmp_dir);
    std::fs::create_dir_all(&tmp_dir)?;

    let archive_path = tmp_dir.join(&filename);
    std::fs::write(&archive_path, &bytes)?;

    // Extract
    println!("正在解压...");
    let binary_name = if cfg!(windows) {
        "bili-hardcore.exe"
    } else {
        "bili-hardcore"
    };

    #[cfg(windows)]
    {
        let archive_str = archive_path.to_string_lossy();
        let exit = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!("Expand-Archive -Path '{archive_str}' -DestinationPath '{}' -Force", tmp_dir.display()),
            ])
            .status()?;
        if !exit.success() {
            eprintln!("解压失败");
            std::process::exit(1);
        }
    }

    #[cfg(unix)]
    {
        let exit = std::process::Command::new("tar")
            .args(["-xzf", &archive_path.to_string_lossy(), "-C", &tmp_dir.to_string_lossy()])
            .status()?;
        if !exit.success() {
            eprintln!("解压失败");
            std::process::exit(1);
        }
    }

    let new_binary = tmp_dir.join(binary_name);
    if !new_binary.exists() {
        eprintln!("解压后未找到 {binary_name}");
        std::process::exit(1);
    }

    // Replace current binary
    let exe_path = std::env::current_exe()?;
    println!("正在替换 {}...", exe_path.display());

    #[cfg(unix)]
    {
        let _ = std::fs::remove_file(&exe_path);
        std::fs::copy(&new_binary, &exe_path)?;
    }

    #[cfg(windows)]
    {
        // Windows can't replace running exe, rename old then copy new
        let old = exe_path.with_extension("old.exe");
        let _ = std::fs::remove_file(&old);
        std::fs::rename(&exe_path, &old)?;
        std::fs::copy(&new_binary, &exe_path)?;
        let _ = std::fs::remove_file(&old);
    }

    let _ = std::fs::remove_dir_all(&tmp_dir);

    println!("更新完成: v{current_version} → {latest_tag}");
    Ok(())
}

fn detect_platform() -> Result<(&'static str, &'static str), Box<dyn std::error::Error>> {
    let os = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        return Err("不支持的操作系统".into());
    };

    let arch = if cfg!(target_arch = "x86_64") {
        "x64"
    } else if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        return Err("不支持的架构".into());
    };

    Ok((os, arch))
}
