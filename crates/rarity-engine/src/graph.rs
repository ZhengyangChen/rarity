#![allow(dead_code)]
use std::collections::HashMap;

use atomic_refcell::AtomicRefCell;

use crate::{
    AudioBuffer, AudioBufferMut, AudioBufferRef, AudioEffectNode, AudioSourceNode, MessageBuffer,
    MidiEffectNode, PlayHead, RawDesc, RawNode,
};

pub struct Graph {
    name: String,
    nodes: HashMap<String, RawNode>,
    node_descs: HashMap<String, RawDesc>,
    audio_buffers: Vec<AtomicRefCell<AudioBuffer>>,
    message_buffers: Vec<AtomicRefCell<MessageBuffer>>,
    sequences: Vec<Operation>,
    audio_links: Vec<Link>,
    message_links: Vec<Link>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Link(String, String);

enum Operation {
    AudioZeros(Vec<usize>),
    AudioFromInput(Vec<usize>),
    AudioToOutput(Vec<usize>),
    AudioClone(usize, Vec<usize>),
    AudioMerge(usize, Vec<usize>),
    MessageZeros(Vec<usize>),
    MessageFromInput(Vec<(usize, String)>),
    MessageClone(usize, Vec<usize>),
    MessageMerge(usize, Vec<usize>),
    Process(String, Vec<usize>, Vec<usize>, usize, Vec<usize>),
}

pub static A_OUT_NODE: &str = "A_OUT_NODE";
pub static A_IN_NODE: &str = "A_IN_NODE";

impl Graph {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            nodes: HashMap::default(),
            node_descs: HashMap::default(),
            audio_buffers: Vec::default(),
            message_buffers: Vec::default(),
            sequences: Vec::default(),
            audio_links: Vec::default(),
            message_links: Vec::default(),
        }
    }

    pub fn add_audio_source<T: AudioSourceNode>(&mut self, node: T) {
        let node = RawNode::with_audio_source(node);
        self.nodes.insert(node.name(), node);
    }

    pub fn add_audio_effect<T: AudioEffectNode>(&mut self, node: T) {
        let node = RawNode::with_audio_effect(node);
        self.nodes.insert(node.name(), node);
    }

    pub fn add_midi_effect<T: MidiEffectNode>(&mut self, node: T) {
        let node = RawNode::with_midi_effect(node);
        self.nodes.insert(node.name(), node);
    }

    pub fn add_audio_link(&mut self, from: &str, to: &str) {
        self.audio_links
            .push(Link(from.to_string(), to.to_string()));
    }

    pub fn add_message_link(&mut self, from: &str, to: &str) {
        self.message_links
            .push(Link(from.to_string(), to.to_string()));
    }

    // TODO: 假的
    pub fn mock_prepare(&mut self, sample_rate: f64) {
        self.node_descs.clear();
        for (name, node) in self.nodes.iter_mut() {
            self.node_descs
                .insert(name.clone(), node.prepare(sample_rate));
        }
        let name1 = "simple_saw".to_string();
        let name2 = "overdrive".to_string();
        self.audio_buffers.clear();
        self.audio_buffers
            .push(AtomicRefCell::new(AudioBuffer::new(4096)));
        self.audio_buffers
            .push(AtomicRefCell::new(AudioBuffer::new(4096)));
        self.message_buffers
            .push(AtomicRefCell::new(MessageBuffer::new()));
        self.message_buffers
            .push(AtomicRefCell::new(MessageBuffer::new()));
        self.sequences.clear();
        self.sequences.push(Operation::AudioZeros(vec![0, 1]));
        self.sequences.push(Operation::MessageFromInput(vec![
            (0, name1.clone()),
            (1, name2.clone()),
        ]));
        self.sequences
            .push(Operation::Process(name1, vec![], vec![0], 0, vec![]));
        self.sequences
            .push(Operation::Process(name2, vec![0], vec![1], 1, vec![]));
        self.sequences.push(Operation::AudioToOutput(vec![1]))
    }

    pub fn process(
        &mut self,
        playhead: &PlayHead,
        frames: usize,
        audio_in: AudioBufferRef,
        mut audio_out: AudioBufferMut,
        message_in: &MessageBuffer,
    ) {
        for op in &self.sequences {
            match op {
                Operation::AudioZeros(tgt) => {
                    for i in tgt {
                        self.audio_buffers[*i]
                            .borrow_mut()
                            .next_n_frames_mut(frames)
                            .clear();
                    }
                }
                Operation::AudioFromInput(tgt) => {
                    let tgt = tgt.iter().map(|i| self.audio_buffers[*i].borrow_mut());
                    for mut tgt in tgt {
                        let tgt = tgt.next_n_frames_mut(frames);
                        for (ft, fc) in tgt.into_iter().zip(audio_in) {
                            *ft.0 = *fc.0;
                            *ft.1 = *fc.1;
                        }
                    }
                }
                Operation::AudioToOutput(src) => {
                    let src = src.iter().map(|i| self.audio_buffers[*i].borrow());
                    for src in src {
                        let mut src = src.next_n_frames_ref(frames).into_iter();
                        let mut tgt = audio_out.into_iter();
                        for ft in tgt.by_ref() {
                            let fc = src.next().unwrap();
                            *ft.0 += fc.0;
                            *ft.1 += fc.1;
                        }
                        audio_out = tgt.into_mut();
                    }
                }
                Operation::AudioClone(src, tgt) => {
                    assert!(!tgt.contains(src));
                    let src = self.audio_buffers[*src].borrow();
                    let tgt = tgt.iter().map(|i| self.audio_buffers[*i].borrow_mut());
                    for mut tgt in tgt {
                        let tgt = tgt.next_n_frames_mut(frames);
                        for (ft, fc) in tgt.into_iter().zip(src.next_n_frames_ref(frames)) {
                            *ft.0 = *fc.0;
                            *ft.1 = *fc.1;
                        }
                    }
                }
                Operation::AudioMerge(tgt, src) => {
                    assert!(!src.contains(tgt));
                    let src = src.iter().map(|i| self.audio_buffers[*i].borrow());
                    let mut tgt = self.audio_buffers[*tgt].borrow_mut();
                    for src in src {
                        let tgt = tgt.next_n_frames_mut(frames);
                        for (ft, fc) in tgt.into_iter().zip(src.next_n_frames_ref(frames)) {
                            *ft.0 += fc.0;
                            *ft.1 += fc.1;
                        }
                    }
                }
                Operation::MessageZeros(tgt) => {
                    for i in tgt {
                        let mut tgt = self.message_buffers[*i].borrow_mut();
                        tgt.clear();
                        tgt.set_frames(frames);
                    }
                }
                Operation::MessageFromInput(tgt) => {
                    let tgt = tgt
                        .iter()
                        .map(|(i, name)| (self.message_buffers[*i].borrow_mut(), name));
                    for (mut tgt, name) in tgt {
                        tgt.clear();
                        tgt.set_frames(frames);
                        for mc in message_in {
                            if let Some(addr) = mc.1.addr.last() {
                                if addr == name {
                                    let mut msg = mc.1.clone();
                                    msg.addr.pop();
                                    tgt.add(*mc.0, msg);
                                }
                            }
                        }
                    }
                }
                Operation::MessageClone(src, tgt) => {
                    assert!(!tgt.contains(src));
                    let src = self.message_buffers[*src].borrow();
                    let tgt = tgt.iter().map(|i| self.message_buffers[*i].borrow_mut());
                    for mut tgt in tgt {
                        for mc in src.iter() {
                            tgt.add(*mc.0, mc.1.clone());
                        }
                    }
                }
                Operation::MessageMerge(tgt, src) => {
                    assert!(!src.contains(tgt));
                    let src = src.iter().map(|i| self.message_buffers[*i].borrow());
                    let mut tgt = self.message_buffers[*tgt].borrow_mut();
                    for src in src {
                        for mc in src.iter() {
                            tgt.add(*mc.0, mc.1.clone());
                        }
                    }
                }
                Operation::Process(name, audio_in, audio_out, message_in, message_out) => {
                    assert!(audio_in.iter().all(|i| !audio_out.contains(i)));
                    assert!(!message_out.contains(message_in));
                    let audio_in_borrow = audio_in
                        .iter()
                        .map(|i| self.audio_buffers[*i].borrow())
                        .collect::<Vec<_>>();
                    let mut audio_out_borrow = audio_out
                        .iter()
                        .map(|i| self.audio_buffers[*i].borrow_mut())
                        .collect::<Vec<_>>();
                    let message_in = &*self.message_buffers[*message_in].borrow();
                    let mut message_out_borrow = message_out
                        .iter()
                        .map(|i| self.message_buffers[*i].borrow_mut())
                        .collect::<Vec<_>>();
                    let audio_in = audio_in_borrow
                        .iter()
                        .map(|b| b.next_n_frames_ref(frames))
                        .collect();
                    let audio_out = audio_out_borrow
                        .iter_mut()
                        .map(|b| b.next_n_frames_mut(frames))
                        .collect();
                    let message_out = message_out_borrow
                        .iter_mut()
                        .map(|b| &mut **b)
                        .collect::<Vec<_>>();
                    self.nodes[name].process(
                        playhead,
                        frames,
                        audio_in,
                        audio_out,
                        message_in,
                        message_out,
                    );
                }
            }
        }
        for audio_buffer in &self.audio_buffers {
            audio_buffer.borrow_mut().forward(frames);
        }
    }
}
