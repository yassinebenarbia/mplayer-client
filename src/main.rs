use std::{env, io::{self, stdout}, path::PathBuf};

use utils::RunStatus;
use zbus::{proxy, Connection, Result};
mod states;
mod utils;
mod ui;
mod parser;
mod fuzzy_search;
use crossterm::{
    event::{self, Event},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{self, Terminal, backend::CrosstermBackend};

use ui::Music;
use parser::*;
#[allow(unused_imports)]
use utils::log;

#[derive(serde::Deserialize, serde::Serialize, zbus::zvariant::Type, Debug, Default, Clone)]
struct Picture{
    data: Vec<u8>,
    typ: String
}

#[derive(serde::Deserialize, serde::Serialize, zbus::zvariant::Type, Debug, Default, Clone)]
pub struct Metadata {
    title: String, 
    artis: String,
    genre: String,
    cover: Picture,
}

#[proxy(
    interface = "org.zbus.mplayerServer",
    default_service = "org.zbus.mplayer",
    default_path = "/org/zbus/mplayer"
)]
pub trait Server {
    /// Returns the player status:
    /// - status: <Playing|Pausing|Paused>
    fn status(&self) -> Result<String>;
    /// Plays the music from the file path, returns true if no panic happened
    fn play(&self, path: &PathBuf) -> Result<RunStatus>;
    /// Terminate playing, returns true if no panic happened
    fn end(&self) -> Result<RunStatus>;
    /// Resumes playing the currently paused song, returns true if no panic happened
    fn resume(&self) -> Result<RunStatus>;
    /// Pauses playing the currently playing song, returns true if no panic happened
    fn pause(&self) -> Result<RunStatus>;
    /// played duration over the the total duration of the music
    /// format: full length / played duration
    fn timer(&self) -> Result<String>;
    /// seeks the player by the given duration relative to the current playing timer
    /// negative number meens seking backward and vice versa
    /// 
    /// - if state is playing:
    ///     - seeks by the give nduration
    /// - if state is Stopping it:
    ///     - plays the preivously played song
    ///     - seeks by the given duration
    /// - if state is pausing it:
    ///     - resumes the currently playing song 
    ///     - seeks by the given duration
    fn seek(&self, duration: f64) -> Result<RunStatus>;
    /// changes the volume of the player, return true if no panic happened
    fn volume(&self, amount:f64) -> Result<RunStatus>;
    fn metadata(&self) -> Result<Metadata>;
    /// Gets the currently playing [Music]
    fn playing(&self) -> Result<Music>;
    fn toggle_mute(&self) -> Result<RunStatus>;
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
        panic!("Mplayer server is not Up, aborting..");
    });

    init_panic_hook();
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let args: Vec<String> = env::args().collect();
    let wrapper = if args.len() > 1 {
        Config::parse_config(&args[1])
    }else {
        let mut home = env::var("HOME").unwrap().to_string();
        home.push_str("/.config/mplayer-client/config.toml");
        Config::parse_config(&home)
    };

    let mut ui = ui::UI::default(proxy);
    let config = wrapper.config.unwrap_or_default();

    let mut musics = config.extract_music();
    musics.sort(config.sorting);

    ui.update_from_config(&config);
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

fn handle_events<'a>(ui: &mut ui::UI<'a>) -> io::Result<bool>{
    if event::poll(std::time::Duration::from_millis(50))? {
        if let Event::Key(key) = event::read()? {
            ui::Region::handle_global(ui, &key);
            // keybind depend on reagion
            match ui.region {
                // list region
                ui::Region::List =>{
                    if ui::Region::handle_list(ui, &key).is_ok_and(|x| x == true) {
                        return Ok(true)
                    }
                }
                // seeker region
                ui::Region::Seeker => {
                    if ui::Region::handle_seeker(ui, &key).is_ok_and(|x| x == true) {
                        return Ok(true)
                    }
                }
                // volume region
                ui::Region::Volume => {
                    if ui::Region::handle_volume(ui, &key).is_ok_and(|x| x == true) {
                        return Ok(true)
                    }
                }
                // actions region
                ui::Region::Action => {
                    if ui::Region::handle_action(ui, &key).is_ok_and(|x| x == true) {
                        return Ok(true)
                    }
                }
            }
        }
    }
    return Ok(false);
}
