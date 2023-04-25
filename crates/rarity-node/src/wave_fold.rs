use rarity_engine::{
    AudioBufferMut, AudioBufferRef, AudioEffectDesc, AudioEffectNode, FloatRange, Message,
    MessageBuffer, MessageValue, ParaRange, Parameter, PlayHead,
};

pub struct WaveFold {
    name: String,
    drive: f64,
    level: f64,
}

impl WaveFold {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            drive: 0.0,
            level: 1.0,
        }
    }

    pub fn prepare() -> AudioEffectDesc {
        AudioEffectDesc {
            audio_in: 1,
            parameters: vec![
                Parameter {
                    addr: vec![],
                    range: ParaRange::Float(FloatRange {
                        name: "Drive".to_string(),
                        min: 0.0,
                        max: 1.0,
                        default: 0.0,
                    }),
                },
                Parameter {
                    addr: vec![],
                    range: ParaRange::Float(FloatRange {
                        name: "Level".to_string(),
                        min: 0.0,
                        max: 1.0,
                        default: 1.0,
                    }),
                },
            ],
        }
    }

    pub fn process(
        &mut self,
        frames: usize,
        audio_in: AudioBufferRef,
        audio_out: AudioBufferMut,
        message_in: &MessageBuffer,
    ) {
        let mut curr_frame = 0;
        let mut in_remain = audio_in;
        let mut out_remain = audio_out;
        for (f, msg) in message_in {
            if *f >= frames {
                break;
            }
            let f = *f.min(&frames);
            if curr_frame < f {
                let (output, tmp) = out_remain.split_at_mut(f - curr_frame);
                out_remain = tmp;
                let (input, tmp) = in_remain.split_at(f - curr_frame);
                in_remain = tmp;
                self.forward(input, output);
                curr_frame = f;
            }
            self.set_state(msg);
        }
        if curr_frame < frames {
            let (output, _) = out_remain.split_at_mut(frames - curr_frame);
            let (input, _) = in_remain.split_at(frames - curr_frame);
            self.forward(input, output);
        }
    }

    pub fn set_state(&mut self, message: &Message) {
        if !message.addr.is_empty() {
            return;
        }
        if let MessageValue::Float(msg) = &message.value {
            if &msg.name == "Drive" {
                self.set_drive(msg.value);
            } else if &msg.name == "Level" {
                self.set_level(msg.value);
            }
        }
    }

    pub fn set_drive(&mut self, value: f64) {
        self.drive = value;
    }

    pub fn set_level(&mut self, value: f64) {
        self.level = value;
    }

    pub fn forward(&self, input: AudioBufferRef, output: AudioBufferMut) {
        let clamp = (1.0 - self.drive).max(0.05);
        let gain = 1.0 / clamp;
        for ((li, ri), (lo, ro)) in input.iter().zip(output) {
            let vl = (li + clamp).rem_euclid(4.0 * clamp);
            let vl = if vl <= 2.0 * clamp {vl - clamp} else { 3.0 * clamp - vl };
            let vr = (ri + clamp).rem_euclid(4.0 * clamp);
            let vr = if vr <= 2.0 * clamp {vr - clamp} else { 3.0 * clamp - vr };
            *lo += vl * gain * self.level;
            *ro += vr * gain * self.level;
        }
    }
}

impl AudioEffectNode for WaveFold {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn prepare(&mut self, _sample_rate: f64) -> AudioEffectDesc {
        Self::prepare()
    }

    fn process(
        &mut self,
        _playhead: &PlayHead,
        frames: usize,
        audio_in: Vec<AudioBufferRef>,
        audio_out: AudioBufferMut,
        message_in: &MessageBuffer,
    ) {
        let mut audio_in = audio_in;
        let audio_in = audio_in.remove(0);
        self.process(frames, audio_in, audio_out, message_in)
    }
}

#[cfg(test)]
mod test {
    use std::{fs::File, path::Path};

    use rarity_engine::AudioBuffer;

    use crate::SimpleSaw;

    use super::*;

    #[test]
    fn main() {
        let mut src = SimpleSaw::new("saw", 1);
        src.set_r(0.8);
        src.set_note_on(65, 80);
        let mut audio = AudioBuffer::new(48000 * 10);
        let buffer = audio.next_n_frames_mut(48000 * 10);
        let (a, b) = buffer.split_at_mut(4800);
        src.forward(a);
        src.set_note_off(65);
        src.forward(b);

        let mut wave_fold = WaveFold::new("wave_fold");
        wave_fold.set_drive(1.0);
        let input = audio.next_n_frames_ref(48000 * 10);
        let mut audio2 = AudioBuffer::new(48000 * 10);
        let output = audio2.next_n_frames_mut(48000 * 10);
        wave_fold.forward(input, output);

        let data = audio2
            .next_n_frames_ref(48000 * 10)
            .into_iter()
            .map(|(l, r)| [*l as f32, *r as f32])
            .flatten()
            .collect::<Vec<_>>();
        let mut out_file =
            File::create(Path::new("/Users/chenzhengyang/Desktop/wave_fold.wav")).unwrap();
        let header = wav::Header::new(3, 2, 48000, 32);
        let data = wav::BitDepth::ThirtyTwoFloat(data);
        wav::write(header, &data, &mut out_file).unwrap();
    }
}
