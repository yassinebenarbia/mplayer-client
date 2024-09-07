// goal:
// make it possible to take a vector of the Music struct, preform a fuzzy finding 
// based on the title criteria, then return a new vector of the Music struct
use std::iter;
use crate::ui::{Music, UI};

fn trigrams(s: &str) -> Vec<(char, char, char)> {
    let it_1 = iter::once(' ').chain(iter::once(' ')).chain(s.chars());
    let it_2 = iter::once(' ').chain(s.chars());
    let it_3 = s.chars().chain(iter::once(' '));

    let res: Vec<(char, char, char)> = it_1
        .zip(it_2)
        .zip(it_3)
        .map(|((a, b), c): ((char, char), char)| (a, b, c))
        .collect();
    res
}

pub fn fuzzy_compare(a: &str, b: &str) -> f32 {
    let a_binding = a.to_lowercase();
    let b_binding = b.to_lowercase();
    let _a = a_binding.as_str();
    let _b = b_binding.as_str();

    // gets length of first input string plus 1 (because of the 3 added spaces (' '))
    let string_len = _a.chars().count() + 1;

    // gets the trigrams for both strings
    let trigrams_a = trigrams(_a);
    let trigrams_b = trigrams(_b);

    // accumulator
    let mut acc: f32 = 0.0f32;
    // counts the number of trigrams of the first string that are also present in the second one
    for t_a in &trigrams_a {
        for t_b in &trigrams_b {
            if t_a == t_b {
                acc += 1.0f32;
                break;
            }
        }
    }
    let res = acc / (string_len as f32);
    // crops between zero and one
    if (0.0f32..=1.0f32).contains(&res) {
        res
    } else {
        0.0f32
    }
}

pub fn fuzzy_compare_durations(a: &str, b: &str) -> f32 {
    let a_binding = a.to_lowercase();
    let b_binding = b.to_lowercase();
    let _a = a_binding.as_str();
    let _b = b_binding.as_str();

    // gets length of first input string plus 1 (because of the 3 added spaces (' '))
    let string_len = _a.chars().count() + 1;

    // gets the trigrams for both strings
    let trigrams_a = trigrams(_a);
    let trigrams_b = trigrams(_b);

    // accumulator
    let mut acc: f32 = 0.0f32;
    // counts the number of trigrams of the first string that are also present in the second one
    for t_a in &trigrams_a {
        for t_b in &trigrams_b {
            if t_a == t_b {
                acc += 1.0f32;
                break;
            }
        }
    }
    let res = acc / (string_len as f32);
    // crops between zero and one
    if (0.0f32..=1.0f32).contains(&res) {
        res
    } else {
        0.0f32
    }
}


#[allow(dead_code)]
pub fn fuzzy_search<'a, T: AsRef<str>>(s: &'a str, list: &'a [T]) -> Vec<(&'a str, f32)> {
    list.iter()
        .map(|value| {
            let res = fuzzy_compare(s, value.as_ref());
            (value.as_ref(), res)
        })
        .collect()
}

pub fn fuzzy_search_musics_by_title<'a>(s: &'a str, list: &'a Vec<Music>) -> Vec<(Music, f32)> {
    list.iter()
        .map(|value| {
            let res = fuzzy_compare(s, value.title.as_ref());
            (Music::unchecked_new(value.path.clone()), res)
        })
        .collect()
}

pub fn fuzzy_search_music_titles_sorted<'a>(s: &'a str, list: &'a Vec<Music>) -> Vec<(Music, f32)> {
    let mut res = fuzzy_search_musics_by_title(s, list);
    res.sort_by(|(_, d1), (_, d2)| d2.partial_cmp(d1).unwrap()); // TODO to fix the unwrap call
    res
}

#[allow(dead_code)]
pub fn fuzzy_search_sorted<'a, T: AsRef<str>>(s: &'a str, list: &'a [T]) -> Vec<(&'a str, f32)> {
    let mut res = fuzzy_search(s, list);
    res.sort_by(|(_, d1), (_, d2)| d2.partial_cmp(d1).unwrap()); // TODO to fix the unwrap call
    res
}

#[allow(dead_code)]
pub fn fuzzy_search_threshold<'a, T: AsRef<str>>(
    s: &'a str,
    list: &'a [T],
    threshold: f32,
) -> Vec<(&'a str, f32)> {
    fuzzy_search(s, list)
        .into_iter()
        .filter(|&(_, score)| score >= threshold)
        .collect()
}

pub fn fuzzy_search_music_titles_best_n<'a>(
    s: &'a str,
    list: &'a Vec<Music>,
    n: usize,
) -> Vec<(Music, f32)> {
    fuzzy_search_music_titles_sorted(s, list).into_iter().take(n).collect()
}

#[allow(dead_code)]
pub fn fuzzy_search_best_n<'a, T: AsRef<str>>(
    s: &'a str,
    list: &'a [T],
    n: usize,
) -> Vec<(&'a str, f32)> {
    fuzzy_search_sorted(s, list).into_iter().take(n).collect()
}

pub fn fuzzy_search_music_duration_best_n(
    s: & str,
    list: &Vec<Music>,
    n: usize,
) -> Vec<(Music, f32)> {
    fuzzy_search_music_duration_sorted(s, list).into_iter().take(n).collect()
}

fn fuzzy_search_music_duration_sorted(s: &str, list: &Vec<Music>) -> Vec<(Music, f32)>  {
    let mut res = fuzzy_search_musics_by_duration(s, list);
    res.sort_by(|(_, d1), (_, d2)| d2.partial_cmp(d1).unwrap()); // TODO to fix the unwrap call
    res
}

fn fuzzy_search_musics_by_duration(s: &str, list: &Vec<Music>) -> Vec<(Music, f32)> {
    list.iter()
        .map(|value| {
            let res = fuzzy_compare_durations(
                s,
                UI::duration_to_string(value.length.as_secs()).as_str()
            );
            (Music::unchecked_new(value.path.clone()), res)
        })
        .collect()
}

pub fn fuzzy_search_music_artist_best_n(
    s: &str,
    list: &Vec<Music>,
    n: usize
) -> Vec<(Music, f32)> {
    fuzzy_search_artist_sorted(s, list).into_iter().take(n).collect()
}

fn fuzzy_search_artist_sorted(s: &str, list: &Vec<Music>) -> Vec<(Music, f32)> {
    let mut res = fuzzy_search_musics_by_artist(s, list);
    res.sort_by(|(_, d1), (_, d2)| d2.partial_cmp(d1).unwrap()); // TODO to fix the unwrap call
    res
}

fn fuzzy_search_musics_by_artist(s: &str, list: &Vec<Music>) -> Vec<(Music, f32)> {
    list.iter()
        .map(|value| {
            let res = fuzzy_compare(
                s,
                &value.artist,
            );
            (Music::unchecked_new(value.path.clone()), res) 
        })
        .collect()
}
