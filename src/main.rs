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

use app::{App, ImportConfirm, Screen};
use config::Config;

fn main() -> io::Result<()> {
    // Load or create config
    let mut config = Config::load();
    skills::normalize_skill_state_keys(&mut config);
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
                        if app.delete_confirm.is_some() {
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
        KeyCode::Up | KeyCode::Char('k') => {
            app.move_up();
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.move_down();
        }
        KeyCode::Char(' ') | KeyCode::Enter => {
            app.toggle_selected();
        }
        KeyCode::Char('/') => {
            app.start_search();
        }
        KeyCode::Esc => {
            if !app.search_query.is_empty() {
                app.clear_search();
            }
        }
        KeyCode::Char('a') => {
            // Activate all
            for skill in &mut app.skills {
                skill.active = true;
                app.config
                    .skills
                    .insert(skill.key.clone(), config::SkillState { active: true });
            }
            app.config.save();
            skills::sync_symlinks(&app.config);
        }
        KeyCode::Char('x') => {
            app.request_delete();
        }
        KeyCode::Char('d') => {
            // Deactivate all
            for skill in &mut app.skills {
                skill.active = false;
                app.config
                    .skills
                    .insert(skill.key.clone(), config::SkillState { active: false });
            }
            app.config.save();
            skills::sync_symlinks(&app.config);
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
        KeyCode::Up => app.move_up(),
        KeyCode::Down => app.move_down(),
        _ => {}
    }
}
