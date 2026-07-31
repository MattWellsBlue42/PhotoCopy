use crossterm::event::{self, KeyCode};

use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, List, ListState};
use ratatui::{DefaultTerminal, Frame};

use std::path::{Path, PathBuf};

use std::io;
use std::fs;

use std::env;

/// Everything the UI needs to know at any moment.
pub struct App {
    /// Directory we are currently looking inside of.
    cwd: PathBuf,
    /// Subdirectories of `cwd`, sorted by name. Index 0 is always "..".
    entries: Vec<PathBuf>,
    /// Which row the cursor is on. Owned by ratatui so it can scroll for us.
    list_state: ListState,
    /// Set when the user presses space; the loop exits and main prints it.
    pub chosen: Option<PathBuf>,
}

impl App {
    pub fn new() -> io::Result<Self> {
        let start = env::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let mut app = Self {
            cwd: start,
            entries: Vec::new(),
            list_state: ListState::default(),
            chosen: None,
        };
        app.refresh()?;
        ratatui::run(|terminal| app.run(terminal))?;
        Ok(app)
    }

    /// Re-read `cwd` and put the cursor back at the top.
    fn refresh(&mut self) -> io::Result<()> {
        let mut dirs: Vec<PathBuf> = fs::read_dir(&self.cwd)?
            // read_dir hands us a Result<DirEntry> per item; drop the ones that
            // failed so one unreadable file doesn't kill the whole listing.
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .collect();

        dirs.sort();

        self.entries = Vec::with_capacity(dirs.len() + 1);
        self.entries.push(self.cwd.join(".."));
        self.entries.extend(dirs);

        self.list_state.select(Some(0));
        Ok(())
    }

    /// Text shown for each row.
    fn labels(&self) -> Vec<String> {
        self.entries
            .iter()
            .map(|path| match path.file_name() {
                Some(name) => name.to_string_lossy().into_owned(),
                // ".." has no file_name(), and neither does "/".
                None => "..".to_string(),
            })
            .collect()
    }

    fn selected(&self) -> Option<&Path> {
        let index = self.list_state.selected()?;
        self.entries.get(index).map(PathBuf::as_path)
    }

    /// Move the cursor by `delta` rows, wrapping around both ends.
    fn move_cursor(&mut self, delta: isize) {
        let len = self.entries.len();
        if len == 0 {
            return;
        }

        let current = self.list_state.selected().unwrap_or(0) as isize;
        // rem_euclid always returns a non-negative remainder, so -1 wraps to len - 1.
        let next = (current + delta).rem_euclid(len as isize) as usize;

        self.list_state.select(Some(next));
    }

    /// Walk into the highlighted directory.
    fn enter(&mut self) -> io::Result<()> {
        let Some(target) = self.selected().map(Path::to_path_buf) else {
            return Ok(());
        };

        // canonicalize() resolves ".." and symlinks into a real absolute path,
        // so `cwd` never turns into ~/a/b/../../c nonsense.
        let target = target.canonicalize()?;

        // Only move if the destination is actually readable; otherwise put the
        // old directory back and let the user pick something else.
        let previous = std::mem::replace(&mut self.cwd, target);
        if self.refresh().is_err() {
            self.cwd = previous;
            self.refresh()?;
        }
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        let block = Block::bordered()
            .title(format!(" {} ", self.cwd.display()))
            .title_bottom(" ↑/↓ move · enter open · space confirm · q quit ");

        let list = List::new(self.labels())
            .block(block)
            .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
            .highlight_symbol("> ");

        // "Stateful" because the widget writes back into `list_state` (scroll
        // offset) while it draws.
        frame.render_stateful_widget(list, frame.area(), &mut self.list_state);
    }

    /// Returns Ok(true) when the loop should stop.
    fn handle_key(&mut self, code: KeyCode) -> io::Result<bool> {
        match code {
            KeyCode::Up => self.move_cursor(-1),
            KeyCode::Down => self.move_cursor(1),
            KeyCode::Enter => self.enter()?,
            KeyCode::Char(' ') => {
                self.chosen = Some(self.cwd.clone());
                return Ok(true);
            }
            KeyCode::Char('q') | KeyCode::Esc => return Ok(true),
            _ => {}
        }
        Ok(false)
    }

    fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        loop {
            terminal.draw(|frame| self.draw(frame))?;

            // Ignore key *releases* and mouse/resize events; only react to presses.
            if let Some(key) = event::read()?.as_key_press_event() {
                if self.handle_key(key.code)? {
                    return Ok(());
                }
            }
        }
    }
}
