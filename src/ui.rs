use std::{time::Duration, path::PathBuf};
use async_std::io;
use crossterm::event::{self, KeyCode, KeyEvent, KeyModifiers};
use rand::Rng;
use async_std::task::block_on;
use ratatui::{prelude::*, widgets::*, style::Stylize};
use lofty;
use lofty::{
    file::{AudioFile, TaggedFileExt},
    tag::Accessor
};
use serde::{Deserialize, Serialize};

use crate::states::{State, Status};
use crate::utils::StringFeatures;
use crate::{fuzzy_search, Config, ServerProxy, Sorting};

#[derive(Default, Debug)]
pub enum ListMode {
    Search,
    #[default]
    Select,
    AfterSearch
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

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
/// Represents the music repeat option
pub enum Repeat {
    /// repeat the currently playing music
    ThisMusic,
    /// cyclye through all the playlist
    AllMusics,
    #[default]
    /// stop after the currently playing music
    Dont
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
    /// last used action
    just_preformed_action: Action,
    /// search buffer, used to search through the musics list
    search_bufr: String,
    /// helps reading a combination of keys like `gg`
    pub anticipation_mode: AncitipationMode,
    /// what to repeat <ThisMusic, AllMusics, None>
    repeat: Repeat,
    /// order list <Yes, No>
    order: Sorting,
    state: State<'a>,
}

/// all the possible actions with the play button
enum Action {
    Play, Pause, Resume,
}

#[derive(Clone)]
/// all the displayed region
pub enum Region {
    List, Action, Seeker, Volume
}

impl Default for Region {
    fn default() -> Self {
        Region::List
    }
}


impl ListMode {
    pub fn handle_search<'a>(ui: &mut UI<'a>, key: &KeyEvent) {
        if key.kind == event::KeyEventKind::Press && key.modifiers == KeyModifiers::NONE {
            match key.code {
                KeyCode::Char(c) => {
                    block_on(ui.register_querry(c));
                },
                KeyCode::Backspace => {
                    ui.delete_char_querry()
                },
                KeyCode::Enter => {
                    ui.play_selected_music();
                    ui.reset_querry();
                    ui.mode = ListMode::Select;
                },
                KeyCode::Esc => {
                    ui.mode = ListMode::AfterSearch;
                },
                _ => {},
            }
        }
    }

    pub fn handle_after_search<'a>(ui: &mut UI<'a>, key: &KeyEvent) -> io::Result<bool> {
        if key.kind == event::KeyEventKind::Press  {
            match key.modifiers {
                KeyModifiers::NONE => {
                    match key.code {
                        KeyCode::Char(c) => {
                            match c {
                                'j' => ui.list_down(),
                                'k' => ui.list_up(),
                                '/' => ui.change_list_mode(ListMode::Search), 
                                'q' => ui.change_list_mode(ListMode::Select),
                                ' ' => ui.play_after_search(),
                                _ => {}
                            }
                        },
                        KeyCode::Enter => ui.play_after_search(),
                        KeyCode::Esc => ui.change_list_mode(ListMode::Select),
                        _=>{}
                    }
                },
                KeyModifiers::ALT => {
                    match key.code {
                        KeyCode::Char(c) => {
                            match c {
                                'j' => ui.list_down(),
                                'k' => ui.list_up(),
                                '/' => ui.change_list_mode(ListMode::Search), 
                                _ => {}
                            }
                        },
                        _ => {}
                    }
                },
                _ => {}
            }
        }
        Ok(false)
    }

    pub fn handle_select<'a>(ui: &mut UI<'a>, key: &KeyEvent) -> io::Result<bool>{
        if key.kind == event::KeyEventKind::Press  {
            match key.modifiers {
                // No modifiers
                KeyModifiers::NONE => {
                    match ui.anticipation_mode {
                        // No `g` key pressed beforehand
                        AncitipationMode::Normal => {
                            match key.code {
                                KeyCode::Enter => ui.play_selected_music(),
                                KeyCode::Char(c) => {
                                    match c {
                                        'j' => ui.list_down(),
                                        'k' => ui.list_up(),
                                        '/' => ui.change_list_mode(ListMode::Search), 
                                        ' ' => ui.play_selected_music(),
                                        's' => ui.goto_playing(),
                                        'g' => ui.anticipate('g'),
                                        'G' => ui.goto_bottom(),
                                        'q' => return Ok(true), 
                                        _ => {}
                                    }
                                },
                                _ => {}
                            }
                        },
                        AncitipationMode::Char(c) => {
                            if c == 'g' {
                                match key.code {
                                    KeyCode::Char(c) => {
                                        match c {
                                            'g' => ui.goto_top(),
                                            _ => {}
                                        }
                                    },
                                    _ => {}
                                }
                            }
                            ui.anticipation_mode = AncitipationMode::Normal;
                        },
                    }
                }
                KeyModifiers::ALT => {
                    if let KeyCode::Char(c) = key.code {
                        match c {
                            'k' => ui.select_bar_region(),
                            'j' => ui.select_action_region(),
                            _ => {}
                        }
                    }
                }
                KeyModifiers::SHIFT => {
                    match key.code {
                        KeyCode::Char(c) => {
                            match c {
                                'G' => ui.goto_bottom(),
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
                KeyModifiers::CONTROL => {
                    match key.code {
                        KeyCode::Char(c) => {
                            match c {
                                'd' => ui.scroll_list_down(),
                                'u' => ui.scroll_list_up(),
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
                _ =>{}
            }
        }
        Ok(false)
    }
}

impl Region {
    pub fn handle_global<'a>(ui: &mut UI<'a>, key: &KeyEvent) {
        match key.code {
            KeyCode::Char(c) => {
                if c == 'm' {
                    ui.toggle_mute();
                }else if c =='p' {
                    ui.toggle_play();
                }else if c == 'n' {
                    ui.play_next();
                } else if c == 'N' {
                    ui.play_preivous();
                }
            }
            _ => {}
        }
    }

    pub fn handle_list<'a>(ui: &mut UI<'a>, key: &KeyEvent) -> std::io::Result<bool>{
        match ui.mode {
            ListMode::Search => {
                ListMode::handle_search(ui, key);
            },
            ListMode::AfterSearch => {
                if ListMode::handle_after_search(ui, key).is_ok_and(|x| x == true ) {
                    return Ok(true)
                }
            }
            ListMode::Select => {
                if ListMode::handle_select(ui, key).is_ok_and(|x| x == true ) {
                    return Ok(true)
                }
            }
        }
        Ok(false)
    }

    pub fn handle_action<'a>(ui: &mut UI<'a>, key: &KeyEvent) -> std::io::Result<bool> {
        if key.kind == event::KeyEventKind::Press {
            match key.modifiers {
                KeyModifiers::NONE => {
                    match key.code {
                        KeyCode::Char(c) => {
                            match c {
                                'q' => return Ok(true),
                                'l' => ui.next_action(),
                                'h' => ui.previous_action(),
                                _ => {}
                            }
                        }
                        KeyCode::Enter => {
                            ui.preform_action();
                        },
                        _ => {}
                    }
                }
                KeyModifiers::ALT => {
                    match key.code {
                        KeyCode::Char(c) => {
                            match c {
                                'j' => ui.select_bar_region(),
                                'k' =>ui.select_list_region(),
                                _ => {}
                            }
                        }
                        KeyCode::Enter => {
                            ui.cycle_back();
                        },
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        Ok(false)
    }

    pub fn handle_seeker<'a>(ui: &mut UI<'a>, key: &KeyEvent) -> std::io::Result<bool> {
        if key.kind == event::KeyEventKind::Press {
            match key.modifiers {
                KeyModifiers::NONE => {
                    match key.code {
                        KeyCode::Char(c) => {
                            match c {
                                'l' => ui.next_5s(),
                                'h' => ui.previous_5s(),
                                'k' => ui.toggle_play(),
                                'q' => return Ok(true), 
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
                KeyModifiers::ALT => {
                    match key.code {
                        KeyCode::Char(c) => {
                            match c {
                                'j' => ui.select_list_region(),
                                'k' => ui.select_action_region(),
                                'y' | 'l' => ui.select_volume_region(),
                                _ => {}
                            }
                        }
                        _ => {}
                    }

                }
                _ => {}
            }
        }
        Ok(false)
    }

    pub fn handle_volume<'a>(ui: &mut UI<'a>, key: &KeyEvent) -> std::io::Result<bool> {
        if key.kind == event::KeyEventKind::Press {
            match key.modifiers {
                KeyModifiers::NONE => {
                    match key.code {
                        KeyCode::Char(c) => {
                            match c {
                                'k' | 'l' => ui.increase_volume(),
                                'h' | 'j' => ui.decrease_volume(),
                                'q' => return Ok(true),
                                _ => {}
                            }
                        }
                        _ => {}
                    }

                },
                KeyModifiers::ALT => {
                    match key.code {
                        KeyCode::Char(c) => {
                            match c {
                                'j' => ui.select_list_region(),
                                'k' => ui.select_action_region(),
                                 'l' | 'h' => ui.select_bar_region(),
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                },
                KeyModifiers::SHIFT=> {
                    match key.code {
                        KeyCode::Char(c) => {
                            match c {
                                'j' | 'h' => ui.increase_volume(),
                                'k' | 'l' => ui.decrease_volume(),
                                _ => {}
                            }
                        }
                        _ => {}
                    }

                }
                _ => {}
            }
        }
        Ok(false)
    }
}

#[derive(Default)]
pub struct UIStyle {
    list_style: ListStyle,
    action_style: ActionStyle,
    seeker_style: SeekerStyle,
    volume_style: VolumeStyle,
}

pub struct ListStyle {
    hilight_color: Color,
    active_region_color: Color,
    active_search_region_color: Color,
    active_after_search_region_color: Color,
    passive_region_color: Color,
    selector: String,
    playing_selector: String,
    playing_region_color: Color,
}

pub struct SeekerStyle {
    active_region_color: Color,
    passive_region_color: Color,
    fg_seeker_color: Color,
    bg_seeker_color: Color,
}

pub struct VolumeStyle {
    active_region_color: Color,
    passive_region_color: Color,
    fg_volume_color: Color,
    bg_volume_color: Color,
}

pub struct ActionStyle {
    hilight_color: Color,
    active_region_color: Color,
    passive_region_color: Color,
}

impl Default for ListStyle {
    fn default() -> Self {
        ListStyle {
            hilight_color: Color::default(),
            playing_region_color: Color::Gray,
            active_region_color: Color::Magenta,
            active_after_search_region_color: Color::Cyan,
            active_search_region_color: Color::DarkGray,
            passive_region_color: Color::default(),
            selector: String::from(">>"),
            playing_selector: String::from("*")
        }
    }
}

impl UIStyle {
    #[allow(dead_code)]
    pub fn new(
        list_style: ListStyle,
        action_style: ActionStyle,
        seeker_style: SeekerStyle,
        volume_style: VolumeStyle,
    ) -> Self {
        UIStyle {
            list_style, action_style, seeker_style, volume_style
        }
    }
}

impl Default for SeekerStyle {
    fn default() -> Self {
        SeekerStyle {
            active_region_color: Color::Magenta,
            passive_region_color: Color::default(),
            fg_seeker_color: Color::Gray,
            bg_seeker_color: Color::Black,
        }
    }
}

impl Default for VolumeStyle {
    fn default() -> Self {
        VolumeStyle {
            active_region_color: Color::Magenta,
            passive_region_color: Color::default(),
            fg_volume_color: Color::Gray,
            bg_volume_color: Color::Black,
        }
    }
}


impl Default for ActionStyle {
    fn default() -> Self {
        ActionStyle {
            hilight_color: Color::Yellow,
            active_region_color: Color::Magenta,
            passive_region_color: Color::default(),
        }
    }
}


impl<'a> UI<'a>{
    pub fn default(proxy: ServerProxy<'a>) -> Self {
        UI {
            power_bar: PowerBar::default(),
            music_list: Musics::default(),
            region: Region::default(),
            style : UIStyle::default(),
            action: PowerActions::BackwardSkip,
            just_preformed_action: Action::Pause,
            state: State::new(proxy),
            mode: ListMode::default() ,
            search_bufr: String::default(),
            anticipation_mode: AncitipationMode::default(),
            repeat: Repeat::default(),
            order: Sorting::default(),
        }
    }

    pub fn update_from_config(&mut self, config: &Config) {
        let config = config.clone();
        self.repeat = config.repeat.unwrap_or_default();
        self.order = config.sorting.unwrap_or_default();
    }

    pub fn musics(&mut self, musics: Musics) {
        self.power_bar.song_length = self.state.playing_music_duration();
        self.music_list = musics;
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

    pub async fn pause(&mut self) {
        self.state.async_pause().await;
        self.just_preformed_action = Action::Pause;
    }

    /// resumes the currently playing song
    pub async fn resume(&mut self) {
        self.state.async_resume().await;
        self.just_preformed_action = Action::Resume;
    }

    pub fn preform_action(&mut self) {
        // match the selected action
        match self.action {
            // we are on the toggle play botton
            PowerActions::TogglePlay => {
                self.toggle_play();
            },
            PowerActions::ForwardSkip => {
                self.play_next();
            },
            PowerActions::BackwardSkip => {
                self.play_preivous();
            },
            PowerActions::Stop => {
                self.stop();
            },
            PowerActions::Sort => {
                match self.order {
                    Sorting::ByTitleAscending => {
                        self.order = Sorting::ByTitleDescending;
                        self.music_list.sort(Some(self.order));
                    },
                    Sorting::ByTitleDescending => {
                        self.order = Sorting::ByDurationAscending;
                        self.music_list.sort(Some(self.order));
                    },
                    Sorting::ByDurationAscending => {
                        self.order = Sorting::ByDurationDescending;
                        self.music_list.sort(Some(self.order));
                    },
                    Sorting::ByDurationDescending => {
                        self.order = Sorting::Shuffle;
                        self.music_list.sort(Some(self.order));
                    },
                    Sorting::Shuffle => {
                        self.order = Sorting::ByTitleAscending;
                        self.music_list.sort(Some(self.order));
                    },
                }
            }
            PowerActions::Repeat => {
                match self.repeat {
                    Repeat::ThisMusic => {
                        self.repeat = Repeat::AllMusics;
                    },
                    Repeat::AllMusics => {
                        self.repeat = Repeat::Dont;
                    },
                    Repeat::Dont => {
                        self.repeat = Repeat::ThisMusic;
                    },
                }
            }
        }
    }

    #[allow(dead_code)]
    /// Plays the provided *Music*
    /// This should be used when the index of the music to play in the full music list is 
    /// known ahead of time 
    pub async fn o1_play_this_music(&mut self, toplay: &Music, playing_index: usize) {
        self.music_list.playing_index = playing_index;
        self.just_preformed_action = Action::Play;
        match self.state.status() {
            Status::Playing => {
                self.state.end();
                self.state.play(&toplay.path);
            },
            Status::Pausing => {
                self.state.end();
                self.state.resume();
                self.state.play(&toplay.path);
            },
            Status::Stopping => {
                self.state.play(&toplay.path);
                self.state.resume();
            },
        }
    }

    /// plays the provided *Music*
    pub async fn play_this_music(&mut self, toplay: &Music) {
        self.music_list.playing_index = 
            self.music_list.full_que.iter().position(|x| x == toplay).unwrap_or(0);
        self.just_preformed_action = Action::Play;
        match self.state.status() {
            Status::Playing => {
                self.state.end();
                self.state.play(&toplay.path);
            },
            Status::Pausing => {
                self.state.end();
                self.state.resume();
                self.state.play(&toplay.path);
            },
            Status::Stopping => {
                self.state.play(&toplay.path);
            },
        }
    }

    /// plays the *selected* song in the music list
    pub fn play_selected_music(&mut self) {
        let toplay = self.music_list.que.get(self.music_list.selected).unwrap().clone();
        block_on(self.play_this_music(&toplay));
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

    /// Renders the region of the music list
    pub fn render_list(&mut self, frame: &mut Frame) {
        let mut rows = vec![];
        let playing = self.state.playing_music();
        for music in self.music_list.que.iter() {
            let mut title = music.title.to_owned();
            let artist = music.artist.to_owned();
            let time = UI::duration_to_string(music.length.as_secs());
            if &playing == music {
                title.insert_str(0, self.style.list_style.playing_selector.as_str());
                rows.push(
                    Row::new(vec![title, artist, time])
                    .style(self.style.list_style.playing_region_color)
                )
            }else {
                rows.push(
                    Row::new(vec![title, artist, time])
                )
            }
        }

        let widths = [
            Constraint::Fill(4),
            Constraint::Fill(2),
            Constraint::Fill(1),
        ];

        let block = match self.region {
            Region::List => {
                match self.mode {
                    ListMode::Search => {
                        Block::default().title("Musics")
                            .borders(Borders::ALL)
                            .fg(self.style.list_style.active_search_region_color)
                    },
                    ListMode::AfterSearch => {
                        Block::default().title("Musics")
                            .borders(Borders::ALL)
                            .fg(self.style.list_style.active_after_search_region_color)
                    },
                    ListMode::Select => {
                        Block::default().title("Musics")
                            .borders(Borders::ALL)
                            .fg(self.style.list_style.active_region_color)
                    },
                }
            },
            _ => {
                Block::default().title("Musics")
                    .borders(Borders::ALL)
                    .fg(self.style.list_style.passive_region_color)
            }
        };

        let table = Table::new(rows, widths)
            .block(block)
            .highlight_style(
                Style::new().add_modifier(Modifier::REVERSED)
                .fg(self.style.list_style.hilight_color))
            .highlight_symbol(self.style.list_style.selector.as_str())
            .header(
                Row::new(vec!["Title", "artist", "duration"])
                .style(Style::new().bold().italic()));

        let mut size = frame.area();
        let mut search_size = frame.area();
        size.height = size.height  - 3 - 4;

        match self.mode {
            ListMode::Search | ListMode::AfterSearch => {
                search_size.y = size.height - 3;
                search_size.height = 3;
                size.height = size.height - 3;
                frame.render_widget(
                    Paragraph::new(
                        self.search_bufr.as_str()
                    ).block(Block::bordered().title("Querry")),
                    search_size
                );
            },
            ListMode::Select => {},
        }
        frame.render_stateful_widget(table, size, &mut self.music_list.state);
    }

    fn get_action_index(&self) -> usize {
        return match self.action {
            PowerActions::BackwardSkip => 0,
            PowerActions::TogglePlay => 1,
            PowerActions::ForwardSkip => 2,
            PowerActions::Stop => 3,
            PowerActions::Repeat => 4,
            PowerActions::Sort => 5,
        }
    }

    /// Converst timer in the u64 form to a string of form xx:yy
    pub fn duration_to_string(time: u64) -> String {
        let seconds = time % 60;
        let minities = time / 60;

        let sseconds = if seconds > 9 {
            format!("{}", seconds)
        }else {
            format!("0{}", seconds)
        };

        let sminutes = if minities > 9 {
            format!("{}", minities)
        }else {
            format!("0{}", minities)
        };

        return format!("{}:{}", sminutes, sseconds)
    }

    /// Returns the current playing timer as a string "xx:yy"
    fn timer(&mut self) -> String {
        format!("{}/{}",
            UI::duration_to_string(self.state.played_duration().as_secs()),
            UI::duration_to_string(self.state.playing_music().length.as_secs())
        )
    }

    /// Calculates the percentage of the seeker with respect with the full song length
    fn seeker_percent(&self) -> f64 {
        let current = self.state.played_duration().as_secs();
        let max = self.state.playing_music().length.as_secs();
        if max != 0 {
            return current as f64 / max as f64
        }else {
            return 0.0
        };
    }

    /// Seeks playing time forward by 5 seconds
    pub fn next_5s(&mut self) {
        let current = self.state.played_duration();
        let max = self.state.playing_music_duration();
        if current + Duration::from_secs(5) < max {
            self.power_bar.current_timer = current + Duration::from_secs(5);  
            self.state.seek(5.0);
        }else {
            self.play_next();
        }
    }

    /// Seeks playing time backward by 5 seconds
    pub fn previous_5s(&mut self) {
        let current = self.state.played_duration();
        match current.checked_sub(Duration::from_secs(5)) {
            Some(dur) => {
                self.power_bar.current_timer = dur;  
                self.state.seek(-5.0);
            },
            None => {
                self.play_preivous();
            },
        }
    }

    /// Renders the displayed time seeker
    pub fn render_seeker(&mut self, frame: &mut Frame) {
        let mut area = frame.area();
        area.y = area.height - 3;
        area.height = 3;
        area.width = (area.width / 5)*4 as u16;

        let selected_music = self.state.playing_music();
        let style = match self.region {
            Region::Seeker => {
                Style::new().fg(self.style.seeker_style.active_region_color)
            }
            _ => {
                Style::new().fg(self.style.seeker_style.passive_region_color)
            }
        };

        LineGauge::default()
            .block(Block::default().borders(Borders::ALL).title(
                    selected_music.title.to_owned()))
            .style(style)
            .unfilled_style(Style::default().fg(Color::Black))
            .line_set(symbols::line::THICK)
            .filled_style(Style::default()
                .fg(self.style.seeker_style.fg_seeker_color)
                .bg(self.style.seeker_style.bg_seeker_color)
                .add_modifier(Modifier::ITALIC))
            .label(self.timer())
            .ratio(self.seeker_percent())
            .render(area, frame.buffer_mut());
    }

    /// Renders the displayed volume slider
    pub fn render_volume(&mut self, frame: &mut Frame) {
        let mut area = frame.area();
        area.y = area.height - 3; // v
        area.x = (area.width / 5) * 4 + 1 as u16; // >

        area.width = area.width / 5 + 2 as u16;
        area.height = 3;

        let style = match self.region {
            Region::Volume=> {
                Style::new().fg(self.style.volume_style.active_region_color)
            }
            _ => {
                Style::new().fg(self.style.volume_style.passive_region_color)
            }
        };


        LineGauge::default()
            .block(Block::default().borders(Borders::ALL).title("Volume"))
            .style(style)
            .unfilled_style(Style::default().fg(Color::Black))
            .filled_style(
                Style::default()
                .fg(self.style.volume_style.fg_volume_color)
                .bg(self.style.volume_style.bg_volume_color)
                .add_modifier(Modifier::ITALIC))
            .ratio(self.state.volume())
            .render(area, frame.buffer_mut());
    }

    /// Renders the displayed actions
    pub fn render_actions(&mut self, frame: &mut Frame) {
        let mut area = frame.area();
        // - seeker hight - action height
        area.y = area.height - 3 - 4;
        area.height = 3;

        let style = match self.region {
            Region::Action => {
                Style::new().fg(self.style.action_style.active_region_color)
            }
            _ => {
                Style::new().fg(self.style.action_style.passive_region_color)
            }
        };

        let mut actions = vec![ "⏮", "⏵", "⏭", "⏹"];
        match self.repeat {
            Repeat::ThisMusic => {
                actions.push("RepeatMusic")
            },
            Repeat::AllMusics => {
                actions.push("RepeatList")
            },
            Repeat::Dont => {
                actions.push("NoRepeat")
            },
        }

        match self.order {
            Sorting::ByTitleAscending => {
                actions.push("TitleAscending")
            },
            Sorting::ByTitleDescending => {
                actions.push("TitleDescending")
            },
            Sorting::ByDurationAscending => {
                actions.push("DurationAscending")
            },
            Sorting::ByDurationDescending => {
                actions.push("DurationDescending")
            },
            Sorting::Shuffle => {
                actions.push("Shuffle")
            },
        }
        let status = self.state.status();
        match status {
            Status::Playing => {
                actions[1] = "⏸";
            },
            _ => {}
        }

        Tabs::new(actions)
            .block(Block::default().title("Actions").borders(Borders::ALL))
            .style(style)
            .highlight_style(
                Style::default()
                .fg(self.style.action_style.hilight_color)
                .underline_color(self.style.action_style.hilight_color))
            .select(self.get_action_index())
            .padding(" ", " ")
            .render(area, frame.buffer_mut());
    }


    /// Updates the music playing state
    pub fn update_state(&mut self) {
        block_on(self.state.async_batch_calls());
        self.handle_music_selection();
        self.handle_repeat();
    }

    /// Renders the displayed UI
    pub fn render(&mut self, frame: &mut Frame) {
        self.update_state();
        self.render_seeker(frame);
        self.render_list(frame);
        self.render_actions(frame);
        self.render_volume(frame);
    }

    /// Handles repeating music
    fn handle_repeat(&mut self) {
        if self.state.status() == Status::Playing {
            match self.repeat {
                Repeat::ThisMusic => {
                    if self.state.played_duration().as_secs() == self.state.playing_music_duration().as_secs() {
                        self.restart_playing_music();
                    }
                },
                Repeat::AllMusics => {
                    if self.state.played_duration().checked_add(Duration::from_millis(200)).unwrap().as_secs() >=
                        self.state.playing_music_duration().as_secs() 
                    {
                        self.play_next();
                    }
                },
                Repeat::Dont => {},
            }
        }

    }

    /// Handles music selection in the music list
    pub fn handle_music_selection(&mut self) {
        self.power_bar.song_name = self.state.playing_music().title;
        self.music_list.state.select(Some(self.music_list.selected));
    }

    /// Selectes the upper element in the music list (goes up by 1)
    pub fn list_up(&mut self) {
        let quesize = self.music_list.que.len();
        let selected_index = self.music_list.selected;
        if selected_index == 0 {
            self.music_list.selected = quesize - 1;
        }else {
            self.music_list.selected = self.music_list.selected - 1;
        }
    }

    /// Select the next element in the music list (goes down by 1)
    pub fn list_down(&mut self) {
        let quesize = self.music_list.que.len();
        let selected_index = self.music_list.selected;

        if selected_index == quesize - 1 {
            self.music_list.selected = 0;
        }else {
            self.music_list.selected = self.music_list.selected + 1;
        }
    }

    /// Appends the char to the existing search querry and search it
    pub async fn register_querry(&mut self, c: char) {
        self.search_bufr.push(c);
        self.music_list.search(self.search_bufr.to_owned());
    }

    /// Resets the search querry to an empty string
    pub fn reset_querry(&mut self) {
        self.music_list.reset_search();
    }

    /// Delets a character in the search querry and search the result
    pub fn delete_char_querry(&mut self) {
        self.search_bufr.pop();
        self.music_list.search(self.search_bufr.to_owned());
    }

    /// Selects the first element in the music list
    pub fn goto_top(&mut self) {
        self.music_list.selected = 0;
    }

    /// Selects the currently playing song in the music list
    pub fn goto_playing(&mut self) {
        self.music_list.selected = self.music_list.playing_index;
    }

    /// Selects the last element in the music list
    pub fn goto_bottom(&mut self) {
        self.music_list.selected = self.music_list.que.len() - 1;
    }

    /// Increases volume by 5
    pub fn increase_volume(&self) {
        let volume = self.state.volume() * 100.0;
        if volume < 101.0 {
            self.state.change_volume(volume + 1.0);
        }
    }

    /// Decreases volume by 5
    pub fn decrease_volume(&self) {
        let volume = self.state.volume() * 100.0;
        if  volume > 0.0 {
            self.state.change_volume(volume - 1.0);
        }
    }

    /// selects the volume reagion in the ui
    pub fn select_volume_region(&mut self) {
        self.region = Region::Volume
    }

    /// Plays the previous music in the list
    fn play_preivous(&mut self) {
        let previous = self.music_list.previous_song().to_owned();
        block_on(self.play_this_music(&previous));
    }

    #[allow(dead_code)]
    /// Plays the previous music in the list
    /// This should be used when the playing index of the full music list 
    /// is known ahead of time
    fn o1_play_preivous(&mut self, playing_index: usize) {
        let previous = self.music_list.previous_song().to_owned();
        block_on(self.o1_play_this_music(&previous, playing_index));
    }

    /// Plays the next song in the music list
    fn restart_playing_music(&mut self) {
        let playing = self.state.playing_music();
        block_on(self.play_this_music(&playing));
    }

    #[allow(dead_code)]
    /// Plays the next song in the music list
    /// This should be used when the playing index of the full music list 
    /// is known ahead of time
    fn o1_play_next(&mut self, playing_index: usize) {
        let next = self.music_list.next_song().to_owned();
        block_on(self.o1_play_this_music(&next, playing_index));
    }

    /// Plays the next song in the music list
    fn play_next(&mut self) {
        let next = self.music_list.next_song().to_owned();
        block_on(self.play_this_music(&next));
    }

    /// Toggls the playing music
    /// - if playing:
    ///     - pause
    /// - if pausing:
    ///     - resume
    /// - if stopping
    ///     - play
    pub fn toggle_play(&mut self) {
        match self.state.status() {
            // if we are playing we pause
            Status::Playing => {
                block_on(self.pause());
            },
            // if we are pausing we resume
            Status::Pausing => {
                block_on(self.resume());
            }
            // if we stopped we play
            Status::Stopping => {
                match self.state.playing_music().is_valid() {
                    Some(_) => {
                        block_on(self.play_this_music(&self.state.playing_music()));
                    }
                    None => {
                        self.play_selected_music();
                    }
                }
            }
        }
    }

    /// Restores playing state
    pub fn restore_state(&mut self) {
        self.music_list.playing_index = self.state.get_playing_index(&self.music_list.que);
    }

    /// Cycles through actions in this orders
    /// - sort action:
    ///     ByTitleAscending -> ByTitleDescending -> ByDurationAscending -> ByDurationDescending -> Shuffle -> ByTitleAscending
    /// - repeat action:
    ///     ThisMusic -> AllMusics -> Dont -> ThisMusic
    pub fn cycle_back(&mut self) {
        match self.action {
            PowerActions::Sort => {
                match self.order {
                    Sorting::ByTitleAscending => {
                        self.order = Sorting::Shuffle;
                        self.music_list.sort(Some(self.order));
                    },
                    Sorting::ByTitleDescending => {
                        self.order = Sorting::ByTitleAscending;
                        self.music_list.sort(Some(self.order));
                    },
                    Sorting::ByDurationAscending => {
                        self.order = Sorting::ByTitleDescending;
                        self.music_list.sort(Some(self.order));
                    },
                    Sorting::ByDurationDescending => {
                        self.order = Sorting::ByDurationAscending;
                        self.music_list.sort(Some(self.order));
                    },
                    Sorting::Shuffle => {
                        self.order = Sorting::ByDurationDescending;
                        self.music_list.sort(Some(self.order));
                    },
                }
            }
            // ThisMusic -> AllMusics -> Dont
            PowerActions::Repeat => {
                match self.repeat {
                    Repeat::ThisMusic => {
                        self.repeat = Repeat::Dont;
                    },
                    Repeat::AllMusics => {
                        self.repeat = Repeat::ThisMusic;
                    },
                    Repeat::Dont => {
                        self.repeat = Repeat::AllMusics;
                    },
                }
            }
            _ => {}
        }
    }

    /// Scrolls through the music list *up* by 7 units
    pub fn scroll_list_up(&mut self) {
        let quesize = self.music_list.que.len();
        let selected_index = self.music_list.selected;
        if selected_index == 0 {
            self.music_list.selected = quesize - 1;
        }else {
            self.music_list.selected = self.music_list.selected - 7;
        }
    }

    /// Scrolls through the music list *down* by 7 units
    pub fn scroll_list_down(&mut self) {
        let quesize = self.music_list.que.len();
        let selected_index = self.music_list.selected;

        if selected_index == quesize - 1 {
            self.music_list.selected = 0;
        }else {
            self.music_list.selected = self.music_list.selected + 7;
        }
    }

    pub fn toggle_mute(&self) {
        self.state.toggle_mute();
    }

    fn stop(&self) {
        self.state.end();
    }

    fn change_list_mode(&mut self, mode: ListMode) {
        match mode {
            ListMode::Select => self.reset_querry(),
            _ => {}
        }
        self.mode = mode;
    }

    fn play_after_search(&mut self) {
        self.play_selected_music();
        self.reset_querry();
        self.mode = ListMode::Select;
    }

    fn quit(&self) -> Result<bool, ()>{
        Ok(true)
    }

    fn anticipate(&mut self, arg: char) {
        self.anticipation_mode = AncitipationMode::Char(arg); 
    }
}

#[derive(PartialEq, Eq, Debug, Ord, PartialOrd, Clone)]
#[derive(serde::Serialize, serde::Deserialize, zbus::zvariant::Type)]
pub struct Music {
    pub title: String,
    pub length: Duration,
    pub path: PathBuf,
    pub artist: String,
    pub genre: String,
}

impl Music {
    pub fn new(
        title: String, path: PathBuf,
        length: Duration, artist: String, genre: String
        ) -> Self {
        Self {
            title, path, length, artist, genre
        }
    }

    pub fn default (path: PathBuf) -> Self {
        Self {
            title: Music::derive_title_from_path(&path),
            path,
            length: Duration::ZERO,
            artist: String::from("Unknown"),
            genre: String::from("Unknown"),
        }
    }

    pub fn unchecked_new(path: PathBuf) -> Self {
        let res = lofty::probe::Probe::open(&path);
        // can read file
        match res {
            // can read properties
            Ok(probe) => {
                if let Ok(mut x) = probe.read() {
                    let properties = x.properties();
                    let length = properties.duration();
                    // can read metadata
                    if let Some(tag) = x.primary_tag_mut() {
                        let mut title = tag.title().unwrap_or_default().to_string();
                        let mut artist = tag.artist().unwrap_or_default().to_string();
                        let mut genre = tag.genre().unwrap_or_default().to_string();
                        title.insert_if_empty("Unknown");
                        artist.insert_if_empty("Unknown");
                        genre.insert_if_empty("Unknown");
                        return Self {
                            title, length, path, artist, genre
                        }
                        // can't read metadata
                    }else {
                        return Self {
                            path, length,
                            title: String::from("Unknown"),
                            artist: String::from("Unknown"),
                            genre: String::from("Unknown"),
                        }
                    }
                    // can't read propertes
                }else {
                    return Self::default(path)
                }
            },
            // invalid file
            _ => {
                return Self::default(path)
            }
        };
    }

    pub fn simple_new(path: PathBuf) -> Option<Self> {
        // vaild file check
        if path.is_file() {
            let res = lofty::probe::Probe::open(&path);
            // can read file
            match res {
                // can read properties
                Ok(probe) => {
                    if let Ok(mut x) = probe.read() {
                        let properties = x.properties();
                        let length = properties.duration();
                        // can read metadata
                        if let Some(tag) = x.primary_tag_mut() {
                            let mut title = tag.title().unwrap_or_default().to_string();
                            let mut artist = tag.artist().unwrap_or_default().to_string();
                            let mut genre = tag.genre().unwrap_or_default().to_string();
                            title.insert_if_empty("Unknown");
                            artist.insert_if_empty("Unknown");
                            genre.insert_if_empty("Unknown");

                            return Some(Self {
                                title, length, path, artist, genre
                            })
                        // can't read metadata/tags
                        }else {
                            return Some(Self {
                                path, length,
                                title: String::from("Unknown"),
                                artist: String::from("Unknown"),
                                genre: String::from("Unknown"),
                            })
                        }
                    // can't read propertes
                    }else {
                        return None
                    }
                },
                // invalid file
                Err(e) => {
                    println!("File {:?} is not valid, if you think it's valid, try renaming it", path);
                    println!("Error: {}", e);
                    return None
                }
            };
        }
        println!("File {:?} is not valid", path);
        return None
    }

    fn is_valid(&self) -> Option<&str> {
        if self.path.exists() {
            return Some(&self.path.to_str()?)
        }
        None
    }

    fn derive_title_from_path(path: &PathBuf) -> String {
        match audiotags::Tag::new().read_from_path(path.to_owned()) {
            Ok(p) => p.title().unwrap_or(path.file_name().unwrap().to_str().unwrap_or("")).to_string(),
            Err(_) => String::from("Default"),
        }
    }
}

impl Default for Music {
    fn default() -> Self {
        Self {
            title: String::new(),
            length: Duration::ZERO,
            path: PathBuf::default(),
            artist: String::new(),
            genre: String::new()
        }
    }
}

#[derive(Default, Debug)]
pub struct Musics {
    /// displayed music list
    pub que: Vec<Music>,
    /// index of the currently *selected* music in the displayed music list
    pub selected: usize,
    pub state: TableState,
    /// full music list (not filtered)
    pub full_que: Vec<Music>,
    /// index of the currently playing song in the full music list
    playing_index: usize,
}

impl Musics {
    pub fn new(que: Vec<Music>) -> Self {
        Musics {
            full_que: que.clone(),
            que,
            selected: 0,
            state: TableState::default().with_selected(0),
            playing_index: 0,
        }
    }

    /// Returns a [`Music`] reference to the next song in playing quee
    fn next_song(&self) -> &Music {
        if self.playing_index + 1 < self.que.len() {
            return &self.que.get(self.playing_index + 1).unwrap()
        }else {
            return self.que.get(0).unwrap()
        }
    }

    /// Seturns a [`Music`] reference to the previous song in the palying quee
    fn previous_song(&self) -> &Music {
        match self.playing_index.checked_sub(1) {
            Some(number) => return &self.que.get(number).unwrap(),
            None => return self.que.get(self.que.len() - 1).unwrap(),
        }
    }

    /// Sorts the music list acoording to the Sorting enum
    pub fn sort(&mut self, sorting: Option<Sorting>) {
        match sorting {
            Some(o) => {
                self.full_que.sort_by(|x, y| {
                    match o {
                        Sorting::ByTitleAscending => {
                            if x.title > y.title {
                                return std::cmp::Ordering::Greater
                            }else if x.title < y.title {
                                return std::cmp::Ordering::Less
                            }
                            return std::cmp::Ordering::Equal
                        },
                        Sorting::ByTitleDescending => {
                            if x.title < y.title {
                                return std::cmp::Ordering::Greater
                            }
                            else if x.title > y.title {
                                return std::cmp::Ordering::Less
                            }
                            std::cmp::Ordering::Equal
                        }
                        Sorting::ByDurationAscending => {
                            if x.length > y.length {
                                return std::cmp::Ordering::Greater
                            }
                            else if x.length < y.length {
                                return std::cmp::Ordering::Less
                            }
                            std::cmp::Ordering::Equal
                        }
                        Sorting::ByDurationDescending => {
                            if x.length < y.length {
                                return std::cmp::Ordering::Greater
                            }
                            else if x.length > y.length {
                                return std::cmp::Ordering::Less
                            }
                            std::cmp::Ordering::Equal
                        }
                        Sorting::Shuffle => {
                            let num = rand::thread_rng().gen_range(0..3);
                            if num == 0 {
                                return std::cmp::Ordering::Greater
                            }else if num == 1 {
                                return std::cmp::Ordering::Less
                            }
                            std::cmp::Ordering::Equal
                        }
                    }
                });
            },
            None => {},
        }
        self.que = self.full_que.clone();
    }

    /// search the music list wit respect to the search buffer and 
    pub fn search(&mut self, search_bufr: String) {
        let n = 20;
        let paire = search_bufr.splitn(2, ":").collect::<Vec<&str>>();
        match paire.get(1) {
        // thus the xxxx: yyyy pattern matches
            Some(s) => {
                let criteria = paire.get(0).unwrap();
                if criteria == &"duration" {
                    let mut toreturn: Vec<Music> = vec![];
                    let results = fuzzy_search::fuzzy_search_music_duration_best_n(&s, &self.full_que, n);
                    for (music, _) in results {
                        toreturn.push(music);
                    }
                    self.que = toreturn;
                } else if criteria == &"artist" {
                    let mut toreturn: Vec<Music> = vec![];
                    let results = fuzzy_search::fuzzy_search_music_artist_best_n(&s, &self.full_que, n);
                    for (music, _) in results {
                        toreturn.push(music);
                    }
                    self.que = toreturn;
                }
            },
            // normal search querry
            None => {
                if search_bufr.is_empty() {
                    self.que = self.full_que.clone();
                }else {
                    let mut toreturn: Vec<Music> = vec![];
                    let results = fuzzy_search::fuzzy_search_music_titles_best_n(&search_bufr, &self.full_que, n);
                    for (music, _) in results {
                        toreturn.push(music);
                    }
                    self.que = toreturn;
                }

            },
        }
    }

    pub fn reset_search(&mut self) {
        let mut toreturn: Vec<Music> = vec![];
        let results = fuzzy_search::fuzzy_search_musics_by_title("", &self.que);
        for (music, _) in results {
            toreturn.push(music);
        }
        self.que = self.full_que.clone();
    }

}

#[derive(Default)]
pub struct PowerBar{
    song_length: Duration,
    current_timer: Duration,
    song_name: String,
}

impl From<Musics> for PowerBar {
    fn from(value: Musics) -> Self {
        let song = value.que.get(value.selected).unwrap();
        PowerBar{
            song_length: song.length,
            current_timer: Duration::ZERO,
            song_name: song.title.to_owned()
        }
    }
}

#[derive(Debug)]
pub enum PowerActions {
    TogglePlay,
    ForwardSkip,
    BackwardSkip,
    Repeat,
    Sort,
    Stop
}

pub mod test {
    #[allow(unused_imports)]
    use super::*;
    #[allow(unused_imports)]
    use crate::*;

    #[test]
    pub fn test_search() {
        let config = Config::parse_config("./config.toml");
        let mut musics = config.config.clone().unwrap().extract_music();
        musics.search("Hilo".to_string());
    }

    #[test]
    pub fn test_que() {
        let config = Config::parse_config("./config.toml");
        let mut musics = config.config.clone().unwrap().extract_music();
        musics.sort(config.config.clone().unwrap().sorting);
        assert!(musics.que == musics.full_que);
    }
}
