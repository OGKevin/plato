use crate::framebuffer::Pixmap;
use crate::geom::Rectangle;

/// Writer-side handle: owns the back pixmap, can render without locking.
#[derive(Debug)]
pub struct BufferWriter {
    pub back: Pixmap,
    pub dirty_rect: Option<Rectangle>,
}

/// Shared state protected by mutex: only touched during swap.
#[derive(Debug)]
pub struct DoubleBuffer {
    pub front: Pixmap,
    pub dirty: bool,
    pub dirty_rect: Option<Rectangle>,
}

impl DoubleBuffer {
    pub fn new(width: u32, height: u32) -> (Self, BufferWriter) {
        let shared = Self {
            front: Pixmap::new(width, height, 1),
            dirty: false,
            dirty_rect: None,
        };
        let writer = BufferWriter {
            back: Pixmap::new(width, height, 1),
            dirty_rect: None,
        };
        (shared, writer)
    }

    /// Swap front and back pixmaps. Called by writer thread after rendering.
    /// After swap, copy front to back so subsequent incremental renders work correctly.
    pub fn swap(&mut self, writer: &mut BufferWriter) {
        std::mem::swap(&mut self.front, &mut writer.back);
        self.dirty_rect = writer.dirty_rect.take();
        self.dirty = true;
        // Copy front to back so incremental rendering has correct baseline
        writer.back.data.copy_from_slice(&self.front.data);
    }
}
