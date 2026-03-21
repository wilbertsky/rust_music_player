use mpd::{Client, Idle};

pub fn check_player_change() {
    let mut client = Client::connect("127.0.0.1:6600").unwrap();
    client.idle(&[mpd::idle::Subsystem::Player]).ok();
}

#[cfg(test)]
mod tests {
    use crate::mpd_client::{LiveMpdClient, MpdClient};

    #[test]
    #[ignore = "requires a running MPD server at 127.0.0.1:6600"]
    fn integration_get_song_info_does_not_panic() {
        let client = LiveMpdClient::new("127.0.0.1:6600");
        let info = client.get_song_info();
        assert!(!info.title.is_empty());
    }

    #[test]
    #[ignore = "requires a running MPD server at 127.0.0.1:6600"]
    fn integration_toggle_play_changes_playing_state() {
        let client = LiveMpdClient::new("127.0.0.1:6600");
        let before = client.get_song_info().playing;
        client.toggle_play();
        let after = client.get_song_info().playing;
        assert_ne!(before, after);
        client.toggle_play(); // restore original state
    }

    #[test]
    #[ignore = "requires a running MPD server at 127.0.0.1:6600"]
    fn integration_album_art_bytes_are_valid_image_format() {
        let client = LiveMpdClient::new("127.0.0.1:6600");
        let bytes = client.get_album_art_bytes();
        if !bytes.is_empty() {
            let is_jpeg = bytes.starts_with(&[0xFF, 0xD8, 0xFF]);
            let is_png = bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]);
            assert!(is_jpeg || is_png, "album art must be JPEG or PNG, got: {:02X?}", &bytes[..4]);
        }
    }
}
