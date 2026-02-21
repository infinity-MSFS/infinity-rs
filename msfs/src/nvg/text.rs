/// Vertical metrics returned by NvgContext::text_metrics.
#[derive(Debug, Clone, Copy)]
pub struct TextMetrics {
    pub ascender: f32,
    pub descender: f32,
    pub line_height: f32,
}

/// Bounding box of measured text, plus the horizontal advance.
#[derive(Debug, Clone, Copy)]
pub struct TextBounds {
    /// Horizontal advance (where the next character would go).
    pub advance: f32,
    /// Bounding box: `[xmin, ymin, xmax, ymax]`.
    pub bounds: [f32; 4],
}

impl TextBounds {
    /// Width of the bounding box.
    pub fn width(&self) -> f32 {
        self.bounds[2] - self.bounds[0]
    }
    /// Height of the bounding box.
    pub fn height(&self) -> f32 {
        self.bounds[3] - self.bounds[1]
    }
}

/// Glyph position info for hit-testing and cursor placement.
#[derive(Debug, Clone, Copy)]
pub struct GlyphPosition {
    /// Byte index into the original string.
    pub byte_index: usize,
    /// Logical x position.
    pub x: f32,
    /// Left edge of the glyph shape.
    pub min_x: f32,
    /// Right edge of the glyph shape.
    pub max_x: f32,
}

/// A single row of broken text from `NvgContext::text_break_lines`.
#[derive(Debug, Clone)]
pub struct TextRow {
    /// Byte offset of the row start in the original string.
    pub start: usize,
    /// Byte offset of the row end (exclusive).
    pub end: usize,
    /// Byte offset of where the *next* row begins.
    pub next: usize,
    /// Logical width of the row.
    pub width: f32,
    /// Actual min-x bound.
    pub min_x: f32,
    /// Actual max-x bound.
    pub max_x: f32,
}
