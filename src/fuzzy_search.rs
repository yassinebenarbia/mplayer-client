// goal:
// make it possible to take a vector of the Music struct, preform a fuzzy finding 
// based on the title criteria, then return a new vector of the Music struct
use std::iter;

use crate::ui::Music;

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

pub fn fuzzy_search<'a, T: AsRef<str>>(s: &'a str, list: &'a [T]) -> Vec<(&'a str, f32)> {
    list.iter()
        .map(|value| {
            let res = fuzzy_compare(s, value.as_ref());
            (value.as_ref(), res)
        })
        .collect()
}

pub fn fuzzy_search_musics<'a>(s: &'a str, list: &'a Vec<Music>) -> Vec<(Music, f32)> {
    list.iter()
        .map(|value| {
            let res = fuzzy_compare(s, value.title.as_ref());
            (Music::simple_new(value.path.clone()), res) // TODO: change the simple_new to new
                                                         // so you don't allways check paths for
                                                         // title
        })
        .collect()
}

pub fn fuzzy_search_musics_sorted<'a>(s: &'a str, list: &'a Vec<Music>) -> Vec<(Music, f32)> {
    let mut res = fuzzy_search_musics(s, list);
    res.sort_by(|(_, d1), (_, d2)| d2.partial_cmp(d1).unwrap()); // TODO to fix the unwrap call
    res
}

pub fn fuzzy_search_sorted<'a, T: AsRef<str>>(s: &'a str, list: &'a [T]) -> Vec<(&'a str, f32)> {
    let mut res = fuzzy_search(s, list);
    res.sort_by(|(_, d1), (_, d2)| d2.partial_cmp(d1).unwrap()); // TODO to fix the unwrap call
    res
}

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

pub fn fuzzy_search_musics_best_n<'a>(
    s: &'a str,
    list: &'a Vec<Music>,
    n: usize,
) -> Vec<(Music, f32)> {
    fuzzy_search_musics_sorted(s, list).into_iter().take(n).collect()
}

pub fn fuzzy_search_best_n<'a, T: AsRef<str>>(
    s: &'a str,
    list: &'a [T],
    n: usize,
) -> Vec<(&'a str, f32)> {
    fuzzy_search_sorted(s, list).into_iter().take(n).collect()
}

// #[cfg(test)]
// mod tests {
//     use crate::{
//         fuzzy_compare, fuzzy_search, fuzzy_search_best_n, fuzzy_search_sorted,
//         fuzzy_search_threshold,
//     };
//
//     #[test]
//     fn perfect_match_1() {
//         assert_eq!(fuzzy_compare("kolbasobulko", "kolbasobulko"), 1.0f32)
//     }
//     #[test]
//     fn perfect_match_2() {
//         assert_eq!(fuzzy_compare("sandviĉo", "sandviĉo"), 1.0f32)
//     }
//     #[test]
//     fn perfect_match_3() {
//         assert_eq!(fuzzy_compare("domo", "domo"), 1.0f32)
//     }
//     #[test]
//     fn perfect_match_4() {
//         assert_eq!(fuzzy_compare("ŝatas", "ŝatas"), 1.0f32)
//     }
//     #[test]
//     fn perfect_match_5() {
//         assert_eq!(fuzzy_compare("mirinda estonto", "mirinda estonto"), 1.0f32)
//     }
//     #[test]
//     fn no_match() {
//         assert_eq!(fuzzy_compare("abc", "def"), 0.0f32)
//     }
//     #[test]
//     fn empty_word() {
//         assert_eq!(fuzzy_compare("", ""), 1.0f32)
//     }
//     #[test]
//     fn one_letter() {
//         assert_eq!(fuzzy_compare("a", "a"), 1.0f32)
//     }
//     #[test]
//     fn utf8_one_letter_1() {
//         assert_eq!(fuzzy_compare("ĉ", "ĉ"), 1.0f32)
//     }
//     #[test]
//     fn utf8_one_letter_2() {
//         assert_eq!(fuzzy_compare("ł", "ł"), 1.0f32)
//     }
//     #[test]
//     fn utf8_no_match() {
//         assert_eq!(fuzzy_compare("cgs", "ĉĝŝ"), 0.0f32)
//     }
//     #[test]
//     fn test_fuzzy_search_1() {
//         let s: &str = "bulko";
//         let list: Vec<&str> = vec!["kolbasobulko", "sandviĉo", "kolbasobulkejo"];
//         let res: Vec<(&str, f32)> = fuzzy_search(s, &list);
//         assert_eq!(res.into_iter().count(), 3);
//     }
//     #[test]
//     fn test_fuzzy_search_owned() {
//         let s: &str = "bulko";
//         let list: Vec<String> = vec![
//             String::from("kolbasobulko"),
//             String::from("sandviĉo"),
//             String::from("kolbasobulkejo"),
//         ];
//         let res: Vec<(&str, f32)> = fuzzy_search(s, &list);
//         assert_eq!(res.into_iter().count(), 3);
//     }
//     #[test]
//     fn test_fuzzy_search_sorted() {
//         let s: &str = "bulko";
//         let list: Vec<&str> = vec!["kolbasobulko", "sandviĉo", "kolbasobulkejo"];
//         let res: Vec<(&str, f32)> = fuzzy_search_sorted(s, &list);
//         assert_eq!(res.into_iter().count(), 3);
//     }
//     #[test]
//     fn no_lowers() {
//         let threshold = 0.5f32;
//         let s: &str = "bulko";
//         let list: Vec<&str> = vec!["kolbasobulko", "sandviĉo", "kolbasobulkejo"];
//         for (_word, score) in fuzzy_search_threshold(s, &list, threshold) {
//             assert!(score > threshold)
//         }
//     }
//     #[test]
//     fn test_fuzzy_search_best_n() {
//         let s: &str = "bulko";
//         let list: Vec<&str> = vec!["kolbasobulko", "sandviĉo", "kolbasobulkejo"];
//         let res: Vec<(&str, f32)> = fuzzy_search_best_n(s, &list, 2);
//         assert_eq!(res.into_iter().count(), 2);
//     }
// }
