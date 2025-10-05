// TODO: rename actions to pannel
// TODO: add actions box for searching and commands e.g:
// TODO: make a keybind to create a playlist
// TODO: make a keybind to add a music to a playlist
// TODO: make search golobal command, when canceling, you go back to original region when search is
// done (this also allows for the after search effect, you temporary get transported to the list
// region)
// TODO: implement :delete_from_this_playlist and :delete_from_playlist [playlist_name] [name or index of music]
// TODO: implement :add_from_this_playlist [destination] and :add_from_playlist [source] [name or
// index] [destination]
use crate::parser::Scripts;
use crate::style::UIStyle;
use crate::types::Music;
use crate::{Repeat, Sort};
use crossterm::event::Event as CrosstermEvent;
use crossterm::event::{self, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use futures::StreamExt;
use mlua::prelude::*;
use mlua::Lua;
use mlua::{UserData, UserDataMethods};
use ratatui::{prelude::*, style::Stylize, widgets::*};
use serde::{Deserialize, Serialize};
use std::io;
use std::str::FromStr;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};
use tokio::time::Interval;
use tui_input::{Input, InputRequest, StateChanged};

use crate::allowed_commands::{CommandContext, CommandName};
use crate::states::State;
use crate::utils::{log, FPSController};
use crate::{fuzzy_search, Config, Lyrics, PlayingStatus, ServerProxy};

pub struct ViewConfiguration {
    lyrics: bool,
    search: bool,
    command: bool,
    playlists: bool,
}

pub struct ViewGeometry {
    frame: Rect,
    list: Rect,
    lyrics: Option<Rect>,
    playlists: Option<Rect>,
    actions: Rect,
    volume: Rect,
    slider: Rect,
    search: Option<Rect>,
    command: Option<Rect>,
    configuration: ViewConfiguration,
}

impl ViewGeometry {
    pub fn new(
        frame: Rect,
        list: Rect,
        lyrics: Option<Rect>,
        playlists: Option<Rect>,
        search: Option<Rect>,
        command: Option<Rect>,
        actions: Rect,
        slider: Rect,
        volume: Rect,
        configuration: ViewConfiguration,
    ) -> Self {
        Self {
            frame,
            list,
            lyrics,
            playlists,
            actions,
            volume,
            slider,
            search,
            command,
            configuration,
        }
    }

    fn default(frame: Rect) -> Self {
        let mut init = Self::new(
            frame,
            frame,
            None,
            None,
            None,
            None,
            frame,
            frame,
            frame,
            ViewConfiguration {
                search: false,
                lyrics: false,
                playlists: false,
                command: false,
            },
        );
        init.calculate_gemoetry();
        init
    }

    fn calculate_list(&mut self) {
        let mut init = self.frame;
        init.height = init.height.saturating_sub(7);
        if self.configuration.search || self.configuration.command {
            init.height = init.height.saturating_sub(3);
        }
        if self.configuration.lyrics || self.configuration.playlists {
            init.height /= 2;
            init.height = init.height.saturating_sub(1);
        }
        self.list = init
    }

    fn calculate_lyrics(&mut self) {
        if self.configuration.lyrics {
            let mut init = self.frame;
            init.height = init.height.saturating_sub(4) / 2;
            init.y = init.height.saturating_sub(2);
            if self.configuration.search {
                init.height = init.height.saturating_sub(1);
                init.y = init.y + 1;
            }
            self.lyrics = Some(init);
        } else {
            self.lyrics = None;
        }
    }

    fn calculate_playlists(&mut self) {
        if self.configuration.playlists {
            let mut init = self.frame;
            init.height = init.height.saturating_sub(4) / 2;
            init.y = init.height.saturating_sub(2);
            if self.configuration.search {
                init.height = init.height.saturating_sub(1);
                init.y = init.y + 1;
            }
            self.playlists = Some(init);
        } else {
            self.playlists = None;
        }
    }

    fn calculate_search(&mut self) {
        if self.configuration.search {
            let mut init = self.frame;
            init.y = init.height.saturating_sub(10);
            if self.configuration.lyrics {
                init.y = init.y / 2;
                init.y = init.y.saturating_sub(1);
            }
            init.height = 3;
            self.search = Some(init);
        } else {
            self.search = None;
        }
    }

    fn calculate_command(&mut self) {
        if self.configuration.command {
            let mut init = self.frame;
            init.y = init.height.saturating_sub(10);
            if self.configuration.lyrics {
                init.y = init.y / 2;
                init.y = init.y.saturating_sub(1);
            }
            init.height = 3;
            self.command = Some(init);
        } else {
            self.command = None;
        }
    }

    fn calculate_actions(&mut self) {
        let mut area = self.frame;
        if area.height > 7 {
            area.y = area.height.saturating_sub(3).saturating_sub(4);
            area.height = 3;
        }
        self.actions = area;
    }

    fn calculate_slider(&mut self) {
        let mut area = self.frame;
        area.y = area.height.saturating_sub(4);
        area.height = 3;
        area.width = (area.width / 5) * 4 as u16;
        self.slider = area;
    }

    fn calculate_volume(&mut self) {
        let mut area = self.frame;
        area.y = area.height.saturating_sub(4);
        area.x = (area.width / 5) * 4 as u16;
        area.width = area.width - area.x as u16;
        area.height = 3;
        self.volume = area;
    }

    pub fn calculate_gemoetry(&mut self) {
        self.calculate_list();
        self.calculate_lyrics();
        self.calculate_playlists();
        self.calculate_search();
        self.calculate_command();
        self.calculate_actions();
        self.calculate_slider();
        self.calculate_volume();
    }

    pub fn set_frame(&mut self, frame: Rect) {
        self.frame = frame;
    }

    pub fn list_geo(&self) -> Rect {
        self.list
    }

    pub fn lyrics_geo(&self) -> Option<Rect> {
        self.lyrics
    }

    pub fn playlists_geo(&self) -> Option<Rect> {
        self.playlists
    }

    pub fn actions_geo(&self) -> Rect {
        self.actions
    }

    pub fn volume_geo(&self) -> Rect {
        self.volume
    }

    pub fn slider_geo(&self) -> Rect {
        self.slider
    }

    pub fn search_geo(&self) -> Option<Rect> {
        self.search
    }

    pub fn command_geo(&self) -> Option<Rect> {
        self.command
    }

    pub fn enable_search(&mut self) {
        self.configuration.search = true;
    }

    pub fn enable_command(&mut self) {
        self.configuration.command = true;
    }

    pub fn disable_search(&mut self) {
        self.configuration.search = false;
    }

    pub fn disable_command(&mut self) {
        self.configuration.command = false;
    }

    #[allow(unused)]
    pub fn disable_toggled(&mut self) {
        self.configuration.lyrics = false;
        self.configuration.playlists = false;
    }

    pub fn toggle_lyrics(&mut self) {
        self.configuration.lyrics = !self.configuration.lyrics;
    }

    pub fn disable_lyrics(&mut self) {
        self.configuration.lyrics = false;
    }

    #[allow(unused)]
    pub fn enable_lyrics(&mut self) {
        self.configuration.lyrics = true;
    }

    pub fn toggle_playlists(&mut self) {
        self.configuration.playlists = !self.configuration.playlists;
    }

    pub fn disable_playlists(&mut self) {
        self.configuration.playlists = false;
    }

    #[allow(unused)]
    pub fn enable_playlists(&mut self) {
        self.configuration.playlists = true;
    }
}

#[derive(Default, Debug, Deserialize, Serialize)]
pub enum ListMode {
    Command,
    Search,
    #[default]
    Select,
    AfterSearch,
}

#[derive(Default)]
/// Represents a two chars combo in a keybind
pub enum AncitipationMode {
    #[default]
    /// no anticipation
    Normal,
    /// contains the anticipated next character
    Char(char),
}

pub struct UI<'a> {
    /// list of all the musics to play (*dsiplayed* musics)
    pub music_list: Musics,
    /// bar indecate the playing timer
    pub power_bar: PowerBar,
    /// currently selected action
    pub action: PowerActions,
    /// currently selected region
    pub region: Region,
    /// UI style
    pub style: UIStyle,
    /// slection mod <Search, AfterSearch, Select>
    pub mode: ListMode,
    /// search buffer, used to search through the musics list
    search_bufr: Input,
    /// command buffer, used to input commands
    command_bufr: Input,
    /// helps reading a combination of keys like `gg`
    pub anticipation_mode: AncitipationMode,
    /// what to repeat <ThisMusic, AllMusics, None>
    displayed_repeat: Repeat,
    /// playlist order
    displayed_sort: Sort,
    /// used to fetch playing state from server
    pub state: State<'a>,
    // TODO: make this private
    pub fps_controller: FPSController,
    /// Displayed lyrics
    displayed_lyrics: DisplayedLyrics,
    /// Displayed Playlists
    displayed_playlists: DisplayedPlaylists,
    /// Views/widgets geometry
    pub geometry: ViewGeometry,
    /// Used for event checking
    pub internal_clock: Interval,
    /// Lua scripts
    pub scripts: Option<Functions>,
    /// Lua interpreter
    pub lua: Lua,
}

#[allow(unused)]
pub struct Functions {
    pub all: Option<mlua::Function>,
    pub list: Option<mlua::Function>,
    pub lyrics: Option<mlua::Function>,
    pub actions: Option<mlua::Function>,
    pub seeker: Option<mlua::Function>,
    pub volume: Option<mlua::Function>,
}

impl Functions {
    fn new(
        all: Option<mlua::Function>,
        list: Option<mlua::Function>,
        lyrics: Option<mlua::Function>,
        actions: Option<mlua::Function>,
        seeker: Option<mlua::Function>,
        volume: Option<mlua::Function>,
    ) -> Self {
        Self {
            all,
            list,
            lyrics,
            actions,
            seeker,
            volume,
        }
    }
}

#[derive(Clone, Deserialize, Serialize)]
/// all the displayed region
pub enum Region {
    List,
    Action,
    Seeker,
    Volume,
    Lyrics,
    Playlist,
    Search,
    Command,
}

impl Default for Region {
    fn default() -> Self {
        Region::List
    }
}

impl ListMode {
    pub async fn handle_search<'a>(ui: &mut UI<'a>, key: &KeyEvent) {
        if key.kind == event::KeyEventKind::Press {
            ui.run_list_script(key, ListMode::Search);
            match key.modifiers {
                KeyModifiers::NONE => match key.code {
                    KeyCode::Enter => {
                        ui.play_selected_music().await;
                        ui.reset_querry();
                        ui.mode = ListMode::Select;
                        ui.geometry.disable_search();
                        return;
                    }
                    KeyCode::Esc => {
                        ui.change_list_mode(ListMode::AfterSearch);
                        ui.select_region(Region::List);
                        return;
                    }
                    _ => {}
                },
                _ => {}
            }
            ui.register_search_query(key.to_owned()).await;
        }
    }

    pub async fn handle_after_search<'a>(ui: &mut UI<'a>, key: &KeyEvent) -> std::io::Result<bool> {
        if key.kind == event::KeyEventKind::Press {
            ui.run_list_script(key, ListMode::AfterSearch);
            match key.modifiers {
                KeyModifiers::NONE => match key.code {
                    KeyCode::Char(c) => match c {
                        'j' => ui.list_down(),
                        'k' => ui.list_up(),
                        '/' => ui.change_list_mode(ListMode::Search),
                        'q' => ui.change_list_mode(ListMode::Select),
                        ' ' => ui.play_after_search().await,
                        _ => {}
                    },
                    KeyCode::Enter => ui.play_after_search().await,
                    KeyCode::Esc => ui.change_list_mode(ListMode::Select),
                    _ => {}
                },
                KeyModifiers::ALT => match key.code {
                    KeyCode::Char(c) => match c {
                        '/' => ui.change_list_mode(ListMode::Search),
                        _ => {}
                    },
                    _ => {}
                },
                _ => {}
            }
        }
        Ok(false)
    }

    pub async fn handle_select<'a>(ui: &mut UI<'a>, key: &KeyEvent) -> std::io::Result<bool> {
        if key.kind == event::KeyEventKind::Press {
            ui.run_list_script(key, ListMode::Select);
            match key.modifiers {
                // No modifiers
                KeyModifiers::NONE => {
                    match ui.anticipation_mode {
                        // No `g` key pressed beforehand
                        AncitipationMode::Normal => match key.code {
                            KeyCode::Enter => ui.play_selected_music().await,
                            KeyCode::Char(c) => match c {
                                'j' => ui.list_down(),
                                'k' => ui.list_up(),
                                '/' => {
                                    ui.select_region(Region::Search);
                                    ui.change_list_mode(ListMode::Search)
                                }
                                ':' => {
                                    ui.change_list_mode(ListMode::Command);
                                    ui.select_region(Region::Command);
                                }
                                ' ' => ui.play_selected_music().await,
                                's' => ui.goto_playing(),
                                'g' => ui.anticipate('g'),
                                'G' => ui.goto_bottom(),
                                'q' => return ui.quit(),
                                _ => {}
                            },
                            _ => {}
                        },
                        AncitipationMode::Char(c) => {
                            if c == 'g' {
                                match key.code {
                                    KeyCode::Char(c) => match c {
                                        'g' => ui.goto_top(),
                                        _ => {}
                                    },
                                    _ => {}
                                }
                            }
                            ui.anticipation_mode = AncitipationMode::Normal;
                        }
                    }
                }
                KeyModifiers::SHIFT => match key.code {
                    KeyCode::Char(c) => match c {
                        'G' => ui.goto_bottom(),
                        _ => {}
                    },
                    _ => {}
                },
                KeyModifiers::CONTROL => match key.code {
                    KeyCode::Char(c) => match c {
                        'd' => ui.scroll_list_down(),
                        'u' => ui.scroll_list_up(),
                        _ => {}
                    },
                    _ => {}
                },
                _ => {}
            }
        }
        Ok(false)
    }

    async fn handle_command(ui: &mut UI<'_>, key: &KeyEvent) -> std::io::Result<bool> {
        if key.kind == event::KeyEventKind::Press {
            ui.run_list_script(key, ListMode::Search);
            match key.modifiers {
                KeyModifiers::NONE => match key.code {
                    KeyCode::Enter => {
                        if ui.run_command().await? {
                            return Ok(true);
                        }
                        ui.change_list_mode(ListMode::Select);
                        ui.select_region(Region::List);
                        return Ok(false);
                    }
                    KeyCode::Esc => {
                        ui.change_list_mode(ListMode::Select);
                        ui.select_region(Region::List);
                        return Ok(false);
                    }
                    _ => {}
                },
                _ => {}
            }
            ui.register_command_query(key.to_owned()).await;
        }
        Ok(false)
    }
}

impl Region {
    pub async fn handle_events<'a>(ui: &mut UI<'a>, key: &KeyEvent) -> io::Result<bool> {
        Region::handle_global(ui, &key).await;
        // keybind depend on reagion
        match ui.region {
            Region::List => {
                if Region::handle_list(ui, &key)
                    .await
                    .is_ok_and(|should_quit| should_quit == true)
                {
                    return Ok(true);
                }
            }
            Region::Seeker => {
                if Region::handle_seeker(ui, &key)
                    .await
                    .is_ok_and(|should_quit| should_quit == true)
                {
                    return Ok(true);
                }
            }
            Region::Volume => {
                if Region::handle_volume(ui, &key)
                    .await
                    .is_ok_and(|should_quit| should_quit == true)
                {
                    return Ok(true);
                }
            }
            Region::Action => {
                if Region::handle_action(ui, &key)
                    .await
                    .is_ok_and(|should_quit| should_quit == true)
                {
                    return Ok(true);
                }
            }
            Region::Lyrics => {
                if Region::handle_lyrics(ui, &key)
                    .await
                    .is_ok_and(|should_quit| should_quit == true)
                {
                    return Ok(true);
                }
            }
            Region::Search => {
                if Region::handle_search(ui, &key)
                    .await
                    .is_ok_and(|should_quit| should_quit == true)
                {
                    return Ok(true);
                }
            }
            Region::Command => {
                if Region::handle_command(ui, &key)
                    .await
                    .is_ok_and(|should_quit| should_quit == true)
                {
                    return Ok(true);
                }
            }
            Region::Playlist => {
                if Region::handle_playlist(ui, &key)
                    .await
                    .is_ok_and(|should_quit| should_quit == true)
                {
                    return Ok(true);
                }
            }
        }
        return Ok(false);
    }

    pub async fn handle_global<'a>(ui: &mut UI<'a>, key: &KeyEvent) {
        if key.kind == event::KeyEventKind::Press {
            ui.run_global_script(key, ui.region.clone());
            match key.modifiers {
                KeyModifiers::NONE => match key.code {
                    KeyCode::Char(c) => match ui.mode {
                        ListMode::Search | ListMode::Command => {}
                        _ => match c {
                            'm' => ui.toggle_mute().await,
                            'p' => ui.toggle_play().await,
                            'n' => ui.play_next().await,
                            _ => {}
                        },
                    },
                    _ => {}
                },
                KeyModifiers::ALT => match ui.mode {
                    ListMode::Select => match key.code {
                        KeyCode::Char(c) => match c {
                            'j' => match ui.region {
                                Region::List => {
                                    if ui.displayed_lyrics.displayable {
                                        ui.select_lyrics_region()
                                    } else if ui.displayed_playlists.displayable {
                                        ui.select_playlist_region()
                                    } else {
                                        ui.select_action_region()
                                    }
                                }
                                Region::Lyrics | Region::Playlist => ui.select_action_region(),
                                Region::Action => ui.select_bar_region(),
                                Region::Seeker => ui.select_list_region(),
                                Region::Volume => ui.select_list_region(),
                                Region::Search | Region::Command => {}
                            },
                            'k' => match ui.region {
                                Region::Search | Region::Command => {}
                                Region::List => ui.select_volume_region(),
                                Region::Lyrics | Region::Playlist => ui.select_list_region(),
                                Region::Action => {
                                    if ui.displayed_lyrics.displayable {
                                        ui.select_lyrics_region()
                                    } else if ui.displayed_playlists.displayable {
                                        ui.select_playlist_region()
                                    } else {
                                        ui.select_list_region()
                                    }
                                }
                                Region::Seeker => ui.select_action_region(),
                                Region::Volume => ui.select_action_region(),
                            },
                            'h' | 'l' => match ui.region {
                                Region::Seeker => ui.select_volume_region(),
                                Region::Volume => ui.select_bar_region(),
                                _ => {}
                            },
                            _ => {}
                        },
                        _ => {}
                    },
                    _ => {}
                },
                KeyModifiers::CONTROL => match key.code {
                    KeyCode::Char(c) => match c {
                        'l' => ui.toggle_lyrics(),
                        'p' => ui.toggle_playlists(),
                        _ => {}
                    },
                    _ => {}
                },
                KeyModifiers::SHIFT => match key.code {
                    KeyCode::Char(c) => match c {
                        'N' => ui.play_preivous().await,
                        'D' => ui.download_lyrics(),
                        _ => {}
                    },
                    _ => {}
                },
                _ => {}
            }
        }
    }

    pub async fn handle_list<'a>(ui: &mut UI<'a>, key: &KeyEvent) -> std::io::Result<bool> {
        match ui.mode {
            ListMode::AfterSearch => {
                if ListMode::handle_after_search(ui, key)
                    .await
                    .is_ok_and(|x| x == true)
                {
                    return Ok(true);
                }
            }
            ListMode::Select => {
                if ListMode::handle_select(ui, key)
                    .await
                    .is_ok_and(|x| x == true)
                {
                    return Ok(true);
                }
            }
            _ => {}
        }
        Ok(false)
    }

    pub async fn handle_action<'a>(ui: &mut UI<'a>, key: &KeyEvent) -> std::io::Result<bool> {
        if key.kind == event::KeyEventKind::Press {
            ui.run_actions_script(key);
            match key.modifiers {
                KeyModifiers::SHIFT => match key.code {
                    KeyCode::BackTab => {
                        ui.cycle_back();
                    }
                    _ => {}
                },
                KeyModifiers::NONE => match key.code {
                    KeyCode::Char(c) => match c {
                        'q' => return ui.quit(),
                        'l' => ui.next_action(),
                        'h' => ui.previous_action(),
                        _ => {}
                    },
                    KeyCode::Enter => {
                        ui.preform_action().await;
                    }
                    KeyCode::Tab => {
                        ui.cycle();
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        Ok(false)
    }

    pub async fn handle_seeker<'a>(ui: &mut UI<'a>, key: &KeyEvent) -> std::io::Result<bool> {
        if key.kind == event::KeyEventKind::Press {
            ui.run_seeker_script(key);
            match key.modifiers {
                KeyModifiers::NONE => match key.code {
                    KeyCode::Char(c) => match c {
                        'l' => ui.next_5s().await,
                        'h' => ui.previous_5s().await,
                        'k' => ui.toggle_play().await,
                        'q' => return ui.quit(),
                        _ => {}
                    },
                    _ => {}
                },
                _ => {}
            }
        }
        Ok(false)
    }

    pub async fn handle_volume<'a>(ui: &mut UI<'a>, key: &KeyEvent) -> std::io::Result<bool> {
        if key.kind == event::KeyEventKind::Press {
            ui.run_volume_script(key);
            match key.modifiers {
                KeyModifiers::NONE => match key.code {
                    KeyCode::Char(c) => match c {
                        'k' | 'l' => ui.increase_volume().await,
                        'h' | 'j' => ui.decrease_volume().await,
                        'q' => return ui.quit(),
                        _ => {}
                    },
                    _ => {}
                },
                KeyModifiers::SHIFT => match key.code {
                    KeyCode::Char(c) => match c {
                        'j' | 'h' => ui.increase_volume().await,
                        'k' | 'l' => ui.decrease_volume().await,
                        _ => {}
                    },
                    _ => {}
                },
                _ => {}
            }
        }
        Ok(false)
    }

    pub async fn handle_lyrics(ui: &mut UI<'_>, key: &KeyEvent) -> std::io::Result<bool> {
        if key.kind == event::KeyEventKind::Press {
            ui.run_lyrics_script(key);
            ui.displayed_lyrics.last_active_interaction = Instant::now();
            match key.modifiers {
                KeyModifiers::NONE => match ui.anticipation_mode {
                    AncitipationMode::Normal => match key.code {
                        KeyCode::Char(c) => match c {
                            'q' => return ui.quit(),
                            'j' => ui.select_next_lyrics_line(),
                            'k' => ui.select_previous_lyrics_line(),
                            'g' => ui.anticipate('g'),
                            _ => {}
                        },
                        KeyCode::Enter => {
                            ui.play_music_from_lyrics().await;
                        }
                        _ => {}
                    },
                    AncitipationMode::Char(c) => {
                        match c {
                            'g' => match key.code {
                                KeyCode::Char(c) => match c {
                                    'g' => ui.goto_top_lyrics(),
                                    _ => {}
                                },
                                _ => {}
                            },
                            _ => {}
                        }
                        ui.anticipation_mode = AncitipationMode::Normal;
                    }
                },
                KeyModifiers::SHIFT => match key.code {
                    KeyCode::Char(c) => match c {
                        'G' => ui.goto_bottom_lyrics(),
                        _ => {}
                    },
                    _ => {}
                },
                KeyModifiers::CONTROL => match key.code {
                    KeyCode::Char(c) => match c {
                        'd' => ui.scroll_lyrics_down(),
                        'u' => ui.scroll_lyrics_up(),
                        _ => {}
                    },
                    _ => {}
                },
                _ => {}
            }
        }
        Ok(false)
    }

    async fn handle_playlist(ui: &mut UI<'_>, key: &KeyEvent) -> std::io::Result<bool> {
        if key.kind == event::KeyEventKind::Press {
            match key.modifiers {
                KeyModifiers::NONE => match key.code {
                    KeyCode::Char(c) => match c {
                        'q' => return ui.quit(),
                        'j' => ui.select_next_playlist_name(),
                        'k' => ui.select_previous_playlist_name(),
                        _ => {}
                    },
                    KeyCode::Enter => {
                        ui.switch_to_selected_playlist().await;
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        return Ok(false);
    }

    async fn handle_search(ui: &mut UI<'_>, key: &KeyEvent) -> std::io::Result<bool> {
        match ui.mode {
            ListMode::Search => {
                ListMode::handle_search(ui, key).await;
            }
            _ => {}
        }
        // Never quit on search mode
        Ok(false)
    }

    async fn handle_command(ui: &mut UI<'_>, key: &KeyEvent) -> std::io::Result<bool> {
        match ui.mode {
            ListMode::Command => ListMode::handle_command(ui, key).await,
            _ => Ok(false),
        }
    }
}

pub fn to_input_request(evt: &CrosstermEvent) -> Option<InputRequest> {
    use InputRequest::*;
    use KeyCode::*;
    match evt {
        CrosstermEvent::Key(KeyEvent {
            code,
            modifiers,
            kind,
            state: _,
        }) if *kind == KeyEventKind::Press => match (*code, *modifiers) {
            (Backspace, KeyModifiers::NONE) | (Char('h'), KeyModifiers::CONTROL) => {
                Some(DeletePrevChar)
            }
            (Delete, KeyModifiers::NONE) => Some(DeleteNextChar),
            (Tab, KeyModifiers::NONE) => None,
            (Left, KeyModifiers::NONE) | (Char('b'), KeyModifiers::CONTROL) => Some(GoToPrevChar),
            (Left, KeyModifiers::CONTROL) | (Char('b'), KeyModifiers::META) => Some(GoToPrevWord),
            (Right, KeyModifiers::NONE) | (Char('f'), KeyModifiers::CONTROL) => Some(GoToNextChar),
            (Right, KeyModifiers::CONTROL) | (Char('f'), KeyModifiers::META) => Some(GoToNextWord),
            (Char('u'), KeyModifiers::CONTROL) => Some(DeleteLine),

            (Char('w'), KeyModifiers::CONTROL)
            | (Char('d'), KeyModifiers::META)
            | (Backspace, KeyModifiers::META)
            | (Backspace, KeyModifiers::ALT) => Some(DeletePrevWord),

            (Delete, KeyModifiers::CONTROL) => Some(DeleteNextWord),
            (Char('k'), KeyModifiers::CONTROL) => Some(DeleteTillEnd),
            (Char('a'), KeyModifiers::CONTROL) | (Home, KeyModifiers::NONE) => Some(GoToStart),
            (Char('e'), KeyModifiers::CONTROL) | (End, KeyModifiers::NONE) => Some(GoToEnd),
            (Char(c), KeyModifiers::NONE) => Some(InsertChar(c)),
            (Char(c), KeyModifiers::SHIFT) => Some(InsertChar(c)),
            (_, _) => None,
        },
        _ => None,
    }
}

/// Import this trait to implement `Input::handle_event()` for crossterm.
pub trait EventHandler {
    /// Handle crossterm event.
    fn handle_event(&mut self, evt: &CrosstermEvent) -> Option<StateChanged>;
}

impl EventHandler for Input {
    /// Handle crossterm event.
    fn handle_event(&mut self, evt: &CrosstermEvent) -> Option<StateChanged> {
        to_input_request(evt).and_then(|req| self.handle(req))
    }
}

impl<'a> UI<'a> {
    fn run_script(
        &self,
        f: &Option<mlua::Function>,
        key: &KeyEvent,
        region: Region,
        mode: Option<ListMode>,
    ) -> mlua::Result<()> {
        if let Some(function) = f {
            let ev = Event::new(key.modifiers.into(), key.code.into(), region, mode);
            let userdata = self.lua.create_userdata(ev)?;
            return function.call::<()>(userdata);
        }
        Ok(())
    }

    pub fn run_list_script(&self, key: &KeyEvent, mode: ListMode) {
        match &self.scripts {
            Some(scripts) => match &scripts.list {
                Some(function) => {
                    let ev = Event::new(
                        key.modifiers.into(),
                        key.code.into(),
                        Region::List,
                        Some(mode),
                    );
                    let userdata = self.lua.create_userdata(ev);
                    function.call::<()>(userdata).unwrap();
                }
                None => {}
            },
            None => {}
        }
    }

    pub fn run_global_script(&self, key: &KeyEvent, region: Region) {
        match &self.scripts {
            Some(scripts) => {
                let _ = self.run_script(&scripts.all, key, region, None);
            }
            None => {}
        }
    }

    pub fn run_seeker_script(&self, key: &KeyEvent) {
        match &self.scripts {
            Some(script) => {
                let _ = self.run_script(&script.seeker, key, Region::Seeker, None);
            }
            None => {}
        }
    }

    pub fn run_actions_script(&self, key: &KeyEvent) {
        match &self.scripts {
            Some(script) => {
                let _ = self.run_script(&script.seeker, key, Region::Action, None);
            }
            None => {}
        }
    }

    pub fn run_volume_script(&self, key: &KeyEvent) {
        match &self.scripts {
            Some(script) => {
                let _ = self.run_script(&script.seeker, key, Region::Volume, None);
            }
            None => {}
        }
    }

    pub fn run_lyrics_script(&self, key: &KeyEvent) {
        match &self.scripts {
            Some(script) => {
                let _ = self.run_script(&script.seeker, key, Region::Lyrics, None);
            }
            None => {}
        }
    }

    pub fn new(proxy: ServerProxy<'a>) -> Self {
        let lua = unsafe { mlua::Lua::unsafe_new() };
        UI {
            scripts: None,
            power_bar: PowerBar::default(),
            music_list: Musics::default(),
            region: Region::default(),
            style: UIStyle::default(),
            action: PowerActions::BackwardSkip,
            state: State::new(proxy),
            mode: ListMode::default(),
            search_bufr: Input::default(),
            command_bufr: Input::default(),
            anticipation_mode: AncitipationMode::default(),
            displayed_repeat: Repeat::default(),
            displayed_sort: Sort::default(),
            displayed_lyrics: DisplayedLyrics::default(),
            displayed_playlists: DisplayedPlaylists::default(),
            fps_controller: FPSController::default(),
            geometry: ViewGeometry::default(Rect::default()),
            internal_clock: tokio::time::interval(Duration::from_secs(1)),
            lua,
        }
    }

    pub async fn check_state_streams(&mut self) {
        let mut ctx = Context::from_waker(&futures::task::noop_waker_ref());
        let paused_stream = self.state.dbus_streams.paused_stream.as_mut().unwrap();
        let resumed_stream = self.state.dbus_streams.resumed_stream.as_mut().unwrap();
        let stopped_stream = self.state.dbus_streams.stopped_stream.as_mut().unwrap();
        let playlist_updated_stream = self.state.dbus_streams.playlist_updated.as_mut().unwrap();
        let volume_stream = self
            .state
            .dbus_streams
            .volume_changed_stream
            .as_mut()
            .unwrap();
        let music_played = self
            .state
            .dbus_streams
            .music_played_stream
            .as_mut()
            .unwrap();
        let playlist_created_stream = self.state.dbus_streams.playlist_created.as_mut().unwrap();
        let playlist_deleted_stream = self.state.dbus_streams.playlist_deleted.as_mut().unwrap();
        let playlist_loaded_stream = self.state.dbus_streams.playlist_loaded.as_mut().unwrap();
        let sorted_stream = self.state.dbus_streams.sorted_stream.as_mut().unwrap();

        match playlist_updated_stream.poll_next_unpin(&mut ctx) {
            Poll::Ready(_) => {
                let playlist = self.state.proxy.playlist().await.unwrap_or_default();
                self.music_list.unfiltered_music_list.clear();
                for (index, music) in playlist.musics.iter().enumerate() {
                    let music = music.to_owned();
                    self.music_list.unfiltered_music_list.push(Music {
                        path: music.path,
                        title: music.title,
                        length: music.length,
                        artist: music.artist,
                        genre: music.genre,
                        index,
                    });
                }
                self.music_list.musics = self.music_list.unfiltered_music_list.clone();
                self.music_list.playing_index = playlist.playing_index as usize;
                self.state.batch.metadata = self.state.proxy.metadata().await.unwrap_or_default();

                let timer = self.state.proxy.timer().await.unwrap_or_default();
                let played = timer.0;
                let full = timer.1;
                (
                    self.state.batch.played_duration,
                    self.state.batch.music_duration,
                ) = (
                    Duration::from_secs_f32(played),
                    Duration::from_secs_f32(full),
                );
                self.state.batch.playing_music =
                    self.state.proxy.playing().await.unwrap_or_default();
            }
            _ => {}
        }

        match paused_stream.poll_next_unpin(&mut ctx) {
            Poll::Ready(_) => {
                self.state.batch.status = PlayingStatus::Pausing;
            }
            _ => {}
        }

        match resumed_stream.poll_next_unpin(&mut ctx) {
            Poll::Ready(_) => {
                self.state.batch.status = PlayingStatus::Playing;
            }
            _ => {}
        }

        match stopped_stream.poll_next_unpin(&mut ctx) {
            Poll::Ready(_) => {
                self.state.batch.status = PlayingStatus::Stopped;
            }
            _ => {}
        }

        match music_played.poll_next_unpin(&mut ctx) {
            Poll::Ready(_) => {
                self.state.batch.metadata.lyrics =
                    self.state.proxy.lyrics().await.unwrap_or_default();
                self.music_list.playing_index =
                    self.state.proxy.get_playing_index().await.unwrap() as usize;
                self.state.batch.playing_music = self.state.proxy.playing().await.unwrap();
                self.state.batch.status = PlayingStatus::Playing;
            }
            _ => {}
        }

        match volume_stream.poll_next_unpin(&mut ctx) {
            Poll::Ready(v) => {
                let volume = v.unwrap().args().unwrap().amount.max(0.0).min(1.0);
                self.state.batch.volume = volume;
            }
            _ => {}
        }

        match playlist_created_stream.poll_next_unpin(&mut ctx) {
            Poll::Ready(v) => {
                self.state
                    .batch
                    .playlists_names
                    .push(v.unwrap().args().unwrap().id().to_string());
            }
            _ => {}
        }

        match playlist_deleted_stream.poll_next_unpin(&mut ctx) {
            Poll::Ready(v) => {
                let id = v.unwrap().args().unwrap().id().to_string();
                if let Some(pos) = self
                    .state
                    .batch
                    .playlists_names
                    .iter()
                    .position(|x| x.eq(&id))
                {
                    self.state.batch.playlists_names.remove(pos);
                }
            }
            _ => {}
        }

        match playlist_loaded_stream.poll_next_unpin(&mut ctx) {
            Poll::Ready(_) => {
                self.state.batch.status = PlayingStatus::Stopped;

                let playlist = self.state.proxy.playlist().await.unwrap_or_default();
                self.music_list.unfiltered_music_list.clear();
                for (index, music) in playlist.musics.iter().enumerate() {
                    let music = music.to_owned();
                    self.music_list.unfiltered_music_list.push(Music {
                        path: music.path,
                        title: music.title,
                        length: music.length,
                        artist: music.artist,
                        genre: music.genre,
                        index,
                    });
                }
                self.music_list.musics = self.music_list.unfiltered_music_list.clone();
                self.music_list.playing_index = playlist.playing_index as usize;
                self.state.batch.metadata = self.state.proxy.metadata().await.unwrap_or_default();

                let timer = self.state.proxy.timer().await.unwrap_or_default();
                let played = timer.0;
                let full = timer.1;
                (
                    self.state.batch.played_duration,
                    self.state.batch.music_duration,
                ) = (
                    Duration::from_secs_f32(played),
                    Duration::from_secs_f32(full),
                );
                self.state.batch.playing_music =
                    self.state.proxy.playing().await.unwrap_or_default();
            }
            _ => {}
        }

        match sorted_stream.poll_next_unpin(&mut ctx) {
            Poll::Ready(v) => {
                self.state.batch.sort = v.unwrap().args().unwrap().sort;
                self.fetch_playlist_data().await;
            }
            _ => {}
        }
    }

    pub fn update_scripts(&mut self, scripts: &Option<Scripts>) {
        self.scripts = match scripts {
            Some(scripts) => {
                let fall = match &scripts.all {
                    Some(path) => {
                        let file_content = std::fs::read_to_string(path).unwrap();
                        let func: mlua::Function = self.lua.load(&file_content).eval().unwrap();
                        Some(func)
                    }
                    None => None,
                };

                let flist = match &scripts.list {
                    None => None,
                    Some(path) => {
                        let file_content = std::fs::read_to_string(path).unwrap();
                        let func: mlua::Function = self.lua.load(&file_content).eval().unwrap();
                        Some(func)
                    }
                };

                let flyrics = match &scripts.lyrics {
                    None => None,
                    Some(path) => {
                        let file_content = std::fs::read_to_string(path).unwrap();
                        let func: mlua::Function = self.lua.load(&file_content).eval().unwrap();
                        Some(func)
                    }
                };

                let factions = match &scripts.actions {
                    Some(path) => {
                        let file_content = std::fs::read_to_string(path).unwrap();
                        let func: mlua::Function = self.lua.load(&file_content).eval().unwrap();
                        Some(func)
                    }
                    None => None,
                };

                let fseeker = match &scripts.seeker {
                    Some(path) => {
                        let file_content = std::fs::read_to_string(path).unwrap();
                        let func: mlua::Function = self.lua.load(&file_content).eval().unwrap();
                        Some(func)
                    }
                    None => None,
                };

                let fvolume = match &scripts.volume {
                    Some(path) => {
                        let file_content = std::fs::read_to_string(path).unwrap();
                        let func: mlua::Function = self.lua.load(&file_content).eval().unwrap();
                        Some(func)
                    }
                    None => None,
                };

                Some(Functions::new(
                    fall, flist, flyrics, factions, fseeker, fvolume,
                ))
            }
            None => None,
        }
    }

    pub fn update_from_config(&mut self, config: &Config) {
        self.update_scripts(&config.scripts);
        let config = config.clone();
        self.fps_controller.change_fps(config.fps.unwrap_or(30));
        self.displayed_lyrics.displayable = config.lyrics.unwrap_or(false);
        self.music_list.display_genre = config.genre.unwrap_or(false);
        self.music_list.setp_szie = config.step_size.unwrap_or(7);
        self.geometry.configuration.lyrics = self.displayed_lyrics.displayable;
    }

    pub fn previous_action(&mut self) {
        self.action = match self.action {
            PowerActions::Sort => PowerActions::Repeat,
            PowerActions::Repeat => PowerActions::Stop,
            PowerActions::Stop => PowerActions::ForwardSkip,
            PowerActions::ForwardSkip => PowerActions::TogglePlay,
            PowerActions::TogglePlay => PowerActions::BackwardSkip,
            PowerActions::BackwardSkip => PowerActions::Sort,
        }
    }

    pub fn next_action(&mut self) {
        self.action = match self.action {
            PowerActions::BackwardSkip => PowerActions::TogglePlay,
            PowerActions::TogglePlay => PowerActions::ForwardSkip,
            PowerActions::ForwardSkip => PowerActions::Stop,
            PowerActions::Stop => PowerActions::Repeat,
            PowerActions::Repeat => PowerActions::Sort,
            PowerActions::Sort => PowerActions::BackwardSkip,
        }
    }

    pub async fn preform_action(&mut self) {
        // match the selected action
        match self.action {
            // we are on the toggle play botton
            PowerActions::TogglePlay => self.toggle_play().await,
            PowerActions::ForwardSkip => self.play_next().await,
            PowerActions::BackwardSkip => self.play_previous().await,
            PowerActions::Stop => self.stop().await,
            PowerActions::Sort => self.sort().await,
            PowerActions::Repeat => self.repeat().await,
        }
    }

    /// plays the *selected* song in the music list
    pub async fn play_selected_music(&mut self) {
        self.state
            .play_from_index(
                self.music_list
                    .musics
                    .get(self.music_list.selected)
                    .unwrap()
                    .index,
            )
            .await;
    }

    /// Moves slection to the `List` region
    pub fn select_list_region(&mut self) {
        self.region = Region::List;
    }

    /// Moves slection to the `Bar` region
    pub fn select_bar_region(&mut self) {
        self.region = Region::Seeker;
    }

    /// Moves slection to the `Action` region
    pub fn select_action_region(&mut self) {
        self.region = Region::Action
    }

    fn render_list_with_genre(&mut self, frame: &mut Frame) {
        let mut rows = vec![];
        let playing_index = self.music_list.playing_index;
        for (index, music) in self.music_list.musics.iter().enumerate() {
            let mut title = music.title.to_owned();
            let artist = music.artist.to_owned();
            let time = UI::duration_to_string(music.length.as_secs());
            let genre = music.genre.to_owned();
            if playing_index == index {
                title.insert_str(0, self.style.list_style.playing_selector.as_str());
                rows.push(
                    Row::new(vec![title, artist, genre, time])
                        .style(self.style.list_style.playing_region_color),
                )
            } else {
                rows.push(Row::new(vec![title, artist, genre, time]))
            }
        }

        let widths = [
            Constraint::Fill(3),
            Constraint::Fill(2),
            Constraint::Fill(1),
            Constraint::Fill(1),
        ];

        let block = match self.region {
            Region::List => match self.mode {
                ListMode::Command => Block::default()
                    .title("Musics")
                    .borders(Borders::ALL)
                    .fg(self.style.list_style.active_command_region_color),
                ListMode::Search => Block::default()
                    .title("Musics")
                    .borders(Borders::ALL)
                    .fg(self.style.list_style.active_search_region_color),
                ListMode::AfterSearch => Block::default()
                    .title("Musics")
                    .borders(Borders::ALL)
                    .fg(self.style.list_style.active_after_search_region_color),
                ListMode::Select => Block::default()
                    .title("Musics")
                    .borders(Borders::ALL)
                    .fg(self.style.list_style.active_region_color),
            },
            _ => Block::default()
                .title("Musics")
                .borders(Borders::ALL)
                .fg(self.style.list_style.passive_region_color),
        };

        let table = Table::new(rows, widths)
            .block(block)
            .highlight_style(
                Style::new()
                    .add_modifier(Modifier::REVERSED)
                    .fg(self.style.list_style.hilight_color),
            )
            .highlight_symbol(self.style.list_style.selector.as_str())
            .header(
                Row::new(vec!["Title", "artist", "genre", "duration"])
                    .style(Style::new().bold().italic()),
            );

        frame.render_stateful_widget(table, self.geometry.list_geo(), &mut self.music_list.state);
    }

    fn render_list_without_genre(&mut self, frame: &mut Frame) {
        let mut rows = vec![];
        let playing_index = self.music_list.playing_index;
        for (index, music) in self.music_list.musics.iter().enumerate() {
            let mut title = music.title.to_owned();
            let artist = music.artist.to_owned();
            let time = UI::duration_to_string(music.length.as_secs());
            if playing_index == index {
                title.insert_str(0, self.style.list_style.playing_selector.as_str());
                rows.push(
                    Row::new(vec![title, artist, time])
                        .style(self.style.list_style.playing_region_color),
                )
            } else {
                rows.push(Row::new(vec![title, artist, time]))
            }
        }

        let widths = [
            Constraint::Fill(3),
            Constraint::Fill(2),
            Constraint::Fill(1),
        ];

        let block = match self.region {
            Region::List => match self.mode {
                ListMode::Search => Block::default()
                    .title("Musics")
                    .borders(Borders::ALL)
                    .fg(self.style.list_style.active_search_region_color),
                ListMode::AfterSearch => Block::default()
                    .title("Musics")
                    .borders(Borders::ALL)
                    .fg(self.style.list_style.active_after_search_region_color),
                ListMode::Select => Block::default()
                    .title("Musics")
                    .borders(Borders::ALL)
                    .fg(self.style.list_style.active_region_color),
                ListMode::Command => Block::default()
                    .title("Musics")
                    .borders(Borders::ALL)
                    .fg(self.style.list_style.active_command_region_color),
            },
            _ => Block::default()
                .title("Musics")
                .borders(Borders::ALL)
                .fg(self.style.list_style.passive_region_color),
        };

        let table = Table::new(rows, widths)
            .block(block)
            .highlight_style(
                Style::new()
                    .add_modifier(Modifier::REVERSED)
                    .fg(self.style.list_style.hilight_color),
            )
            .highlight_symbol(self.style.list_style.selector.as_str())
            .header(
                Row::new(vec!["Title", "artist", "duration"]).style(Style::new().bold().italic()),
            );

        match self.mode {
            ListMode::Command => match self.geometry.command_geo() {
                Some(geo) => {
                    frame.render_widget(
                        Paragraph::new(self.command_bufr.value())
                            .block(Block::bordered().title("Querry")),
                        geo,
                    );
                }
                None => {}
            },
            ListMode::Search | ListMode::AfterSearch => match self.geometry.search_geo() {
                Some(geo) => {
                    frame.render_widget(
                        Paragraph::new(self.search_bufr.value())
                            .block(Block::bordered().title("Querry")),
                        geo,
                    );
                }
                None => {}
            },
            ListMode::Select => {}
        }

        frame.render_stateful_widget(table, self.geometry.list_geo(), &mut self.music_list.state);
    }

    /// Renders the region of the music list
    pub fn render_list(&mut self, frame: &mut Frame) {
        if self.music_list.display_genre {
            self.render_list_with_genre(frame)
        } else {
            self.render_list_without_genre(frame)
        }
    }

    fn get_action_index(&self) -> usize {
        return match self.action {
            PowerActions::BackwardSkip => 0,
            PowerActions::TogglePlay => 1,
            PowerActions::ForwardSkip => 2,
            PowerActions::Stop => 3,
            PowerActions::Repeat => 4,
            PowerActions::Sort => 5,
        };
    }

    /// Converst timer in the u64 form to a string of form xx:yy
    pub fn duration_to_string(time: u64) -> String {
        let seconds = time % 60;
        let minities = time / 60;

        let sseconds = if seconds > 9 {
            format!("{}", seconds)
        } else {
            format!("0{}", seconds)
        };

        let sminutes = if minities > 9 {
            format!("{}", minities)
        } else {
            format!("0{}", minities)
        };

        return format!("{}:{}", sminutes, sseconds);
    }

    /// Returns the current playing timer as a string "xx:yy"
    fn timer(&mut self) -> String {
        format!(
            "{}/{}",
            UI::duration_to_string(self.state.played_duration().as_secs()),
            UI::duration_to_string(self.state.playing_music().length.as_secs())
        )
    }

    /// Calculates the percentage of the seeker with respect with the full song length
    fn seeker_percent(&self) -> f64 {
        let current = self.state.played_duration().as_secs();
        let music_length = self.state.playing_music().length.as_secs();
        if music_length != 0 {
            let mut percent = current as f64 / music_length as f64;
            percent = percent.max(0.0).min(1.0);
            return percent;
        } else {
            return 0.0;
        };
    }

    /// Seeks playing time forward by 5 seconds
    pub async fn next_5s(&mut self) {
        let current = self.state.played_duration();
        let max = self.state.playing_music_duration();
        if current + Duration::from_secs(5) < max {
            self.power_bar.current_timer = current + Duration::from_secs(5);
            self.state
                .seek(self.power_bar.current_timer.as_secs_f64())
                .await;
        } else {
            self.play_next().await;
        }
    }

    /// Seeks playing time to the target duration if it fits, otherwise
    /// plays the next music
    pub async fn goto_xs(&mut self, target: Duration) {
        self.state.seek(target.as_secs_f64()).await;
    }

    /// Seeks playing time backward by 5 seconds
    pub async fn previous_5s(&mut self) {
        let current = self.state.played_duration();
        match current.checked_sub(Duration::from_secs(5)) {
            Some(dur) => {
                self.power_bar.current_timer = dur;
                self.state.seek(dur.as_secs_f64()).await;
            }
            None => {
                self.play_preivous().await;
            }
        }
    }

    /// Renders the displayed time seeker
    pub fn render_seeker(&mut self, frame: &mut Frame) {
        let selected_music = self.state.playing_music();
        let style = match self.region {
            Region::Seeker => Style::new().fg(self.style.seeker_style.active_region_color),
            _ => Style::new().fg(self.style.seeker_style.passive_region_color),
        };

        LineGauge::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(selected_music.title.to_owned()),
            )
            .style(style)
            .unfilled_style(Style::default().fg(Color::Black))
            .line_set(symbols::line::THICK)
            .filled_style(
                Style::default()
                    .fg(self.style.seeker_style.fg_seeker_color)
                    .bg(self.style.seeker_style.bg_seeker_color)
                    .add_modifier(Modifier::ITALIC),
            )
            .label(self.timer())
            .ratio(self.seeker_percent())
            .render(self.geometry.slider_geo(), frame.buffer_mut());
    }

    /// calculate_volume_area() -> Rect
    /// Renders the displayed volume slider
    pub fn render_volume(&mut self, frame: &mut Frame) {
        let style = match self.region {
            Region::Volume => Style::new().fg(self.style.volume_style.active_region_color),
            _ => Style::new().fg(self.style.volume_style.passive_region_color),
        };

        LineGauge::default()
            .block(Block::default().borders(Borders::ALL).title("Volume"))
            .style(style)
            .unfilled_style(Style::default().fg(Color::Black))
            .filled_style(
                Style::default()
                    .fg(self.style.volume_style.fg_volume_color)
                    .bg(self.style.volume_style.bg_volume_color)
                    .add_modifier(Modifier::ITALIC),
            )
            .ratio(self.state.volume() as f64)
            .render(self.geometry.volume_geo(), frame.buffer_mut());
    }

    /// Renders the displayed actions
    pub fn render_actions(&mut self, frame: &mut Frame) {
        let mut area = frame.area();
        // - seeker hight - action height
        area.y = area.height - 3 - 4;
        area.height = 3;

        let style = match self.region {
            Region::Action => Style::new().fg(self.style.action_style.active_region_color),
            _ => Style::new().fg(self.style.action_style.passive_region_color),
        };

        let mut actions = vec![
            Span::from("⏮"),
            Span::from("⏵"),
            Span::from("⏭"),
            Span::from("⏹"),
        ];

        if self.displayed_repeat.eq(&self.state.batch.repeat) {
            actions.push(self.displayed_repeat.as_span().underlined());
        } else {
            actions.push(self.displayed_repeat.as_span());
        }

        if self.displayed_sort.eq(&self.state.batch.sort) {
            actions.push(self.displayed_sort.as_span().underlined());
        } else {
            actions.push(self.displayed_sort.as_span());
        }

        let status = self.state.status();
        match status {
            PlayingStatus::Playing => {
                actions[1] = Span::from("⏸");
            }
            _ => {}
        }

        Tabs::new(actions)
            .block(Block::default().title("Actions").borders(Borders::ALL))
            .style(style)
            .highlight_style(
                Style::default()
                    .fg(self.style.action_style.hilight_color)
                    .underline_color(self.style.action_style.hilight_color),
            )
            .select(self.get_action_index())
            .padding(" ", " ")
            .render(self.geometry.actions_geo(), frame.buffer_mut());
    }

    /// Updates the music playing state
    pub async fn update_state(&mut self) {
        self.check_state_streams().await;
        self.update_music_selection_state().await;
        self.update_played_duration().await;
        self.update_lyrics_state().await;
    }

    /// fetch the currently playing music data, e.g. photo, lyrics, etc
    pub async fn fetch_playlist_data(&mut self) {
        let playlist = self.state.proxy.playlist().await.unwrap_or_default();
        self.music_list.unfiltered_music_list.clear();
        for (index, music) in playlist.musics.iter().enumerate() {
            let music = music.to_owned();
            self.music_list.unfiltered_music_list.push(Music {
                path: music.path,
                title: music.title,
                length: music.length,
                artist: music.artist,
                genre: music.genre,
                index,
            });
        }
        self.music_list.musics = self.music_list.unfiltered_music_list.clone();
        self.music_list.playing_index = playlist.playing_index as usize;
        self.state.fetch_playlist_data().await
    }

    /// Renders the displayed UI
    pub fn render(&mut self, frame: &mut Frame) {
        self.update_geometry(frame);
        self.render_list(frame);
        self.render_search(frame);
        self.render_lyrics(frame);
        self.render_playlists(frame);
        self.render_commands(frame);
        self.render_actions(frame);
        self.render_volume(frame);
        self.render_seeker(frame);
    }

    /// Handles music selection in the music list
    pub async fn update_music_selection_state(&mut self) {
        self.music_list.state.select(Some(self.music_list.selected));
    }

    /// Selectes the upper element in the music list (goes up by 1)
    pub fn list_up(&mut self) {
        let quesize = self.music_list.musics.len();
        let selected_index = self.music_list.selected;
        if selected_index == 0 {
            self.music_list.selected = quesize - 1;
        } else {
            self.music_list.selected = self.music_list.selected - 1;
        }
    }

    /// Select the next element in the music list (goes down by 1)
    pub fn list_down(&mut self) {
        let quesize = self.music_list.musics.len();
        let selected_index = self.music_list.selected;

        if Some(selected_index) == quesize.checked_sub(1) {
            self.music_list.selected = 0;
        } else {
            self.music_list.selected = self.music_list.selected + 1;
        }
    }

    /// Appends the char to the existing search buffer
    pub async fn register_search_query(&mut self, c: KeyEvent) {
        self.search_bufr
            .handle_event(&crossterm::event::Event::Key(c));
        self.music_list.search(self.search_bufr.value().to_owned());
    }

    /// Appends the char to the existing command buffer
    pub async fn register_command_query(&mut self, c: KeyEvent) {
        self.command_bufr
            .handle_event(&crossterm::event::Event::Key(c));
    }

    /// Resets the search querry to an empty string
    pub fn reset_querry(&mut self) {
        self.music_list.reset_search();
    }

    /// Selects the first element in the music list
    pub fn goto_top(&mut self) {
        self.music_list.selected = 0;
    }

    /// Selects the first element in the lyrics list
    pub fn goto_top_lyrics(&mut self) {
        if self.displayed_lyrics.displayable {
            self.displayed_lyrics.state.select(Some(0))
        }
    }

    /// Selects the currently playing song in the music list
    pub fn goto_playing(&mut self) {
        self.music_list.selected = self.music_list.playing_index;
    }

    /// Selects the last element in the music list
    pub fn goto_bottom(&mut self) {
        self.music_list.selected = self.music_list.musics.len() - 1;
    }

    /// Selects the last element in the lyrics list
    pub fn goto_bottom_lyrics(&mut self) {
        if self.displayed_lyrics.displayable {
            self.displayed_lyrics
                .state
                .select(Some(self.displayed_lyrics.lyrics.lines.len() - 1))
        }
    }

    /// Increases volume
    pub async fn increase_volume(&self) {
        self.state.increase_volume().await;
    }

    /// Decreases volume
    pub async fn decrease_volume(&self) {
        self.state.decrease_volume().await;
    }

    /// selects the volume reagion in the ui
    pub fn select_volume_region(&mut self) {
        self.region = Region::Volume
    }

    /// Plays the previous music in the list
    async fn play_preivous(&mut self) {
        self.state.play_preivous().await;
    }

    /// Plays the next song in the music list
    async fn play_next(&mut self) {
        self.state.play_next().await;
    }

    /// Plays the next song in the music list
    async fn play_previous(&mut self) {
        self.state.play_preivous().await;
    }

    /// Toggls the playing music
    /// - if playing:
    ///     - pause
    /// - if pausing:
    ///     - resume
    /// - if stopping
    ///     - play
    pub async fn toggle_play(&mut self) {
        self.state.toggle_play().await;
    }

    /// derives the displayed lyrics state:  
    /// - highlight index
    async fn derive_lyrics_state(&mut self) {
        let lyrics = self.state.metadata().lyrics;
        // duration since the last action is greater than 2 secs
        if Instant::now().duration_since(self.displayed_lyrics.last_active_interaction)
            > Duration::from_secs(2)
        {
            // music has a lyrics, and the lyrics timestamp is correct
            if self.displayed_lyrics.displayable && lyrics.time_is_correct {
                // get the first line
                lyrics.lines.get(0).unwrap().timestamp;
                let current = self.state.played_duration();
                let index = lyrics.lines.binary_search_by(|l| l.timestamp.cmp(&current));
                if let Err(i) = index {
                    if i == 0 || i == 1 {
                        self.displayed_lyrics.state.select(Some(0));
                    } else {
                        self.displayed_lyrics.state.select(Some(i - 1));
                    }
                } else {
                    let index = index.unwrap();
                    if index == 0 || index == 1 {
                        self.displayed_lyrics.state.select(Some(0));
                    } else {
                        self.displayed_lyrics.state.select(Some(index - 1));
                    }
                }
            }
        }
    }

    /// restore the displayable lyrics **widget** state
    async fn restore_displayable_lyrics(&mut self) {
        self.fetch_playlist_data().await;
        self.derive_lyrics().await;
        self.derive_lyrics_state().await;
    }

    /// Restores playing state
    pub async fn restore_state(&mut self) {
        //----------------------------------------------------
        self.state.dbus_streams.paused_stream =
            Some(self.state.proxy.receive_paused().await.unwrap());
        self.state.dbus_streams.resumed_stream =
            Some(self.state.proxy.receive_resumed().await.unwrap());
        self.state.dbus_streams.stopped_stream =
            Some(self.state.proxy.receive_ended().await.unwrap());
        self.state.dbus_streams.music_played_stream =
            Some(self.state.proxy.receive_music_played().await.unwrap());
        self.state.dbus_streams.volume_changed_stream =
            Some(self.state.proxy.receive_volume_changed().await.unwrap());
        self.state.dbus_streams.sorted_stream =
            Some(self.state.proxy.receive_sorted().await.unwrap());
        self.state.dbus_streams.playlist_updated =
            Some(self.state.proxy.receive_playlists_updated().await.unwrap());
        self.state.dbus_streams.playlist_loaded =
            Some(self.state.proxy.receive_playlist_loaded().await.unwrap());
        self.state.dbus_streams.playlist_deleted =
            Some(self.state.proxy.receive_playlist_deleted().await.unwrap());
        self.state.dbus_streams.playlist_created =
            Some(self.state.proxy.receive_playlist_created().await.unwrap());
        //----------------------------------------------------
        //TODO: make this something more readable
        self.displayed_repeat = self.state.get_repeat();
        self.displayed_sort = self.state.get_order();
        let player_state = self.state.proxy.player_status().await.unwrap();
        self.state.batch.volume = player_state.volume.max(0.0).min(1.0);
        self.state.batch.status = player_state.status;
        self.music_list.playing_index = player_state.index as usize;
        //----------------------------------------------------
        self.restore_displayable_lyrics().await;
    }

    pub fn set_frame(&mut self, frame: &mut Frame) {
        self.geometry.frame = frame.area()
    }

    /// Cycles through actions in this orders
    /// - sort action:
    ///     ByTitleAscending -> ByTitleDescending -> ByDurationAscending -> ByDurationDescending -> Shuffle -> ByTitleAscending
    /// - repeat action:
    ///     ThisMusic -> AllMusics -> Dont -> ThisMusic
    pub fn cycle_back(&mut self) {
        // FIXME: prefor sorts?
        match self.action {
            PowerActions::Sort => match self.displayed_sort {
                Sort::ByTitleAscending => self.displayed_sort = Sort::Shuffle,
                Sort::ByTitleDescending => self.displayed_sort = Sort::ByTitleAscending,
                Sort::ByDurationAscending => self.displayed_sort = Sort::ByTitleDescending,
                Sort::ByDurationDescending => self.displayed_sort = Sort::ByDurationAscending,
                Sort::ArtistAscending => self.displayed_sort = Sort::ByDurationDescending,
                Sort::ArtistDescending => self.displayed_sort = Sort::ArtistAscending,
                Sort::Shuffle => self.displayed_sort = Sort::ArtistDescending,
            },
            // ThisMusic -> AllMusics -> Dont
            PowerActions::Repeat => match self.displayed_repeat {
                Repeat::SameMusic => self.displayed_repeat = Repeat::Dont,
                Repeat::AllMusics => self.displayed_repeat = Repeat::SameMusic,
                Repeat::Dont => self.displayed_repeat = Repeat::AllMusics,
            },
            _ => {}
        }
    }

    /// Scrolls through the music list *up* by 7 units
    pub fn scroll_list_up(&mut self) {
        let queue_size = self.music_list.musics.len();
        let selected_index = self.music_list.selected;
        if selected_index == 0 {
            self.music_list.selected = queue_size - 1;
        } else if selected_index.saturating_sub(self.music_list.setp_szie) == 0 {
            self.music_list.selected = 0;
        } else {
            self.music_list.selected = self.music_list.selected - 7;
        }
    }

    /// Scrolls through the music list *down* by 7 units
    pub fn scroll_list_down(&mut self) {
        let queue_size = self.music_list.musics.len();
        let selected_index = self.music_list.selected;

        if selected_index == queue_size - 1 {
            self.music_list.selected = 0;
        } else if selected_index.saturating_add(self.music_list.setp_szie) >= queue_size {
            self.music_list.selected = queue_size - 1;
        } else {
            self.music_list.selected = self
                .music_list
                .selected
                .saturating_add(self.music_list.setp_szie);
        }
    }

    pub async fn toggle_mute(&self) {
        self.state.toggle_mute().await;
    }

    async fn stop(&self) {
        self.state.end().await;
    }

    fn change_list_mode(&mut self, mode: ListMode) {
        match mode {
            ListMode::Select => {
                self.geometry.disable_search();
                self.geometry.disable_command();
                match self.mode {
                    ListMode::Command => self.swap_list_index_to_normal_from_command(),
                    ListMode::Search => self.swap_list_index_to_normal_from_search(),
                    ListMode::AfterSearch => self.swap_list_index_to_normal_from_aftersearch(),
                    _ => {}
                }
                self.reset_querry()
            }
            ListMode::Search => {
                self.geometry.enable_search();
                self.swap_list_index_to_search_mode();
            }
            ListMode::Command => {
                self.geometry.enable_command();
            }
            _ => {}
        }
        self.mode = mode;
    }

    async fn play_after_search(&mut self) {
        self.play_selected_music().await;
        self.reset_querry();
        self.change_list_mode(ListMode::Select);
    }

    fn quit(&self) -> Result<bool, std::io::Error> {
        Ok(true)
    }

    fn anticipate(&mut self, arg: char) {
        self.anticipation_mode = AncitipationMode::Char(arg);
    }

    // TODO: improve this to download the file respecting the lrc standard
    fn download_lyrics(&self) {
        log(
            &format!("{:#?}", self.state.metadata().lyrics),
            &format!("{}.lrc", self.state.metadata().title),
        )
        .unwrap();
    }

    // TODO: add a way to handle time_is_correct per line instead
    fn render_lyrics(&mut self, frame: &mut Frame) {
        if self.displayed_lyrics.displayable {
            match self.geometry.lyrics_geo() {
                Some(geo) => {
                    let mut lyrics = self.state.metadata().lyrics;
                    let mut lines = vec![];
                    let widths = [Constraint::Fill(4)];

                    for line in lyrics.lines.iter_mut() {
                        lines.push(Row::new(vec![line.content.as_str()]))
                    }

                    let style = match self.region {
                        Region::Lyrics => {
                            Style::new().fg(self.style.lyrics_style.active_region_color)
                        }
                        _ => Style::new().fg(self.style.lyrics_style.passive_region_color),
                    };

                    let block = match self.region {
                        Region::Lyrics => Block::default()
                            .title("Lyrics")
                            .borders(Borders::ALL)
                            .fg(self.style.lyrics_style.active_region_color),
                        _ => Block::default()
                            .title("Lyrics")
                            .borders(Borders::ALL)
                            .fg(self.style.lyrics_style.passive_region_color),
                    };

                    let table = Table::new(lines, widths)
                        .block(block)
                        .style(style)
                        .highlight_symbol(self.style.lyrics_style.selector.as_str())
                        .highlight_style(
                            Style::new()
                                .add_modifier(Modifier::REVERSED)
                                .fg(self.style.list_style.hilight_color),
                        );
                    frame.render_stateful_widget(table, geo, &mut self.displayed_lyrics.state)
                }
                None => {}
            }
        }
    }

    /// updates the lyrics state, by changing the displayble property
    /// according to the music metadata
    async fn update_lyrics_state(&mut self) {
        self.derive_lyrics_state().await;
    }

    async fn derive_lyrics(&mut self) {
        self.displayed_lyrics.lyrics = self.state.metadata().lyrics;
    }

    // TODO: make moving between lyrics as natural as in spotify
    fn select_next_lyrics_line(&mut self) {
        self.displayed_lyrics.state.select_next();
    }

    // TODO: make moving between lyrics as natural as in spotify
    fn select_previous_lyrics_line(&mut self) {
        self.displayed_lyrics.state.select_previous();
    }

    fn select_lyrics_region(&mut self) {
        self.region = Region::Lyrics;
    }

    fn select_region(&mut self, region: Region) {
        self.region = region;
    }

    fn select_playlist_region(&mut self) {
        self.region = Region::Playlist;
    }

    fn toggle_lyrics(&mut self) {
        self.disable_playlists();
        self.displayed_lyrics.displayable = !self.displayed_lyrics.displayable;
        self.geometry.toggle_lyrics();
    }

    fn update_geometry(&mut self, frame: &mut Frame) {
        self.geometry.set_frame(frame.area());
        self.geometry.calculate_gemoetry();
    }

    async fn play_music_from_lyrics(&mut self) {
        match self.displayed_lyrics.state.selected() {
            Some(i) => {
                if self.state.metadata().lyrics.time_is_correct {
                    if let Some(target_time) = self.displayed_lyrics.lyrics.lines.get(i) {
                        self.goto_xs(target_time.timestamp).await;
                    }
                }
            }
            None => {}
        }
    }

    fn scroll_lyrics_down(&mut self) {
        let quesize = self.displayed_lyrics.lyrics.lines.len();
        let selected_index = self.displayed_lyrics.state.selected_mut().unwrap_or(0);

        if selected_index == quesize - 1 {
            self.displayed_lyrics.state.select_first();
        } else {
            self.displayed_lyrics.state.select(Some(selected_index + 7));
        }
    }

    fn scroll_lyrics_up(&mut self) {
        let selected_index = self.displayed_lyrics.state.selected().unwrap_or(0);

        if selected_index == 0 {
            self.displayed_lyrics.state.select_last();
        } else {
            self.displayed_lyrics.state.select(Some(selected_index - 7));
        }
    }

    async fn sort(&mut self) {
        self.state.sort(self.displayed_sort).await;
        self.fetch_playlist_data().await;
    }

    async fn repeat(&mut self) {
        self.state.repat(self.displayed_repeat).await;
        self.fetch_repeat().await;
    }

    async fn update_played_duration(&mut self) {
        tokio::select! {
            _ = self.internal_clock.tick() => {
                let time = Box::pin(self.state.proxy.played_duration()).await.unwrap();
                self.state.batch.played_duration =
                    Duration::from_secs_f32(time);
            },
            _ = futures::future::ready(()) => {
                return
            },
        }
    }

    fn cycle(&mut self) {
        match self.action {
            PowerActions::Sort => match self.displayed_sort {
                Sort::ByTitleAscending => self.displayed_sort = Sort::ByTitleDescending,
                Sort::ByTitleDescending => self.displayed_sort = Sort::ByDurationAscending,
                Sort::ByDurationAscending => self.displayed_sort = Sort::ByDurationDescending,
                Sort::ByDurationDescending => self.displayed_sort = Sort::ArtistAscending,
                Sort::ArtistAscending => self.displayed_sort = Sort::ArtistDescending,
                Sort::ArtistDescending => self.displayed_sort = Sort::Shuffle,
                Sort::Shuffle => self.displayed_sort = Sort::ByTitleAscending,
            },
            // ThisMusic -> AllMusics -> Dont
            PowerActions::Repeat => match self.displayed_repeat {
                Repeat::SameMusic => self.displayed_repeat = Repeat::AllMusics,
                Repeat::AllMusics => self.displayed_repeat = Repeat::Dont,
                Repeat::Dont => self.displayed_repeat = Repeat::SameMusic,
            },
            _ => {}
        }
    }

    async fn fetch_repeat(&mut self) {
        self.state.batch.repeat = self.state.proxy.get_repeat().await.unwrap_or_default();
    }

    fn swap_list_index_to_normal_from_command(&mut self) {
        self.music_list.switch_selected_buffer = self.music_list.selected;
        self.music_list.switch_selected_buffer = 0
    }

    fn swap_list_index_to_normal_from_aftersearch(&mut self) {
        self.music_list.switch_selected_buffer = self.music_list.selected;
        self.music_list.selected = 0;
        self.music_list.switch_selected_buffer = 0
    }

    fn swap_list_index_to_normal_from_search(&mut self) {
        self.music_list.switch_selected_buffer = self.music_list.selected;
        self.music_list.selected = 0;
        self.music_list.switch_selected_buffer = 0
    }

    fn swap_list_index_to_search_mode(&mut self) {
        self.music_list.selected = self.music_list.switch_selected_buffer;
        self.music_list.switch_selected_buffer = 0
    }

    fn disable_playlists(&mut self) {
        self.displayed_playlists.displayable = false;
        self.geometry.disable_playlists();
    }

    fn disable_lyrics(&mut self) {
        self.displayed_lyrics.displayable = false;
        self.geometry.disable_lyrics();
    }

    fn toggle_playlists(&mut self) {
        self.disable_lyrics();
        self.displayed_playlists.displayable = !self.displayed_playlists.displayable;
        self.geometry.toggle_playlists();
    }

    fn render_playlists(&mut self, frame: &mut Frame<'_>) {
        if self.displayed_playlists.displayable {
            match self.geometry.playlists_geo() {
                Some(geo) => {
                    let names = self.state.playlist_names();

                    let names = names
                        .iter()
                        .map(|v| Row::new(vec![v.as_str()]))
                        .collect::<Vec<Row<'_>>>();

                    let widths = [Constraint::Fill(4)];

                    let style = match self.region {
                        Region::Lyrics => {
                            Style::new().fg(self.style.playlists_style.active_region_color)
                        }
                        _ => Style::new().fg(self.style.playlists_style.passive_region_color),
                    };

                    let block = match self.region {
                        Region::Playlist => Block::default()
                            .title("Playlists")
                            .borders(Borders::ALL)
                            .fg(self.style.playlists_style.active_region_color),
                        _ => Block::default()
                            .title("Playlists")
                            .borders(Borders::ALL)
                            .fg(self.style.playlists_style.passive_region_color),
                    };

                    let table = Table::new(names, widths)
                        .block(block)
                        .style(style)
                        .highlight_symbol(self.style.playlists_style.selector.as_str())
                        .highlight_style(
                            Style::new()
                                .add_modifier(Modifier::REVERSED)
                                .fg(self.style.list_style.hilight_color),
                        );
                    frame.render_stateful_widget(table, geo, &mut self.displayed_playlists.state)
                }
                None => {}
            }
        }
    }

    fn select_next_playlist_name(&mut self) {
        self.displayed_playlists.state.select_next();
    }

    fn select_previous_playlist_name(&mut self) {
        self.displayed_playlists.state.select_previous();
    }

    async fn switch_to_selected_playlist(&mut self) {
        if let Some(index) = self.displayed_playlists.state.selected() {
            let titles = self.state.playlist_names();
            let title = titles.get(index).unwrap();
            let _ = self.state.proxy.use_playlist(title.clone()).await;
        }
    }

    fn render_commands(&self, frame: &mut Frame<'_>) {
        if matches!(self.mode, ListMode::Command) {
            if let Some(geo) = self.geometry.command_geo() {
                frame.render_widget(
                    Paragraph::new(self.command_bufr.value())
                        .block(Block::bordered().title("Command")),
                    geo,
                );
            }
        }
    }

    async fn run_command(&mut self) -> std::io::Result<bool> {
        let command = self.command_bufr.to_string();
        let mut commands: Vec<String> = command.split_whitespace().map(|v| v.to_string()).collect();
        if commands.len() > 0 {
            let command_name = commands.remove(0);
            let command_args = commands;
            match CommandName::from_str(&command_name) {
                Ok(name) => return self.preform(CommandContext::new(name, command_args)).await,
                Err(e) => todo!("{:?}", e),
            }
        }
        Ok(false)
    }

    fn render_search(&self, frame: &mut Frame<'_>) {
        if matches!(self.mode, ListMode::Search | ListMode::AfterSearch) {
            if let Some(geo) = self.geometry.search_geo() {
                frame.render_widget(
                    Paragraph::new(self.search_bufr.value())
                        .block(Block::bordered().title("Querry")),
                    geo,
                );
            }
        }
    }

    async fn preform(&mut self, context: CommandContext) -> Result<bool, io::Error> {
        let args = context.args;
        if args.len() > context.name.number_of_required_args() {
            return Err(std::io::Error::other("Too many arguments"));
        } else if args.len() < context.name.number_of_required_args() {
            return Err(std::io::Error::other("Few arguments"));
        }
        match context.name {
            CommandName::Play => {
                // guarded by upper if check
                let direction = args.get(0).unwrap();
                match direction.as_str() {
                    "previous" => self.play_preivous().await,
                    "next" => self.play_next().await,
                    _ => {}
                }
            }
            CommandName::Move => {
                // guarded by upper if check
                let direction = args.get(0).unwrap();
                match direction.as_str() {
                    "up" => self.list_up(),
                    "down" => self.list_down(),
                    _ => {}
                }
            }
            CommandName::AddToPlaylist => {
                let name = args.get(0).unwrap();
                self.add_selected_to_playlist(name).await;
            }
            CommandName::Quit => return Ok(true),
            CommandName::CreatePlaylist => {
                let name = args.get(0).unwrap();
                self.crate_playlist(name).await;
            }
            CommandName::ToggleMute => self.toggle_mute().await,
            CommandName::Pause => self.state.pause_stream(),
            CommandName::Resume => self.state.resume_stream(),
            CommandName::Reload => self.reload().await,
            CommandName::RenamePlaylist => {
                let old = args.get(0).unwrap();
                let new = args.get(1).unwrap();
                self.rename_playlist(old, new).await;
            }
            CommandName::RemovePlaylist => {
                let name = args.get(0).unwrap();
                self.remove_playlist(name).await;
            }
        }
        Ok(false)
    }

    async fn add_selected_to_playlist(&self, destination: &String) {
        let index = self
            .music_list
            .musics
            .get(self.music_list.selected)
            .unwrap()
            .index;

        let source = self.state.proxy.get_playlist_name().await.unwrap();

        self.state
            .proxy
            .save_to_playlist(index, source.clone(), destination.clone())
            .await
            .unwrap();
    }

    async fn crate_playlist(&self, name: &String) {
        self.state
            .proxy
            .create_playlist(name.clone())
            .await
            .unwrap();
    }

    async fn rename_playlist(&self, old: &str, new: &str) {
        self.state.proxy.rename_playlist(old, new).await.unwrap();
    }

    async fn remove_playlist(&self, name: &str) {
        self.state.proxy.remove_playlist(name).await.unwrap();
    }

    async fn reload(&self) {
        self.state.proxy.reload_config().await.unwrap();
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct DisplayedLyrics {
    /// Used to disable lyrics, feched from config or changed
    /// via a keybind
    displayable: bool,
    /// State/Index of the lyrics list pointer
    state: TableState,
    /// Lyrics
    lyrics: Lyrics,
    /// Used to sync current lyrics line with the audio
    pub last_active_interaction: Instant,
}

#[derive(Debug, Clone)]
pub struct DisplayedPlaylists {
    /// Used to toggle playlists view
    displayable: bool,
    /// State/Index of the playlist list pointer
    state: TableState,
}

impl Default for DisplayedPlaylists {
    fn default() -> Self {
        Self {
            displayable: false,
            state: TableState::default(),
        }
    }
}

impl Default for DisplayedLyrics {
    fn default() -> Self {
        Self {
            displayable: false,
            state: TableState::new(),
            lyrics: Lyrics::default(),
            last_active_interaction: Instant::now().checked_add(Duration::from_secs(1)).unwrap(),
        }
    }
}

#[derive(Default, Debug)]
pub struct Musics {
    /// weather we should display lyrics or not
    pub display_genre: bool,
    /// displayed music list
    /// TODO: change this to a HashMap
    pub musics: Vec<Music>,
    /// index of the currently *selected* music in the displayed music list
    pub selected: usize,
    /// Buffer used to switch between selected index values
    /// Holds the previously selected index
    /// search/aftersearch <===> normal
    pub switch_selected_buffer: usize,
    pub state: TableState,
    /// full music list (not filtered)
    pub unfiltered_music_list: Vec<Music>,
    /// index of the currently playing song in the full music list
    playing_index: usize,
    pub setp_szie: usize,
}

impl Musics {
    /// search the music list
    pub fn search(&mut self, search_bufr: String) {
        let n = 20;
        let paire = search_bufr.split_once(":");
        match paire {
            Some((left, right)) => {
                let criteria = left;
                if criteria == "genre" {
                    self.musics = fuzzy_search::fuzzy_search_music_grene_best_n(
                        right.trim(),
                        &self.unfiltered_music_list,
                        n,
                    );
                } else if criteria == "duration" {
                    // TODO: use duration instead of str
                    self.musics = fuzzy_search::fuzzy_search_music_grene_best_n(
                        right.trim(),
                        &self.unfiltered_music_list,
                        n,
                    );
                } else if criteria == "artist" {
                    self.musics = fuzzy_search::fuzzy_search_music_artist_best_n(
                        right.trim(),
                        &self.unfiltered_music_list,
                        n,
                    );
                }
            }
            None => {
                self.musics = fuzzy_search::fuzzy_search_music_titles_best_n(
                    &search_bufr,
                    &self.unfiltered_music_list,
                    n,
                );
            }
        }
    }

    pub fn reset_search(&mut self) {
        self.musics = self.unfiltered_music_list.clone();
    }
}

#[derive(Default)]
pub struct PowerBar {
    current_timer: Duration,
}

#[derive(Debug)]
pub enum PowerActions {
    TogglePlay,
    ForwardSkip,
    BackwardSkip,
    Repeat,
    Sort,
    Stop,
}

#[derive(Deserialize, Serialize, Clone, Copy)]
enum Kmodifier {
    SHIFT,
    CONTROL,
    ALT,
    SUPER,
    HYPER,
    META,
    NONE,
}

impl From<KeyModifiers> for Kmodifier {
    fn from(value: KeyModifiers) -> Self {
        match value {
            KeyModifiers::SHIFT => Kmodifier::SHIFT,
            KeyModifiers::CONTROL => Kmodifier::CONTROL,
            KeyModifiers::ALT => Kmodifier::ALT,
            KeyModifiers::SUPER => Kmodifier::SUPER,
            KeyModifiers::HYPER => Kmodifier::HYPER,
            KeyModifiers::META => Kmodifier::META,
            KeyModifiers::NONE => Kmodifier::NONE,
            _ => todo!(),
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Copy)]
enum KCode {
    /// Backspace key.
    Backspace,
    /// Enter key.
    Enter,
    /// Left arrow key.
    Left,
    /// Right arrow key.
    Right,
    /// Up arrow key.
    Up,
    /// Down arrow key.
    Down,
    /// Home key.
    Home,
    /// End key.
    End,
    /// Page up key.
    PageUp,
    /// Page down key.
    PageDown,
    /// Tab key.
    Tab,
    /// Shift + Tab key.
    BackTab,
    /// Delete key.
    Delete,
    /// Insert key.
    Insert,
    /// F key.
    ///
    /// `KeyCode::F(1)` represents F1 key, etc.
    F(u8),
    /// A character.
    ///
    /// `KeyCode::Char('c')` represents `c` character, etc.
    Char(char),
    /// Null.
    Null,
    /// Escape key.
    Esc,
    /// Caps Lock key.
    ///
    /// **Note:** this key can only be read if
    /// [`KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES`] has been enabled with
    /// [`PushKeyboardEnhancementFlags`].
    CapsLock,
    /// Scroll Lock key.
    ///
    /// **Note:** this key can only be read if
    /// [`KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES`] has been enabled with
    /// [`PushKeyboardEnhancementFlags`].
    ScrollLock,
    /// Num Lock key.
    ///
    /// **Note:** this key can only be read if
    /// [`KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES`] has been enabled with
    /// [`PushKeyboardEnhancementFlags`].
    NumLock,
    /// Print Screen key.
    ///
    /// **Note:** this key can only be read if
    /// [`KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES`] has been enabled with
    /// [`PushKeyboardEnhancementFlags`].
    PrintScreen,
    /// Pause key.
    ///
    /// **Note:** this key can only be read if
    /// [`KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES`] has been enabled with
    /// [`PushKeyboardEnhancementFlags`].
    Pause,
    /// Menu key.
    ///
    /// **Note:** this key can only be read if
    /// [`KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES`] has been enabled with
    /// [`PushKeyboardEnhancementFlags`].
    Menu,
    /// The "Begin" key (often mapped to the 5 key when Num Lock is turned on).
    ///
    /// **Note:** this key can only be read if
    /// [`KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES`] has been enabled with
    /// [`PushKeyboardEnhancementFlags`].
    KeypadBegin,
    /// A media key.
    ///
    /// **Note:** these keys can only be read if
    /// [`KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES`] has been enabled with
    /// [`PushKeyboardEnhancementFlags`].
    Media(MediaKey),
    /// A modifier key.
    ///
    /// **Note:** these keys can only be read if **both**
    /// [`KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES`] and
    /// [`KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES`] have been enabled with
    /// [`PushKeyboardEnhancementFlags`].
    Modifier(ModifierKey),
}

#[derive(Deserialize, Serialize, Clone, Copy)]
enum MediaKey {
    /// Play media key.
    Play,
    /// Pause media key.
    Pause,
    /// Play/Pause media key.
    PlayPause,
    /// Reverse media key.
    Reverse,
    /// Stop media key.
    Stop,
    /// Fast-forward media key.
    FastForward,
    /// Rewind media key.
    Rewind,
    /// Next-track media key.
    TrackNext,
    /// Previous-track media key.
    TrackPrevious,
    /// Record media key.
    Record,
    /// Lower-volume media key.
    LowerVolume,
    /// Raise-volume media key.
    RaiseVolume,
    /// Mute media key.
    MuteVolume,
}

#[derive(Deserialize, Serialize, Clone, Copy)]
pub enum ModifierKey {
    /// Left Shift key.
    LeftShift,
    /// Left Control key.
    LeftControl,
    /// Left Alt key.
    LeftAlt,
    /// Left Super key.
    LeftSuper,
    /// Left Hyper key.
    LeftHyper,
    /// Left Meta key.
    LeftMeta,
    /// Right Shift key.
    RightShift,
    /// Right Control key.
    RightControl,
    /// Right Alt key.
    RightAlt,
    /// Right Super key.
    RightSuper,
    /// Right Hyper key.
    RightHyper,
    /// Right Meta key.
    RightMeta,
    /// Iso Level3 Shift key.
    IsoLevel3Shift,
    /// Iso Level5 Shift key.
    IsoLevel5Shift,
}

impl From<KeyCode> for KCode {
    fn from(value: KeyCode) -> Self {
        match value {
            KeyCode::Backspace => KCode::Backspace,
            KeyCode::Enter => KCode::Enter,
            KeyCode::Left => KCode::Left,
            KeyCode::Right => KCode::Right,
            KeyCode::Up => KCode::Up,
            KeyCode::Down => KCode::Down,
            KeyCode::Home => KCode::Home,
            KeyCode::End => KCode::End,
            KeyCode::PageUp => KCode::PageUp,
            KeyCode::PageDown => KCode::PageDown,
            KeyCode::Tab => KCode::Tab,
            KeyCode::BackTab => KCode::BackTab,
            KeyCode::Delete => KCode::Delete,
            KeyCode::Insert => KCode::Insert,
            KeyCode::F(k) => KCode::F(k),
            KeyCode::Char(c) => KCode::Char(c),
            KeyCode::Null => KCode::Null,
            KeyCode::Esc => KCode::Esc,
            KeyCode::CapsLock => KCode::CapsLock,
            KeyCode::ScrollLock => KCode::ScrollLock,
            KeyCode::NumLock => KCode::Null,
            KeyCode::PrintScreen => KCode::PrintScreen,
            KeyCode::Pause => KCode::Pause,
            KeyCode::Menu => KCode::Menu,
            KeyCode::KeypadBegin => KCode::KeypadBegin,
            KeyCode::Media(_) => todo!(),
            KeyCode::Modifier(_) => todo!(),
        }
    }
}

impl KCode {}

#[derive(Deserialize, Serialize)]
struct Event {
    modifier: Kmodifier,
    code: KCode,
    region: Region,
    list_mode: Option<ListMode>,
}

impl Event {
    fn modifier(&self) -> Kmodifier {
        self.modifier.clone()
    }

    fn code(&self) -> KCode {
        self.code.clone()
    }

    fn region(&self) -> Region {
        self.region.clone()
    }
}

impl UserData for Event {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("modifier", |_lua, this, ()| {
            Ok(_lua.to_value(&this.modifier()))
        });
        methods.add_method("code", |_lua, this, ()| Ok(_lua.to_value(&this.code())));
        methods.add_method("region", |_lua, this, ()| Ok(_lua.to_value(&this.region())));
    }
}

impl Event {
    fn new(modifier: Kmodifier, code: KCode, region: Region, list_mode: Option<ListMode>) -> Self {
        Self {
            modifier,
            code,
            region,
            list_mode,
        }
    }
}
