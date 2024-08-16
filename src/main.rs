use std::io::{stdout, self};

use async_std::task::block_on;
use zbus::{proxy, Connection, Result};
mod utils;
mod ui;
mod parser;
mod fuzzy_search;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{self, Terminal, backend::CrosstermBackend};

use crate::ui::Music;
use crate::parser::*;
use crate::utils::log;

// TODO(s)
// - make closing and opening the client does not affect the index of the currently playing song
// - migrate from audiotags to lofty
// - add keybind to pause playing
// - add effect in the search bar to signify AfterSearch mode
// - add keybind to pause
// - add G, gg and g<n> vim keybinds
// - add audio slider
// - musics should play one after the other automatically, unless stated otherwise
// - searching should be extanded with to genre, artist, etc.
// - It would be nice to highlight the matched chars
// - It would be nice to display the artist 
// - forward and backward skip should play the next and the previous song of the filtered 
// song list, if a search happened
// - sort the full music list like the displyed one (if needed)
// - add row number to the music list
// - change highlight color to display that we are anticipation another character after the first g
// - add j,k and a number to go x number down or up
// - add lyrics display between the list and actions

#[derive(Default, Debug)]
pub enum ListMode {
    Search,
    #[default]
    Select,
    AfterSearch
}

#[derive(serde::Deserialize, serde::Serialize, zbus::zvariant::Type, Debug)]
struct Picture{
    data: Vec<u8>,
    typ: String
}

#[derive(serde::Deserialize, serde::Serialize, zbus::zvariant::Type, Debug)]
pub struct Metadata {
    title: String, 
    artis: String,
    genre: String,
    cover: Picture,
}

#[proxy(
    interface = "org.zbus.mplayer1",
    default_service = "org.zbus.mplayer",
    default_path = "/org/zbus/mplayer"
)]
pub trait Server {
    fn status(&self) -> Result<String>;
    fn play(&self, path: String) -> Result<bool>;
    fn end(&self) -> Result<bool>;
    fn resume(&self) -> Result<bool>;
    fn pause(&self) -> Result<bool>;
    fn show(&self) -> Result<String>;
    fn timer(&self) -> Result<String>;
    fn seek(&self, duration: f64) -> Result<bool>;
    fn volume(&self, amount:u8) -> Result<bool>;
    fn metadata(&self) -> Result<Metadata>;
    fn playing(&self) -> Result<Music>;
}

#[async_std::main]
async fn main() -> Result<()> {
    let connection = Connection::session().await.unwrap_or_else(|_|{
        panic!("Could not connect to the bus address, aborting...");
    });
    let proxy = ServerProxy::new(&connection).await?;
    proxy.status().await.unwrap_or_else(|_|{
        panic!("Server is not connected to the bust address, aborting..");
    });

    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let mut ui = ui::UI::default(proxy);
    // TODO: this should be done automatically from parsing the config and reading 
    // the music directory
    let config = Config::parse_config("./config.toml");
    let mut musics = config.config.clone().unwrap().extract_music();
    musics.sort(config.config.clone().unwrap().sorting);

    ui.musics(musics);

    let mut should_quit = false;
    while !should_quit {
        terminal.draw(|frame| {
            ui.render(frame);
        })?;
        should_quit = handle_events(&mut ui)?;
    }

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

// UGLY code!
fn handle_events<'a>(ui: &mut ui::UI<'a>) -> io::Result<bool>{
    if event::poll(std::time::Duration::from_millis(50))? {
        if let Event::Key(key) = event::read()? {
            match ui.region {
                ui::Region::List => {
                    match ui.mode {
                        ListMode::Search => {
                            if key.kind == event::KeyEventKind::Press && key.modifiers == KeyModifiers::NONE {
                                match key.code {
                                    KeyCode::Char(c) => {
                                        block_on(ui.register_querry(c));
                                    },
                                    KeyCode::Backspace => {
                                        ui.delete_char_querry()
                                    },
                                    KeyCode::Enter => {
                                        block_on(ui.play_selected_song());
                                        ui.reset_querry();
                                        ui.mode = ListMode::Select;
                                    },
                                    KeyCode::Esc => {
                                        ui.mode = ListMode::AfterSearch;
                                    },
                                    _ => {},
                                }
                            }
                        },
                        ListMode::AfterSearch => {
                            if key.kind == event::KeyEventKind::Press  {
                                match key.modifiers {
                                    KeyModifiers::NONE => {
                                        match key.code {
                                            KeyCode::Char(c) => {
                                                if c == 'j' {
                                                    ui.list_down();
                                                }else if c == 'k' {
                                                    ui.list_up();
                                                } else if c == '/' {
                                                    ui.mode = ListMode::Search;
                                                } else if c == 'q' {
                                                    ui.reset_querry();
                                                    ui.mode = ListMode::Select;
                                               } else if c == ' ' {
                                                    block_on(ui.play_selected_song());
                                                    ui.reset_querry();
                                                    ui.mode = ListMode::Select;
                                                }
                                            },
                                            KeyCode::Enter => {
                                                block_on(ui.play_selected_song());
                                                ui.reset_querry();
                                                ui.mode = ListMode::Select;
                                            },
                                            KeyCode::Esc => {
                                                ui.reset_querry();
                                                ui.mode = ListMode::Select;
                                            },
                                            _=>{}
                                        }
                                    },
                                    KeyModifiers::ALT => {
                                        match key.code {
                                            KeyCode::Char(c) => {
                                                if c == 'j' {
                                                    ui.list_down();
                                                }else if c == 'k' {
                                                    ui.list_up();
                                                }else if c == '/' {
                                                    ui.mode = ListMode::Search;
                                                }
                                            },
                                            _ => {}
                                        }
                                    },
                                    _ => {
                                    },
                                }
                            }
                        },
                        ListMode::Select => {
                            if key.kind == event::KeyEventKind::Press  {
                                match key.modifiers {
                                    // No modifiers
                                    KeyModifiers::NONE => {
                                        match ui.anticipation_mode {
                                            // No `g` key pressed beforehand
                                            ui::AncitipationMode::Normal => {
                                                match key.code {
                                                    KeyCode::Enter => {
                                                        block_on(ui.play_selected_song());
                                                    },
                                                    KeyCode::Char(c) => {
                                                        if c == '/' {
                                                            ui.mode = ListMode::Search;
                                                        }else if c == 'q' {
                                                            return Ok(true);
                                                        }else if c == 'k' {
                                                            ui.list_up();
                                                        }else if c == 'j' {
                                                            ui.list_down();
                                                        }else if c == ' ' || key.code == KeyCode::Enter {
                                                            block_on(ui.play_selected_song());
                                                        }else if c == 's' {
                                                            ui.goto_playing();
                                                        }else if c == 'g' {
                                                            ui.anticipation_mode = ui::AncitipationMode::Char('g'); 
                                                        }
                                                    },
                                                    _ => {}
                                                }
                                            },
                                            ui::AncitipationMode::Char(c) => {
                                                if c == 'g' {
                                                    match key.code {
                                                        KeyCode::Char(c) => {
                                                            if c == 'g' {
                                                                ui.goto_top();
                                                            }                                                        },
                                                        _ => {}
                                                    }
                                                }
                                                ui.anticipation_mode = ui::AncitipationMode::Normal;
                                            },
                                        }
                                    }
                                    KeyModifiers::ALT => {
                                        if let KeyCode::Char(c) = key.code {
                                            if c == 'k' {
                                                ui.select_bar_region();
                                            }else if c == 'j' {
                                                ui.select_action_region();
                                            }
                                        }
                                    }
                                    KeyModifiers::SHIFT => {
                                        match key.code {
                                            KeyCode::Char(c) => {
                                                if c == 'G' {
                                                    ui.goto_bottom();
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                    // Other Modifiers
                                    _ =>{}
                                }
                            }
                        },
                    }
                },
                ui::Region::Bar => {
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('q') {
                        return Ok(true);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('l') {
                        block_on(ui.next_5s())
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('h') {
                        block_on(ui.previous_5s())
                    }
                    if key.kind == event::KeyEventKind::Press 
                        && key.code == KeyCode::Char('j') && key.modifiers == KeyModifiers::ALT {
                            ui.select_list_region();
                    }
                    if key.kind == event::KeyEventKind::Press 
                        && key.code == KeyCode::Char('k') && key.modifiers == KeyModifiers::ALT {
                            ui.select_action_region();
                    }
                },
                ui::Region::Action => {
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('q') {
                        return Ok(true);
                    }
                    if key.kind == event::KeyEventKind::Press 
                        && key.code == KeyCode::Char('j') && key.modifiers == KeyModifiers::ALT {
                            ui.select_bar_region();
                    }
                    if key.kind == event::KeyEventKind::Press 
                        && key.code == KeyCode::Char('k') && key.modifiers == KeyModifiers::ALT {
                            ui.select_list_region();
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('l') {
                        ui.next_action();
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('h') {
                        ui.previous_action();
                    }
                    if key.kind == event::KeyEventKind::Press &&
                        key.code == KeyCode::Char(' ') || key.code == KeyCode::Enter {
                        block_on(ui.preform_action());
                    }
                },
            }
        }
    }
    return Ok(false);
}
