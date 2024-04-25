use std::{time::Duration, path::{Path, PathBuf}};
use async_std::task::block_on;
use futures_util::Future;
use ratatui::{prelude::*, widgets::*, style::Stylize};

use crate::ServerProxy;


pub struct UI<'a> {
    /// list of all the musics to play
    pub music_list: Musics,
    /// bar that indecate the playing timer
    pub power_bar: PowerBar,
    /// currently selected action 
    action: PowerActions,
    /// currently selected region
    pub region: Region,
    /// UI style
    pub style: UIStyle,
    /// last used action
    just_preformed_action: Action,
    /// proxy that communicates with the dbus server
    pub proxy: ServerProxy<'a>,
}

/// all the possible actions with the play button
enum Action {
    Play, Pause, Resume, None
}

#[derive(Clone)]
/// all the displayed region
pub enum Region {
    List, Action, Bar
}

impl Default for Region {
    fn default() -> Self {
        Region::List
    }
}

#[derive(Default)]
pub struct UIStyle {
    list_style: ListStyle,
    bar_style: BarStyle,
    action_style: ActionStyle
}

pub struct ListStyle {
    hilight_color: Color,
    active_region_color: Color,
    passive_region_color: Color,
    selector: String
}

struct BarStyle {
    active_region_color: Color,
    passive_region_color: Color,
}

impl Default for ListStyle {
    fn default() -> Self {
        ListStyle {
            hilight_color: Color::default(),
            active_region_color: Color::Magenta,
            passive_region_color: Color::default(),
            selector: String::from(">>")
        }
    }
}

impl UIStyle {
    pub fn new(
        list_style: ListStyle,
        action_style: ActionStyle,
        bar_style: BarStyle
        ) -> Self {
        UIStyle {
            list_style, action_style, bar_style
        }
    }
}


impl Default for BarStyle {
    fn default() -> Self {
        BarStyle {
            active_region_color: Color::Magenta,
            passive_region_color: Color::default(),
        }
    }
}

struct ActionStyle {
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
            action: PowerActions::Play,
            just_preformed_action: Action::Pause,
            proxy
        }
    }

    pub fn musics(&mut self, musics: Musics) {
        self.power_bar.max_timer = musics.que.get(musics.selected).unwrap().length;
        self.music_list = musics;
    }

    pub fn previous_action(&mut self) {
        self.action = match self.action {
            PowerActions::Play =>  PowerActions::Stop,
            PowerActions::Pause => PowerActions::Play,
            PowerActions::BackwardSkip => PowerActions::Pause,
            PowerActions::ForwardSkip => PowerActions::BackwardSkip,
            PowerActions::Stop => PowerActions::ForwardSkip,
        }
    }

    //// returns the music player status <Playing|Pausing|Stopping>
    pub fn status(&self) -> Status {
        let status = block_on(self.proxy.status()).unwrap();
        if status.contains("Stop") {
            return Status::Stopping
        } else if status.contains("Paus") {
            return Status::Pausing
        }else {
            return Status::Playing
        }
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
            PowerActions::Play =>  PowerActions::Pause,
            PowerActions::Pause => PowerActions::BackwardSkip,
            PowerActions::BackwardSkip => PowerActions::ForwardSkip,
            PowerActions::ForwardSkip => PowerActions::Stop,
            PowerActions::Stop => PowerActions::Play,
        }
    }

    pub async fn preform_action(&mut self) {
        // match the selected action
        match self.action {
            // if we are on the play button
            PowerActions::Play => {
                let selected_song = self.music_list.que.get(self.music_list.selected).unwrap();
                // if the currently selected song different from the currently playing one
                if self.power_bar.song_name != selected_song.title {
                    self.just_preformed_action = Action::Play;
                    let playing = self.music_list.que.get(self.music_list.selected).unwrap();
                    self.proxy.play(playing.path.as_os_str().to_str().unwrap().to_string()).await;
                    return
                }
                match self.status() {
                    Status::Playing => {
                        self.just_preformed_action = Action::Pause;
                        self.proxy.pause().await;
                    },
                    Status::Pausing => {
                        self.just_preformed_action = Action::Resume;
                        self.proxy.resume().await;
                    },
                    Status::Stopping => {
                        self.just_preformed_action = Action::Play;
                        let playing = self.music_list.que.get(self.music_list.selected).unwrap();
                        self.proxy.play(playing.path.as_os_str().to_str().unwrap().to_string()).await;
                    },
                }
            },
            PowerActions::Pause => {
                self.proxy.pause().await;
                self.just_preformed_action = Action::Pause;
            },
            PowerActions::ForwardSkip => todo!(),
            PowerActions::BackwardSkip => todo!(),
            PowerActions::Stop => {
                self.proxy.end().await;
            },
        }
    }

    pub async fn play_song(&mut self) {
        match self.status() {
            Status::Playing => {
                self.proxy.end().await;
                self.just_preformed_action = Action::Play;
                let playing = self.music_list.que.get(self.music_list.selected).unwrap();
                self.proxy.play(playing.path.as_os_str().to_str().unwrap().to_string()).await;
            },
            Status::Pausing => {
                self.proxy.end().await;
                self.proxy.resume().await;
                self.just_preformed_action = Action::Play;
                let playing = self.music_list.que.get(self.music_list.selected).unwrap();
                self.proxy.play(playing.path.as_os_str().to_str().unwrap().to_string()).await;
            },
            Status::Stopping => {
                self.just_preformed_action = Action::Play;
                let playing = self.music_list.que.get(self.music_list.selected).unwrap();
                self.proxy.play(playing.path.as_os_str().to_str().unwrap().to_string()).await;
            },
        }
    }

    pub fn select_list_region(&mut self) {
        self.region = Region::List;
    }

    pub fn select_bar_region(&mut self) {
        self.region = Region::Bar;
    }

    pub fn select_action_region(&mut self) {
        self.region = Region::Action
    } 
    
    pub fn render_list(&mut self, frame: &mut Frame) {
        let mut rows = vec![];
        for music in self.music_list.que.iter() {
            rows.push(
                Row::new(
                    vec![
                        music.title.to_owned(),
                        format!("{}", music.length.as_secs())
                    ]
                )
            )
        }

        let widths = [
            Constraint::Fill(15),
            Constraint::Fill(1)
        ];

        let block = match self.region {
            Region::List => {
                // Style::new().fg(self.style.list_style.active_region_color)
                Block::default().title("Table")
                .borders(Borders::ALL)
                .fg(self.style.list_style.active_region_color)
            },
            _ => {
                Block::default().title("Table")
                .borders(Borders::ALL)
                .fg(self.style.list_style.passive_region_color)
            }
        };

        let table = Table::new(rows, widths)
            .block(block)
            .highlight_style(
                Style::new().add_modifier(Modifier::REVERSED)
                .fg(self.style.list_style.hilight_color)
                )
            .highlight_symbol(self.style.list_style.selector.as_str())
            .header(
                Row::new(vec!["Title", "Time"])
                .style(Style::new().bold().italic())
                );

        let mut size = frame.size();
        size.height = size.height / 2;

        frame.render_stateful_widget(table, size, &mut self.music_list.state);
    }

    fn get_action_index(&self) -> usize {
        return match self.action {
            PowerActions::Play => 0,
            PowerActions::Pause => 1,
            PowerActions::BackwardSkip => 2,
            PowerActions::ForwardSkip => 3,
            PowerActions::Stop => 4,
        }
    }

    fn timer_to_string(&self, time: u64) -> String {
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

    fn timer(&mut self) -> String {
        format!("{}/{}",
               self.timer_to_string(self.power_bar.current_timer.as_secs()),
               self.timer_to_string(self.power_bar.max_timer.as_secs()))
    }

    fn calculate_percent(&self) -> u16 {
        let current = self.power_bar.current_timer.as_secs_f32();
        let max = self.power_bar.max_timer.as_secs_f32();
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

    pub async fn next_5s(&mut self) {
        let current = self.power_bar.current_timer.as_secs();
        let max = self.power_bar.max_timer.as_secs();
        if current + 5 < max {
            self.power_bar.current_timer =
                self.power_bar.current_timer.checked_add(Duration::from_secs(5)).unwrap();
        }else {
            let next_song = self.music_list.next_song();
            self.power_bar.current_timer = Duration::ZERO;
            self.power_bar.max_timer = next_song.length;
            self.music_list.select_next_song()
        }
        self.proxy.seek(5.0).await;
    }

    pub async fn previous_5s(&mut self) {
        let current = self.power_bar.current_timer.as_secs();
        match current.checked_sub(5) {
            Some(_) => {
                self.power_bar.current_timer =
                    self.power_bar.current_timer.checked_sub(Duration::from_secs(5)).unwrap();
            },
            None => {
                let next_song = self.music_list.previous_song();
                self.power_bar.current_timer = Duration::ZERO;
                self.power_bar.max_timer = next_song.length;
                self.music_list.select_previous_song();
            },
        }
        self.proxy.seek(-5.0).await;
    }

    pub fn render_bar(&mut self, frame: &mut Frame) {
        let mut area = frame.size();
        area.y = area.height - 3;
        area.height = 3;
        let selected_music = self.music_list.que.get(self.music_list.selected).unwrap();
        let style = match self.region {
            Region::Bar => {
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
            .gauge_style(
                Style::default()
                .fg(Color::White)
                .bg(Color::Black)
                .add_modifier(Modifier::ITALIC)
                )
            .label(self.timer())
            .ratio(self.calculate_percent() as f64 / 100.0)
            // .percent(self.calculate_percent())
            .render(area, frame.buffer_mut());
    }

    pub fn render_actions(&mut self, frame: &mut Frame) {
        let mut area = frame.size();
        area.y = area.height / 2;
        area.height = (area.height / 2) - 3;

        let style = match self.region {
            Region::Action => {
                Style::new().fg(self.style.action_style.active_region_color)
            }
            _ => {
                Style::new().fg(self.style.action_style.passive_region_color)
            }
        };

        Tabs::new(vec!["⏵", "⏮", "⏭", "⏹"])
            .block(
                Block::default().title("Actions").borders(Borders::ALL)
                )
            .style(style)
            .highlight_style(
                Style::default()
                .fg(self.style.action_style.hilight_color)
                .underline_color(self.style.action_style.hilight_color)
                )
            .select(self.get_action_index())
            .padding(" ", " ")
            .render(area, frame.buffer_mut());
    }


    pub fn render(&mut self, frame: &mut Frame) {
        self.update_state();
        self.render_bar(frame);
        self.render_list(frame);
        self.render_actions(frame);
    }

    pub fn update_state(&mut self) {
        let time = block_on(self.proxy.timer()).unwrap();
        let times = time.splitn(2, "/").collect::<Vec<&str>>();
        let max_time = times.get(0).unwrap().to_owned();
        let current_time = times.get(1).unwrap().to_owned();
        let max_time = max_time.parse::<f32>().unwrap();
        let current_time = current_time.parse::<f32>().unwrap();
        self.power_bar.max_timer = Duration::from_secs_f32(max_time);
        self.power_bar.current_timer = Duration::from_secs_f32(current_time);

        self.power_bar.song_name =
            self.music_list.que.get(self.music_list.selected).unwrap().title.clone();
    }

    /// selectes the upper element from the list
    pub fn list_up(&mut self) {
        let quesize = self.music_list.que.len();
        match self.music_list.state.selected() {
            Some(index) => {
                if index == 0 {
                    self.music_list.state.select(Some(quesize))
                }else {
                    self.music_list.selected = self.music_list.selected - 1;
                    self.music_list.state.select(Some(index - 1))
                }
            },
            None => {}
        }
    }

    pub fn list_down(&mut self) {
        let quesize = self.music_list.que.len();
        match self.music_list.state.selected() {
            Some(index) => {
                if index == quesize {
                    self.music_list.state.select(Some(0))
                }else {
                    self.music_list.selected = self.music_list.selected + 1;
                    self.music_list.state.select(Some(index + 1))
                }
            },
            None => {}
        }
    }
}

pub struct Music {
    title: String,
    length: Duration,
    path: PathBuf
}

impl Music {
    pub fn new(path: PathBuf, length: Duration) -> Self {
        Self {
            title: Music::derive_title_from_path(&path), path, length
        }
    }

    fn derive_title_from_path(path: &PathBuf) -> String {
        path.file_name().unwrap().to_str().unwrap().to_string()
    }
}

impl Default for Music {
    fn default() -> Self {
        Self {
            title: String::new(),
            length: Duration::ZERO,
            path: PathBuf::default()
        }
    }
}

#[derive(Default)]
pub struct Musics {
    pub que: Vec<Music>,
    pub selected: usize,
    pub state: TableState,
}

impl Musics {
    pub fn new(que: Vec<Music>) -> Self {
        Musics {
            que,
            selected: 0,
            state: TableState::default().with_selected(0),
        }
    }

    fn next_song(&self) -> &Music {
        if self.selected + 1 < self.que.len() {
            return &self.que.get(self.selected + 1).unwrap()
        }else {
            return self.que.get(0).unwrap()
        }
    }

    fn previous_song(&self) -> &Music {
        match self.selected.checked_sub(1) {
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
}

#[derive(Default)]
struct PowerBar{
    max_timer: Duration,
    current_timer: Duration,
    song_name: String,
}

impl From<Musics> for PowerBar {
    fn from(value: Musics) -> Self {
        let song = value.que.get(value.selected).unwrap();
        PowerBar{
            max_timer: song.length,
            current_timer: Duration::ZERO,
            song_name: song.title.to_owned()
        }
    }
}

enum PowerActions {
    Play,
    Pause,
    ForwardSkip,
    BackwardSkip,
    Stop
}

pub enum Status {
    Playing, Pausing, Stopping
}
