pub struct Paragraph(parley::Layout<ParleyBrush>);

#[derive(Clone, PartialEq, Debug)]
pub struct ParleyBrush(pub vello::peniko::Brush);

impl Default for ParleyBrush {
    fn default() -> ParleyBrush {
        ParleyBrush(vello::peniko::Brush::Solid(vello::peniko::Color::rgb8(
            0, 0, 0,
        )))
    }
}

impl parley::style::Brush for ParleyBrush {}

pub struct ParagraphBuilder {}

impl ParagraphBuilder {
    fn new() -> Self {
        todo!()
    }
    fn add_text(&mut self, text: &str) {}

    fn push_style(&mut self, style: TextStyle) {}

    fn pop_style(&mut self) {}

    fn build(self) -> Paragraph {
        todo!()
    }
}

pub struct TextStyle {}
