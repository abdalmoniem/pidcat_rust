#[derive(Debug, Clone)]
pub struct AnsiSegment {
    pub code: String,       // The ANSI escape sequence
    pub visible_pos: usize, // Position in the visible (plain) text
}
