use std::{time::Duration, path::PathBuf};
use rand::{Rng, distributions::DistIter};
use async_std::task::block_on;
use ratatui::{prelude::*, widgets::*, style::Stylize};
use lofty;
use lofty::{
    file::{AudioFile, TaggedFileExt},
    tag::Accessor
};

use crate::utils::{log, StringFeatures};
use crate::{fuzzy_search, ListMode, ServerProxy, Sorting};

#[derive(Default)]
pub enum AncitipationMode {
    #[default]
    Normal,
    Char(char),
}

#[derive(Default)]
enum Repeat {
    ThisMusic,
    AllMusics,
    #[default]
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
    /// proxy that communicates with the dbus server
    pub proxy: ServerProxy<'a>,
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

#[derive(Default)]
pub struct UIStyle {
    list_style: ListStyle,
    action_style: ActionStyle,
    bar_style: SeekerStyle,
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
    seeker_color: Color,
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
    pub fn new(
        list_style: ListStyle,
        action_style: ActionStyle,
        bar_style: SeekerStyle
        ) -> Self {
        UIStyle {
            list_style, action_style, bar_style
        }
    }
}

impl Default for SeekerStyle {
    fn default() -> Self {
        SeekerStyle {
            active_region_color: Color::Magenta,
            passive_region_color: Color::default(),
            seeker_color: Color::Gray
        }
    }
}

pub(crate) struct ActionStyle {
    hilight_color: Color,
    active_region_color: Color,
    passive_region_color: Color,
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
            proxy,
            mode: ListMode::default() ,
            search_bufr: String::default(),
            anticipation_mode: AncitipationMode::default(),
            repeat: Repeat::default(),
            order: Sorting::default(),
        }
    }

    pub fn musics(&mut self, musics: Musics) {
        self.power_bar.song_length = self.playing_music().length;
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

    /// returns the music player status <Playing|Pausing|Stopping>
    pub fn status(&self) -> Status {
        let status = block_on(self.proxy.status())
            .unwrap();
        status.split_terminator("\n")
            .next().unwrap();
        // stopping or stopped
        if status.contains("Stop") {
            return Status::Stopping
        // Pausing or paused
        } else if status.contains("Paus") {
            return Status::Pausing
        }else {
            return Status::Playing
        }
    }

    /// returns the currently playing music
    pub fn playing_music(&self) -> Music {
        block_on(self.proxy.playing()).unwrap()
    }

    /// returns true if pausing is permissable, else false
    pub fn can_pause(&mut self) -> bool {
        // selected another song
        if self.power_bar.song_name != 
            self.music_list.que.get(self.music_list.selected).unwrap().title {
                return false
            }
        return true
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
        self.proxy.pause().await.unwrap();
        self.just_preformed_action = Action::Pause;
    }

    /// resumes the currently playing song
    pub async fn resume(&mut self) {
        block_on(self.resume_playing_song());
    }

    // TODO play_next and play_preivous could be improved by removing the search of music index
    // and only incrementing the already existing one
    pub async fn preform_action(&mut self) {
        // match the selected action
        match self.action {
            // we are on the toggle play botton
            PowerActions::TogglePlay => {
                self.toggle_play().await;
            },
            PowerActions::ForwardSkip => {
                self.play_next();
            },
            PowerActions::BackwardSkip => {
                self.play_preivous();
            },
            PowerActions::Stop => {
                self.proxy.end().await.unwrap();
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

    #[deprecated(note = "now called playing_music")]
    /// returns the currently plaing song
    /// __deprecated__
    pub fn playing_song(&self) -> Music {
        block_on(self.proxy.playing()).unwrap()
    }

    pub async fn resume_playing_song(&mut self) {
        match self.status() {
            Status::Pausing => {
                self.proxy.end().await.unwrap();
                self.proxy.resume().await.unwrap();
                self.just_preformed_action = Action::Play;
            },
            _ => {}
        }
    }

    /// plays the provided *Music*
    /// This should be used when the index of the music to play in the full music list is 
    /// known ahead of time 
    pub async fn o1_play_this_music(&mut self, toplay: &Music, playing_index: usize) {
        self.music_list.playing_index = playing_index;
        self.just_preformed_action = Action::Play;
        match self.status() {
            Status::Playing => {
                self.proxy.end().await.unwrap();
                self.proxy.play(toplay.path.as_os_str().to_str().unwrap().to_string()).await.unwrap();
            },
            Status::Pausing => {
                self.proxy.end().await.unwrap();
                self.proxy.resume().await.unwrap();
                self.proxy.play(toplay.path.as_os_str().to_str().unwrap().to_string()).await.unwrap();
            },
            Status::Stopping => {
                self.proxy.play(toplay.path.as_os_str().to_str().unwrap().to_string()).await.unwrap();
                self.proxy.resume().await.unwrap();
            },
        }
    }

    /// plays the provided *Music*
    pub async fn play_this_music(&mut self, toplay: &Music) {
        self.music_list.playing_index = 
            self.music_list.full_que.iter().position(|x| x == toplay).unwrap_or(0);
        self.just_preformed_action = Action::Play;
        match self.status() {
            Status::Playing => {
                self.proxy.end().await.unwrap();
                self.proxy.play(toplay.path.as_os_str().to_str().unwrap().to_string()).await.unwrap();
            },
            Status::Pausing => {
                self.proxy.end().await.unwrap();
                self.proxy.resume().await.unwrap();
                self.proxy.play(toplay.path.as_os_str().to_str().unwrap().to_string()).await.unwrap();
            },
            Status::Stopping => {
                self.proxy.play(toplay.path.as_os_str().to_str().unwrap().to_string()).await.unwrap();
                self.proxy.resume().await.unwrap();
            },
        }
    }

    /// plays the *selected* song in the music list
    pub async fn play_selected_music(&mut self) {
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
        // let playing = self.music_list.full_que.get(self.music_list.playing_index).unwrap();
        let playing = self.playing_music();
        // NOTE: clipping clipping artist or title won't work if their length is bigger then 
        // the available width, check `Side Effects` in the readme 
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

        let sseconds = if seconds > 10 {
            format!("{}", seconds)
        }else {
            format!("0{}", seconds)
        };

        let sminutes = if minities> 10 {
            format!("{}", minities)
        }else {
            format!("0{}", minities)
        };

        return format!("{sminutes}:{sseconds}")
    }

    /// returns the full duration of the currently playing song
    pub fn full_duration(&self) -> Duration{
        let time = block_on(self.proxy.timer()).unwrap();
        let s = time.splitn(2,'/').collect::<Vec<&str>>();
        let secs = s.get(0).unwrap().parse::<f64>().unwrap();
        Duration::from_secs_f64(secs)
    }

    /// returns the played time of the current song as a duration
    pub fn played_duration(&self) -> Duration{
        let time = block_on(self.proxy.timer()).unwrap();
        let s = time.splitn(2,'/').collect::<Vec<&str>>();
        let secs = s.get(1).unwrap().parse::<f64>().unwrap();
        Duration::from_secs_f64(secs)
    }

    /// Returns the current playing timer as a string "xx:yy"
    //TODO: migrate this to use the timer api call
    fn timer(&mut self) -> String {
        format!("{}/{}",
            UI::duration_to_string(self.power_bar.current_timer.as_secs()),
            UI::duration_to_string(self.playing_music().length.as_secs())
        )
    }

    /// returns the volume percentage as a fraction value between 0 and 1
    async fn volume_percent(&self) -> f64 {
        let data = self.proxy.show().await.unwrap();
        let lines = data.lines().collect::<Vec<&str>>();
        let volume_s = lines.get(1).unwrap().split_at(8).1;
        let volume = volume_s.parse::<f64>().unwrap();
        return volume
    }

    /// TODO: make this undependent of the update_stae method my making it sync
    /// calculates the percentage of the seeker with respect with the full song length
    fn seeker_percent(&self) -> u16 {
        let current = self.power_bar.current_timer.as_secs_f32();
        let max = self.power_bar.song_length.as_secs_f32();
        let ratio = if max != 0.0 {
            Some(current / max)
        }else {
            None
        };
        match ratio {
            Some(number) => {
                return (number * 100.0) as u16
            },
            None => return 0
        }
    }

    // TODO: Fix this wacky code, make it:
    // - play seeks 5s if song is playing
    // - plays the song and seeks 5s if song is not playing
    // TODO: use another method for getting the song length
    /// Seeks playing time forward by 5 seconds
    pub async fn next_5s(&mut self) {
        let current = self.played_duration();
        let max = self.full_duration();
        if current + Duration::from_secs(5) < max {
            self.power_bar.current_timer = current + Duration::from_secs(5);  
            self.proxy.seek(5.0).await.unwrap();
        }else {
                self.play_next();
        }
    }

    // TODO: Fix this wacky code, make it:
    // - play seeks 5s if song is playing
    // - plays the song and seeks 5s if song is not playing
    /// Seeks playing time backward by 5 seconds
    pub async fn previous_5s(&mut self) {
        let current = self.played_duration();
        match current.checked_sub(Duration::from_secs(5)) {
            Some(dur) => {
                self.power_bar.current_timer = dur;  
                self.proxy.seek(-5.0).await.unwrap();
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

        let selected_music = self.playing_music();
        let style = match self.region {
            Region::Seeker => {
                Style::new().fg(self.style.bar_style.active_region_color)
            }
            _ => {
                Style::new().fg(self.style.bar_style.passive_region_color)
            }
        };

        LineGauge::default()
            .block(Block::default().borders(Borders::ALL).title(
                    selected_music.title.to_owned()))
            .style(style)
            .unfilled_style(Style::default().fg(Color::Black))
            .line_set(symbols::line::THICK)
            .filled_style(Style::default()
                .fg(Color::White)
                .bg(Color::Black)
                .add_modifier(Modifier::ITALIC))
            .label(self.timer())
            .ratio(self.seeker_percent() as f64 / 100.0)
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
                Style::new().fg(self.style.bar_style.active_region_color)
            }
            _ => {
                Style::new().fg(self.style.bar_style.passive_region_color)
            }
        };


        LineGauge::default()
            .block(Block::default().borders(Borders::ALL).title("Volume"))
            .style(style)
            .unfilled_style(Style::default().fg(Color::Black))
            .filled_style(
                Style::default()
                .fg(Color::White)
                .bg(Color::Black)
                .add_modifier(Modifier::ITALIC))
            .ratio(block_on(self.volume_percent()))
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
        let status = block_on(self.proxy.status()).unwrap();
        let mut lines = status.lines();
        // check if the selected song is playing, if so inser the pause icon
        if lines.next().unwrap().to_lowercase().contains("playing") {
            actions[1] = "⏸";
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

    /// Renders the displayed UI
    pub fn render(&mut self, frame: &mut Frame) {
        self.update_state();
        self.render_seeker(frame);
        self.render_list(frame);
        self.render_actions(frame);
        self.render_volume(frame);
    }

    /// TODO: changed the calls inside the update state to a set of functions/methods
    /// _Suceptible to change!_ for now it updates some states
    /// like song the powerbar song length, current song,
    /// song name, and the slected element in the music list,
    /// this will be used later on to make all necessery dbus requests
    pub fn update_state(&mut self) {
        let time = block_on(self.proxy.timer()).unwrap();
        let times = time.splitn(2, "/").collect::<Vec<&str>>();
        let max_time = times.get(0).unwrap().to_owned();
        let current_time = times.get(1).unwrap().to_owned();
        let max_time = max_time.parse::<f32>().unwrap();
        let current_time = current_time.parse::<f32>().unwrap();

        self.power_bar.song_length = Duration::from_secs_f32(max_time);
        self.power_bar.current_timer = Duration::from_secs_f32(current_time);
        self.power_bar.song_name = self.playing_music().title;

        self.music_list.state.select(Some(self.music_list.selected));


        if self.status() == Status::Playing {
            match self.repeat {
                Repeat::ThisMusic => {
                    if current_time.trunc() == max_time.trunc() {
                        self.restart_playing_music();
                    }
                },
                Repeat::AllMusics => {
                    if current_time.trunc() == max_time.trunc() {
                        self.play_next();
                    }
                },
                Repeat::Dont => {},
            }
        }
    }

    /// Selectes the upper element in the music list (goes up by 1)
    pub fn list_up(&mut self) {
        let quesize = self.music_list.que.len();
        let selected_index = self.music_list.selected;
        if selected_index == 0 {
            self.music_list.selected = quesize - 1;
            // self.music_list.state.select(Some(quesize))
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

    // there there should be a buffer to append the char to it and then search 
    // where should the bufr be? self?
    // FOR NOW, SEARCH ONLY BY TITLE
    // TODO: make search wwork on _artist_, and _genre_
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

    // TODO: is this needed?
    /// Gets the selected index of the music list
    pub fn get_selected_index(&mut self) -> usize{
       self.music_list.selected 
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
        let volume = block_on(self.volume_percent()) * 100.0;
        if volume < 101.0 {
            // TODO: remove the unwrap since its not a fatal error
            block_on(self.proxy.volume(volume + 1.0)).unwrap();
        }
    }

    /// Decreases volume by 5
    pub fn decrease_volume(&self) {
        let volume = block_on(self.volume_percent()) * 100.0;
        if  volume > 0.0 {
            // TODO: remove the unwrap since its not a fatal error
            block_on(self.proxy.volume(volume - 1.0)).unwrap();
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

    /// Plays the previous music in the list
    /// This should be used when the playing index of the full music list 
    /// is known ahead of time
    fn o1_play_preivous(&mut self, playing_index: usize) {
        let previous = self.music_list.previous_song().to_owned();
        block_on(self.o1_play_this_music(&previous, playing_index));
    }

    /// Plays the next song in the music list
    fn restart_playing_music(&mut self) {
        let playing = self.playing_music();
        block_on(self.play_this_music(&playing));
    }

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
    pub async fn toggle_play(&mut self) {
        match self.status() {
            // if we are playing we pause
            Status::Playing => {
                self.pause().await;
            },
            // if we are pausing we resume
            Status::Pausing => {
                self.resume().await;
            }
            // if we stopped we play
            Status::Stopping => {
                self.play_this_music(&self.playing_music()).await;
            },
        }
    }

    /// restores playing state
    /// - restors the playing_index
    pub fn restore_state(&mut self) {
        for (index, music) in self.music_list.que.iter().enumerate() {
            if &self.playing_music() == music {
                self.music_list.playing_index = index;
            }
        }
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

    pub fn simple_new(path: PathBuf) -> Self {
        // let music = ffprobe::ffprobe(&path);
        let res = lofty::probe::Probe::open(&path);
        match res {
            Ok(probe) => {
                if let Ok(mut x) = probe.read() {
                    let properties = x.properties();
                    let length = properties.duration();
                    if let Some(tag) = x.primary_tag_mut() {
                        // TODO: check if empty
                        let mut title = tag.title().unwrap_or_default().to_string();
                        let mut artist = tag.artist().unwrap_or_default().to_string();
                        let mut genre = tag.genre().unwrap_or_default().to_string();
                        title.insert_if_empty("Unknown");
                        artist.insert_if_empty("Unknown");
                        genre.insert_if_empty("Unknown");

                        return Self {
                            title, length, path, artist, genre
                        }
                    }else {
                        return Self {
                             path, length,
                            title: String::from("Unknown"),
                            artist: String::from("Unknown"),
                            genre: String::from("Unknown"),
                        }
                    }
                }else {
                    println!("couldn't read the music prob in {:?}, falling to default method.", path);
                    return Self::default(path)
                }
            },
            Err(e) => {
                println!("couldn't open the music in {:?}, falling to default method.", path);
                println!("Error: {}", e);
                return Self::default(path);
            }
        };
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
            playing_index: 0, // TODO: change this to -1 if no song is playing
        }
    }

    /// returns a [`Music`] reference to the next song in playing quee
    fn next_song(&self) -> &Music {
        if self.playing_index + 1 < self.que.len() {
            return &self.que.get(self.playing_index + 1).unwrap()
        }else {
            return self.que.get(0).unwrap()
        }
    }

    /// returns a [`Music`] reference to the previous song in the palying quee
    fn previous_song(&self) -> &Music {
        match self.playing_index.checked_sub(1) {
            Some(number) => return &self.que.get(number).unwrap(),
            None => return self.que.get(self.que.len() - 1).unwrap(),
        }
    }

    fn select_next_song(&mut self) {
        if self.selected + 1 < self.que.len() {
            self.selected+=1;
            self.state.select(Some(self.selected));
        }else {
            self.selected=0;
            self.state.select(Some(self.selected));
        }
    }

    fn select_previous_song(&mut self) {
        match self.selected.checked_sub(1) {
            Some(number) => {
                self.selected = number;
                self.state.select(Some(self.selected));
            },
            None => {
                self.selected = self.que.len() - 1;
                self.state.select(Some(self.selected));
            },
        }
    }

    /// Sorts the music list acoording to the Sorting enum
    // TODO: change this to accept Sorting instead of Option<Sorting>
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

#[derive(Debug, PartialEq, Eq)]
pub enum Status {
    Playing, Pausing, Stopping
}

pub mod test {
    use super::*;
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
