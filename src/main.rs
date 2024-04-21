use std::{io::{stdout, self}, time::Duration};

use zbus::{zvariant::ObjectPath, proxy, Connection, Result};
mod ui;
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{self, Terminal, backend::CrosstermBackend};
use ratatui::widgets::TableState;

use crate::ui::{Musics, Music};

#[proxy(
    interface = "org.zbus.mplayer1",
    default_service = "org.zbus.mplayer",
    default_path = "/org/zbus/mplayer"
)]
pub trait Server {
    fn status(&self) -> Result<bool>;
    fn play(&self, path: String) -> Result<bool>;
    fn end(&self) -> Result<bool>;
    fn resume(&self) -> Result<bool>;
    fn pause(&self) -> Result<bool>;
    fn show(&self) -> Result<String>;
    fn timer(&self) -> Result<String>;
    fn seek(&self, duration: f64) -> Result<bool>;
    fn volume(&self, amount:u8) -> Result<bool>;
}

#[async_std::main]
async fn main() -> Result<()> {
    // let connection = Connection::session().await?;
    // let proxy = ServerProxy::new(&connection).await?;
    // proxy.play(String::from("/home/yassine/Music/ЗАВОД.mp3")).await?;
    // let thing = proxy.show().await?;
    // let time = proxy.timer().await?;
    //
    // println!("{}", thing);
    // println!("{}", time);
    //

    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let mut ui = ui::UI::default();
    let musics:Musics = Musics::new(vec![
            Music::new(String::from("The state of art"), Duration::from_secs(10)),
            Music::new(String::from("What Art"), Duration::from_secs(20)),
            Music::new(String::from("It's just - ART"), Duration::from_secs(30)),
    ]);

    ui.musics(musics);

    let mut should_quit = false;
    while !should_quit {
        terminal.draw(|frame| {
            ui.render(frame);
        })?;
        should_quit = handle_events(&mut ui)?;
    }

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

fn handle_events(ui: &mut ui::UI) -> io::Result<bool> {
    // ALT + <hjkl> should switch between regions
    // / should search song on the list region
    if event::poll(std::time::Duration::from_millis(50))? {
        if let Event::Key(key) = event::read()? {
            match ui.region {
                ui::Region::List => {
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('q') {
                        return Ok(true);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('k') {
                        ui.list_up()
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('j') {
                        ui.list_down()
                    }
                },
                ui::Region::Bar => {
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('q') {
                        return Ok(true);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('l') {
                        ui.next_5s()
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('h') {
                        ui.previous_5s()
                    }
                } ,
                ui::Region::Action => {},
            }
        }
    }
    Ok(false)
}
