mod app;
mod config;
mod skills;
mod ui;

use std::io;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use app::{App, Focus, ImportConfirm, Screen};
use config::Config;

fn main() -> io::Result<()> {
    // Load or create config
    let mut config = Config::load();
    skills::normalize_skill_state_keys(&mut config);
    skills::normalize_group_skill_keys(&mut config);
    config.save(); // Ensure dirs exist

    // Ensure central store exists
    std::fs::create_dir_all(Config::central_store()).ok();

    let mut app = App::new(config);

    // Check for unmanaged skills before launching TUI
    app.check_unmanaged();

    // Sync existing symlinks on startup
    skills::sync_symlinks(&app.config);

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Main loop
    let result = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> io::Result<()> {
    while app.running {
        terminal.draw(|f| ui::draw(f, app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match app.screen {
                    Screen::Import => handle_import_keys(app, key.code),
                    Screen::Main => {
                        if app.group_name_input.is_some() {
                            handle_group_name_keys(app, key.code, key.modifiers);
                        } else if app.group_editor.is_some() {
                            handle_group_editor_keys(app, key.code);
                        } else if app.delete_confirm.is_some() {
                            handle_delete_keys(app, key.code);
                        } else if app.searching {
                            handle_search_keys(app, key.code, key.modifiers);
                        } else {
                            handle_main_keys(app, key.code);
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn handle_import_keys(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Left | KeyCode::Char('h') => {
            app.import_confirm = ImportConfirm::Yes;
        }
        KeyCode::Right | KeyCode::Char('l') => {
            app.import_confirm = ImportConfirm::No;
        }
        KeyCode::Tab => {
            app.import_confirm = match app.import_confirm {
                ImportConfirm::Yes => ImportConfirm::No,
                ImportConfirm::No => ImportConfirm::Yes,
            };
        }
        KeyCode::Char('y') => {
            app.confirm_import();
        }
        KeyCode::Char('n') | KeyCode::Esc => {
            app.skip_import();
        }
        KeyCode::Enter => match app.import_confirm {
            ImportConfirm::Yes => app.confirm_import(),
            ImportConfirm::No => app.skip_import(),
        },
        _ => {}
    }
}

fn handle_main_keys(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('q') => {
            app.running = false;
        }
        KeyCode::Tab => {
            app.toggle_focus();
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if app.focus == Focus::Groups {
                app.move_group_up();
            } else {
                app.move_skill_up();
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.focus == Focus::Groups {
                app.move_group_down();
            } else {
                app.move_skill_down();
            }
        }
        KeyCode::Char(' ') | KeyCode::Enter => {
            if app.focus == Focus::Groups {
                app.toggle_selected_group();
            } else {
                app.toggle_selected_skill();
            }
        }
        KeyCode::Char('/') => {
            app.start_search();
        }
        KeyCode::Esc => {
            if !app.search_query.is_empty() {
                app.clear_search();
            } else if app.focus == Focus::Groups {
                app.focus_skills();
            }
        }
        KeyCode::Char('a') => {
            app.activate_all();
        }
        KeyCode::Char('n') => {
            if app.focus == Focus::Groups {
                app.request_new_group();
            }
        }
        KeyCode::Char('e') => {
            if app.focus == Focus::Groups {
                app.request_edit_group();
            }
        }
        KeyCode::Char('r') => {
            if app.focus == Focus::Groups {
                app.request_rename_group();
            }
        }
        KeyCode::Char('x') => {
            if app.focus == Focus::Groups {
                app.request_delete_group();
            } else {
                app.request_delete_skill();
            }
        }
        KeyCode::Char('d') => {
            app.deactivate_all();
        }
        _ => {}
    }
}

fn handle_delete_keys(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('y') => app.confirm_delete(),
        KeyCode::Char('n') | KeyCode::Esc => app.cancel_delete(),
        _ => {}
    }
}

fn handle_group_name_keys(app: &mut App, key: KeyCode, modifiers: KeyModifiers) {
    match key {
        KeyCode::Enter => app.submit_group_name_input(),
        KeyCode::Esc => app.cancel_group_name_input(),
        KeyCode::Backspace => app.group_name_pop(),
        KeyCode::Char(c) => {
            if modifiers.contains(KeyModifiers::CONTROL) && c == 'c' {
                app.cancel_group_name_input();
            } else {
                app.group_name_push(c);
            }
        }
        _ => {}
    }
}

fn handle_group_editor_keys(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Enter => app.save_group_editor(),
        KeyCode::Esc => app.cancel_group_editor(),
        KeyCode::Char(' ') => app.toggle_group_editor_member(),
        KeyCode::Up | KeyCode::Char('k') => app.move_group_editor_up(),
        KeyCode::Down | KeyCode::Char('j') => app.move_group_editor_down(),
        _ => {}
    }
}

fn handle_search_keys(app: &mut App, key: KeyCode, modifiers: KeyModifiers) {
    match key {
        KeyCode::Enter => {
            app.end_search();
        }
        KeyCode::Esc => {
            app.clear_search();
            app.end_search();
        }
        KeyCode::Backspace => {
            app.search_pop();
        }
        KeyCode::Char(c) => {
            if modifiers.contains(KeyModifiers::CONTROL) && c == 'c' {
                app.clear_search();
                app.end_search();
            } else {
                app.search_push(c);
            }
        }
        KeyCode::Up => app.move_skill_up(),
        KeyCode::Down => app.move_skill_down(),
        _ => {}
    }
}
