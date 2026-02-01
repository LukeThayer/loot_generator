mod app;
mod input;
mod persistence;
mod views;

use app::{App, Mode};
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::path::Path;

fn main() -> Result<(), io::Error> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Load config and create app
    let config_path = Path::new("config");
    let app = App::new(config_path);

    // Run app
    let res = run_app(&mut terminal, app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("Error: {err:?}");
    }

    Ok(())
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, mut app: App) -> io::Result<()> {
    loop {
        terminal.draw(|f| app.render(f))?;

        if let Event::Key(key) = event::read()? {
            // Only handle key press events
            if key.kind != KeyEventKind::Press {
                continue;
            }

            // Clear message on any keypress
            app.clear_message();

            // Handle quit
            if key.code == KeyCode::Char('q') && app.mode == Mode::Browse {
                if app.dirty.is_dirty() {
                    app.show_quit_confirm = true;
                } else {
                    return Ok(());
                }
            }

            // Ctrl+C always quits
            if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                return Ok(());
            }

            // Handle quit confirmation popup
            if app.show_quit_confirm {
                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => return Ok(()),
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                        app.show_quit_confirm = false;
                    }
                    _ => {}
                }
                continue;
            }

            // Handle delete confirmation popup
            if app.show_delete_confirm {
                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        app.confirm_delete();
                        app.show_delete_confirm = false;
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                        app.show_delete_confirm = false;
                    }
                    _ => {}
                }
                continue;
            }

            // Handle file picker popup
            if app.show_file_picker {
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => app.file_picker_up(),
                    KeyCode::Down | KeyCode::Char('j') => app.file_picker_down(),
                    KeyCode::Enter => app.file_picker_select(),
                    KeyCode::Char('n') => app.file_picker_new(),
                    KeyCode::Esc => app.show_file_picker = false,
                    _ => {}
                }
                continue;
            }

            // Handle new file name input
            if app.show_new_file_input {
                match key.code {
                    KeyCode::Enter => app.confirm_new_file(),
                    KeyCode::Esc => app.show_new_file_input = false,
                    KeyCode::Backspace => {
                        app.new_file_name.pop();
                    }
                    KeyCode::Char(c) => {
                        if c.is_alphanumeric() || c == '_' || c == '-' {
                            app.new_file_name.push(c);
                        }
                    }
                    _ => {}
                }
                continue;
            }

            // Handle mode-specific input
            match app.mode {
                Mode::Browse => handle_browse_mode(&mut app, key.code),
                Mode::Edit => handle_edit_mode(&mut app, key.code, key.modifiers),
                Mode::Create => handle_create_mode(&mut app, key.code, key.modifiers),
            }
        }
    }
}

fn handle_browse_mode(app: &mut App, code: KeyCode) {
    match code {
        // Tab switching with number keys
        KeyCode::Char('1') => app.switch_tab(app::ConfigTab::BaseTypes),
        KeyCode::Char('2') => app.switch_tab(app::ConfigTab::Affixes),
        KeyCode::Char('3') => app.switch_tab(app::ConfigTab::AffixPools),
        KeyCode::Char('4') => app.switch_tab(app::ConfigTab::Currencies),
        KeyCode::Char('5') => app.switch_tab(app::ConfigTab::Uniques),

        // List navigation
        KeyCode::Up | KeyCode::Char('k') => app.list_up(),
        KeyCode::Down | KeyCode::Char('j') => app.list_down(),

        // Actions
        KeyCode::Enter | KeyCode::Char('e') => app.enter_edit_mode(),
        KeyCode::Char('n') => app.enter_create_mode(),
        KeyCode::Char('d') | KeyCode::Delete => app.request_delete(),
        KeyCode::Char('s') => app.save_current(),

        // Ctrl+S to save all
        _ => {}
    }
}

fn handle_edit_mode(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    // Ctrl+S saves and exits edit mode
    if code == KeyCode::Char('s') && modifiers.contains(KeyModifiers::CONTROL) {
        app.save_and_exit_edit();
        return;
    }

    match code {
        // Nested editor navigation (must come before generic Esc handling)
        KeyCode::Esc if app.in_nested_editor() => app.exit_nested_editor(),
        KeyCode::Enter if app.in_nested_editor() => app.nested_item_edit(),
        KeyCode::Enter if app.is_nested_field() => app.enter_nested_editor(),
        KeyCode::Left if app.in_nested_editor() => app.exit_nested_editor(),
        KeyCode::Char('h') if app.in_nested_editor() && app.text_input.value().is_empty() => {
            app.exit_nested_editor()
        }
        KeyCode::Up if app.in_nested_editor() => app.nested_item_up(),
        KeyCode::Down if app.in_nested_editor() => app.nested_item_down(),
        KeyCode::Char('+') if app.in_nested_editor() && app.text_input.value().is_empty() => {
            app.nested_item_add()
        }
        KeyCode::Delete if app.in_nested_editor() && modifiers.contains(KeyModifiers::CONTROL) => {
            app.nested_item_remove()
        }
        // 'x' to remove in nested editor (when not typing in text input)
        KeyCode::Char('x') if app.in_nested_editor() && app.text_input.value().is_empty() => {
            app.nested_item_remove()
        }

        KeyCode::Esc => app.cancel_edit(),
        KeyCode::Tab => app.next_field(),
        KeyCode::BackTab => app.prev_field(),
        KeyCode::Up | KeyCode::Char('k') if modifiers.contains(KeyModifiers::CONTROL) => {
            app.prev_field()
        }
        KeyCode::Down | KeyCode::Char('j') if modifiers.contains(KeyModifiers::CONTROL) => {
            app.next_field()
        }

        // Enum picker navigation
        KeyCode::Up if app.is_enum_field() => app.enum_picker_up(),
        KeyCode::Down if app.is_enum_field() => app.enum_picker_down(),
        KeyCode::Enter if app.is_enum_field() => app.enum_picker_select(),

        // List field navigation
        KeyCode::Up if app.is_list_field() => app.list_field_up(),
        KeyCode::Down if app.is_list_field() => app.list_field_down(),
        KeyCode::Enter if app.is_list_field() => app.list_field_add(),
        KeyCode::Delete | KeyCode::Backspace
            if app.is_list_field() && modifiers.contains(KeyModifiers::CONTROL) =>
        {
            app.list_field_remove()
        }
        // Alternative: 'x' to delete selected item in list field (like vim)
        KeyCode::Char('x') if app.is_list_field() && app.text_input.value().is_empty() => {
            app.list_field_remove()
        }

        // Text input
        KeyCode::Char(c) => app.text_input_char(c),
        KeyCode::Backspace => app.text_input_backspace(),
        KeyCode::Delete => app.text_input_delete(),
        KeyCode::Left => app.text_input_left(),
        KeyCode::Right => app.text_input_right(),
        KeyCode::Home => app.text_input_home(),
        KeyCode::End => app.text_input_end(),

        _ => {}
    }
}

fn handle_create_mode(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    // Same as edit mode but first field is always the ID
    handle_edit_mode(app, code, modifiers);
}
