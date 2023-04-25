use std::f64::consts::PI;

use rarity_engine::{
    AudioBufferMut, AudioSourceDesc, AudioSourceNode, FloatRange, Message, MessageBuffer,
    MessageValue, MidiMessage, ParaRange, Parameter, PlayHead,
};

// static PREPARE_SAMPLES: usize = 32;
// static PREPARE_SAMPLES_F64: f64 = PREPARE_SAMPLES as f64;

pub struct SimpleSaw {
    name: String,
    voices: Vec<Voice>,
    sf: f64,
    voice_counter: usize,
}

impl SimpleSaw {
    pub fn new(name: &str, max_voice: usize) -> Self {
        Self {
            name: name.to_string(),
            voices: (0..max_voice).map(|_| Voice::new()).collect(),
            sf: 48000.0,
            voice_counter: 0,
        }
    }

    pub fn prepare(&mut self, sample_rate: f64) -> AudioSourceDesc {
        self.sf = sample_rate;
        for v in self.voices.iter_mut() {
            v.set_sample_rate(sample_rate);
        }
        AudioSourceDesc {
            parameters: vec![
                Parameter {
                    addr: vec![],
                    range: ParaRange::Float(FloatRange {
                        name: "Volume".to_string(),
                        min: 0.0,
                        max: 1.0,
                        default: 1.0,
                    }),
                },
                Parameter {
                    addr: vec![],
                    range: ParaRange::Float(FloatRange {
                        name: "A".to_string(),
                        min: 0.0,
                        max: 10.0,
                        default: 0.0,
                    }),
                },
                Parameter {
                    addr: vec![],
                    range: ParaRange::Float(FloatRange {
                        name: "D".to_string(),
                        min: 0.0,
                        max: 10.0,
                        default: 0.0,
                    }),
                },
                Parameter {
                    addr: vec![],
                    range: ParaRange::Float(FloatRange {
                        name: "S".to_string(),
                        min: 0.0,
                        max: 1.0,
                        default: 1.0,
                    }),
                },
                Parameter {
                    addr: vec![],
                    range: ParaRange::Float(FloatRange {
                        name: "R".to_string(),
                        min: 0.0,
                        max: 10.0,
                        default: 0.0,
                    }),
                },
            ],
        }
    }

    pub fn process(
        &mut self,
        frames: usize,
        audio_out: AudioBufferMut,
        message_in: &MessageBuffer,
    ) {
        let mut curr_frame = 0;
        let mut remain = audio_out;
        for (f, msg) in message_in {
            if *f >= frames {
                break;
            }
            let f = *f.min(&frames);
            if curr_frame < f {
                let (output, tmp) = remain.split_at_mut(f - curr_frame);
                remain = tmp;
                self.forward(output);
                curr_frame = f;
            }
            self.set_state(msg);
        }
        if curr_frame < frames {
            let (output, _) = remain.split_at_mut(frames - curr_frame);
            self.forward(output);
        }
    }

    pub fn set_state(&mut self, message: &Message) {
        if !message.addr.is_empty() {
            return;
        }
        match &message.value {
            MessageValue::Midi(msg) => match msg {
                MidiMessage::NoteOn(note_on) => {
                    self.set_note_on(note_on.pitch, note_on.velocity);
                }
                MidiMessage::NoteOff(note_off) => {
                    self.set_note_off(note_off.pitch);
                }
                MidiMessage::ControlChange(_control_change) => {}
                MidiMessage::PitchBend(_pitch_bend) => {}
            },
            MessageValue::Float(msg) => {
                if &msg.name == "Volume" {
                    self.set_volume(msg.value);
                } else if &msg.name == "A" {
                    self.set_a(msg.value);
                } else if &msg.name == "D" {
                    self.set_d(msg.value);
                } else if &msg.name == "S" {
                    self.set_s(msg.value);
                } else if &msg.name == "R" {
                    self.set_r(msg.value);
                }
            }
            _ => {}
        }
    }

    pub fn set_volume(&mut self, value: f64) {
        for voice in self.voices.iter_mut() {
            voice.set_volume(value);
        }
    }

    pub fn set_a(&mut self, value: f64) {
        for voice in self.voices.iter_mut() {
            voice.set_a(value);
        }
    }

    pub fn set_d(&mut self, value: f64) {
        for voice in self.voices.iter_mut() {
            voice.set_d(value);
        }
    }

    pub fn set_s(&mut self, value: f64) {
        for voice in self.voices.iter_mut() {
            voice.set_s(value);
        }
    }

    pub fn set_r(&mut self, value: f64) {
        for voice in self.voices.iter_mut() {
            voice.set_r(value);
        }
    }

    pub fn set_note_off(&mut self, pitch: u8) {
        for voice in self.voices.iter_mut() {
            if voice.pitch == pitch {
                voice.set_note_off();
            }
        }
    }

    pub fn set_note_on(&mut self, pitch: u8, velocity: u8) {
        if velocity == 0 {
            self.set_note_off(pitch);
        } else {
            let ind = self.find_voice(pitch);
            self.voice_counter += 1;
            self.voices[ind].set_note_on(pitch, velocity, self.voice_counter);
        }
    }

    pub fn forward(&mut self, mut output: AudioBufferMut) {
        for voice in self.voices.iter_mut() {
            output = voice.forward(output);
        }
    }

    fn find_voice(&self, pitch: u8) -> usize {
        for (i, v) in self.voices.iter().enumerate() {
            if v.is_silent() {
                return i;
            }
        }
        let lowest_pitch = self.voices.iter().min_by_key(|v| v.pitch).unwrap().pitch;
        if lowest_pitch >= pitch {
            self.voices
                .iter()
                .enumerate()
                .min_by_key(|(_, v)| v.counter)
                .unwrap()
                .0
        } else {
            self.voices
                .iter()
                .enumerate()
                .filter(|(_, v)| v.pitch != lowest_pitch)
                .min_by_key(|(_, v)| v.counter)
                .unwrap_or_else(|| {
                    self.voices
                        .iter()
                        .enumerate()
                        .min_by_key(|(_, v)| v.counter)
                        .unwrap()
                })
                .0
        }
    }
}

impl AudioSourceNode for SimpleSaw {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn prepare(&mut self, sample_rate: f64) -> AudioSourceDesc {
        self.prepare(sample_rate)
    }

    fn process(
        &mut self,
        _playhead: &PlayHead,
        frames: usize,
        audio_out: AudioBufferMut,
        message_in: &MessageBuffer,
    ) {
        self.process(frames, audio_out, message_in);
    }
}

struct Voice {
    counter: usize,
    pitch: u8,
    volume: f64,
    osc: SawOSC,
    amp: ADSR,
    sr: f64,
    // prepare_counter: usize,
    // prepare_step: f64,
    // prepare_last_output: f64,
}

impl Voice {
    fn new() -> Self {
        Self {
            counter: 0,
            pitch: 0,
            volume: 1.0,
            osc: SawOSC::new(0, 48000.0),
            amp: ADSR::new(0.0, 0.0, 1.0, 0.0, 48000.0),
            sr: 48000.0,
            // prepare_counter: 0,
            // prepare_step: 0.0,
            // prepare_last_output: 0.0,
        }
    }

    fn is_silent(&self) -> bool {
        self.amp.phase == ADSRPhase::Silent
    }

    fn set_sample_rate(&mut self, sample_rate: f64) {
        if sample_rate != self.sr {
            self.osc.set_sample_rate(sample_rate);
            self.amp.set_sample_rate(sample_rate);
            self.sr = sample_rate;
        }
    }

    fn set_note_on(&mut self, pitch: u8, velocity: u8, counter: usize) {
        self.counter = counter;
        self.pitch = pitch;
        // let curr_value =
        //     self.amp.next().unwrap_or_default() * self.osc.next().unwrap_or_default() * self.volume;
        self.osc.set_on(pitch, velocity);
        self.amp.set_on();
        // let next_value =
        //     self.amp.next().unwrap_or_default() * self.osc.next().unwrap_or_default() * self.volume;
        // self.prepare_counter = PREPARE_SAMPLES + 1;
        // self.prepare_step = (next_value - curr_value) / PREPARE_SAMPLES_F64;
        // self.prepare_last_output = curr_value;
    }

    fn set_note_off(&mut self) {
        self.osc.set_off();
        self.amp.set_off();
    }

    fn set_volume(&mut self, volume: f64) {
        self.volume = volume;
    }

    fn set_a(&mut self, a_in_sec: f64) {
        self.amp.set_a_second(a_in_sec);
    }

    fn set_d(&mut self, d_in_half_decay_sec: f64) {
        self.amp.set_d_half_decay_second(d_in_half_decay_sec);
    }

    fn set_s(&mut self, s_in_ratio: f64) {
        self.amp.set_s_level(s_in_ratio);
    }

    fn set_r(&mut self, r_in_half_decay_sec: f64) {
        self.amp.set_r_half_decay_second(r_in_half_decay_sec);
    }

    fn forward<'a>(&mut self, output: AudioBufferMut<'a>) -> AudioBufferMut<'a> {
        let mut iter_mut = output.into_iter();
        for (l, r) in iter_mut.by_ref() {
            // if self.prepare_counter > 0 {
            //     *l += self.prepare_last_output;
            //     *r += self.prepare_last_output;
            //     self.prepare_counter -= 1;
            //     self.prepare_last_output += self.prepare_step;
            // } else {
            let s1 = self.osc.next().unwrap_or_default();
            let s2 = self.amp.next().unwrap_or_default();
            let s = s1 * s2 * self.volume;
            *l += s;
            *r += s;
            // }
        }
        iter_mut.into_mut()
    }
}

struct SawOSC {
    pitch: u8,
    sr: f64,
    pos: f64,
    step: f64,
    volume: f64,
    velocity_volume: f64,
    last_output: f64,
}

impl SawOSC {
    fn new(pitch: u8, sample_rate: f64) -> Self {
        Self {
            pitch,
            sr: sample_rate,
            pos: 0.0,
            volume: 1.0,
            velocity_volume: 0.0,
            step: 440.0 * 2_f64.powf((pitch as f64 - 81.0) / 12.0) / sample_rate,
            last_output: 0.0,
        }
    }

    fn set_on(&mut self, pitch: u8, velocity: u8) {
        self.pitch = pitch;
        self.step = 440.0 * 2_f64.powf((pitch as f64 - 81.0) / 12.0) / self.sr;
        self.velocity_volume = (velocity as f64 / 128.0).sqrt();
    }

    fn set_off(&mut self) {}

    fn set_sample_rate(&mut self, sample_rate: f64) {
        if sample_rate != self.sr {
            self.step *= self.sr / sample_rate;
            self.sr = sample_rate;
        }
    }
}

impl Iterator for SawOSC {
    type Item = f64;

    fn next(&mut self) -> Option<Self::Item> {
        let res = (self.pos * PI * 2.0).sin();
        // let res = self.pos * 2.0 - 1.0;
        // let res = (-3..=3)
        //     .map(|i| (self.pos + i as f64 * self.step).clamp(0.0, 1.0) * 2.0 - 1.0)
        //     .fold(0.0, |acc, x| acc + x)
        //     / 7.0;
        self.pos += self.step;
        if self.pos >= 1.0 {
            self.pos -= 1.0;
        }
        self.last_output = res * self.volume * self.velocity_volume;
        Some(self.last_output)
    }
}

#[allow(clippy::upper_case_acronyms)]
struct ADSR {
    a_second: f64,
    d_half_decay_second: f64,
    s_level: f64,
    r_half_decay_second: f64,
    phase: ADSRPhase,
    a_delta: f64,
    d_step_ratio: f64,
    r_step_ratio: f64,
    last_output: f64,
    sr: f64,
}

#[derive(PartialEq, Eq)]
enum ADSRPhase {
    A,
    D,
    S,
    R,
    Silent,
}

impl ADSR {
    fn new(
        a_second: f64,
        d_half_decay_second: f64,
        s_level: f64,
        r_half_decay_second: f64,
        sample_rate: f64,
    ) -> Self {
        Self {
            a_second,
            d_half_decay_second,
            s_level,
            r_half_decay_second,
            sr: sample_rate,
            a_delta: 1.0 / (a_second.max(0.001) * sample_rate),
            d_step_ratio: 2_f64.powf(-1.0 / (d_half_decay_second.max(0.001) * sample_rate)),
            r_step_ratio: 2_f64.powf(-1.0 / (r_half_decay_second.max(0.001) * sample_rate)),
            phase: ADSRPhase::Silent,
            last_output: 0.0,
        }
    }

    fn set_on(&mut self) {
        self.phase = ADSRPhase::A;
    }

    fn set_off(&mut self) {
        match self.phase {
            ADSRPhase::A | ADSRPhase::D | ADSRPhase::S => self.phase = ADSRPhase::R,
            _ => {}
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f64) {
        if sample_rate != self.sr {
            self.sr = sample_rate;
            self.a_delta = 1.0 / (self.a_second.max(0.001) * sample_rate);
            self.d_step_ratio =
                2_f64.powf(-1.0 / (self.d_half_decay_second.max(0.001) * sample_rate));
            self.r_step_ratio =
                2_f64.powf(-1.0 / (self.r_half_decay_second.max(0.001) * sample_rate));
        }
    }

    fn set_a_second(&mut self, a_second: f64) {
        if a_second != self.a_second {
            self.a_second = a_second;
            self.a_delta = 1.0 / (self.sr * a_second.max(0.001));
        }
    }

    fn set_d_half_decay_second(&mut self, d_half_decay_second: f64) {
        if d_half_decay_second != self.d_half_decay_second {
            self.d_half_decay_second = d_half_decay_second;
            self.d_step_ratio = 2_f64.powf(-1.0 / (d_half_decay_second.max(0.001) * self.sr));
        }
    }

    fn set_s_level(&mut self, s_level: f64) {
        if s_level != self.s_level {
            self.s_level = s_level;
        }
    }

    fn set_r_half_decay_second(&mut self, r_half_decay_second: f64) {
        if r_half_decay_second != self.r_half_decay_second {
            self.r_half_decay_second = r_half_decay_second;
            self.r_step_ratio = 2_f64.powf(-1.0 / (r_half_decay_second.max(0.001) * self.sr));
        }
    }
}

impl Iterator for ADSR {
    type Item = f64;

    fn next(&mut self) -> Option<Self::Item> {
        match self.phase {
            ADSRPhase::A => {
                self.last_output += self.a_delta;
                if self.last_output >= 1.0 {
                    self.phase = ADSRPhase::D;
                }
                Some(self.last_output)
            }
            ADSRPhase::D => {
                self.last_output *= self.d_step_ratio;
                if self.last_output <= 0.001 {
                    self.phase = ADSRPhase::Silent;
                } else if self.last_output <= self.s_level {
                    self.phase = ADSRPhase::S;
                }
                Some(self.last_output)
            }
            ADSRPhase::S => Some(self.last_output),
            ADSRPhase::R => {
                self.last_output *= self.r_step_ratio;
                if self.last_output <= 0.001 {
                    self.phase = ADSRPhase::Silent;
                }
                Some(self.last_output)
            }
            ADSRPhase::Silent => None,
        }
    }
}

#[cfg(test)]
mod test {
    use std::{fs::File, path::Path};

    use rarity_engine::AudioBuffer;

    use super::*;

    #[test]
    fn main() {
        let mut voice = Voice::new();
        voice.set_a(0.2);
        voice.set_d(0.8);
        voice.set_s(0.5);
        voice.set_r(0.2);
        voice.set_note_on(65, 80, 1);
        let mut audio = AudioBuffer::new(48000 * 5);
        let buffer = audio.next_n_frames_mut(48000 * 5);
        let (a, b) = buffer.split_at_mut(48000);
        voice.forward(a);
        voice.set_note_off();
        let (b, c) = b.split_at_mut(120);
        voice.forward(b);
        voice.set_note_on(65, 10, 2);
        let (c, d) = c.split_at_mut(48000);
        voice.forward(c);
        voice.set_note_off();
        voice.forward(d);

        let data = audio
            .next_n_frames_ref(48000 * 5)
            .into_iter()
            .map(|(l, r)| [*l as f32, *r as f32])
            .flatten()
            .collect::<Vec<_>>();
        let mut out_file =
            File::create(Path::new("/Users/chenzhengyang/Desktop/simple_saw.wav")).unwrap();
        let header = wav::Header::new(3, 2, 48000, 32);
        let data = wav::BitDepth::ThirtyTwoFloat(data);
        wav::write(header, &data, &mut out_file).unwrap();
    }
}
