use mpd::{Client, Idle};

pub fn check_player_change(addr: &str) {
    match Client::connect(addr) {
        Ok(mut client) => {
            client.wait(&[mpd::idle::Subsystem::Player]).ok();
        }
        Err(_) => {
            std::thread::sleep(std::time::Duration::from_secs(5));
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Config;
    use crate::mpd_client::{LiveMpdClient, MpdClient};

    // Get the config settings.

    #[test]
    #[ignore = "requires a running MPD server at Config::default address."]
    fn integration_get_song_info_does_not_panic() {
        let config = Config::default();
        let client = LiveMpdClient::new(config.mpd_address);
        let info = client.get_song_info();
        assert!(!info.title.is_empty());
    }

    #[test]
    #[ignore = "requires a running MPD server at Config::default address."]
    fn integration_toggle_play_changes_playing_state() {
        let config = Config::default();
        let client = LiveMpdClient::new(config.mpd_address);
        let before = client.get_song_info().playing;
        client.toggle_play();
        let after = client.get_song_info().playing;
        assert_ne!(before, after);
        client.toggle_play(); // restore original state
    }

    #[test]
    #[ignore = "requires a running MPD server at Config::default address."]
    fn integration_album_art_bytes_are_valid_image_format() {
        let config = Config::default();
        let client = LiveMpdClient::new(config.mpd_address);
        let bytes = client.get_album_art_bytes();
        if !bytes.is_empty() {
            let is_jpeg = bytes.starts_with(&[0xFF, 0xD8, 0xFF]);
            let is_png = bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]);
            assert!(
                is_jpeg || is_png,
                "album art must be JPEG or PNG, got: {:02X?}",
                &bytes[..4]
            );
        }
    }

    #[test]
    #[ignore = "requires a running MPD server at Config::default address. Also, a queue for testing"]
    fn integration_get_queue() {
        let config = Config::default();
        let client = LiveMpdClient::new(config.mpd_address);
        let queue = client.get_queue();
        if !queue.is_empty() {
            let song_info = queue.first().unwrap();
            assert!(!song_info.title.is_empty());
            assert!(!song_info.artist.is_empty());
            assert!(!song_info.album.is_empty());
        }
    }
}
