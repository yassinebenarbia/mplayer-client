#![allow(unused)]
use fuzzy_matcher::FuzzyMatcher;
use std::cmp::Ordering;

use crate::ui::{Music, UI};

pub fn fuzzy_search_music_titles_best_n<'a>(
    s: &'a str,
    list: &'a Vec<Music>,
    n: usize,
) -> Vec<Music> {
    fuzzy_search_music_titles_sorted(s, list)
        .into_iter()
        .take(n)
        .collect()
}

pub fn fuzzy_search_music_titles_sorted<'a>(s: &'a str, list: &'a Vec<Music>) -> Vec<Music> {
    let matcher = fuzzy_matcher::skim::SkimMatcherV2::default();
    let mut toreturn: Vec<Music> = list.clone();
    toreturn.sort_by(|x, y| {
        if let Some(x_score) = matcher.fuzzy_match(&x.title, s) {
            if let Some(y_score) = matcher.fuzzy_match(&y.title, s) {
                if x_score > y_score {
                    Ordering::Less
                } else if x_score == y_score {
                    Ordering::Equal
                } else {
                    Ordering::Greater
                }
            } else {
                Ordering::Less
            }
        } else {
            Ordering::Greater
        }
    });
    return toreturn;
}

pub fn fuzzy_search_music_grene_best_n(s: &str, list: &Vec<Music>, n: usize) -> Vec<Music> {
    fuzzy_search_music_genre_sorted(s, list)
        .into_iter()
        .take(n)
        .collect()
}

fn fuzzy_search_music_genre_sorted(s: &str, list: &Vec<Music>) -> Vec<Music> {
    let matcher = fuzzy_matcher::skim::SkimMatcherV2::default();
    let mut toreturn: Vec<Music> = list.clone();
    toreturn.sort_by(|x, y| {
        if let Some(x_score) = matcher.fuzzy_match(&x.genre, s) {
            if let Some(y_score) = matcher.fuzzy_match(&y.genre, s) {
                if x_score > y_score {
                    Ordering::Less
                } else if x_score == y_score {
                    Ordering::Equal
                } else {
                    Ordering::Greater
                }
            } else {
                Ordering::Less
            }
        } else {
            Ordering::Greater
        }
    });
    return toreturn;
}

pub fn fuzzy_search_music_duration_best_n(s: &str, list: &Vec<Music>, n: usize) -> Vec<Music> {
    fuzzy_search_music_duration_sorted(s, list)
        .into_iter()
        .take(n)
        .collect()
}

fn fuzzy_search_music_duration_sorted(s: &str, list: &Vec<Music>) -> Vec<Music> {
    let mut toreturn = list.clone();
    let matcher = fuzzy_matcher::skim::SkimMatcherV2::default();

    toreturn.sort_by(|x, y| {
        if let Some(x_score) =
            matcher.fuzzy_match(UI::duration_to_string(x.length.as_secs()).as_str(), s)
        {
            if let Some(y_score) =
                matcher.fuzzy_match(UI::duration_to_string(y.length.as_secs()).as_str(), s)
            {
                if x_score > y_score {
                    Ordering::Less
                } else if x_score == y_score {
                    Ordering::Equal
                } else {
                    Ordering::Greater
                }
            } else {
                Ordering::Less
            }
        } else {
            Ordering::Greater
        }
    });
    return toreturn;
}

pub fn fuzzy_search_music_artist_best_n(s: &str, list: &Vec<Music>, n: usize) -> Vec<Music> {
    fuzzy_search_artist_sorted(s, list)
        .into_iter()
        .take(n)
        .collect()
}

fn fuzzy_search_artist_sorted(s: &str, list: &Vec<Music>) -> Vec<Music> {
    let matcher = fuzzy_matcher::skim::SkimMatcherV2::default();
    let mut toreturn: Vec<Music> = list.clone();
    toreturn.sort_by(|x, y| {
        if let Some(x_score) = matcher.fuzzy_match(&x.artist, s) {
            if let Some(y_score) = matcher.fuzzy_match(&y.artist, s) {
                if x_score > y_score {
                    Ordering::Less
                } else if x_score == y_score {
                    Ordering::Equal
                } else {
                    Ordering::Greater
                }
            } else {
                Ordering::Less
            }
        } else {
            Ordering::Greater
        }
    });
    return toreturn;
}
