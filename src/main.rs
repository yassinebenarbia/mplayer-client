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

// NOTE(s)
// - the fist lin of method/function documentation should be uppercase
// TODO(s)
// - make closing and opening the client does not affect the index of the currently playing song
// - migrate from audiotags to lofty
// - add keybind to pause playing
// - add keybind to pause
// - add g<n> vim keybinds
// - add ctr+d and ctr+u vim keybinds
// - musics should play one after the other automatically, unless stated otherwise
// - searching should be extanded with to genre, etc.
// - It would be nice to highlight the matched chars
// - sort the full music list like the displyed one (if needed)
// - add row number to the music list
// - change highlight color to display that we are anticipation another character after the first g
// - add numbering to for the music list
// - add j,k and a number to go x number down or up
// - add lyrics display between the list and actions
// - add volume slider besides the actions bar
// - make commands for search, for example duration: or genre:
// - add search using commmands like :genre or :artist
// - add song to a .m3u playlist
// - navigate throught playlists from the config file
// - register only the necessery dbus calls, and make them happen in the update_state method
// -- this can go by adding an intermidary method that register calls and remove redendant
//      and repetitive calls
// - make every call that needs to access the dbus on the update_state method level
// and then access those elements with a shared object method
// - add repeat song, repeat list, don't repeate features
// - Shift+Enter to cycle back on repeat and shuffle actions

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
    fn volume(&self, amount:f64) -> Result<bool>;
    fn metadata(&self) -> Result<Metadata>;
    fn playing(&self) -> Result<Music>;
}

pub fn init_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        // intentionally ignore errors here since we're already in a panic
        disable_raw_mode().unwrap();
        stdout().execute(LeaveAlternateScreen).unwrap();
        original_hook(panic_info);
    }));
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

    init_panic_hook();
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
    ui.restore_state();

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
                                        block_on(ui.play_selected_music());
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
                                                    block_on(ui.play_selected_music());
                                                    ui.reset_querry();
                                                    ui.mode = ListMode::Select;
                                                }
                                            },
                                            KeyCode::Enter => {
                                                block_on(ui.play_selected_music());
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
                                                        block_on(ui.play_selected_music());
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
                                                        }else if c == ' ' {
                                                            block_on(ui.play_selected_music());
                                                        }else if c == 's' {
                                                            ui.goto_playing();
                                                        }else if c == 'g' {
                                                            ui.anticipation_mode = ui::AncitipationMode::Char('g'); 
                                                            // in case of uppercase 
                                                        } else if c == 'G' {
                                                            ui.goto_bottom();
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
                                                            }                                                        
                                                        },
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
                                    _ =>{}
                                }
                            }
                        },
                    }
                },
                ui::Region::Seeker => {
                    if key.kind == event::KeyEventKind::Press {
                        match key.modifiers {
                            KeyModifiers::NONE => {
                                match key.code {
                                    KeyCode::Char(c) => {
                                        if c == 'q' {
                                            return Ok(true);
                                        }else if c == 'l'{
                                            block_on(ui.next_5s())
                                        }else if c == 'h' {
                                            block_on(ui.previous_5s())
                                        }else if c == 'k' {
                                            block_on(ui.toggle_play());
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            KeyModifiers::ALT => {
                                match key.code {
                                    KeyCode::Char(c) => {
                                        if c == 'j' {
                                            ui.select_list_region();
                                        } else if c == 'k' {
                                            ui.select_action_region();
                                        }else if c == 'h' || c == 'l' {
                                            ui.select_volume_region();
                                        }
                                    }
                                    _ => {}
                                }

                            }
                            _ => {}
                        }
                    }
                },
                ui::Region::Volume => {
                    if key.kind == event::KeyEventKind::Press {
                        match key.modifiers {
                            KeyModifiers::NONE => {
                                match key.code {
                                    KeyCode::Char(c) => {
                                        if c == 'q' {
                                            return Ok(true);
                                        }else if c == 'k' || c == 'l' {
                                            ui.increase_volume();
                                        }else if c == 'h' || c == 'j'{
                                            ui.decrease_volume();
                                        }
                                    }
                                    _ => {}
                                }

                            },
                            KeyModifiers::ALT => {
                                match key.code {
                                    KeyCode::Char(c) => {
                                        if c == 'j' {
                                            ui.select_list_region();
                                        }else if c == 'k' {
                                            ui.select_action_region();
                                        }else if c == 'l' || c == 'h'{
                                            ui.select_bar_region();
                                        }
                                    }
                                    _ => {}
                                }
                            },
                            // TODO make the increse add by 5
                            KeyModifiers::SHIFT=> {
                                match key.code {
                                    KeyCode::Char(c) => {
                                        if c == 'j' || c == 'h' {
                                            ui.increase_volume();
                                        }else if c == 'k' || c == 'l'{
                                            ui.decrease_volume();
                                        }
                                    }
                                    _ => {}
                                }

                            }
                            _ => {}
                        }
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
