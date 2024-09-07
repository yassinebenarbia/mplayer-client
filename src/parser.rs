use crate::ui::{Music, Musics};
use basic_toml;
use crate::ui::Repeat;
use serde::{self, Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Copy, Clone, Default)]
pub enum Sorting{
    #[default]
    ByTitleAscending,
    ByTitleDescending,
    ByDurationAscending,
    ByDurationDescending,
    Shuffle,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct Config {
    pub path: Option<String>,
    pub sorting: Option<Sorting>,
    pub repeat: Option<Repeat>
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Wrapper {
    pub config: Option<Config>
}

impl Config {
    fn visit_dirs(dir: &std::path::Path) -> Musics {
        let mut musics = vec![];
        if dir.is_dir() {
            for entry in std::fs::read_dir(dir).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();
                if path.is_dir() {
                    Config::visit_dirs(&path).que.iter().for_each(|v| {
                        match Music::simple_new(v.path.to_owned()) {
                            Some(music) => {
                                musics.push(music)
                            },
                            None => {} 
                        }
                    });
                } else {
                    match Music::simple_new(path) {
                        Some(music) => {
                            musics.push(music)
                        },
                        None => {} 
                    }
                }
            }
        }
        Musics::new(musics)
    }

    pub fn parse_config(_path: &str) -> Wrapper {
        let conf_content = std::fs::read_to_string(_path).expect(
            &format!("Couldn't read config path '{}', aborting...", _path)
        );
        let skeleton: Wrapper = basic_toml::from_str(&conf_content).expect(
            &format!("Couldn't parse config path '{}', aborting...", _path)
        );
        skeleton
    }

    pub fn extract_music(&self) -> Musics {
        let musics = match self.path.to_owned() {
            Some(p) => {
                Config::visit_dirs(std::path::Path::new(&p))
            },
            None => panic!("playist path is None!"),
        };
        return musics
    }
}

mod test {
    #[allow(unused_imports)]
    use super::*;
    #[test]
    fn test_config_parse() {
        Config::parse_config("./config.toml");
    }

    #[test]
    fn test_playlist_load() {
        let res = Config::parse_config("./config.toml");
        res.config.unwrap().extract_music();
    }
    
    #[test]
    fn sort_musics() {
        let res = Config::parse_config("./config.toml");
        let mut musics = res.config.clone().unwrap().extract_music();
        musics.sort(res.config.clone().unwrap().sorting);
    }

    #[test]
    #[should_panic]
    fn test_failing() {
        Config::parse_config("./config");
    }
}
