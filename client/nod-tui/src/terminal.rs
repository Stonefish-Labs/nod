use std::{
    io::{self, Write},
    time::Duration,
};

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use crate::{app::AppState, ui};

const INPUT_POLL_INTERVAL: Duration = Duration::from_millis(50);

pub struct TerminalSession {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
}

impl TerminalSession {
    pub fn enter() -> Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        terminal.clear()?;
        Ok(Self { terminal })
    }

    pub fn draw(&mut self, app: &AppState) -> Result<()> {
        self.terminal.draw(|frame| ui::render(frame, app))?;
        Ok(())
    }

    pub fn read_key(&self) -> Result<Option<KeyEvent>> {
        if !event::poll(INPUT_POLL_INTERVAL)? {
            return Ok(None);
        }

        match event::read()? {
            Event::Key(key) => Ok(Some(key)),
            _ => Ok(None),
        }
    }

    pub fn ring_bell(&mut self) -> Result<()> {
        io::stdout().write_all(b"\x07")?;
        io::stdout().flush()?;
        Ok(())
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        // Terminal cleanup runs during process exit and panic unwinding, so the
        // destructor cannot report failures usefully.
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}
