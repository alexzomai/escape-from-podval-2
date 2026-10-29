//! Музыка и звуки (`pygame.mixer`). Если звуковое устройство недоступно, игра идёт молча.

use crate::settings::MUSIC_VOLUME;
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player, Source};
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct Audio {
    device: Option<MixerDeviceSink>,
    music: Option<Player>,
    shot: Option<Arc<[u8]>>,
    sound_dir: PathBuf,
}

fn decode(bytes: &Arc<[u8]>) -> Option<Decoder<Cursor<Arc<[u8]>>>> {
    Decoder::new(Cursor::new(bytes.clone()))
        .map_err(|e| eprintln!("audio: {e}"))
        .ok()
}

impl Audio {
    pub fn new(root: &Path) -> Audio {
        let sound_dir = root.join("resources/sound");
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
        let shot = device
            .as_ref()
            .and_then(|_| std::fs::read(sound_dir.join("shotgun.mp3")).ok())
            .map(Arc::from);
        Audio {
            device,
            music: None,
            shot,
            sound_dir,
        }
    }

    pub fn silent() -> Audio {
        Audio {
            device: None,
            music: None,
            shot: None,
            sound_dir: PathBuf::new(),
        }
    }

    /// `pygame.mixer.music.load(file); music.play(loops)` — всего `loops + 1` проигрываний.
    pub fn play_music(&mut self, file: &str, loops: usize) {
        self.stop_music();
        let Some(device) = &self.device else { return };
        let bytes: Arc<[u8]> = match std::fs::read(self.sound_dir.join(file)) {
            Ok(b) => b.into(),
            Err(e) => return eprintln!("audio: {file}: {e}"),
        };
        let player = Player::connect_new(device.mixer());
        player.set_volume(MUSIC_VOLUME as _);
        for _ in 0..=loops {
            match decode(&bytes) {
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
        if let (Some(device), Some(bytes)) = (&self.device, &self.shot)
            && let Some(source) = decode(bytes)
        {
            device.mixer().add(source.amplify(0.5));
        }
    }
}
