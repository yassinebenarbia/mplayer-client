#![allow(dead_code)]
use futures::executor::block_on;
use std::{collections::HashMap, time::Duration};

use crate::{
    EndedStream, Metadata, MusicPlayedStream, OuterMusic, PausedStream, PlayingStatus, Playlist,
    PlaylistCreatedStream, PlaylistDeletedStream, PlaylistLoadedStream, PlaylistsUpdatedStream,
    Repeat, ResumedStream, ServerProxy, Sort, SortedStream, VolumeChangedStream,
};

#[derive(Debug, PartialEq, Eq, Default, Clone)]
pub enum Status {
    Playing,
    Pausing,
    #[default]
    Stopping,
}

/// Stores non displayeable informations about the music list
pub struct MusicListState {
    selected_music: usize,
    playing_music: usize,
}

/// Batch of all possible possible values that could be gotten
/// from the bus server
#[derive(Default)]
pub struct Batch {
    /// played duration
    pub played_duration: Duration,
    /// full music duration
    pub music_duration: Duration,
    /// currently playing music
    pub playing_music: OuterMusic,
    /// Names of given playlists
    pub playlists_names: Vec<String>,
    /// status <Playing|Pausing|Stopping>
    pub status: PlayingStatus,
    /// playing volume
    pub volume: f32,
    /// currently playing music path
    music_path: String,
    /// currently playing music metadata
    pub metadata: Metadata,
    /// what to repeat <ThisMusic, AllMusics, None>
    pub repeat: Repeat,
    /// playlist order
    pub sort: Sort,
}

pub struct Streams {
    pub paused_stream: Option<PausedStream>,
    pub resumed_stream: Option<ResumedStream>,
    pub stopped_stream: Option<EndedStream>,
    pub music_played_stream: Option<MusicPlayedStream>,
    pub volume_changed_stream: Option<VolumeChangedStream>,
    pub sorted_stream: Option<SortedStream>,
    pub playlist_updated: Option<PlaylistsUpdatedStream>,
    pub playlist_loaded: Option<PlaylistLoadedStream>,
    pub playlist_deleted: Option<PlaylistDeletedStream>,
    pub playlist_created: Option<PlaylistCreatedStream>,
}

impl Default for Streams {
    fn default() -> Self {
        Self {
            paused_stream: None,
            resumed_stream: None,
            stopped_stream: None,
            music_played_stream: None,
            volume_changed_stream: None,
            sorted_stream: None,
            playlist_updated: None,
            playlist_loaded: None,
            playlist_deleted: None,
            playlist_created: None,
        }
    }
}

/// Handle state management with the bus server
pub struct State<'a> {
    /// proxy that communicates with the dbus server
    pub proxy: ServerProxy<'a>,
    /// batch of all possible derived values from the server proxy
    pub batch: Batch,
    pub dbus_streams: Streams,
}

impl<'a> State<'a> {
    pub async fn get_playing_index(&self) -> usize {
        self.proxy.get_playing_index().await.unwrap_or_default() as usize
    }

    fn handle_state(input: &str) -> Status {
        // stopping or stopped
        if input.contains("Stopped") {
            return Status::Stopping;
        // Pausing or paused
        } else if input.contains("Pausing") {
            return Status::Pausing;
        } else {
            return Status::Playing;
        }
    }

    fn handle_path(input: &str) -> String {
        let res = input.splitn(2, ":").collect::<Vec<&str>>();
        res.get(1).unwrap_or(&"").trim().to_string()
    }

    fn handle_volume(input: &str) -> f64 {
        let lines = input.splitn(2, ":").collect::<Vec<&str>>();
        let volume_s = lines.get(1).unwrap_or(&"0.5").trim();
        let volume = volume_s.parse::<f64>().unwrap();
        volume
    }

    /// gets the syncronized played duration
    fn handle_timer_call(&self) -> (Duration, Duration) {
        let timer = block_on(self.proxy.timer()).unwrap_or_default();
        let played = timer.0;
        let full = timer.1;
        (
            Duration::from_secs_f32(played),
            Duration::from_secs_f32(full),
        )
    }

    /// gets the syncronized played duration
    fn handle_playing_music_call(&self) -> OuterMusic {
        block_on(self.proxy.playing()).unwrap_or_default()
    }

    /// gets metadata of the metadata of the currently playing music
    fn handle_metadata_call(&self) -> Metadata {
        block_on(self.proxy.metadata()).unwrap_or_default()
    }

    pub fn finished_playing(&self) -> bool {
        matches!(self.status(), PlayingStatus::Stopped)
    }

    pub async fn async_finished_playing(&self) -> bool {
        matches!(self.status(), PlayingStatus::Stopped)
    }

    /// gets the syncronized played duration
    async fn async_handle_timer_call(&mut self) {
        let timer = self.proxy.timer().await.unwrap_or_default();
        let played = timer.0;
        let full = timer.1;
        (self.batch.played_duration, self.batch.music_duration) = (
            Duration::from_secs_f32(played),
            Duration::from_secs_f32(full),
        )
    }

    /// gets the syncronized played duration
    async fn async_handle_playing_music_call(&mut self) {
        self.batch.playing_music = self.proxy.playing().await.unwrap_or_default();
    }

    /// gets the metadata of the metadata of the currently playing [Music]
    async fn async_handle_metadata_call(&mut self) {
        self.batch.metadata = self.proxy.metadata().await.unwrap_or_default()
    }

    pub fn new(proxy: ServerProxy<'_>) -> State<'_> {
        State {
            proxy,
            batch: Batch::default(),
            dbus_streams: Streams::default(),
        }
    }

    /// Played duration of the currently playing music
    pub fn played_duration(&self) -> Duration {
        self.batch.played_duration
    }

    /// Playing music full duration
    pub fn playing_music_duration(&self) -> Duration {
        self.batch.music_duration
    }

    /// Currently playing [Music]
    ///
    /// used only for GUI
    pub fn playing_music(&self) -> OuterMusic {
        self.batch.playing_music.to_owned()
    }

    /// Playing status <Playing|Pausing|Stopping>
    ///
    /// used only for GUI
    pub fn status(&self) -> PlayingStatus {
        self.batch.status.to_owned()
    }

    /// Volume level between 0 and 1
    ///
    /// used only for GUI
    pub fn volume(&self) -> f32 {
        self.batch.volume
    }

    /// Path of the currently playing music
    ///
    /// used only for GUI
    pub fn music_path(&self) -> String {
        self.batch.music_path.to_owned()
    }

    /// Metadata about the currently playing music
    ///
    /// used only for GUI
    pub fn metadata(&self) -> Metadata {
        self.batch.metadata.to_owned()
    }

    pub fn playlist_names(&self) -> Vec<String> {
        self.batch.playlists_names.to_owned()
    }

    /// plays the music from the path
    pub fn play(&self) {
        block_on(self.proxy.play()).unwrap();
    }

    pub async fn play_from_index(&self, index: usize) {
        self.proxy.play_from_index(index as u32).await.unwrap();
    }

    /// Stops the music playre
    pub async fn end(&self) {
        self.proxy.end().await.unwrap();
    }

    /// Seeks by x secons from the current playing time stamp
    pub async fn seek(&self, amount: f64) {
        self.proxy.seek(amount).await.unwrap();
    }

    /// Resumes the player
    pub fn resume_stream(&self) {
        block_on(self.proxy.resume()).unwrap();
    }

    /// Pauses the player
    pub fn pause_stream(&self) {
        block_on(self.proxy.pause()).unwrap();
    }

    /// Changes playing volume, positive value increase
    /// volume, and negative decreases
    pub fn change_volume(&self, amount: f64) {
        block_on(self.proxy.volume(amount as f32)).unwrap();
    }

    /// Toggle mtue sate
    pub async fn toggle_mute(&self) {
        self.proxy.toggle_mute().await.unwrap();
    }

    /// plays the music from the path
    pub async fn async_play(&self) {
        self.proxy.play().await.unwrap();
    }

    /// Stops the music playre
    pub async fn async_end(&self) {
        self.proxy.end().await.unwrap();
    }

    /// Seeks by x secons from the current playing time stamp
    pub async fn async_seek(&self, amount: f64) {
        self.proxy.seek(amount).await.unwrap();
    }

    /// Resumes the player
    pub async fn async_resume(&self) {
        self.proxy.resume().await.unwrap();
    }

    /// Pauses the player
    pub async fn async_pause(&self) {
        self.proxy.pause().await.unwrap();
    }

    /// Changes playing volume, positive value increase
    /// volume, and negative decreases
    pub async fn async_change_volume(&self, amount: f32) {
        self.proxy.volume(amount).await.unwrap();
    }

    /// Toggle mtue sate
    pub async fn async_toggle_mute(&self) {
        self.proxy.toggle_mute().await.unwrap();
    }

    /// Fetch currently playing [Music] [Metadata] e.g. Lyrics, Cover, etc.
    pub async fn fetch_playlist_data(&mut self) {
        self.fetch_music_metadata().await;
        self.fetch_duration_sync().await;
        self.fetch_playing_music().await;
        self.fetch_playlists_names().await;
    }

    pub async fn fetch_playing_music(&mut self) {
        self.batch.playing_music = self.proxy.playing().await.unwrap_or_default();
    }

    pub async fn fetch_duration_sync(&mut self) {
        let timer = self.proxy.timer().await.unwrap_or_default();
        let played = timer.0;
        let full = timer.1;
        (self.batch.played_duration, self.batch.music_duration) = (
            Duration::from_secs_f32(played),
            Duration::from_secs_f32(full),
        );
    }

    pub async fn fetch_playlists_names(&mut self) {
        self.batch.playlists_names = self.proxy.get_playlists_names().await.unwrap_or_default();
    }

    pub async fn fetch_music_metadata(&mut self) {
        self.batch.metadata = self.proxy.metadata().await.unwrap_or_default();
    }

    pub async fn toggle_play(&self) {
        self.proxy.toggle_play().await.unwrap();
    }

    pub async fn play_next(&self) {
        self.proxy.play_next().await.unwrap();
    }

    pub async fn play_preivous(&self) {
        self.proxy.play_previous().await.unwrap();
    }

    pub fn get_repeat(&self) -> Repeat {
        block_on(async { self.proxy.get_repeat().await.unwrap_or_default() })
    }

    pub fn get_order(&self) -> crate::Sort {
        block_on(async { self.proxy.get_sort().await.unwrap_or_default() })
    }

    pub(crate) fn get_volume(&self) -> f32 {
        block_on(async { self.proxy.get_volume().await.unwrap_or_default() })
    }

    pub fn get_playlists(&self) -> HashMap<String, Playlist> {
        block_on(async { self.proxy.get_playlists().await.unwrap_or_default() })
    }

    /// Gets playlist names directly from the dbus proxy
    pub fn get_playlists_names(&self) -> Vec<String> {
        block_on(async { self.proxy.get_playlists_names().await.unwrap_or_default() })
    }

    pub(crate) async fn increase_volume(&self) {
        self.proxy.volume(self.volume() + 0.1).await.unwrap();
    }

    pub(crate) async fn decrease_volume(&self) {
        self.proxy.volume(self.volume() as f32 - 0.1).await.unwrap();
    }

    pub(crate) async fn sort(&self, sort: crate::Sort) {
        self.proxy.sort(sort).await.unwrap();
    }

    pub(crate) async fn repat(&self, repeat: Repeat) {
        self.proxy.repeat(repeat).await.unwrap();
    }
}
