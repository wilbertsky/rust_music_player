use mpd::{Client, State};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct SongInfo {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub playing: bool,
    pub position: Option<u32>,
}

pub trait MpdClient: Send + Sync + 'static {
    fn get_song_info(&self) -> SongInfo;
    fn get_album_art_bytes(&self) -> Vec<u8>;
    fn toggle_play(&self);
    fn next_song(&self);
    fn previous_song(&self);
    fn get_queue(&self) -> Vec<SongInfo>;
    fn play_queue_position(&self, position: u32);
    fn delete_queue_position(&self, position: u32);
}

pub struct LiveMpdClient {
    addr: String,
}

impl LiveMpdClient {
    pub fn new(addr: impl Into<String>) -> Self {
        Self { addr: addr.into() }
    }

    fn connect(&self) -> Option<Client> {
        Client::connect(&self.addr).ok()
    }
}

impl MpdClient for LiveMpdClient {
    fn get_song_info(&self) -> SongInfo {
        let mut client = match self.connect() {
            Some(c) => c,
            None => {
                return SongInfo {
                    title: "Disconnected".to_string(),
                    artist: String::new(),
                    album: String::new(),
                    playing: false,
                    position: None,
                };
            }
        };

        let playing = client
            .status()
            .map(|s| s.state == State::Play)
            .unwrap_or(false);
        let current_song = client.currentsong().unwrap_or(None);

        let (title, artist, album, position) = if let Some(song) = current_song {
            let title = song.title.unwrap_or_else(|| "Unknown".to_string());
            let artist = song.artist.unwrap_or_else(|| "Unknown".to_string());
            let tags: HashMap<String, String> = song.tags.into_iter().collect();
            let album = tags
                .get("Album")
                .cloned()
                .unwrap_or_else(|| "Album Unknown".to_string());
            let position = song.place.map(|p| p.pos);
            (title, artist, album, position)
        } else {
            (
                "Unknown".to_string(),
                "Unknown".to_string(),
                "Album Unknown".to_string(),
                None,
            )
        };

        SongInfo {
            title,
            artist,
            album,
            playing,
            position,
        }
    }

    fn get_album_art_bytes(&self) -> Vec<u8> {
        let mut client = match self.connect() {
            Some(c) => c,
            None => return vec![],
        };
        if let Some(song) = client.currentsong().unwrap_or(None) {
            client.albumart(&song).unwrap_or_default()
        } else {
            vec![]
        }
    }

    fn toggle_play(&self) {
        let mut client = match self.connect() {
            Some(c) => c,
            None => return,
        };
        match client.status().map(|s| s.state) {
            Ok(State::Play) => {
                client.pause(true).ok();
            }
            Ok(_) => {
                client.play().ok();
            }
            Err(_) => {}
        }
    }

    fn next_song(&self) {
        let mut client = match self.connect() {
            Some(c) => c,
            None => return,
        };
        client.next().ok();
    }

    fn previous_song(&self) {
        let mut client = match self.connect() {
            Some(c) => c,
            None => return,
        };
        client.prev().ok();
    }

    fn play_queue_position(&self, position: u32) {
        let mut client = match self.connect() {
            Some(c) => c,
            None => return,
        };

        let _result = client.switch(position);
    }

    fn delete_queue_position(&self, position: u32) {
        let mut client = match self.connect() {
            Some(c) => c,
            None => return,
        };

        let _result = client.delete(position);
    }

    fn get_queue(&self) -> Vec<SongInfo> {
        let mut client = match self.connect() {
            Some(c) => c,
            None => return vec![],
        };

        let queue = client.queue().unwrap_or_default();

        queue
            .into_iter()
            .map(|song| {
                let title = song.title.unwrap_or_else(|| "Unknown".to_string());
                let artist = song.artist.unwrap_or_else(|| "Unknown".to_string());
                let tags: HashMap<String, String> = song.tags.into_iter().collect();
                let album = tags
                    .get("Album")
                    .cloned()
                    .unwrap_or_else(|| "Album Unknown".to_string());
                let playing = false;
                let position = song.place.map(|p| p.pos);

                SongInfo {
                    title,
                    artist,
                    album,
                    playing,
                    position,
                }
            })
            .collect()
    }
}

#[cfg(test)]
pub mod mock {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    pub struct CallLog {
        pub toggle_play: usize,
        pub next_song: usize,
        pub previous_song: usize,
        pub get_song_info: usize,
        pub get_album_art_bytes: usize,
        pub queue: usize,
        pub get_queue: Vec<SongInfo>,
        pub play_queue_postion: usize,
        pub delete_queue_position: usize,
    }

    pub struct MockMpdClient {
        pub log: Arc<Mutex<CallLog>>,
        pub song_info: SongInfo,
        pub album_art_bytes: Vec<u8>,
    }

    impl MockMpdClient {
        pub fn new(song_info: SongInfo) -> (Self, Arc<Mutex<CallLog>>) {
            let log = Arc::new(Mutex::new(CallLog::default()));
            let mock = Self {
                log: Arc::clone(&log),
                song_info,
                album_art_bytes: vec![],
            };
            (mock, log)
        }

        pub fn with_album_art(mut self, bytes: Vec<u8>) -> Self {
            self.album_art_bytes = bytes;
            self
        }
    }

    impl MpdClient for MockMpdClient {
        fn get_song_info(&self) -> SongInfo {
            self.log.lock().unwrap().get_song_info += 1;
            self.song_info.clone()
        }

        fn get_album_art_bytes(&self) -> Vec<u8> {
            self.log.lock().unwrap().get_album_art_bytes += 1;
            self.album_art_bytes.clone()
        }

        fn toggle_play(&self) {
            self.log.lock().unwrap().toggle_play += 1;
        }

        fn next_song(&self) {
            self.log.lock().unwrap().next_song += 1;
        }

        fn previous_song(&self) {
            self.log.lock().unwrap().previous_song += 1;
        }

        fn get_queue(&self) -> Vec<SongInfo> {
            self.log.lock().unwrap().queue += 1;
            let song_info = SongInfo {
                title: "Disconnected".to_string(),
                artist: String::new(),
                album: String::new(),
                playing: false,
                position: None,
            };

            vec![song_info]
        }

        fn delete_queue_position(&self, _position: u32) {
            self.log.lock().unwrap().delete_queue_position += 1;
        }

        fn play_queue_position(&self, _position: u32) {
            self.log.lock().unwrap().play_queue_postion += 1;
        }
    }
}
