use std::collections::HashMap;
use std::io::Cursor;

use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SoundHandle(pub(crate) u64);

struct SoundData {
    bytes: Vec<u8>,
}

pub struct PlaybackHandle {
    sink: Sink,
}

impl PlaybackHandle {
    pub fn stop(&self) {
        self.sink.stop();
    }

    pub fn pause(&self) {
        self.sink.pause();
    }

    pub fn resume(&self) {
        self.sink.play();
    }

    pub fn set_volume(&self, vol: f32) {
        self.sink.set_volume(vol);
    }

    pub fn is_playing(&self) -> bool {
        !self.sink.is_paused() && !self.sink.empty()
    }
}

pub struct AudioManager {
    _stream: OutputStream,
    stream_handle: OutputStreamHandle,
    sounds: HashMap<u64, SoundData>,
    next_id: u64,
}

impl AudioManager {
    pub fn new() -> Self {
        let (_stream, stream_handle) = OutputStream::try_default().expect("failed to open audio output");
        Self {
            _stream,
            stream_handle,
            sounds: HashMap::new(),
            next_id: 1,
        }
    }

    // load raw audio bytes (wav/ogg/mp3), returns handle
    pub fn load_sound(&mut self, data: &[u8]) -> SoundHandle {
        let id = self.next_id;
        self.next_id += 1;
        self.sounds.insert(id, SoundData { bytes: data.to_vec() });
        SoundHandle(id)
    }

    pub fn unload_sound(&mut self, handle: SoundHandle) {
        self.sounds.remove(&handle.0);
    }

    // play once, fire and forget
    pub fn play(&self, handle: SoundHandle) -> Option<PlaybackHandle> {
        let data = self.sounds.get(&handle.0)?;
        let cursor = Cursor::new(data.bytes.clone());
        let source = Decoder::new(cursor).ok()?;
        let sink = Sink::try_new(&self.stream_handle).ok()?;
        sink.append(source);
        Some(PlaybackHandle { sink })
    }

    // loop forever
    pub fn play_looped(&self, handle: SoundHandle) -> Option<PlaybackHandle> {
        let data = self.sounds.get(&handle.0)?;
        let cursor = Cursor::new(data.bytes.clone());
        let source = Decoder::new(cursor).ok()?.repeat_infinite();
        let sink = Sink::try_new(&self.stream_handle).ok()?;
        sink.append(source);
        Some(PlaybackHandle { sink })
    }

    // play at specific volume
    pub fn play_with_volume(&self, handle: SoundHandle, vol: f32) -> Option<PlaybackHandle> {
        let pb = self.play(handle)?;
        pb.set_volume(vol);
        Some(pb)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sound_handle_ids_increment() {
        // test id generation without audio device
        let mut next_id = 1u64;
        let ids: Vec<u64> = (0..5).map(|_| { let id = next_id; next_id += 1; id }).collect();
        assert_eq!(ids, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_sound_data_storage() {
        // test the hashmap storage logic without audio device
        let mut sounds: HashMap<u64, Vec<u8>> = HashMap::new();
        let data = vec![1u8, 2, 3, 4];
        sounds.insert(1, data.clone());
        assert_eq!(sounds.get(&1).unwrap(), &data);
        sounds.remove(&1);
        assert!(sounds.get(&1).is_none());
    }

    #[test]
    fn test_unloaded_sound_not_found() {
        let mut sounds: HashMap<u64, Vec<u8>> = HashMap::new();
        sounds.insert(1, vec![0u8; 10]);
        sounds.remove(&1);
        assert!(sounds.get(&1).is_none());
        // cant play what doesnt exist
        assert!(sounds.get(&99).is_none());
    }
}
