#[derive(Debug, Default)]
pub struct MessageBuffer(pub(crate) Vec<(usize, Message)>, pub(crate) usize);

pub struct MessageBufferIterMut<'a>(&'a mut Vec<(usize, Message)>, usize);

#[derive(Clone, Copy)]
pub struct MessageBufferIter<'a>(&'a Vec<(usize, Message)>, usize);

impl MessageBuffer {
    pub fn new() -> Self {
        Self(vec![], 0)
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn frame(&self) -> usize {
        self.1
    }
    pub(crate) fn set_frames(&mut self, frames: usize) {
        self.1 = frames;
    }
    pub fn add(&mut self, frame: usize, message: Message) {
        if frame < self.1 {
            let index = match self.0.binary_search_by_key(&frame, |(s, _)| *s) {
                Ok(i) => i + 1,
                Err(i) => i,
            };
            self.0.insert(index, (frame, message));
        } else {
            panic!("{} 超出范围 [0, {})", frame, self.1)
        }
    }
    pub fn clear(&mut self) {
        self.0.clear();
    }
    pub fn iter(&self) -> MessageBufferIter {
        MessageBufferIter(&self.0, 0)
    }
    pub fn get(&self, index: usize) -> Option<(&usize, &Message)> {
        self.0.get(index).map(|t| (&t.0, &t.1))
    }
}

impl<'a> IntoIterator for &'a mut MessageBuffer {
    type Item = (&'a mut usize, &'a mut Message);
    type IntoIter = MessageBufferIterMut<'a>;

    fn into_iter(self) -> Self::IntoIter {
        MessageBufferIterMut(&mut self.0, 0)
    }
}

impl<'a> Iterator for MessageBufferIterMut<'a> {
    type Item = (&'a mut usize, &'a mut Message);

    fn next(&mut self) -> Option<Self::Item> {
        if self.1 < self.0.len() {
            let res = unsafe { self.0.as_mut_ptr().add(self.1).as_mut() };
            self.1 += 1;
            res.map(|t| (&mut t.0, &mut t.1))
        } else {
            None
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let rest = self.0.len() - self.1;
        (rest, Some(rest))
    }
}

impl<'a> ExactSizeIterator for MessageBufferIterMut<'a> {}

impl<'a> IntoIterator for &'a MessageBuffer {
    type Item = (&'a usize, &'a Message);
    type IntoIter = MessageBufferIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        MessageBufferIter(&self.0, 0)
    }
}

impl<'a> Iterator for MessageBufferIter<'a> {
    type Item = (&'a usize, &'a Message);

    fn next(&mut self) -> Option<Self::Item> {
        if self.1 < self.0.len() {
            let res = self.0.get(self.1);
            self.1 += 1;
            res.map(|t| (&t.0, &t.1))
        } else {
            None
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let rest = self.0.len() - self.1;
        (rest, Some(rest))
    }
}

impl<'a> ExactSizeIterator for MessageBufferIter<'a> {}

#[derive(Clone, PartialEq, Debug)]
pub struct Message {
    pub addr: Vec<String>,
    pub value: MessageValue,
}

#[derive(Clone, PartialEq, Debug)]
pub enum MessageValue {
    Midi(MidiMessage),
    Float(FloatMessage),
    Enum(EnumMessage),
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum MidiMessage {
    NoteOn(NoteOn),
    NoteOff(NoteOff),
    ControlChange(ControlChange),
    PitchBend(PitchBend),
}

#[derive(Clone, PartialEq, Debug)]
pub struct FloatMessage {
    pub name: String,
    pub value: f64,
}

#[derive(Clone, PartialEq, Debug)]
pub struct EnumMessage {
    pub name: String,
    pub value: usize,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct NoteOn {
    pub pitch: u8,
    pub velocity: u8,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct NoteOff {
    pub pitch: u8,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ControlChange {
    pub number: u8,
    pub value: u8,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct PitchBend {
    pub value: i16,
}
