use std::time::Duration;

use ratatui::text::Span;
use zbus::zvariant::FilePath;

#[derive(serde::Deserialize, serde::Serialize, zbus::zvariant::Type, Debug, Default, Clone)]
pub struct Picture {
    pub data: Vec<u8>,
    pub typ: String,
}

#[derive(
    PartialEq, Eq, Debug, Clone, zbus::zvariant::Type, serde::Deserialize, serde::Serialize,
)]
pub struct Lyrics {
    pub time_is_correct: bool,
    pub lines: Vec<Line>,
}

impl Default for Lyrics {
    fn default() -> Self {
        Self {
            time_is_correct: false,
            lines: vec![],
        }
    }
}

#[derive(
    PartialEq, Eq, Debug, Clone, zbus::zvariant::Type, serde::Serialize, serde::Deserialize,
)]
pub struct Line {
    pub content: String,
    pub timestamp: Duration,
}

impl Default for Line {
    fn default() -> Self {
        Line {
            content: String::new(),
            timestamp: Duration::ZERO,
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, zbus::zvariant::Type, Debug, Default, Clone)]
pub struct Metadata {
    pub title: String,
    pub artis: String,
    pub genre: String,
    pub cover: Picture,
    pub lyrics: Lyrics,
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
    pub status: PlayingStatus,
    pub music: OuterMusic,
    /// between 0 and 1
    pub volume: f32,
    pub index: u32,
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
    pub fn as_span(&self) -> Span<'_> {
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
    pub fn as_span(&self) -> Span<'_> {
        match self {
            Repeat::SameMusic => Span::from("RepeatMusic"),
            Repeat::AllMusics => Span::from("RepeatList"),
            Repeat::Dont => Span::from("NoRepeat"),
        }
    }
}

#[derive(
    PartialEq,
    Eq,
    Debug,
    Ord,
    PartialOrd,
    Clone,
    serde::Serialize,
    serde::Deserialize,
    zbus::zvariant::Type,
)]
pub struct Music {
    pub title: String,
    pub length: Duration,
    pub path: FilePath<'static>,
    pub artist: String,
    pub genre: String,
    pub index: usize,
}

impl Default for Music {
    fn default() -> Self {
        Self {
            title: String::new(),
            length: Duration::ZERO,
            path: FilePath::default(),
            artist: String::new(),
            genre: String::new(),
            index: 0,
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
    pub name: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, zbus::zvariant::Type)]
pub enum PlaylistOpperation {
    AddedMusic,
    DeletedMusic,
}
