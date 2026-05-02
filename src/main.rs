use config::Config;
use futures::{SinkExt, Stream};
use iced::widget::{
    Column, button, column, container, image as widgetImage, row, scrollable, text,
};
use iced::{Alignment, Element, Fill, Subscription, Task, Theme, stream};
use iced_fonts::LUCIDE_FONT_BYTES;
use iced_fonts::lucide::{pause, play, skip_back, skip_forward, square_minus};
use mpd_client::{LiveMpdClient, MpdClient};
use std::sync::Arc;

use crate::mpd_client::SongInfo;

mod config;
mod mpd_api;
mod mpd_client;

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
            config: config,
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
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let play_button_icon = if self.playing { pause() } else { play() };

        let art_row: Element<'_, Message> = if let Some(handle) = &self.album_art {
            widgetImage(handle.clone()).width(500).height(500).into()
        } else {
            text("").into()
        };

        let queue_list: Vec<Element<Message>> = self
            .queue
            .iter()
            .map(|song| {
                let label = format!("{} - {}", &song.album, &song.title);
                let is_current = song.position.unwrap_or(0) == self.position;
                let song_button = button(text(label))
                    .style(move |theme: &iced::Theme, _status| button::Style {
                        background: None,
                        text_color: if is_current {
                            theme.palette().danger
                        } else {
                            theme.palette().text
                        },
                        ..Default::default()
                    })
                    .width(Fill)
                    .on_press(Message::PlayQueueItem(song.position.unwrap_or(0)));
                let delete_button = button(square_minus())
                    .style(move |theme: &iced::Theme, status| {
                        let color = match status {
                            button::Status::Hovered => theme.palette().danger,
                            _ => theme.palette().text,
                        };

                        button::Style {
                            background: None,
                            text_color: color,
                            ..Default::default()
                        }
                    })
                    .on_press(Message::DeleteQueueItem(song.position.unwrap_or(0)));

                row![column![song_button], column![delete_button]].into()
            })
            .collect();

        container(column![
            container(
                column![
                    row![text(format!(
                        "{} - {} - {}",
                        &self.song_title, &self.artist, &self.album
                    ))]
                    .spacing(10),
                    row![
                        button(skip_back().style(|theme: &iced::Theme| {
                            text::Style {
                                color: Some(theme.palette().primary),
                            }
                        }))
                        .style(|_theme: &iced::Theme, _status| {
                            button::Style {
                                background: None,
                                ..Default::default()
                            }
                        })
                        .on_press(Message::PreviousSong),
                        button(play_button_icon.style(|theme: &iced::Theme| {
                            text::Style {
                                color: Some(theme.palette().primary),
                            }
                        }))
                        .style(|_theme: &iced::Theme, _status| {
                            button::Style {
                                background: None,
                                ..Default::default()
                            }
                        })
                        .on_press(Message::TogglePlay),
                        button(skip_forward().style(|theme: &iced::Theme| {
                            text::Style {
                                color: Some(theme.palette().primary),
                            }
                        }))
                        .style(|_theme: &iced::Theme, _status| {
                            button::Style {
                                background: None,
                                ..Default::default()
                            }
                        })
                        .on_press(Message::NextSong)
                    ]
                    .spacing(10)
                ]
                .align_x(Alignment::Center)
                .spacing(10)
            )
            .padding(10)
            .center_x(Fill),
            container(
                row![
                    art_row,
                    scrollable(Column::with_children(queue_list)).height(500)
                ]
                .spacing(10)
                .height(Fill),
            )
            .center_x(Fill),
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
        let config = Config::load().unwrap_or_default();
        let client = LiveMpdClient::new(&config.mpd_address);
        Self::new(Arc::new(client), config)
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
    let min_window_size = iced::window::Settings {
        min_size: Some(iced::Size::new(800.0, 600.0)),
        ..Default::default()
    };
    iced::application(SongData::default, SongData::update, SongData::view)
        .subscription(SongData::subscription)
        .window(min_window_size)
        .theme(|state: &SongData| theme_from_string(&state.config.theme))
        .font(LUCIDE_FONT_BYTES)
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

fn theme_from_string(theme: &str) -> iced::Theme {
    Theme::ALL
        .iter()
        .find(|t| t.to_string() == theme)
        .cloned()
        .unwrap_or(Theme::Moonfly)
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
    fn test_play_queue_position() {
        let (mock, log) = MockMpdClient::new(test_song_info());
        let mut state = SongData::new(Arc::new(mock));
        let _ = state.update(Message::PlayQueueItem(1));
        assert_eq!(log.lock().unwrap().get_song_info, 1);
    }

    #[test]
    fn test_delete_queue_position() {
        let (mock, log) = MockMpdClient::new(test_song_info());
        let mut state = SongData::new(Arc::new(mock));
        let _ = state.update(Message::DeleteQueueItem(1));
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
