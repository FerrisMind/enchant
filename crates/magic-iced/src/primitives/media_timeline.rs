use iced::widget::image;

#[derive(Clone, Debug, PartialEq)]
pub struct FrameSequence {
    frames: Box<[image::Handle]>,
    frame_duration_ms: u64,
}

impl FrameSequence {
    pub fn new(frames: Vec<image::Handle>, frame_duration_ms: u64) -> Self {
        Self {
            frames: frames.into_boxed_slice(),
            frame_duration_ms: frame_duration_ms.max(1),
        }
    }

    pub fn frame_at(&self, elapsed_ms: u64) -> Option<&image::Handle> {
        if self.frames.is_empty() {
            return None;
        }

        let index = ((elapsed_ms / self.frame_duration_ms) % self.frames.len() as u64) as usize;
        self.frames.get(index)
    }

    pub fn len(&self) -> usize {
        self.frames.len()
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    pub fn frame_duration_ms(&self) -> u64 {
        self.frame_duration_ms
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn handle(color: [u8; 4]) -> image::Handle {
        image::Handle::from_rgba(1, 1, color.to_vec())
    }

    #[test]
    fn frame_sequence_cycles_by_duration() {
        let first = handle([255, 0, 0, 255]);
        let second = handle([0, 255, 0, 255]);
        let sequence = FrameSequence::new(vec![first.clone(), second.clone()], 100);

        assert_eq!(sequence.len(), 2);
        assert_eq!(sequence.frame_at(0), Some(&first));
        assert_eq!(sequence.frame_at(100), Some(&second));
        assert_eq!(sequence.frame_at(200), Some(&first));
        assert_eq!(sequence.frame_duration_ms(), 100);
    }
}
