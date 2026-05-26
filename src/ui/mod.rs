pub mod config_page;
pub mod home;
pub mod quiz;

use crate::app::{App, Page};
use ratatui::Frame;

pub fn draw(f: &mut Frame, app: &App) {
    match app.page {
        Page::Home => home::draw(f, app),
        Page::Config => config_page::draw(f, app),
        Page::Quiz => quiz::draw(f, app),
    }
}
