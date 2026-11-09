 
 
 
 
 
 
 
 

use std::sync::{Arc, Mutex, Once};
use std::time::{Duration, Instant};

 
const POLL_INTERVAL: Duration = Duration::from_millis(500);

 
const MAX_ART_BYTES: u64 = 4 * 1024 * 1024;

 
#[derive(Clone, Debug)]
pub struct Playback {
    pub title: String,
    pub artist: String,
    pub playing: bool,
    pub duration: Duration,
     
    base: Duration,
    polled_at: Instant,
     
    pub art: Option<Arc<[u8]>>,
     
    pub art_generation: u64,
}

impl Playback {
     
     
    pub fn position_at(&self, now: Instant) -> Duration {
        let elapsed = now.saturating_duration_since(self.polled_at);
        interpolate(self.base, elapsed, self.playing, self.duration)
    }
}

 
 
pub fn interpolate(
    base: Duration,
    elapsed: Duration,
    playing: bool,
    duration: Duration,
) -> Duration {
    let position = if playing { base + elapsed } else { base };
    if duration.is_zero() {
        position
    } else {
        position.min(duration)
    }
}

 
pub fn format_clock(time: Duration) -> String {
    let secs = time.as_secs();
    if secs >= 3600 {
        format!("{}:{:02}:{:02}", secs / 3600, (secs / 60) % 60, secs % 60)
    } else {
        format!("{}:{:02}", secs / 60, secs % 60)
    }
}

static SNAPSHOT: Mutex<Option<Playback>> = Mutex::new(None);
static START: Once = Once::new();

 
pub fn current() -> Option<Playback> {
    SNAPSHOT
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}

 
 
pub fn ensure_poller() {
    START.call_once(|| {
        #[cfg(windows)]
        {
            let spawned = std::thread::Builder::new()
                .name("now-playing".into())
                .spawn(platform::run);
            if let Err(error) = spawned {
                log::warn!("now-playing poller failed to start: {error}");
            }
        }
    });
}

fn store(playback: Option<Playback>) {
    *SNAPSHOT
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = playback;
}

#[cfg(windows)]
mod platform {
    use super::{interpolate, store, Playback, MAX_ART_BYTES, POLL_INTERVAL};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

     
     
    const RESYNC_TOLERANCE: Duration = Duration::from_millis(1500);

    use windows::Media::Control::{
        GlobalSystemMediaTransportControlsSession as Session,
        GlobalSystemMediaTransportControlsSessionManager as Manager,
        GlobalSystemMediaTransportControlsSessionMediaProperties as MediaProperties,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus as PlaybackStatus,
    };
    use windows::Storage::Streams::DataReader;
    use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};

     
    const BLEND: f64 = 0.5;

     
    #[derive(Clone, Copy)]
    struct Anchor {
        base: Duration,
        at: Instant,
        playing: bool,
    }

     
    #[derive(Default)]
    struct PollState {
        track: Option<(String, String)>,
        art: Option<Arc<[u8]>>,
        generation: u64,
        anchor: Option<Anchor>,
        last_reported: Option<Duration>,
    }

    pub fn run() {
         
         
        let _ = unsafe { RoInitialize(RO_INIT_MULTITHREADED) };

        let mut state = PollState::default();
        loop {
            match poll_once(&mut state) {
                Ok(playback) => store(playback),
                Err(error) => {
                    log::debug!("now-playing poll failed: {error}");
                    store(None);
                }
            }
            std::thread::sleep(POLL_INTERVAL);
        }
    }

    fn poll_once(state: &mut PollState) -> windows::core::Result<Option<Playback>> {
        let manager: Manager = Manager::RequestAsync()?.get()?;
        let Some(session) = find_spotify(&manager)? else {
            state.track = None;
            return Ok(None);
        };

        let props: MediaProperties = session.TryGetMediaPropertiesAsync()?.get()?;
        let title = props.Title()?.to_string();
        let artist = props.Artist()?.to_string();
        if title.is_empty() {
            return Ok(None);
        }

        let track = (title.clone(), artist.clone());
        let track_changed = state.track.as_ref() != Some(&track);
        if track_changed {
            state.track = Some(track);
            state.generation += 1;
            state.art = read_art(&props).map(Arc::from);
        }

        let playing = session.GetPlaybackInfo()?.PlaybackStatus()? == PlaybackStatus::Playing;
        let timeline = session.GetTimelineProperties()?;
        let start = timeline.StartTime()?.Duration;
        let reported = ticks(timeline.Position()?.Duration - start);
        let duration = ticks(timeline.EndTime()?.Duration - start);

         
         
         
         
        let now = Instant::now();
        let anchor = match state.anchor {
            Some(anchor) if !track_changed && anchor.playing == playing => {
                let predicted = interpolate(
                    anchor.base,
                    now.saturating_duration_since(anchor.at),
                    playing,
                    duration,
                );
                let fresh = state.last_reported != Some(reported);
                let drift = if predicted > reported {
                    predicted - reported
                } else {
                    reported - predicted
                };
                if !fresh {
                     
                    anchor
                } else if drift <= RESYNC_TOLERANCE {
                     
                    let eased = predicted.as_secs_f64()
                        + (reported.as_secs_f64() - predicted.as_secs_f64()) * BLEND;
                    Anchor {
                        base: Duration::from_secs_f64(eased.max(0.0)),
                        at: now,
                        playing,
                    }
                } else {
                    Anchor {
                        base: reported,
                        at: now,
                        playing,
                    }
                }
            }
            _ => Anchor {
                base: reported,
                at: now,
                playing,
            },
        };
        state.anchor = Some(anchor);
        state.last_reported = Some(reported);

        Ok(Some(Playback {
            title,
            artist,
            playing,
            duration,
            base: anchor.base,
            polled_at: anchor.at,
            art: state.art.clone(),
            art_generation: state.generation,
        }))
    }

     
    fn find_spotify(manager: &Manager) -> windows::core::Result<Option<Session>> {
        let sessions = manager.GetSessions()?;
        for index in 0..sessions.Size()? {
            let session = sessions.GetAt(index)?;
            let app_id = session.SourceAppUserModelId()?.to_string().to_lowercase();
            if app_id.contains("spotify") {
                return Ok(Some(session));
            }
        }
        Ok(None)
    }

     
    fn read_art(props: &MediaProperties) -> Option<Vec<u8>> {
        let read = || -> windows::core::Result<Vec<u8>> {
            let stream = props.Thumbnail()?.OpenReadAsync()?.get()?;
            let size = stream.Size()?;
            if size == 0 || size > MAX_ART_BYTES {
                return Ok(Vec::new());
            }
            let reader = DataReader::CreateDataReader(&stream)?;
            let loaded = reader.LoadAsync(size as u32)?.get()?;
            let mut bytes = vec![0u8; loaded as usize];
            reader.ReadBytes(&mut bytes)?;
            Ok(bytes)
        };
        read().ok().filter(|bytes| !bytes.is_empty())
    }

     
    fn ticks(value: i64) -> Duration {
        Duration::from_nanos(value.max(0) as u64 * 100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secs(value: u64) -> Duration {
        Duration::from_secs(value)
    }

    #[test]
    fn format_clock_uses_minutes_and_hours() {
        assert_eq!(format_clock(secs(0)), "0:00");
        assert_eq!(format_clock(secs(9)), "0:09");
        assert_eq!(format_clock(secs(65)), "1:05");
        assert_eq!(format_clock(secs(3599)), "59:59");
        assert_eq!(format_clock(secs(3600)), "1:00:00");
        assert_eq!(format_clock(secs(3725)), "1:02:05");
    }

    #[test]
    fn interpolate_advances_only_while_playing() {
        let base = Duration::from_millis(10_000);
        let elapsed = Duration::from_millis(2_500);
        assert_eq!(
            interpolate(base, elapsed, true, secs(100)),
            Duration::from_millis(12_500)
        );
        assert_eq!(interpolate(base, elapsed, false, secs(100)), base);
    }

    #[test]
    fn interpolate_clamps_to_the_track_length() {
        let base = secs(99);
        assert_eq!(interpolate(base, secs(5), true, secs(100)), secs(100));
    }

    #[test]
    fn interpolate_without_a_known_length_does_not_clamp() {
        assert_eq!(
            interpolate(secs(99), secs(5), true, Duration::ZERO),
            secs(104)
        );
    }
}
