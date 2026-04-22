use futures::{SinkExt, Stream};
use iced::widget::{button, column, container, image as widgetImage, row, text};
use iced::{Element, Fill, Subscription, Task, stream};
use mpd_client::{LiveMpdClient, MpdClient};
use std::sync::Arc;

use crate::mpd_client::SongInfo;

mod mpd_api;
mod mpd_client;

#[derive(Debug, Clone)]
enum Message {
    TogglePlay,
    NextSong,
    PreviousSong,
    RefreshDisplay,
    RefreshSongInfo,
    RefreshAlbumArt,
    SongInfoLoaded(SongInfo),
    AlbumArtLoaded(Option<widgetImage::Handle>),
}

struct SongData {
    song_title: String,
    album: String,
    artist: String,
    playing: bool,
    album_art: Option<widgetImage::Handle>,
    client: Arc<dyn MpdClient>,
}

impl SongData {
    fn new(client: Arc<dyn MpdClient>) -> Self {
        Self {
            song_title: String::new(),
            album: String::new(),
            artist: String::new(),
            playing: false,
            album_art: None,
            client,
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
                Task::none()
            }
            Message::RefreshDisplay => {
                let tasks = vec![
                    Task::done(Message::RefreshSongInfo),
                    Task::done(Message::RefreshAlbumArt),
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
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let play_button_icon = if self.playing { "[ ]" } else { " > " };

        let art_row: Element<'_, Message> = if let Some(handle) = &self.album_art {
            widgetImage(handle.clone()).width(500).height(500).into()
        } else {
            text("").into()
        };

        container(column![
            art_row,
            row![text(&self.song_title)],
            row![text(&self.artist)],
            row![text(&self.album)],
            row![
                button(" << ").on_press(Message::PreviousSong),
                button(play_button_icon).on_press(Message::TogglePlay),
                button(" >> ").on_press(Message::NextSong),
            ]
            .spacing(10)
        ])
        .padding(10)
        .center_x(Fill)
        .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::run(player_change_listener)
    }
}

impl Default for SongData {
    fn default() -> Self {
        Self::new(Arc::new(LiveMpdClient::new("10.0.0.111:6600")))
    }
}

fn decode_album_art(bytes: Vec<u8>) -> Option<widgetImage::Handle> {
    if bytes.is_empty() {
        return None;
    }
    image::load_from_memory(&bytes).ok().map(|img| {
        let rgba = img.to_rgba8();
        let (width, height) = (rgba.width(), rgba.height());
        widgetImage::Handle::from_rgba(width, height, rgba.into_raw())
    })
}

fn main() -> iced::Result {
    iced::application(SongData::default, SongData::update, SongData::view)
        .subscription(SongData::subscription)
        .run()
}

fn player_change_listener() -> impl Stream<Item = Message> {
    stream::channel(100, async |mut output| {
        output
            .send(Message::RefreshDisplay)
            .await
            .expect("Failed to send initial refresh");

        loop {
            let player_changed = tokio::task::spawn_blocking(|| {
                mpd_api::check_player_change();
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
        let mut state = SongData::new(Arc::new(mock));
        let _ = state.update(Message::TogglePlay);
        assert_eq!(log.lock().unwrap().toggle_play, 1);
    }

    #[test]
    fn test_next_song_calls_client() {
        let (mock, log) = MockMpdClient::new(test_song_info());
        let mut state = SongData::new(Arc::new(mock));
        let _ = state.update(Message::NextSong);
        assert_eq!(log.lock().unwrap().next_song, 1);
    }

    #[test]
    fn test_previous_song_calls_client() {
        let (mock, log) = MockMpdClient::new(test_song_info());
        let mut state = SongData::new(Arc::new(mock));
        let _ = state.update(Message::PreviousSong);
        assert_eq!(log.lock().unwrap().previous_song, 1);
    }

    #[test]
    fn test_refresh_display_updates_song_fields() {
        let (mock, _log) = MockMpdClient::new(test_song_info());
        let mut state = SongData::new(Arc::new(mock));
        let _ = state.update(Message::RefreshDisplay);
        assert_eq!(state.song_title, "Test Song");
        assert_eq!(state.artist, "Test Artist");
        assert_eq!(state.album, "Test Album");
        assert!(state.playing);
    }

    #[test]
    fn test_refresh_display_calls_get_song_info() {
        let (mock, log) = MockMpdClient::new(test_song_info());
        let mut state = SongData::new(Arc::new(mock));
        let _ = state.update(Message::RefreshDisplay);
        assert_eq!(log.lock().unwrap().get_song_info, 1);
    }

    #[test]
    fn test_album_art_loaded_some_sets_handle() {
        let (mock, _log) = MockMpdClient::new(test_song_info());
        let mut state = SongData::new(Arc::new(mock));
        let handle = widgetImage::Handle::from_rgba(1, 1, vec![0, 0, 0, 255]);
        let _ = state.update(Message::AlbumArtLoaded(Some(handle)));
        assert!(state.album_art.is_some());
    }

    #[test]
    fn test_album_art_loaded_none_clears_handle() {
        let (mock, _log) = MockMpdClient::new(test_song_info());
        let mut state = SongData::new(Arc::new(mock));
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
