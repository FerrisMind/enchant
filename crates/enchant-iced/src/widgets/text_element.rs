#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TextElementRole {
    Article,
    Div,
    H1,
    H2,
    H3,
    H4,
    H5,
    H6,
    Li,
    P,
    Section,
    #[default]
    Span,
}

impl TextElementRole {
    pub fn default_font_size(self) -> u16 {
        match self {
            Self::H1 => 40,
            Self::H2 => 34,
            Self::H3 => 30,
            Self::H4 => 26,
            Self::H5 => 22,
            Self::H6 => 20,
            Self::Article | Self::Div | Self::Li | Self::P | Self::Section | Self::Span => 18,
        }
    }
}
