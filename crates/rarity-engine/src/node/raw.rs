#![allow(dead_code)]
use std::{
    alloc::{dealloc, Layout},
    any::TypeId,
    ptr,
    sync::Mutex,
};

use crate::{AudioBufferMut, AudioBufferRef, MessageBuffer, Parameter, PlayHead};

pub(crate) static NODE_REGISTER_CENTER: Mutex<Vec<(TypeId, NodeType)>> = Mutex::new(Vec::new());

pub(crate) struct RawNode {
    pub data: *const (),
    pub note_type: NodeType,
    pub vtable: &'static RawNodeVTable,
}

impl RawNode {
    pub fn with_audio_effect<T: AudioEffectNode>(node: T) -> Self {
        let data = Box::into_raw(Box::new(node)) as *const ();
        Self {
            data,
            note_type: NodeType::AudioEffect,
            vtable: &RawNodeVTable {
                name: |d| {
                    let d = unsafe { &*(d as *mut T) };
                    d.name()
                },
                prepare: |d, sample_rate| {
                    let d = unsafe { &mut *(d as *mut T) };
                    RawDesc::with_audio_effect(d.prepare(sample_rate))
                },
                process: |d, playhead, frames, audio_in, audio_out, message_in, _| {
                    let d = unsafe { &mut *(d as *mut T) };
                    let audio_out = audio_out.into_iter().next().unwrap();
                    d.process(playhead, frames, audio_in, audio_out, message_in);
                },
                drop: |d| {
                    let d = d as *mut T;
                    unsafe {
                        ptr::drop_in_place(d);
                        dealloc(d as *mut u8, Layout::new::<T>());
                    }
                },
            },
        }
    }

    pub fn with_midi_effect<T: MidiEffectNode>(node: T) -> Self {
        let data = Box::into_raw(Box::new(node)) as *const ();
        Self {
            data,
            note_type: NodeType::MidiEffect,
            vtable: &RawNodeVTable {
                name: |d| {
                    let d = unsafe { &*(d as *mut T) };
                    d.name()
                },
                prepare: |d, sample_rate| {
                    let d = unsafe { &mut *(d as *mut T) };
                    RawDesc::with_midi_effect(d.prepare(sample_rate))
                },
                process: |d, playhead, frames, _, _, message_in, message_out| {
                    let d = unsafe { &mut *(d as *mut T) };
                    d.process(playhead, frames, message_in, message_out);
                },
                drop: |d| {
                    let d = d as *mut T;
                    unsafe {
                        ptr::drop_in_place(d);
                        dealloc(d as *mut u8, Layout::new::<T>());
                    }
                },
            },
        }
    }

    pub fn with_audio_source<T: AudioSourceNode>(node: T) -> Self {
        let data = Box::into_raw(Box::new(node)) as *const ();
        Self {
            data,
            note_type: NodeType::AudioSource,
            vtable: &RawNodeVTable {
                name: |d| {
                    let d = unsafe { &*(d as *mut T) };
                    d.name()
                },
                prepare: |d, sample_rate| {
                    let d = unsafe { &mut *(d as *mut T) };
                    RawDesc::with_audio_source(d.prepare(sample_rate))
                },
                process: |d, playhead, frames, _, audio_out, message_in, _| {
                    let d = unsafe { &mut *(d as *mut T) };
                    let audio_out = audio_out.into_iter().next().unwrap();
                    d.process(playhead, frames, audio_out, message_in);
                },
                drop: |d| {
                    let d = d as *mut T;
                    unsafe {
                        ptr::drop_in_place(d);
                        dealloc(d as *mut u8, Layout::new::<T>());
                    }
                },
            },
        }
    }

    pub fn process(
        &self,
        playhead: &PlayHead,
        frames: usize,
        audio_in: Vec<AudioBufferRef>,
        audio_out: Vec<AudioBufferMut>,
        message_in: &MessageBuffer,
        message_out: Vec<&mut MessageBuffer>,
    ) {
        unsafe {
            (self.vtable.process)(
                self.data,
                playhead,
                frames,
                audio_in,
                audio_out,
                message_in,
                message_out,
            );
        }
    }

    pub fn prepare(&self, sample_rate: f64) -> RawDesc {
        unsafe { (self.vtable.prepare)(self.data, sample_rate) }
    }

    pub fn name(&self) -> String {
        unsafe { (self.vtable.name)(self.data) }
    }
}

impl Drop for RawNode {
    fn drop(&mut self) {
        unsafe { (self.vtable.drop)(self.data) };
    }
}

pub(crate) struct RawDesc {
    pub audio_in: usize,
    pub audio_out: usize,
    pub message_out: usize,
    pub parameters: Vec<Parameter>,
}

impl RawDesc {
    fn with_audio_effect(desc: AudioEffectDesc) -> Self {
        Self {
            audio_in: desc.audio_in,
            audio_out: 1,
            message_out: 0,
            parameters: desc.parameters,
        }
    }

    fn with_midi_effect(desc: MidiEffectDesc) -> Self {
        Self {
            audio_in: 0,
            audio_out: 0,
            message_out: desc.message_out,
            parameters: desc.parameters,
        }
    }

    fn with_audio_source(desc: AudioSourceDesc) -> Self {
        Self {
            audio_in: 0,
            audio_out: 1,
            message_out: 0,
            parameters: desc.parameters,
        }
    }
}

pub(crate) type FnRawProcess = unsafe fn(
    *const (),
    &PlayHead,
    usize,
    Vec<AudioBufferRef>,
    Vec<AudioBufferMut>,
    &MessageBuffer,
    Vec<&mut MessageBuffer>,
);

pub(crate) struct RawNodeVTable {
    pub name: unsafe fn(*const ()) -> String,
    pub prepare: unsafe fn(*const (), f64) -> RawDesc,
    pub process: FnRawProcess,
    drop: unsafe fn(*const ()),
}

unsafe impl Send for RawNodeVTable {}
unsafe impl Sync for RawNodeVTable {}
unsafe impl Send for RawNode {}
unsafe impl Sync for RawNode {}

pub enum NodeType {
    AudioEffect,
    MidiEffect,
    AudioSource,
}

pub struct AudioEffectDesc {
    pub audio_in: usize,
    pub parameters: Vec<Parameter>,
}

pub trait AudioEffectNode {
    fn name(&self) -> String;
    fn prepare(&mut self, sample_rate: f64) -> AudioEffectDesc;
    fn process(
        &mut self,
        playhead: &PlayHead,
        frames: usize,
        audio_in: Vec<AudioBufferRef>,
        audio_out: AudioBufferMut,
        message_in: &MessageBuffer,
    );
}

pub struct MidiEffectDesc {
    pub message_out: usize,
    pub parameters: Vec<Parameter>,
}

pub trait MidiEffectNode {
    fn name(&self) -> String;
    fn prepare(&mut self, sample_rate: f64) -> MidiEffectDesc;
    fn process(
        &mut self,
        playhead: &PlayHead,
        frames: usize,
        message_in: &MessageBuffer,
        message_out: Vec<&mut MessageBuffer>,
    );
}

pub struct AudioSourceDesc {
    pub parameters: Vec<Parameter>,
}

pub trait AudioSourceNode {
    fn name(&self) -> String;
    fn prepare(&mut self, sample_rate: f64) -> AudioSourceDesc;
    fn process(
        &mut self,
        playhead: &PlayHead,
        frames: usize,
        audio_out: AudioBufferMut,
        message_in: &MessageBuffer,
    );
}
