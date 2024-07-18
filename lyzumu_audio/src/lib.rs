use anyhow::Result;

use kira::{
    manager::{backend::DefaultBackend, AudioManager, AudioManagerSettings},
    sound::static_sound::{StaticSoundData, StaticSoundSettings},
};

pub struct AudioSystem {
    audio_manager: AudioManager,
    static_sounds: Vec<StaticSoundData>,
}

impl AudioSystem {
    pub fn new() -> Result<Self> {
        let audio_manager = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())?;

        Ok(Self {
            audio_manager,
            static_sounds: Vec::new(),
        })
    }

    pub fn load_static_sound_from_file(&mut self, file_name: &str) -> Result<usize> {
        let static_sound = StaticSoundData::from_file(file_name)?;
        let index = self.static_sounds.len();
        self.static_sounds.push(static_sound);

        Ok(index)
    }

    pub fn play_static_sound(&mut self, sound_index: usize) -> Result<()> {
        let _sound_handle = self
            .audio_manager
            .play(self.static_sounds[sound_index].clone())?;

        Ok(())
    }
}
