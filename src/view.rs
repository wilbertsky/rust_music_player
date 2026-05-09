use iced::Element;
use iced::Theme;
use iced::widget::{
    Column, button, column, container, image as widgetImage, pick_list, row, scrollable, text,
    text_input,
};
use iced::{Alignment, Fill};
use iced_fonts::lucide::{pause, play, skip_back, skip_forward, square_minus};

use crate::{Message, SongData};

pub fn decode_album_art(bytes: Vec<u8>) -> Option<widgetImage::Handle> {
    if bytes.is_empty() {
        return None;
    }
    image::load_from_memory(&bytes).ok().map(|img| {
        let rgba = img.to_rgba8();
        let (width, height) = (rgba.width(), rgba.height());
        widgetImage::Handle::from_rgba(width, height, rgba.into_raw())
    })
}

pub fn theme_from_string(theme: &str) -> iced::Theme {
    Theme::ALL
        .iter()
        .find(|t| t.to_string() == theme)
        .cloned()
        .unwrap_or(Theme::Moonfly)
}

pub fn view(state: &SongData) -> Element<'_, Message> {
    let play_button_icon = if state.playing { pause() } else { play() };

    let art_row: Element<'_, Message> = if let Some(handle) = &state.album_art {
        widgetImage(handle.clone()).width(500).height(500).into()
    } else {
        text("").into()
    };

    let queue_list: Vec<Element<Message>> = state
        .queue
        .iter()
        .map(|song| {
            let label = format!("{} - {}", &song.album, &song.title);
            let is_current = song.position.unwrap_or(0) == state.position;
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
                row![
                    text(format!(
                        "{} - {} - {}",
                        &state.song_title, &state.artist, &state.album,
                    )),
                    pick_list(
                        iced::Theme::ALL,
                        Some(theme_from_string(&state.config.theme)),
                        Message::ThemeChanged,
                    ),
                    text_input("Enter MPD address including port", &state.address_input)
                        .on_input(Message::AddressInputChanged),
                    button("Update MPD Address").on_press(Message::AddressConfirmed)
                ]
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
