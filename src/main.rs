use std::{
    env,
    io::{self, stdout},
    time::Duration,
};

use crossterm::{
    event::{self, Event},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use parser::*;
use ratatui::{backend::CrosstermBackend, text::Span, Terminal};
use ui::Music;
use utils::RunStatus;
use zbus::{proxy, zvariant::FilePath, Connection, Result};

mod fuzzy_search;
mod parser;
mod states;
mod ui;
mod utils;

#[allow(unused_imports)]
use utils::log;

#[derive(serde::Deserialize, serde::Serialize, zbus::zvariant::Type, Debug, Default, Clone)]
struct Picture {
    data: Vec<u8>,
    typ: String,
}

#[derive(
    PartialEq, Eq, Debug, Clone, zbus::zvariant::Type, serde::Serialize, serde::Deserialize,
)]
pub struct Line {
    content: String,
    timestamp: Duration,
}

#[derive(
    PartialEq, Eq, Debug, Clone, zbus::zvariant::Type, serde::Deserialize, serde::Serialize,
)]
pub struct Lyrics {
    time_is_correct: bool,
    lines: Vec<Line>,
}

impl Default for Line {
    fn default() -> Self {
        Line {
            content: String::new(),
            timestamp: Duration::ZERO,
        }
    }
}

impl Default for Lyrics {
    fn default() -> Self {
        Self {
            time_is_correct: false,
            lines: vec![],
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, zbus::zvariant::Type, Debug, Default, Clone)]
pub struct Metadata {
    title: String,
    artis: String,
    genre: String,
    cover: Picture,
    lyrics: Lyrics,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, Default, zbus::zvariant::Type)]
pub enum PlayingStatus {
    /// Pausing state
    Playing,
    /// Playing state
    Pausing,
    #[default]
    /// Stopping state
    Stopped,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, zbus::zvariant::Type)]
pub struct PlayerStatus {
    status: PlayingStatus,
    music: OuterMusic,
    /// between 0 and 1
    volume: f32,
    index: u32,
}

#[derive(
    serde::Serialize,
    serde::Deserialize,
    Debug,
    Clone,
    Copy,
    Default,
    zbus::zvariant::Type,
    Eq,
    PartialEq,
)]
pub enum Sort {
    #[default]
    /// Sort  by music title in ascending order
    ByTitleAscending,
    /// Sort by music title in descending order
    ByTitleDescending,
    /// Sort by music length in ascending order
    ByDurationAscending,
    /// Sort by music length in descending order
    ByDurationDescending,
    /// Sort by music artist name in ascending order
    ArtistAscending,
    /// Sort by music artist name in descending order
    ArtistDescending,
    // Sort at random
    Shuffle,
}

impl Sort {
    fn as_span(&self) -> Span {
        match self {
            Sort::ByTitleAscending => Span::from("TitleAscending"),
            Sort::ByTitleDescending => Span::from("TitleDescending"),
            Sort::ByDurationAscending => Span::from("DurationAscending"),
            Sort::ByDurationDescending => Span::from("DurationDescending"),
            Sort::ArtistAscending => Span::from("ArtistAscending"),
            Sort::ArtistDescending => Span::from("ArtistDescending"),
            Sort::Shuffle => Span::from("Shuffle"),
        }
    }
}

#[derive(
    serde::Serialize,
    serde::Deserialize,
    Debug,
    Clone,
    Copy,
    Default,
    zbus::zvariant::Type,
    Eq,
    PartialEq,
)]
pub enum Repeat {
    /// repeat the currently playing music
    SameMusic,
    #[default]
    /// cycle through all the playlist
    AllMusics,
    /// stop after the currently playing music
    Dont,
}

impl Repeat {
    fn as_span(&self) -> Span {
        match self {
            Repeat::SameMusic => Span::from("RepeatMusic"),
            Repeat::AllMusics => Span::from("RepeatList"),
            Repeat::Dont => Span::from("NoRepeat"),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Default, zbus::zvariant::Type)]
pub struct Playlist {
    musics: Vec<Music>,
    sort: Sort,
    repeat: Repeat,
    playing_index: u32,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Default, zbus::zvariant::Type)]
pub struct OuterPlaylist {
    pub musics: Vec<OuterMusic>,
    pub sort: Sort,
    pub repeat: Repeat,
    pub playing_index: u32,
}

#[derive(
    PartialEq, Eq, Debug, Clone, serde::Serialize, serde::Deserialize, zbus::zvariant::Type, Default,
)]
pub struct OuterMusic {
    pub title: String,
    pub length: Duration,
    pub path: FilePath<'static>,
    pub artist: String,
    pub genre: String,
}

// NOTE: the return status of most functions is not currently documented, tho
// usually they represent the success of the opperaiton/call.
#[proxy(
    interface = "org.zbus.mplayerServer",
    default_service = "org.zbus.mplayer",
    default_path = "/org/zbus/mplayer"
)]
pub trait Server {
    /// Toggle playing status
    fn toggle_play(&self) -> Result<RunStatus>;
    /// Gets the player playing status
    fn get_playing_status(&self) -> Result<PlayingStatus>;
    /// Gets the sorting status of the current playlist
    fn get_sort(&self) -> Result<Sort>;
    /// Gets next music on the playlist music list
    fn get_next_music(&self) -> Result<OuterMusic>;
    /// Gets previous music on the playlist music list
    fn get_previous_music(&self) -> Result<OuterMusic>;
    /// Gets the index of the currently playing music
    fn get_playing_index(&self) -> Result<u32>;
    /// Gets the played duration for the currently playing music
    fn played_duration(&self) -> Result<f32>;
    /// Plays previous music on the playlist music list
    fn play_previous(&self) -> Result<RunStatus>;
    /// Plays next music on the playlist music list
    fn play_next(&self) -> Result<RunStatus>;
    /// Plays a music from its indoex on the playlist
    fn play_from_index(&self, index: u32) -> Result<RunStatus>;
    /// Sorts playlist
    fn sort(&self, sort: Sort) -> Result<RunStatus>;
    /// Changes the repeat state of plalyist
    fn repeat(&self, repeat: Repeat) -> Result<RunStatus>;
    /// Returns current playlist
    fn playlist(&self) -> Result<OuterPlaylist>;
    /// Returns the lyrics of the current music
    fn lyrics(&self) -> Result<Lyrics>;
    /// Returns metadata of the current music
    fn metadata(&self) -> Result<Metadata>;
    /// Play the currently paused music
    fn play(&self) -> Result<RunStatus>;
    /// Gets the repeat status of the current playlist
    fn get_repeat(&self) -> Result<Repeat>;
    /// Returns the player status:
    /// - status: <Playing|Pausing|Paused>
    fn player_status(&self) -> Result<PlayerStatus>;
    /// Plays the music from the file path, returns true if no panic happened
    /// Terminate playing, returns true if no panic happened
    fn end(&self) -> Result<RunStatus>;
    /// Resumes playing the currently paused song, returns true if no panic happened
    fn resume(&self) -> Result<RunStatus>;
    /// Pauses playing the currently playing song, returns true if no panic happened
    fn pause(&self) -> Result<RunStatus>;
    /// Played duration over the the total duration of the music
    /// format: full length / played duration
    fn timer(&self) -> Result<(f32, f32)>;
    /// Seeks the player by the given duration relative to the current playing timer
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
    /// changes the volume of the player (valueb between 0 and 1)
    fn volume(&self, amount: f32) -> Result<RunStatus>;
    /// Gets player volume (between 0 and 1)
    fn get_volume(&self) -> Result<f32>;
    /// Gets the currently playing [Music]
    async fn playing(&self) -> Result<OuterMusic>;
    fn toggle_mute(&self) -> Result<RunStatus>;
    /// Signal fires when volume changes
    #[zbus(signal)]
    async fn volume_changed(&self, amount: f32) -> zbus::Result<()>;
    /// Signal fires when player is in pausing state
    #[zbus(signal)]
    async fn paused(&self) -> zbus::Result<()>;
    /// Signal fires when playing resumed from pausing
    #[zbus(signal)]
    async fn resumed(&self) -> zbus::Result<()>;
    /// Signal fires when the player at the stop state
    #[zbus(signal)]
    async fn ended(&self) -> zbus::Result<()>;
    /// Signal fires when a music starts playing
    #[zbus(signal)]
    async fn music_played(&self) -> zbus::Result<()>;
    /// Signal fires when the sorting status changes
    #[zbus(signal)]
    async fn sorted(&self, sort: Sort) -> zbus::Result<()>;
    /// Signal fires when the repeat status changes
    #[zbus(signal)]
    async fn repeat_changed(&self, sort: Repeat) -> zbus::Result<()>;
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

#[tokio::main]
async fn main() -> Result<()> {
    let connection = Connection::session().await.unwrap_or_else(|_| {
        panic!("Could not connect to the bus address, aborting...");
    });
    let proxy = ServerProxy::new(&connection).await?;
    proxy.get_playing_index().await.unwrap_or_else(|_| {
        panic!("Mplayer server is not Up, aborting..");
    });

    init_panic_hook();
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let args: Vec<String> = env::args().collect();
    let config = if args.len() > 1 {
        Config::parse_config(&args[1]).unwrap()
    } else {
        let mut home = env::var("HOME").unwrap().to_string();
        home.push_str("/.config/mplayer-client/config.toml");
        if let Ok(config) = Config::parse_config(&home) {
            config
        } else {
            Config::default()
        }
    };

    let mut ui = ui::UI::new(proxy);

    ui.update_from_config(&config);
    ui.restore_state().await;
    ui.fetch_playlist_data().await;
    ui.set_frame(&mut terminal.get_frame());
    let mut should_quit = false;
    while !should_quit {
        if ui.fps_controller.check_fps() {
            terminal.draw(|frame| {
                ui.render(frame);
            })?;
            ui.update_state().await;
            should_quit = handle_events(&mut ui).await?;
        }
    }

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

async fn handle_events<'a>(ui: &mut ui::UI<'a>) -> io::Result<bool> {
    if event::poll(std::time::Duration::from_millis(50))? {
        if let Event::Key(key) = event::read()? {
            return ui::Region::handle_events(ui, &key).await;
        }
    }
    return Ok(false);
}
