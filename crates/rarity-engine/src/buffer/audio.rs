use std::{cmp::Ordering, fmt};

/// 左右声道交替的音频数据
pub struct AudioBuffer(Vec<f64>, usize);

pub struct AudioBufferMut<'a>(&'a mut [f64], &'a mut [f64]);

#[derive(Clone, Copy)]
pub struct AudioBufferRef<'a>(&'a [f64], &'a [f64]);

pub struct AudioBufferIterMut<'a>(AudioBufferMut<'a>, usize);

#[derive(Clone, Copy)]
pub struct AudioBufferIter<'a>(AudioBufferRef<'a>, usize);

impl AudioBuffer {
    pub fn new(len: usize) -> Self {
        if len == 0 {
            panic!("尝试声明空缓冲")
        }
        Self(vec![0.0; len * 2], 0)
    }
    pub fn next_n_frames_mut(&mut self, frames: usize) -> AudioBufferMut {
        if frames > self.len() {
            panic!("超过最大容量")
        }
        if self.0.len() >= (self.1 + frames) * 2 {
            let (_, r) = self.0.split_at_mut(self.1 * 2);
            let (m, _) = r.split_at_mut(frames * 2);
            AudioBufferMut(m, &mut [])
        } else {
            let rest = (self.1 + frames) - self.len();
            let (l, r) = self.0.split_at_mut(self.1 * 2);
            let (ll, _) = l.split_at_mut(rest * 2);
            AudioBufferMut(r, ll)
        }
    }
    pub fn next_n_frames_ref(&self, frames: usize) -> AudioBufferRef {
        if frames > self.len() {
            panic!("超过最大容量")
        }
        if self.0.len() >= (self.1 + frames) * 2 {
            let (_, r) = self.0.split_at(self.1 * 2);
            let (m, _) = r.split_at(frames * 2);
            AudioBufferRef(m, &[])
        } else {
            let rest = (self.1 + frames) - self.len();
            let (l, r) = self.0.split_at(self.1 * 2);
            let (ll, _) = l.split_at(rest * 2);
            AudioBufferRef(r, ll)
        }
    }
    pub fn len(&self) -> usize {
        self.0.len() / 2
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn forward(&mut self, frames: usize) {
        self.1 = (self.1 + frames) % self.len();
    }
}

impl<'a> AudioBufferMut<'a> {
    pub fn len(&self) -> usize {
        (self.0.len() + self.1.len()) / 2
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty() && self.1.is_empty()
    }
    pub fn clear(&mut self) {
        self.0.fill(0.0);
        self.1.fill(0.0);
    }
    pub fn split_at_mut(self, mid: usize) -> (AudioBufferMut<'a>, AudioBufferMut<'a>) {
        assert!(mid <= self.len());
        match self.0.len().cmp(&(mid * 2)) {
            Ordering::Less => {
                let (l, r) = self.1.split_at_mut(mid * 2 - self.0.len());
                (AudioBufferMut(self.0, l), AudioBufferMut(r, &mut []))
            }
            Ordering::Equal => (
                AudioBufferMut(self.0, &mut []),
                AudioBufferMut(self.1, &mut []),
            ),
            Ordering::Greater => {
                let (l, r) = self.0.split_at_mut(mid * 2);
                (AudioBufferMut(l, &mut []), AudioBufferMut(r, self.1))
            }
        }
    }
}

impl<'a> AudioBufferRef<'a> {
    pub fn len(&self) -> usize {
        (self.0.len() + self.1.len()) / 2
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty() && self.1.is_empty()
    }
    pub fn iter(&self) -> AudioBufferIter {
        AudioBufferIter(*self, 0)
    }
    pub fn split_at(&self, mid: usize) -> (AudioBufferRef<'a>, AudioBufferRef<'a>) {
        assert!(mid <= self.len());
        match self.0.len().cmp(&(mid * 2)) {
            Ordering::Less => {
                let (l, r) = self.1.split_at(mid * 2 - self.0.len());
                (AudioBufferRef(self.0, l), AudioBufferRef(r, &[]))
            }
            Ordering::Equal => (AudioBufferRef(self.0, &[]), AudioBufferRef(self.1, &[])),
            Ordering::Greater => {
                let (l, r) = self.0.split_at(mid * 2);
                (AudioBufferRef(l, &[]), AudioBufferRef(r, self.1))
            }
        }
    }
}

impl<'a> AudioBufferIterMut<'a> {
    pub fn into_mut(self) -> AudioBufferMut<'a> {
        self.0
    }
}

impl fmt::Debug for AudioBuffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut data = String::default();
        let buffer_ref = AudioBufferRef(&self.0, &[]);
        for (i, (l, r)) in buffer_ref.iter().enumerate() {
            data += &format!("{}, {}", l, r);
            if i != self.len() - 1 {
                data += "|";
            }
        }
        f.write_str(&data)
    }
}

impl<'a> IntoIterator for AudioBufferMut<'a> {
    type Item = (&'a mut f64, &'a mut f64);
    type IntoIter = AudioBufferIterMut<'a>;

    fn into_iter(self) -> Self::IntoIter {
        AudioBufferIterMut(self, 0)
    }
}

impl<'a> Iterator for AudioBufferIterMut<'a> {
    type Item = (&'a mut f64, &'a mut f64);

    fn next(&mut self) -> Option<Self::Item> {
        if self.1 < self.0 .0.len() / 2 {
            let res = unsafe {
                (
                    self.0 .0.as_mut_ptr().add(self.1 * 2).as_mut().unwrap(),
                    self.0 .0.as_mut_ptr().add(self.1 * 2 + 1).as_mut().unwrap(),
                )
            };
            self.1 += 1;
            Some(res)
        } else if self.1 < self.0.len() {
            let ind = self.1 - self.0 .0.len() / 2;
            let res = unsafe {
                (
                    self.0 .1.as_mut_ptr().add(ind * 2).as_mut().unwrap(),
                    self.0 .1.as_mut_ptr().add(ind * 2 + 1).as_mut().unwrap(),
                )
            };
            self.1 += 1;
            Some(res)
        } else {
            None
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let rest = self.0.len() - self.1;
        (rest, Some(rest))
    }
}

impl<'a> ExactSizeIterator for AudioBufferIterMut<'a> {}

impl<'a> IntoIterator for AudioBufferRef<'a> {
    type Item = (&'a f64, &'a f64);
    type IntoIter = AudioBufferIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        AudioBufferIter(self, 0)
    }
}

impl<'a> IntoIterator for &AudioBufferRef<'a> {
    type Item = (&'a f64, &'a f64);

    type IntoIter = AudioBufferIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        AudioBufferIter(*self, 0)
    }
}

impl<'a> Iterator for AudioBufferIter<'a> {
    type Item = (&'a f64, &'a f64);

    fn next(&mut self) -> Option<Self::Item> {
        if self.1 < self.0 .0.len() / 2 {
            let res = (&self.0 .0[self.1 * 2], &self.0 .0[self.1 * 2 + 1]);
            self.1 += 1;
            Some(res)
        } else if self.1 < self.0.len() {
            let ind = self.1 - self.0 .0.len() / 2;
            let res = (&self.0 .1[ind * 2], &self.0 .1[ind * 2 + 1]);
            self.1 += 1;
            Some(res)
        } else {
            None
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let rest = self.0.len() - self.1;
        (rest, Some(rest))
    }
}

impl<'a> ExactSizeIterator for AudioBufferIter<'a> {}
