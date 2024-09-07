#![allow(dead_code)]
use std::{path::PathBuf, time::Duration};

use async_std::task::block_on;

use crate::{
    ui::Music, Metadata, ServerProxy
};

#[derive(Debug, PartialEq, Eq, Default, Clone)]
pub enum Status {
    Playing,
    Pausing,
    #[default]
    Stopping
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
    played_duration: Duration,
    /// full music duration
    music_duration: Duration,
    /// currently playing music
    playing_music: Music,
    /// status <Playing|Pausing|Stopping>
    status: Status,
    /// playing volume
    volume: f64,
    /// currently playing music path
    music_path: String,
    /// currently playing music metadata
    metadata: Metadata,
}

/// Handle state management with the bus server
pub struct State<'a> {
    /// proxy that communicates with the dbus server
    pub proxy: ServerProxy<'a>,
    /// batch of all possible derived values from the server proxy
    pub batch: Batch,
}

impl<'a> State<'a> {
    pub fn get_playing_index(&self, musics: &Vec<Music>) -> usize{
        let target = block_on(self.proxy.playing());
        match target {
            Ok(target) => {
                for (index, music) in musics.iter().enumerate() {
                    if music == &target {
                        return index
                    }
                }
                return 0
            },
            Err(_) => {
                return 0
            },
        }
    }

    fn handle_state(input: &str) -> Status {
        // stopping or stopped
        if input.contains("Stopped") {
            return Status::Stopping
        // Pausing or paused
        } else if input.contains("Pausing") {
            return Status::Pausing
        }else {
            return Status::Playing
        }
    }

    fn handle_path(input: &str) -> String {
        let res = input.splitn(2,":").collect::<Vec<&str>>();
        res.get(1).unwrap_or(&"").trim().to_string()
    }

    fn handle_volume(input: &str) -> f64{
        let lines = input.splitn(2, ":").collect::<Vec<&str>>();
        let volume_s = lines.get(1).unwrap_or(&"0.5").trim();
        let volume = volume_s.parse::<f64>().unwrap();
        volume
    }

    /// Handes the bus `status` call
    /// Reutrn tuple of (status, path, duration) where:
    /// - status: status of the server player
    /// - path: path to the currently playing song 
    /// - duration: played duration
    fn hande_status_call(&self) -> (Status, String, f64) {
        // call to be handled
        let status = block_on(self.proxy.status())
            .unwrap();
        let mut l = status.split_terminator("\n");
        let state = l.next().unwrap_or("Stopped");
        let path = l.next().unwrap_or("");
        let volume = l.next().unwrap_or("0.5");
        (State::handle_state(state), State::handle_path(path), State::handle_volume(volume))
    }

    /// gets the syncronized played duration
    fn handle_timer_call(&self) -> (Duration, Duration) {
        let timer = block_on(self.proxy.timer()).unwrap_or(String::from("0.0/0.0"));
        let s = timer.splitn(2,'/').collect::<Vec<&str>>();
        let len = s.get(0).unwrap().parse::<f64>().unwrap();
        let secs = s.get(1).unwrap().parse::<f64>().unwrap();
        (Duration::from_secs_f64(secs), Duration::from_secs_f64(len))
    }

    /// gets the syncronized played duration
    fn handle_playing_music_call(&self) -> Music {
        block_on(self.proxy.playing()).unwrap_or_default()
    }

    /// gets metadata of the metadata of the currently playing music
    fn handle_metadata_call(&self) -> Metadata {
        block_on(self.proxy.metadata()).unwrap_or_default()
    }

    pub fn finished_playing(&self) -> bool {
        let timer = block_on(self.proxy.timer()).unwrap_or(String::from("0.0/0.0"));
        let s = timer.splitn(2,'/').collect::<Vec<&str>>();
        let len = s.get(0).unwrap().parse::<f64>().unwrap();
        let played = s.get(1).unwrap().parse::<f64>().unwrap();
        if len != 0.0 && played == 0.0 {return false} else {return true}
    }


    pub fn batch_calls(&mut self) {
        (self.batch.played_duration, self.batch.music_duration) = self.handle_timer_call();
        self.batch.playing_music = self.handle_playing_music_call();
        (self.batch.status, self.batch.music_path , self.batch.volume) = self.hande_status_call();
        self.batch.metadata = self.handle_metadata_call();
    }

    /// Handes the bus `status` call
    /// Reutrn tuple of (status, path, duration) where:
    /// - status: status of the server player
    /// - path: path to the currently playing song 
    /// - duration: played duration
    async fn async_handle_status_call(&mut self) {
        let status = self.proxy.status().await.unwrap_or_default();
        let mut l = status.split_terminator("\n");
        let state = l.next().unwrap_or("Stopp");
        let path = l.next().unwrap_or("");
        let volume = l.next().unwrap_or("0.5");
        (self.batch.status, self.batch.music_path , self.batch.volume) =
            (State::handle_state(state), State::handle_path(path), State::handle_volume(volume))
    }

    pub async fn async_finished_playing(&self) -> bool {
        let timer = self.proxy.timer().await.unwrap_or(String::from("0.0/0.0"));
        let s = timer.splitn(2,'/').collect::<Vec<&str>>();
        let len = s.get(0).unwrap().parse::<f64>().unwrap();
        let played = s.get(1).unwrap().parse::<f64>().unwrap();
        if len != 0.0 && played == 0.0 {return false} else {return true}
    }

    /// gets the syncronized played duration
    async fn async_handle_timer_call(&mut self) {
        let timer = self.proxy.timer().await.unwrap_or(String::from("0.0/0.0"));
        let s = timer.splitn(2,'/').collect::<Vec<&str>>();
        let len = s.get(0).unwrap().parse::<f64>().unwrap();
        let played = s.get(1).unwrap().parse::<f64>().unwrap();
        (self.batch.played_duration, self.batch.music_duration) = (Duration::from_secs_f64(played), Duration::from_secs_f64(len))
    }

    /// gets the syncronized played duration
    async fn  async_handle_playing_music_call(&mut self) {
        self.batch.playing_music = self.proxy.playing().await.unwrap_or_default();
    }

    /// gets metadata of the metadata of the currently playing music
    async fn async_handle_metadata_call(&mut self) {
        self.batch.metadata = self.proxy.metadata().await.unwrap_or_default()
    }

    /// Dervies what ever is deriveable from a dbus call to the server
    /// and stores everything in the [State] object
    pub async fn async_batch_calls(&mut self) {
        // self.async_handle_metadata_call();
        self.async_handle_timer_call().await;
        self.async_handle_playing_music_call().await;
        self.async_handle_status_call().await;
    }

    pub fn new(proxy: ServerProxy<'a>) -> State{
        State {
            proxy,
            batch:Batch::default(),
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
    pub fn playing_music(&self) -> Music {
        self.batch.playing_music.to_owned()
    }

    /// Playing status <Playing|Pausing|Stopping>
    pub fn status(&self) -> Status {
        self.batch.status.to_owned()
    }

    /// Volume level between 0 and 1
    pub fn volume(&self) -> f64{
        self.batch.volume
    }

    /// Path of the currently playing music
    pub fn music_path(&self) -> String {
        self.batch.music_path.to_owned()
    }

    /// Metadata about the currently playing music
    pub fn metadata(&self) -> Metadata {
        self.batch.metadata.to_owned()
    }

    /// plays the music from the path
    pub fn play(&self, path: &PathBuf) {
        block_on(self.proxy.play(path)).unwrap();
    }

    /// Stops the music playre
    pub fn end(&self) {
        block_on(self.proxy.end()).unwrap();
    }

    /// Seeks by x secons from the current playing time stamp
    pub fn seek(&self, amount: f64) {
        block_on(self.proxy.seek(amount)).unwrap();
    }

    /// Resumes the player 
    pub fn resume(&self) {
        block_on(self.proxy.resume()).unwrap();
    }

    /// Pauses the player
    pub fn pause(&self) {
        block_on(self.proxy.pause()).unwrap();
    }

    /// Changes playing volume, positive value increase 
    /// volume, and negative decreases
    pub fn change_volume(&self, amount: f64) {
        block_on(self.proxy.volume(amount)).unwrap();
    }
    
    /// Toggle mtue sate
    pub fn toggle_mute(&self) {
        block_on(self.proxy.toggle_mute()).unwrap();
    }

    /// plays the music from the path
    pub async fn async_play(&self, path: &PathBuf) {
        self.proxy.play(path).await.unwrap();
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
    pub async fn async_change_volume(&self, amount: f64) {
        self.proxy.volume(amount).await.unwrap();
    }

    /// Toggle mtue sate
    pub async fn async_toggle_mute(&self) {
        self.proxy.toggle_mute().await.unwrap();
    }
}
