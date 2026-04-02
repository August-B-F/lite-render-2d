use crate::types::{Rect, Vec2};

// describes the grid layout of a sprite sheet texture
#[derive(Clone, Debug)]
pub struct SpriteSheet {
    pub frame_width: f32,
    pub frame_height: f32,
    pub columns: u32,
    pub frame_count: u32,
}

impl SpriteSheet {
    pub fn new(frame_width: f32, frame_height: f32, columns: u32, frame_count: u32) -> Self {
        Self { frame_width, frame_height, columns, frame_count }
    }

    // get the source rect for a specific frame index
    pub fn frame_rect(&self, frame_index: u32) -> Rect {
        let idx = frame_index.min(self.frame_count.saturating_sub(1));
        let col = idx % self.columns;
        let row = idx / self.columns;
        Rect {
            pos: Vec2::new(col as f32 * self.frame_width, row as f32 * self.frame_height),
            size: Vec2::new(self.frame_width, self.frame_height),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlaybackMode {
    Loop,
    Once,
    PingPong,
}

// tracks animation state for a sprite sheet
#[derive(Clone, Debug)]
pub struct SpriteAnimation {
    pub sheet: SpriteSheet,
    pub frame_duration: f32,
    pub mode: PlaybackMode,
    current_frame: u32,
    elapsed: f32,
    direction: i32,
    finished: bool,
}

impl SpriteAnimation {
    pub fn new(sheet: SpriteSheet, frame_duration: f32, mode: PlaybackMode) -> Self {
        Self {
            sheet,
            frame_duration,
            mode,
            current_frame: 0,
            elapsed: 0.0,
            direction: 1,
            finished: false,
        }
    }

    // advance the animation by dt seconds
    pub fn update(&mut self, dt: f32) {
        if self.finished || self.sheet.frame_count == 0 {
            return;
        }

        self.elapsed += dt;
        while self.elapsed >= self.frame_duration {
            self.elapsed -= self.frame_duration;
            self.advance_frame();
        }
    }

    fn advance_frame(&mut self) {
        let max = self.sheet.frame_count.saturating_sub(1);
        if max == 0 {
            return;
        }

        match self.mode {
            PlaybackMode::Loop => {
                self.current_frame = (self.current_frame + 1) % self.sheet.frame_count;
            }
            PlaybackMode::Once => {
                if self.current_frame < max {
                    self.current_frame += 1;
                } else {
                    self.finished = true;
                }
            }
            PlaybackMode::PingPong => {
                let next = self.current_frame as i32 + self.direction;
                if next < 0 {
                    self.direction = 1;
                    self.current_frame = 1.min(max);
                } else if next > max as i32 {
                    self.direction = -1;
                    self.current_frame = max.saturating_sub(1);
                } else {
                    self.current_frame = next as u32;
                }
            }
        }
    }

    pub fn current_frame(&self) -> u32 {
        self.current_frame
    }

    // get the source rect for the current animation frame
    pub fn current_src_rect(&self) -> Rect {
        self.sheet.frame_rect(self.current_frame)
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    pub fn reset(&mut self) {
        self.current_frame = 0;
        self.elapsed = 0.0;
        self.direction = 1;
        self.finished = false;
    }

    pub fn set_frame(&mut self, frame: u32) {
        self.current_frame = frame.min(self.sheet.frame_count.saturating_sub(1));
    }
}
