use std::{io, path::PathBuf};
use crossterm::event::{self, Event, KeyCode };
use ratatui::{
    widgets::{Block, List, ListState, ListItem},
    DefaultTerminal,
    Frame,
    style::{Modifier, Style, Color},
};

use crate::Config;

pub struct RomEntry {
    name: String,
    path: PathBuf,
}

pub struct Menu {
    files: Vec<RomEntry>,
    selected: usize,
}

impl Menu {
    fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        loop {
            terminal.draw(|frame| {
                render(frame, self);
            })?;

            match event::read()? {
                Event::Key(key) => {
                    match key.code {
                        KeyCode::Up => self.previous(),
                        KeyCode::Down => self.next(),
                        KeyCode::Esc | KeyCode::Char('q') => break,
                        KeyCode::Enter => {
                            let mut config = Config::load_config();
                            if let Some(rom) = self.files.get(self.selected) {
                                config.write_rom_config(rom.path.clone())
                                    .map_err(io::Error::other)?;
                            }
                            break;
                        },
                        _ =>  {},
                    }
                },
                _ => {}
            }
        }
        Ok(())
    }

    fn previous(&mut self) {
        if self.selected == 0 {
            return;
        }
        self.selected -= 1;
    }

    fn next(&mut self) {
        if self.selected == self.files.len() - 1 {
            return;
        }
        self.selected += 1;
    }
}

fn render(frame: &mut Frame, menu: &Menu) {
    let area = frame.area();
    let items: Vec<ListItem> = menu
        .files
        .iter()
        .map(|rom| ListItem::new(rom.name.clone()))
        .collect();

    let list = List::new(items)
        .block(
            Block::bordered().title("Select a ROM")
        )
        .highlight_symbol(">")
        .highlight_style(Style::default().add_modifier(Modifier::BOLD).fg(Color::Black).bg(Color::White));

    let mut state = ListState::default();
    state.select(Some(menu.selected));
    frame.render_stateful_widget(list, area, &mut state);
}

pub fn load_rom_menu() -> io::Result<()> {
    let files = std::fs::read_dir("./roms/")?
        .filter_map(|rom| rom.ok())
        .filter(|rom| rom.path().is_file())
        .map(|rom| RomEntry {
            name: rom.file_name().to_string_lossy().into_owned(),
            path: rom.path(),
        })
        .collect();

    let mut menu = Menu {
        files,
        selected: 0,
    };
    ratatui::run(|terminal| menu.run(terminal))
}
