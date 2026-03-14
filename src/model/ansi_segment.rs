#[derive(Debug, Clone)]
pub struct AnsiSegment {
    pub code: String, // The ANSI escape sequence
    pub pos: usize,   // Position in the visible (plain) text
}
