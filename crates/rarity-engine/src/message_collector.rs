#![allow(dead_code)]
use std::sync::mpsc::{channel, Receiver, Sender};

use crate::{Message, MessageBuffer};

type Ports = Vec<(Vec<String>, Receiver<(usize, Message)>)>;

/// 控制信息收集器, 将各个其它线程的控制信息收集到所在的线程(通常是音频线程)
#[derive(Default)]
pub struct MessageCollector {
    team: Vec<(usize, Message)>,
    curr: usize,
    ports: Ports,
}

impl MessageCollector {
    pub fn new() -> Self {
        Self {
            team: vec![],
            curr: 0,
            ports: vec![],
        }
    }

    pub fn add_port(&mut self, addr: Vec<String>) -> Sender<(usize, Message)> {
        assert!(self.ports.iter().all(|p| p.0 != addr));
        let (tx, rx) = channel();
        self.ports.push((addr, rx));
        tx
    }

    // TODO: 为了example暂时改为pub
    pub fn collect(&mut self) {
        for (addr, rx) in &self.ports {
            while let Ok((frame, mut message)) = rx.try_recv() {
                let index = match self.team.binary_search_by_key(&frame, |(f, _)| *f) {
                    Ok(i) => i + 1,
                    Err(i) => i,
                };
                message.addr = {
                    let mut addr = addr.clone();
                    if !message.addr.is_empty() {
                        addr.append(&mut message.addr);
                    }
                    addr
                };
                self.team.insert(index, (frame, message));
            }
        }
    }

    // TODO: 为了example暂时改为pub
    pub fn drain_frames(&mut self, frames: usize) -> MessageBuffer {
        let frame = self.curr + frames;
        let index = self
            .team
            .binary_search_by_key(&frame, |(f, _)| *f)
            .unwrap_or_else(|i| i);
        let team = self
            .team
            .drain(0..index)
            .map(|(f, m)| (f.max(self.curr) - self.curr, m))
            .collect::<Vec<_>>();
        self.curr = frame;
        MessageBuffer(team, frames)
    }
}
