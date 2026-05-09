use crate::mpd_client::SongInfo;
use config::Config;
use futures::{SinkExt, Stream};
use iced::widget::image as widgetImage;
use iced::{Subscription, Task, stream};
use iced_fonts::LUCIDE_FONT_BYTES;
use mpd_client::{LiveMpdClient, MpdClient};
use std::sync::Arc;
use view::decode_album_art;
use view::theme_from_string;

mod config;
mod mpd_api;
mod mpd_client;
mod view;

#[derive(Debug, Clone)]
enum Message {
    TogglePlay,
    NextSong,
    PreviousSong,
    RefreshDisplay,
    RefreshSongInfo,
    SongInfoLoaded(SongInfo),
    RefreshAlbumArt,
    AlbumArtLoaded(Option<widgetImage::Handle>),
    RefreshSongQueue,
    SongQueueLoaded(Vec<SongInfo>),
    PlayQueueItem(u32),
    DeleteQueueItem(u32),
    ThemeChanged(iced::Theme),
    AddressInputChanged(String),
    AddressConfirmed,
}

struct SongData {
    song_title: String,
    album: String,
    artist: String,
    playing: bool,
    position: u32,
    album_art: Option<widgetImage::Handle>,
    client: Arc<dyn MpdClient>,
    queue: Vec<SongInfo>,
    config: Config,
    address_input: String,
}

impl SongData {
    fn new(client: Arc<dyn MpdClient>, config: Config) -> Self {
        config.save().ok();
        Self {
            song_title: String::new(),
            album: String::new(),
            artist: String::new(),
            playing: false,
            position: 0,
            album_art: None,
            client,
            queue: vec![],
            address_input: config.mpd_address.clone(),
            config,
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::TogglePlay => {
                self.client.toggle_play();
                Task::done(Message::RefreshSongInfo)
            }
            Message::NextSong => {
                self.client.next_song();
                Task::done(Message::RefreshSongInfo)
            }
            Message::PreviousSong => {
                self.client.previous_song();
                Task::done(Message::RefreshSongInfo)
            }
            Message::RefreshSongInfo => {
                let client = Arc::clone(&self.client);
                Task::perform(
                    async move { client.get_song_info() },
                    Message::SongInfoLoaded,
                )
            }
            Message::SongInfoLoaded(song_info) => {
                self.playing = song_info.playing;
                self.song_title = song_info.title;
                self.artist = song_info.artist;
                self.album = song_info.album;
                self.position = song_info.position.unwrap_or(0);
                Task::none()
            }
            Message::RefreshDisplay => {
                let tasks = vec![
                    Task::done(Message::RefreshSongInfo),
                    Task::done(Message::RefreshAlbumArt),
                    Task::done(Message::RefreshSongQueue),
                ];
                Task::batch(tasks)
            }
            Message::RefreshAlbumArt => {
                let client = Arc::clone(&self.client);
                Task::perform(
                    async move {
                        let bytes = client.get_album_art_bytes();
                        decode_album_art(bytes)
                    },
                    Message::AlbumArtLoaded,
                )
            }
            Message::AlbumArtLoaded(handle) => {
                self.album_art = handle;
                Task::none()
            }
            Message::RefreshSongQueue => {
                let client = Arc::clone(&self.client);
                Task::perform(async move { client.get_queue() }, Message::SongQueueLoaded)
            }
            Message::SongQueueLoaded(song_queue) => {
                self.queue = song_queue;
                Task::none()
            }
            Message::PlayQueueItem(position) => {
                let client = Arc::clone(&self.client);
                client.play_queue_position(position);
                Task::done(Message::RefreshSongInfo)
            }
            Message::DeleteQueueItem(position) => {
                let client = Arc::clone(&self.client);
                client.delete_queue_position(position);
                Task::done(Message::RefreshSongQueue)
            }
            Message::ThemeChanged(theme) => {
                let theme_name = theme.to_string();
                self.config.theme = theme_name;
                self.config.save().ok();
                Task::none()
            }
            Message::AddressInputChanged(address) => {
                self.address_input = address;
                Task::none()
            }
            Message::AddressConfirmed => {
                self.config.mpd_address = self.address_input.clone();
                self.config.save().ok();
                Task::done(Message::RefreshDisplay)
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let addr = Arc::new(self.config.mpd_address.clone());
        Subscription::run_with(addr, |a| player_change_listener(a.clone()))
    }
}

impl Default for SongData {
    fn default() -> Self {
        let config = Config::load().unwrap_or_default();
        let client = LiveMpdClient::new(&config.mpd_address);
        Self::new(Arc::new(client), config)
    }
}

fn main() -> iced::Result {
    let min_window_size = iced::window::Settings {
        min_size: Some(iced::Size::new(800.0, 600.0)),
        ..Default::default()
    };
    iced::application(SongData::default, SongData::update, view::view)
        .subscription(SongData::subscription)
        .window(min_window_size)
        .theme(|state: &SongData| theme_from_string(&state.config.theme))
        .font(LUCIDE_FONT_BYTES)
        .run()
}

fn player_change_listener(addr: Arc<String>) -> impl Stream<Item = Message> {
    stream::channel(100, async move |mut output| {
        output
            .send(Message::RefreshDisplay)
            .await
            .expect("Failed to send initial refresh");

        loop {
            let addr = addr.clone();
            let player_changed = tokio::task::spawn_blocking(move || {
                mpd_api::check_player_change(&addr);
            });

            let _result = player_changed.await;

            output
                .send(Message::RefreshDisplay)
                .await
                .expect("Failed to send player change notification");
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use mpd_client::{SongInfo, mock::MockMpdClient};

    fn test_song_info() -> SongInfo {
        SongInfo {
            title: "Test Song".to_string(),
            artist: "Test Artist".to_string(),
            album: "Test Album".to_string(),
            playing: true,
            position: Some(0),
        }
    }

    fn small_png_bytes() -> Vec<u8> {
        use image::{DynamicImage, RgbaImage};
        use std::io::Cursor;
        let img = RgbaImage::from_pixel(2, 2, image::Rgba([255, 100, 100, 255]));
        let mut buf = Cursor::new(Vec::new());
        DynamicImage::ImageRgba8(img)
            .write_to(&mut buf, image::ImageFormat::Png)
            .expect("failed to encode test PNG");
        buf.into_inner()
    }

    // --- SongData::update ---

    #[test]
    fn test_toggle_play_calls_client() {
        let (mock, log) = MockMpdClient::new(test_song_info());
        let mut state = SongData::new(Arc::new(mock), Config::default());
        let _ = state.update(Message::TogglePlay);
        assert_eq!(log.lock().unwrap().toggle_play, 1);
    }

    #[test]
    fn test_next_song_calls_client() {
        let (mock, log) = MockMpdClient::new(test_song_info());
        let mut state = SongData::new(Arc::new(mock), Config::default());
        let _ = state.update(Message::NextSong);
        assert_eq!(log.lock().unwrap().next_song, 1);
    }

    #[test]
    fn test_previous_song_calls_client() {
        let (mock, log) = MockMpdClient::new(test_song_info());
        let mut state = SongData::new(Arc::new(mock), Config::default());
        let _ = state.update(Message::PreviousSong);
        assert_eq!(log.lock().unwrap().previous_song, 1);
    }

    #[test]
    fn test_refresh_display_updates_song_fields() {
        let (mock, _log) = MockMpdClient::new(test_song_info());
        let mut state = SongData::new(Arc::new(mock), Config::default());
        let _ = state.update(Message::RefreshSongInfo);
        let _ = state.update(Message::SongInfoLoaded(test_song_info()));
        assert_eq!(state.song_title, "Test Song");
        assert_eq!(state.artist, "Test Artist");
        assert_eq!(state.album, "Test Album");
        assert!(state.playing);
    }

    #[test]
    fn test_play_queue_position() {
        let (mock, log) = MockMpdClient::new(test_song_info());
        let mut state = SongData::new(Arc::new(mock), Config::default());
        let _ = state.update(Message::PlayQueueItem(1));
        assert_eq!(log.lock().unwrap().play_queue_postion, 1);
    }

    #[test]
    fn test_delete_queue_position() {
        let (mock, log) = MockMpdClient::new(test_song_info());
        let mut state = SongData::new(Arc::new(mock), Config::default());
        let _ = state.update(Message::DeleteQueueItem(1));
        assert_eq!(log.lock().unwrap().delete_queue_position, 1);
    }

    #[test]
    fn test_album_art_loaded_some_sets_handle() {
        let (mock, _log) = MockMpdClient::new(test_song_info());
        let mut state = SongData::new(Arc::new(mock), Config::default());
        let handle = widgetImage::Handle::from_rgba(1, 1, vec![0, 0, 0, 255]);
        let _ = state.update(Message::AlbumArtLoaded(Some(handle)));
        assert!(state.album_art.is_some());
    }

    #[test]
    fn test_album_art_loaded_none_clears_handle() {
        let (mock, _log) = MockMpdClient::new(test_song_info());
        let mut state = SongData::new(Arc::new(mock), Config::default());
        let handle = widgetImage::Handle::from_rgba(1, 1, vec![0, 0, 0, 255]);
        let _ = state.update(Message::AlbumArtLoaded(Some(handle)));
        let _ = state.update(Message::AlbumArtLoaded(None));
        assert!(state.album_art.is_none());
    }

    // --- decode_album_art ---

    #[test]
    fn test_decode_album_art_empty_bytes_returns_none() {
        assert!(decode_album_art(vec![]).is_none());
    }

    #[test]
    fn test_decode_album_art_invalid_bytes_returns_none() {
        assert!(decode_album_art(b"not an image".to_vec()).is_none());
    }

    #[test]
    fn test_decode_album_art_valid_png_returns_handle() {
        let bytes = small_png_bytes();
        assert!(decode_album_art(bytes).is_some());
    }
}
