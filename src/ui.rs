use std::{time::Duration, path::PathBuf};
use async_std::task::block_on;
use ratatui::{prelude::*, widgets::*, style::Stylize};
use crate::utils::log;
use lofty;
use lofty::file::AudioFile;
use lofty::file::TaggedFileExt;
use lofty::mpeg::MpegFile;
use lofty::probe::Probe;
use lofty::tag::Accessor;


use crate::{fuzzy_search, ListMode, ServerProxy, Sorting};

#[derive(Default)]
pub enum AncitipationMode {
    #[default]
    Normal,
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
    /// proxy that communicates with the dbus server
    pub proxy: ServerProxy<'a>,
    /// slection mod 
    pub mode: ListMode,
    /// last used action
    just_preformed_action: Action,
    /// search buffer, used to search through the musics list
    search_bufr: String,
    /// helps reading a combination of keys like `gg`
    pub anticipation_mode: AncitipationMode
}

/// all the possible actions with the play button
enum Action {
    Play, Pause, Resume,
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
    // playing_region_color: Color::Gray,
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
            anticipation_mode: AncitipationMode::default()
        }
    }

    pub fn musics(&mut self, musics: Musics) {
        self.power_bar.song_length = musics.que.get(musics.selected).unwrap().length;
        self.music_list = musics;
    }

    pub fn previous_action(&mut self) {
        self.action = match self.action {
            PowerActions::Stop => PowerActions::ForwardSkip,
            PowerActions::ForwardSkip => PowerActions::TogglePlay,
            PowerActions::TogglePlay => PowerActions::BackwardSkip,
            PowerActions::BackwardSkip => PowerActions::Stop,
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

    pub fn playing(&self) -> Music {
        block_on(self.proxy.playing()).unwrap_or_default()
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
            PowerActions::Stop => PowerActions::BackwardSkip,
        }
    }

    pub async fn preform_action(&mut self) {
        // match the selected action
        match self.action {
            // we are on the toggle play botton
            PowerActions::TogglePlay => {
                match self.status() {
                    // if we were playing we pause
                    Status::Playing => {
                        self.proxy.pause().await.unwrap();
                        self.just_preformed_action = Action::Pause;
                    },
                    // if we were pausing or stopping we play the currently playing song
                    // if we are on selecting the currently playing song, we just resume, else 
                    // we play the selected song
                    Status::Pausing | Status::Stopping => {
                        // // block_on(self.play_selected_song());
                        let selected_song = self.music_list.que.get(self.music_list.selected).unwrap();
                        let playing_song = self.playing_song();
                        // // if the selected song different from the currently playing one
                        if playing_song.title != selected_song.title {
                            block_on(self.play_selected_song());
                        }else {
                            block_on(self.resume_playing_song());
                        }
                    },
                }
            },
            PowerActions::ForwardSkip => {
                // TODO: next song should be the song just next to the playing one, not the selected
                // one
                // self.playing_index
                let next_song = self.music_list.next_song();
                block_on(self.proxy.play(next_song.path.to_str().unwrap().to_string())).unwrap();
                if self.music_list.playing_index + 1 < self.music_list.que.len() {
                    self.music_list.playing_index+=1
                }else {
                    self.music_list.playing_index=0
                }
            },
            PowerActions::BackwardSkip => {
                let previous_song = self.music_list.previous_song();
                block_on(self.proxy.play(previous_song.path.to_str().unwrap().to_string())).unwrap();
                match self.music_list.playing_index.checked_sub(1) {
                    Some(n) => self.music_list.playing_index = n,
                    None => self.music_list.playing_index = self.music_list.que.len() - 1,
                }
            },
            PowerActions::Stop => {
                self.proxy.end().await.unwrap();
            },
        }
    }

    /// returns the currently plaing song
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
    /// plays the *selected* song in the music list
    pub async fn play_selected_song(&mut self) {
        let toplay = self.music_list.que.get(self.music_list.selected).unwrap();
        self.music_list.playing_index = 
            self.music_list.full_que.iter().position(|x| x == toplay).unwrap_or(0);
        match self.status() {
            Status::Playing => {
                self.proxy.end().await.unwrap();
                self.just_preformed_action = Action::Play;
                self.proxy.play(toplay.path.as_os_str().to_str().unwrap().to_string()).await.unwrap();
            },
            Status::Pausing => {
                self.proxy.end().await.unwrap();
                self.proxy.resume().await.unwrap();
                self.just_preformed_action = Action::Play;
                self.proxy.play(toplay.path.as_os_str().to_str().unwrap().to_string()).await.unwrap();
            },
            Status::Stopping => {
                self.just_preformed_action = Action::Play;
                self.proxy.play(toplay.path.as_os_str().to_str().unwrap().to_string()).await.unwrap();
            },
        }
    }

    /// Moves slection to the `List` region
    pub fn select_list_region(&mut self) {
        self.region = Region::List;
    }

    /// Moves slection to the `Bar` region
    pub fn select_bar_region(&mut self) {
        self.region = Region::Bar;
    }

    /// Moves slection to the `Action` region
    pub fn select_action_region(&mut self) {
        self.region = Region::Action
    } 
    
    /// Renders the region of the music list
    pub fn render_list(&mut self, frame: &mut Frame) {
        let mut rows = vec![];
        let playing = self.music_list.full_que.get(self.music_list.playing_index).unwrap();
        // TODO: map the index of the playing song of the full que to the one in the displayed que
        for music in self.music_list.que.iter() {
            let mut title = music.title.to_owned();
            let time = self.timer_to_string(music.length.as_secs());
            if playing == music {
                title.insert_str(0, self.style.list_style.playing_selector.as_str());
                rows.push(
                    Row::new(vec![title, time])
                    .style(self.style.list_style.playing_region_color)
                )
            }else {
                rows.push(
                    Row::new(vec![ title, time ]))
            }
        }

        let widths = [
            Constraint::Fill(15),
            Constraint::Fill(1)
        ];

        let block = match self.region {
            Region::List => {
                // TOOD: make the color of the reagion express it's state
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
                // Block::default().title("Musics")
                //     .borders(Borders::ALL)
                //     .fg(self.style.list_style.active_region_color)
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
                Row::new(vec!["Title", "Length"])
                .style(Style::new().bold().italic()));

        let mut size = frame.size();
        let mut search_size = frame.size();
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
               self.timer_to_string(self.power_bar.song_length.as_secs()))
    }

    fn calculate_percent(&self) -> u16 {
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

    pub async fn next_5s(&mut self) {
        let current = self.power_bar.current_timer.as_secs();
        let song_length = self.power_bar.song_length.as_secs();
        if current + 5 < song_length {
            self.power_bar.current_timer =
                self.power_bar.current_timer.checked_add(Duration::from_secs(5)).unwrap();
            self.proxy.seek(5.0).await.unwrap();
        }else {
            let next_song = self.music_list.next_song();
            block_on(self.proxy.play(next_song.path.to_str().unwrap().to_string())).unwrap();
            self.music_list.select_next_song();
        }
    }

    pub async fn previous_5s(&mut self) {
        let current = self.power_bar.current_timer.as_secs();
        match current.checked_sub(5) {
            Some(_) => {
                self.power_bar.current_timer =
                    self.power_bar.current_timer.checked_sub(Duration::from_secs(5)).unwrap();
                self.proxy.seek(-5.0).await.unwrap();
            },
            None => {
                let previous_song = self.music_list.previous_song();
                block_on(self.proxy.play(previous_song.path.to_str().unwrap().to_string())).unwrap();
                self.music_list.select_previous_song();
            },
        }
    }

    pub fn render_seeker(&mut self, frame: &mut Frame) {
        let mut area = frame.size();
        area.y = area.height - 3;
        area.height = 3;
        let selected_music = self.playing();
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
                .add_modifier(Modifier::ITALIC))
            .label(self.timer())
            .ratio(self.calculate_percent() as f64 / 100.0)
            .render(area, frame.buffer_mut());
    }

    pub fn render_actions(&mut self, frame: &mut Frame) {
        let mut area = frame.size();
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
        let status =block_on(self.proxy.status()).unwrap();
        let mut lines = status.lines();
        // check if the selected song is playing, if so inser the pause icon
        if lines.next().unwrap().to_lowercase().contains("playing") {
            actions[1] = "⏸";
        }

        Tabs::new(actions)
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
        self.render_seeker(frame);
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

        self.power_bar.song_length = Duration::from_secs_f32(max_time);
        self.power_bar.current_timer = Duration::from_secs_f32(current_time);
        self.power_bar.song_name = self.playing().title;

        self.music_list.state.select(Some(self.music_list.selected));
    }

    /// selectes the upper element from the list
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
    pub async fn register_querry(&mut self, c: char) {
        self.search_bufr.push(c);
        self.music_list.search(self.search_bufr.to_owned());
    }

    pub fn reset_querry(&mut self) {
        self.music_list.reset_search();
    }

    pub fn delete_char_querry(&mut self) {
        self.search_bufr.pop();
        self.music_list.search(self.search_bufr.to_owned());
    }

    pub fn goto_top(&mut self) {
        self.music_list.selected = 0;
    }

    // TODO: is this needed?
    /// gets the selected index of the music list
    pub fn get_selected_index(&mut self) -> usize{
       self.music_list.selected 
    }

    /// selects the currently playing song in the music list
    pub fn goto_playing(&mut self) {
        self.music_list.selected = self.music_list.playing_index;
    }

    pub fn goto_bottom(&mut self) {
       self.music_list.selected = self.music_list.que.len() - 1;
    }
}

#[derive(PartialEq, Eq, Debug, Ord, PartialOrd, Clone)]
#[derive(serde::Serialize, serde::Deserialize, zbus::zvariant::Type)]
pub struct Music {
    pub title: String,
    pub length: Duration,
    pub path: PathBuf
}

impl Music {

    pub fn new(title: String, path: PathBuf, length: Duration) -> Self {
        Self {
            title, path, length
        }
    }

    pub fn default (path: PathBuf) -> Self {
        Self {
            title: Music::derive_title_from_path(&path),
            path,
            length: Duration::ZERO,
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
                        let title = tag.title().unwrap_or_default().to_string();
                        return Self {
                            title, length, path
                        }
                    }else {
                        return Self {
                            path: path,
                            length: length,
                            title: String::from("Unkwn")
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
        // let music = audiotags::Tag::new().read_from_path(&path);
        // match music {
        //     Ok(music) => {
        //         let length = Duration::from_secs_f64(music.duration().unwrap_or_default());
        //         let title = music.title().unwrap_or_default().to_string();
        //         return Self {
        //             title, path, length
        //         }
        //     },
        //     Err(e) => {
        //         println!("couldn't parse the music in {:?}, falling to default method.", path);
        //         println!("Error: {}", e);
        //         return Self {
        //             title: Music::derive_title_from_path(&path),
        //             path,
        //             length: Duration::ZERO,
        //         }
        //     },
        // }

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
            path: PathBuf::default()
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

    pub fn sort(&mut self, sorting: Option<Sorting>) {
        match sorting {
            Some(o) => {
                self.que.sort_by(|x, y| {
                    match o {
                        Sorting::Ascending => {
                            if x > y {
                                return std::cmp::Ordering::Greater
                            }
                            else if x < y {
                                return std::cmp::Ordering::Less
                            }
                            std::cmp::Ordering::Equal
                        },
                        Sorting::Descending => {
                            if x < y {
                                return std::cmp::Ordering::Greater
                            }
                            else if x > y {
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

    // TODO: make a macro that fuzzy search a struct based on a criterion
    // searches through the musics and returns the desired list as a Musics
    pub fn search(&mut self, search_bufr: String) {
        let n = 10;
        if search_bufr.is_empty() {
            self.que = self.full_que.clone();
        }else {
            let mut toreturn: Vec<Music> = vec![];
            let results = fuzzy_search::fuzzy_search_musics_best_n(&search_bufr, &self.full_que, n);
            for (music, _) in results {
                toreturn.push(music);
            }
            self.que = toreturn;
        }
    }

    pub fn reset_search(&mut self) {
        let mut toreturn: Vec<Music> = vec![];
        let results = fuzzy_search::fuzzy_search_musics("", &self.que);
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
    Stop
}

#[derive(Debug)]
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
