//! Музыка и звуки (`pygame.mixer`). Если звуковое устройство недоступно, игра идёт молча.

use crate::embedded;
use crate::settings::MUSIC_VOLUME;
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player, Source};
use std::io::Cursor;

pub struct Audio {
    device: Option<MixerDeviceSink>,
    music: Option<Player>,
}

fn decode(file: &str) -> Option<Decoder<Cursor<&'static [u8]>>> {
    let bytes = embedded::file(&format!("resources/sound/{file}"));
    Decoder::new(Cursor::new(bytes))
        .map_err(|e| eprintln!("audio: {file}: {e}"))
        .ok()
}

impl Audio {
    pub fn open() -> Audio {
        let device = match DeviceSinkBuilder::open_default_sink() {
            Ok(mut device) => {
                device.log_on_drop(false);
                Some(device)
            }
            Err(e) => {
                eprintln!("audio disabled: {e}");
                None
            }
        };
        Audio {
            device,
            music: None,
        }
    }

    pub fn silent() -> Audio {
        Audio {
            device: None,
            music: None,
        }
    }

    /// `pygame.mixer.music.load(file); music.play(loops)` — всего `loops + 1` проигрываний.
    pub fn play_music(&mut self, file: &str, loops: usize) {
        self.stop_music();
        let Some(device) = &self.device else { return };
        let player = Player::connect_new(device.mixer());
        player.set_volume(MUSIC_VOLUME as _);
        for _ in 0..=loops {
            match decode(file) {
                Some(source) => player.append(source),
                None => break,
            }
        }
        self.music = Some(player);
    }

    pub fn stop_music(&mut self) {
        if let Some(music) = self.music.take() {
            music.stop();
        }
    }

    pub fn play_shot(&self) {
        if let Some(device) = &self.device
            && let Some(source) = decode("shotgun.mp3")
        {
            device.mixer().add(source.amplify(0.5));
        }
    }
}
