use std::{time::Duration, path::{Path, PathBuf}};
use async_std::task::block_on;
use futures_util::Future;
use ratatui::{prelude::*, widgets::*, style::Stylize};

use crate::ServerProxy;


pub struct UI<'a> {
    pub music_list: Musics,
    power_bar: PowerBar,
    pub region: Region,
    pub region_progress: u16,
    pub style: UIStyle,
    pub action: Action,
    pub just_preformed_action: Action,
    pub proxy: ServerProxy<'a>
}

#[derive(Clone)]
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

// #[derive(Default)]
struct ListStyle {
    hilight_color: Color,
    active_region_color: Color,
    passive_region_color: Color,
    selector: String
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

struct BarStyle {
    active_region_color: Color,
    passive_region_color: Color,
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
            region_progress: 0,
            power_bar: PowerBar::default(),
            music_list: Musics::default(),
            region: Region::default(),
            style : UIStyle::default(),
            action: Action::Play,
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
            Action::Play =>  Action::Stop,
            Action::Pause => Action::Play,
            Action::BackwardSkip => Action::Pause,
            Action::ForwardSkip => Action::BackwardSkip,
            Action::Stop => Action::ForwardSkip,
        }
    }

    pub fn next_action(&mut self) {
        self.action = match self.action {
            Action::Play =>  Action::Pause,
            Action::Pause => Action::BackwardSkip,
            Action::BackwardSkip => Action::ForwardSkip,
            Action::ForwardSkip => Action::Stop,
            Action::Stop => Action::Play,
        }
    }

    pub async fn preform_action(&mut self) {
        match self.action {
            Action::Play => {
                match self.just_preformed_action {
                    Action::Pause => {
                        self.just_preformed_action = Action::Play;
                        let playing = self.music_list.que.get(self.music_list.selected).unwrap();
                        self.proxy.play(playing.path.as_os_str().to_str().unwrap().to_string()).await;
                    }
                    Action::Play => {
                        self.just_preformed_action = Action::Pause;
                        self.proxy.pause();
                    }
                    _ => {}
                }
            },
            Action::Pause => {
                self.proxy.pause().await;
                self.just_preformed_action = Action::Pause;
            },
            Action::ForwardSkip => todo!(),
            Action::BackwardSkip => todo!(),
            Action::Stop => {
                self.proxy.end().await;
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
            Action::Play => 0,
            Action::Pause => 1,
            Action::BackwardSkip => 2,
            Action::ForwardSkip => 3,
            Action::Stop => 4,
        }
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

    pub fn next_5s(&mut self) {
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
    }

    pub fn previous_5s(&mut self) {
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

        Gauge::default()
            .block(Block::default().borders(Borders::ALL).title(
                    selected_music.title.to_owned()))
            .style(style)
            .gauge_style(
                Style::default()
                .fg(Color::White)
                .bg(Color::Black)
                .add_modifier(Modifier::ITALIC)
                )
            .percent(self.calculate_percent())
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

        Tabs::new(vec!["⏵", "⏸", "⏮", "⏭", "⏹"])
            .block(Block::default().title("Tabs").borders(Borders::ALL))
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
        self.render_list(frame);
        self.render_bar(frame);
        self.render_actions(frame);
        self.update_state();
    }

    pub fn update_state(&mut self) {
        let thing = block_on(self.proxy.show()).unwrap();
        println!("{:#?}", thing);
    }

    pub fn list_up(&mut self) {
        let quesize = self.music_list.que.len();
        match self.music_list.state.selected() {
            Some(index) => {
                if index == 0 {
                    self.music_list.state.select(Some(quesize))
                }else {
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

enum Action {
    Play,
    Pause,
    ForwardSkip,
    BackwardSkip,
    Stop
}
