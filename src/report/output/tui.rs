//! Interactive TUI for exploring lint-quality scan results.

mod app;
mod tree;
mod widgets;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{EnterAlternateScreen, enable_raw_mode},
};
use ratatui::{
    DefaultTerminal,
    layout::{Constraint, Layout},
};

use crate::report::Report;

use app::{App, DATA_VIEWS, DataView, Focus};
use widgets::{draw_data_panel, draw_filter_panel, draw_footer, draw_header};

pub fn run_tui(report: &Report) -> Result<()> {
    enable_raw_mode()?;
    crossterm::execute!(std::io::stdout(), EnterAlternateScreen)?;
    let terminal = ratatui::init();
    let result = run_app(terminal, report);
    ratatui::restore();
    result
}

fn run_app(mut terminal: DefaultTerminal, report: &Report) -> Result<()> {
    let mut app = App::new(report);

    loop {
        terminal.draw(|f| {
            let [header_area, body_area, footer_area] = Layout::vertical([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .areas(f.area());

            draw_header(f, &app, header_area);

            let [left_area, right_area] =
                Layout::horizontal([Constraint::Percentage(35), Constraint::Percentage(65)])
                    .areas(body_area);

            draw_filter_panel(f, &mut app, left_area);
            draw_data_panel(f, &mut app, right_area);
            draw_footer(f, &app, footer_area);
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(()),

                KeyCode::Tab => {
                    app.focus = match app.focus {
                        Focus::Filters => Focus::Data,
                        Focus::Data => Focus::Filters,
                    };
                }

                KeyCode::Left | KeyCode::Char('h') => match app.focus {
                    Focus::Filters => app.prev_dimension(),
                    Focus::Data => app.prev_data_view(),
                },
                KeyCode::Right | KeyCode::Char('l') => match app.focus {
                    Focus::Filters => app.next_dimension(),
                    Focus::Data => app.next_data_view(),
                },

                KeyCode::Down | KeyCode::Char('j') => match app.focus {
                    Focus::Filters => app.scroll_down_filter(),
                    Focus::Data => app.scroll_down_data(),
                },
                KeyCode::Up | KeyCode::Char('k') => match app.focus {
                    Focus::Filters => app.scroll_up_filter(),
                    Focus::Data => app.scroll_up_data(),
                },

                KeyCode::Char(' ') | KeyCode::Enter => match app.focus {
                    Focus::Filters => app.toggle_filter(),
                    Focus::Data if DATA_VIEWS[app.data_view] == DataView::Directories => {
                        app.toggle_dir_expand();
                    }
                    _ => {}
                },

                KeyCode::Char('c') => app.clear_filters(),

                _ => {}
            }
        }
    }
}
