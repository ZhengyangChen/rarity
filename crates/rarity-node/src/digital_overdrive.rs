use rarity_engine::{
    AudioBufferMut, AudioBufferRef, AudioEffectDesc, AudioEffectNode, FloatRange, Message,
    MessageBuffer, MessageValue, ParaRange, Parameter, PlayHead,
};

pub struct DigitalOverDrive {
    name: String,
    drive: f64,
    level: f64,
}

impl DigitalOverDrive {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            drive: 0.0,
            level: 1.0,
        }
    }

    fn prepare() -> AudioEffectDesc {
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

    fn process(
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

    fn set_state(&mut self, message: &Message) {
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

    fn set_drive(&mut self, value: f64) {
        self.drive = value;
    }

    fn set_level(&mut self, value: f64) {
        self.level = value;
    }

    fn forward(&self, input: AudioBufferRef, output: AudioBufferMut) {
        let clamp = (1.0 - self.drive).max(0.05);
        let gain = 1.0 / clamp;
        for ((li, ri), (lo, ro)) in input.iter().zip(output) {
            *lo = (*lo + li * gain).clamp(-clamp, clamp) * self.level;
            *ro = (*ro + ri * gain).clamp(-clamp, clamp) * self.level;
        }
    }
}

impl AudioEffectNode for DigitalOverDrive {
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
