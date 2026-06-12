//! Beam demo brand icons from [theSVG](https://thesvg.org/) (`default` color variants).
//! OpenAI uses the `light` variant (black on white circles).
//! User avatar keeps a generic stroke icon (not a brand mark).

use iced::widget::svg;

pub fn google_drive() -> svg::Handle {
    svg::Handle::from_memory(include_bytes!("../assets/beam/google-drive.svg"))
}

pub fn notion() -> svg::Handle {
    svg::Handle::from_memory(include_bytes!("../assets/beam/notion.svg"))
}

pub fn openai() -> svg::Handle {
    svg::Handle::from_memory(include_bytes!("../assets/beam/openai.svg"))
}

pub fn google_docs() -> svg::Handle {
    svg::Handle::from_memory(include_bytes!("../assets/beam/google-docs.svg"))
}

pub fn whatsapp() -> svg::Handle {
    svg::Handle::from_memory(include_bytes!("../assets/beam/whatsapp.svg"))
}

pub fn zapier() -> svg::Handle {
    svg::Handle::from_memory(include_bytes!("../assets/beam/zapier.svg"))
}

pub fn messenger() -> svg::Handle {
    svg::Handle::from_memory(include_bytes!("../assets/beam/messenger.svg"))
}

pub fn user() -> svg::Handle {
    svg::Handle::from_memory(include_bytes!("../assets/beam/user.svg"))
}
