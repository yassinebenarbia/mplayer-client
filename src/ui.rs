use std::{time::Duration};
use ratatui::{prelude::*, widgets::*, style::Stylize};

pub struct UI {
    pub music_list: Musics,
    power_bar: PowerBar,
    pub region: Region,
    pub region_progress: u16
}

#[derive(Clone)]
pub enum Region {
    List, Action, Bar
}

impl Default for Region {
    fn default() -> Self {
        Region::Bar
    }
}

impl Default for UI {
    fn default() -> Self {
        UI {
            region_progress: 0,
            power_bar: PowerBar::default(),
            music_list: Musics::default(),
            region: Region::default(),
        }
    }
}

impl UI {
    pub fn musics(&mut self, musics: Musics) {
        self.power_bar.max_timer = musics.que.get(musics.selected).unwrap().length;
        self.music_list = musics;
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
        let table = Table::new(rows, widths)
            .block(Block::default().title("Table"))
            .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
            .highlight_symbol(">>");

        let mut size = frame.size();
        size.height = size.height / 2;

        frame.render_stateful_widget(table, size, &mut self.music_list.state);
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
        Gauge::default()
            .block(Block::default().borders(Borders::ALL).title(
                    selected_music.title.to_owned()))
            .gauge_style(
                Style::default()
                .fg(Color::White)
                .bg(Color::Black)
                .add_modifier(Modifier::ITALIC))
            .percent(self.calculate_percent())
            .render(area, frame.buffer_mut());
    }

    pub fn render_actions(&mut self, frame: &mut Frame) {
        let tabls = Tabs::new(vec!["Tab1", "Tab2", "Tab3", "Tab4"])
            .block(Block::default().title("Tabs").borders(Borders::ALL))
            .style(Style::default().white())
            .highlight_style(Style::default().yellow())
            .select(0)
            .divider(symbols::DOT)
            .padding("--", "--");
        let mut area = frame.size();
        area.y = area.height / 2;
        area.height = (area.height / 2) - 3;
        frame.render_widget(tabls, area);
    }

    pub fn render(&mut self, frame: &mut Frame) {
        self.render_list(frame);
        self.render_bar(frame);
        self.render_actions(frame);
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
}

impl Music {
    pub fn new(title: String, length: Duration) -> Self {
        Self {
            title, length
        }
    }
}

impl Default for Music {
    fn default() -> Self {
        Self {
            title: String::new(), length: Duration::ZERO
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
            None => return self.que.get(self.que.len()).unwrap(),
        }
        // if (self.selected - 1) > 0 {
        //     return &self.que.get(self.selected - 1).unwrap()
        // }else {
        //     return self.que.get(self.que.len()).unwrap()
        // }
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
        if self.selected - 1 > 0 {
            self.selected -= 1;
            self.state.select(Some(self.selected));
        }else {
            self.selected = self.que.len();
            self.state.select(Some(self.selected));
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
    Pause, Stop
}
