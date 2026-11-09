 
 
 
 
 
 
 

use crate::graphic::now_playing;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, Once};
use std::time::Duration;

const API_BASE: &str = "https://lrclib.net/api";
const POLL_INTERVAL: Duration = Duration::from_millis(500);
const CACHE_LIMIT: usize = 64;
 
 
const MAX_LINE_SPAN: Duration = Duration::from_secs(8);

 
#[derive(Clone, Debug, PartialEq)]
pub struct LyricWord {
    pub text: String,
    pub start: Duration,
    pub end: Duration,
}

 
#[derive(Clone, Debug, PartialEq)]
pub struct LyricLine {
    pub start: Duration,
    pub end: Duration,
    pub words: Vec<LyricWord>,
}

 
#[derive(Clone, Debug)]
pub enum LyricsState {
     
    Idle,
     
    Loading,
     
    Missing,
    Ready(Arc<[LyricLine]>),
}

struct Store {
    key: Option<(String, String)>,
    state: LyricsState,
}

static STORE: Mutex<Store> = Mutex::new(Store {
    key: None,
    state: LyricsState::Idle,
});
static START: Once = Once::new();

fn store_guard() -> MutexGuard<'static, Store> {
    STORE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

 
pub fn ensure_worker() {
    START.call_once(|| {
         
        let _ = std::thread::Builder::new()
            .name("lyrics".into())
            .spawn(worker);
    });
}

 
 
pub fn state_for(title: &str, artist: &str) -> LyricsState {
    let store = store_guard();
    match &store.key {
        Some((t, a)) if t == title && a == artist => store.state.clone(),
        _ => LyricsState::Idle,
    }
}

 
 
pub fn active_line_index(lines: &[LyricLine], at: Duration) -> Option<usize> {
    lines
        .partition_point(|line| line.start <= at)
        .checked_sub(1)
}

 
pub fn active_word_index(line: &LyricLine, at: Duration) -> Option<usize> {
    line.words
        .partition_point(|word| word.start <= at)
        .checked_sub(1)
}

 
pub fn word_progress(word: &LyricWord, at: Duration) -> f32 {
    if at >= word.end {
        1.0
    } else if at <= word.start {
        0.0
    } else {
        let span = (word.end - word.start).as_secs_f32();
        ((at - word.start).as_secs_f32() / span).clamp(0.0, 1.0)
    }
}

 

fn worker() {
    #[cfg(windows)]
    {
        use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};
         
         
        let _ = unsafe { RoInitialize(RO_INIT_MULTITHREADED) };
    }

    let mut cache: HashMap<(String, String), Option<Arc<[LyricLine]>>> = HashMap::new();
    let mut current: Option<(String, String)> = None;

    loop {
        let playback = now_playing::current();
        let key = playback
            .as_ref()
            .map(|p| (p.title.clone(), p.artist.clone()));

        if key != current {
            current = key.clone();
            match (key, playback) {
                (Some(key), Some(playback)) => {
                    set_state(Some(key.clone()), LyricsState::Loading);
                    let lines = match cache.get(&key) {
                        Some(lines) => lines.clone(),
                        None => {
                            let fetched = fetch(&key.0, &key.1, playback.duration)
                                .map(Arc::<[LyricLine]>::from);
                            if cache.len() >= CACHE_LIMIT {
                                cache.clear();
                            }
                            cache.insert(key.clone(), fetched.clone());
                            fetched
                        }
                    };
                    set_state(
                        Some(key),
                        lines.map_or(LyricsState::Missing, LyricsState::Ready),
                    );
                }
                _ => set_state(None, LyricsState::Idle),
            }
        }

        std::thread::sleep(POLL_INTERVAL);
    }
}

fn set_state(key: Option<(String, String)>, state: LyricsState) {
    let mut store = store_guard();
    store.key = key;
    store.state = state;
}

 
 
fn fetch(title: &str, artist: &str, duration: Duration) -> Option<Vec<LyricLine>> {
    let track_end = (!duration.is_zero()).then_some(duration);
    let query = format!(
        "track_name={}&artist_name={}",
        percent_encode(title),
        percent_encode(artist)
    );

    let direct = http_get(&format!("{API_BASE}/get?{query}"))
        .and_then(|body| serde_json::from_str::<serde_json::Value>(&body).ok())
        .and_then(|value| synced_lyrics(&value).map(str::to_owned));

    let synced = match direct {
        Some(text) => text,
        None => {
            let body = http_get(&format!("{API_BASE}/search?{query}"))?;
            let items: Vec<serde_json::Value> = serde_json::from_str(&body).ok()?;
            items
                .iter()
                .find_map(|item| synced_lyrics(item).map(str::to_owned))?
        }
    };

    let lines = parse_lrc(&synced, track_end);
    (!lines.is_empty()).then_some(lines)
}

fn synced_lyrics(value: &serde_json::Value) -> Option<&str> {
    value
        .get("syncedLyrics")?
        .as_str()
        .filter(|text| !text.trim().is_empty())
}

 
 
#[cfg(windows)]
fn http_get(url: &str) -> Option<String> {
    use windows::core::HSTRING;
    use windows::Foundation::Uri;
    use windows::Web::Http::HttpClient;

    let client = HttpClient::new().ok()?;
    let uri = Uri::CreateUri(&HSTRING::from(url)).ok()?;
    let response = client.GetAsync(&uri).ok()?.get().ok()?;
    if !response.IsSuccessStatusCode().ok()? {
        return None;
    }
    let text = response
        .Content()
        .ok()?
        .ReadAsStringAsync()
        .ok()?
        .get()
        .ok()?;
    Some(text.to_string())
}

#[cfg(not(windows))]
fn http_get(_url: &str) -> Option<String> {
    None
}

 
fn percent_encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

 

 
 
 
 
 
 
pub fn parse_lrc(text: &str, track_end: Option<Duration>) -> Vec<LyricLine> {
    let mut entries: Vec<(Duration, &str)> = Vec::new();
    for raw in text.lines() {
        let mut rest = raw.trim();
        let mut stamps = Vec::new();
        while let Some(stripped) = rest.strip_prefix('[') {
            let Some(close) = stripped.find(']') else {
                break;
            };
            let Some(time) = parse_timestamp(&stripped[..close]) else {
                break;
            };
            stamps.push(time);
            rest = &stripped[close + 1..];
        }
        if stamps.is_empty() {
            continue;
        }
        for time in stamps {
            entries.push((time, rest.trim()));
        }
    }
    entries.sort_by_key(|(time, _)| *time);

    entries
        .iter()
        .enumerate()
        .map(|(index, (start, body))| {
            let end = match entries.get(index + 1) {
                Some((next, _)) => (*next).min(*start + MAX_LINE_SPAN),
                None => track_end
                    .filter(|end| *end > *start)
                    .unwrap_or(*start + MAX_LINE_SPAN),
            };
            LyricLine {
                start: *start,
                end,
                words: split_words(body, *start, end),
            }
        })
        .collect()
}

 
 
fn parse_timestamp(tag: &str) -> Option<Duration> {
    let (minutes, rest) = tag.split_once(':')?;
    let minutes: u64 = minutes.trim().parse().ok()?;
    let (seconds, fraction) = match rest.find(|c: char| c == '.' || c == ':') {
        Some(index) => (&rest[..index], &rest[index + 1..]),
        None => (rest, ""),
    };
    let seconds: u64 = seconds.trim().parse().ok()?;
    let millis = if fraction.is_empty() {
        0
    } else {
        if !fraction.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let digits = &fraction[..fraction.len().min(3)];
        let value: u64 = digits.parse().ok()?;
        value * 10u64.pow(3 - digits.len() as u32)
    };
    Some(Duration::from_millis(
        minutes * 60_000 + seconds * 1000 + millis,
    ))
}

 
 
fn split_words(body: &str, start: Duration, end: Duration) -> Vec<LyricWord> {
    if let Some(words) = enhanced_words(body, start, end) {
        return words;
    }
    let tokens: Vec<&str> = body.split_whitespace().collect();
    let weights: Vec<f64> = tokens.iter().map(|t| t.chars().count() as f64).collect();
    let total: f64 = weights.iter().sum();
    if tokens.is_empty() || total <= 0.0 {
        return Vec::new();
    }
    let origin = start.as_secs_f64();
    let span = end.saturating_sub(start).as_secs_f64();
    let mut cursor = 0.0;
    tokens
        .iter()
        .zip(&weights)
        .map(|(token, weight)| {
            let word_start = origin + span * cursor / total;
            cursor += weight;
            let word_end = origin + span * cursor / total;
            LyricWord {
                text: (*token).to_owned(),
                start: seconds(word_start),
                end: seconds(word_end),
            }
        })
        .collect()
}

 
 
fn enhanced_words(body: &str, start: Duration, end: Duration) -> Option<Vec<LyricWord>> {
    let mut segments: Vec<(Duration, String)> = vec![(start, String::new())];
    let mut rest = body;
    let mut tagged = false;
    while let Some(open) = rest.find('<') {
        let (before, tail) = rest.split_at(open);
        segments.last_mut()?.1.push_str(before);
        let close = tail.find('>')?;
        let time = parse_timestamp(&tail[1..close])?;
        segments.push((time, String::new()));
        tagged = true;
        rest = &tail[close + 1..];
    }
    if !tagged {
        return None;
    }
    segments.last_mut()?.1.push_str(rest);

    let segments: Vec<(Duration, String)> = segments
        .into_iter()
        .filter(|(_, text)| !text.trim().is_empty())
        .collect();
    Some(
        segments
            .iter()
            .enumerate()
            .map(|(index, (time, text))| {
                let word_end = segments.get(index + 1).map_or(end, |(next, _)| *next);
                LyricWord {
                    text: text.trim().to_owned(),
                    start: *time,
                    end: word_end.max(*time),
                }
            })
            .collect(),
    )
}

fn seconds(value: f64) -> Duration {
    Duration::from_secs_f64(value.max(0.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(value: u64) -> Duration {
        Duration::from_millis(value)
    }

    #[test]
    fn timestamps_accept_all_fraction_widths() {
        assert_eq!(parse_timestamp("00:12.34"), Some(ms(12_340)));
        assert_eq!(parse_timestamp("01:02.5"), Some(ms(62_500)));
        assert_eq!(parse_timestamp("00:03"), Some(ms(3_000)));
        assert_eq!(parse_timestamp("00:03.123"), Some(ms(3_123)));
        assert_eq!(parse_timestamp("00:03:50"), Some(ms(3_500)));
    }

    #[test]
    fn metadata_tags_are_not_timestamps() {
        assert_eq!(parse_timestamp("ar:Artist"), None);
        assert_eq!(parse_timestamp("length: 03:20"), None);
        assert_eq!(parse_timestamp("00:0x.10"), None);
    }

    #[test]
    fn lines_are_sorted_and_multi_tagged_lines_repeat() {
        let text = "[ti:Song]\n[00:10.00]second\n[00:05.00][00:20.00]first and last\n";
        let lines = parse_lrc(text, Some(ms(30_000)));
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].start, ms(5_000));
         
        assert_eq!(lines[0].end, ms(10_000));
        assert_eq!(lines[1].start, ms(10_000));
        assert_eq!(lines[2].start, ms(20_000));
        assert_eq!(lines[2].end, ms(30_000));
    }

    #[test]
    fn plain_words_share_the_line_span_by_length() {
        let words = split_words("ab cde", ms(0), ms(5_000));
        assert_eq!(words.len(), 2);
        assert_eq!(words[0].text, "ab");
        assert_eq!(words[0].start, ms(0));
        assert_eq!(words[0].end, ms(2_000));
        assert_eq!(words[1].end, ms(5_000));
    }

    #[test]
    fn enhanced_word_tags_set_word_starts() {
        let words = split_words("<00:01.00>Hel <00:01.50>lo", ms(500), ms(2_500));
        assert_eq!(words.len(), 2);
        assert_eq!(
            (words[0].text.as_str(), words[0].start, words[0].end),
            ("Hel", ms(1_000), ms(1_500))
        );
        assert_eq!(
            (words[1].text.as_str(), words[1].start, words[1].end),
            ("lo", ms(1_500), ms(2_500))
        );
    }

    #[test]
    fn active_word_and_progress_follow_time() {
        let line = LyricLine {
            start: ms(0),
            end: ms(4_000),
            words: vec![
                LyricWord {
                    text: "a".into(),
                    start: ms(0),
                    end: ms(1_000),
                },
                LyricWord {
                    text: "b".into(),
                    start: ms(1_000),
                    end: ms(3_000),
                },
            ],
        };
        assert_eq!(active_word_index(&line, ms(500)), Some(0));
        assert_eq!(active_word_index(&line, ms(2_000)), Some(1));
        assert_eq!(active_word_index(&line, ms(3_500)), Some(1));
        assert!((word_progress(&line.words[1], ms(2_000)) - 0.5).abs() < 1e-6);
        assert_eq!(word_progress(&line.words[0], ms(2_000)), 1.0);
        assert_eq!(word_progress(&line.words[1], ms(500)), 0.0);
    }

    #[test]
    fn active_line_is_none_before_the_first_line() {
        let lines = parse_lrc("[00:05.00]one\n[00:09.00]two", None);
        assert_eq!(active_line_index(&lines, ms(1_000)), None);
        assert_eq!(active_line_index(&lines, ms(6_000)), Some(0));
        assert_eq!(active_line_index(&lines, ms(9_500)), Some(1));
    }

    #[test]
    fn percent_encoding_keeps_only_unreserved_characters() {
        assert_eq!(percent_encode("a b&c"), "a%20b%26c");
        assert_eq!(percent_encode("Az09-_.~"), "Az09-_.~");
    }
}
