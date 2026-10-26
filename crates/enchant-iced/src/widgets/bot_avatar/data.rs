//! Static avatar data ported from the Libraries.dev `bot-avatars` package.
//!
//! The preset table comes from `presets.ts`; the body outlines and the thin
//! antenna parts come from `shapes.ts` (generated there from
//! `scripts/gen-shapes.mjs`). The strings are kept verbatim: they are SVG path
//! data in a 100×100 box centred on (50, 50).

use super::types::{BotAvatarFace, BotAvatarState, BotAvatarType};

/// One body type's default appearance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BotAvatarPreset {
    /// Display name, for labels and accessible names.
    pub label: &'static str,
    /// The type's own body colour.
    pub color: &'static str,
    /// The face the type ships with.
    pub face: BotAvatarFace,
    /// Where the face sits horizontally, in the 100×100 body box.
    pub face_x: f32,
    /// Where the face sits vertically, in the 100×100 body box.
    pub face_y: f32,
    /// How large the face is drawn.
    pub face_scale: f32,
    /// The type's own brightness override, when it has one.
    pub brightness: Option<f32>,
    /// The type's own saturation override, when it has one.
    pub saturation: Option<f32>,
}

/// The eighteen body types, in the web package's order.
pub const BOT_AVATAR_TYPES: [BotAvatarType; 18] = [
    BotAvatarType::Clover,
    BotAvatarType::Flower,
    BotAvatarType::Triangle,
    BotAvatarType::Square,
    BotAvatarType::Blob,
    BotAvatarType::Ghost,
    BotAvatarType::Circle,
    BotAvatarType::Drop,
    BotAvatarType::Star,
    BotAvatarType::Droid,
    BotAvatarType::Mech,
    BotAvatarType::Alien,
    BotAvatarType::Hexagon,
    BotAvatarType::Cat,
    BotAvatarType::Cloud,
    BotAvatarType::Pill,
    BotAvatarType::Pebble,
    BotAvatarType::Puddle,
];

/// The two face kinds, in the web package's order.
pub const BOT_AVATAR_FACES: [BotAvatarFace; 2] = [BotAvatarFace::Eyes, BotAvatarFace::Mouth];

/// The three animation states, in the web package's order.
pub const BOT_AVATAR_STATES: [BotAvatarState; 3] = [
    BotAvatarState::Idle,
    BotAvatarState::Working,
    BotAvatarState::Sleeping,
];

/// The preset for a body type.
#[must_use]
pub const fn bot_avatar_preset(type_: BotAvatarType) -> BotAvatarPreset {
    match type_ {
        BotAvatarType::Clover => BotAvatarPreset {
            label: "Clover",
            color: "#41C4FF",
            face: BotAvatarFace::Eyes,
            face_x: 50.0,
            face_y: 50.0,
            face_scale: 1.0,
            brightness: Some(1.2),
            saturation: Some(1.59),
        },
        BotAvatarType::Flower => BotAvatarPreset {
            label: "Flower",
            color: "#2FCB7A",
            face: BotAvatarFace::Eyes,
            face_x: 50.0,
            face_y: 51.0,
            face_scale: 0.95,
            brightness: None,
            saturation: None,
        },
        BotAvatarType::Triangle => BotAvatarPreset {
            label: "Triangle",
            color: "#DC48FF",
            face: BotAvatarFace::Eyes,
            face_x: 50.0,
            face_y: 61.0,
            face_scale: 0.9,
            brightness: None,
            saturation: None,
        },
        BotAvatarType::Square => BotAvatarPreset {
            label: "Square",
            color: "#35B8FF",
            face: BotAvatarFace::Eyes,
            face_x: 50.0,
            face_y: 50.0,
            face_scale: 1.0,
            brightness: None,
            saturation: None,
        },
        BotAvatarType::Blob => BotAvatarPreset {
            label: "Blob",
            color: "#2FCB7A",
            face: BotAvatarFace::Eyes,
            face_x: 49.5,
            face_y: 50.0,
            face_scale: 1.0,
            brightness: None,
            saturation: None,
        },
        BotAvatarType::Ghost => BotAvatarPreset {
            label: "Ghost",
            color: "#F4F2FA",
            face: BotAvatarFace::Eyes,
            face_x: 50.0,
            face_y: 48.0,
            face_scale: 0.95,
            brightness: None,
            saturation: None,
        },
        BotAvatarType::Circle => BotAvatarPreset {
            label: "Circle",
            color: "#9A62FF",
            face: BotAvatarFace::Eyes,
            face_x: 50.0,
            face_y: 50.0,
            face_scale: 1.0,
            brightness: None,
            saturation: None,
        },
        BotAvatarType::Drop => BotAvatarPreset {
            label: "Drop",
            color: "#1ED3C6",
            face: BotAvatarFace::Eyes,
            face_x: 50.0,
            face_y: 62.0,
            face_scale: 0.9,
            brightness: None,
            saturation: None,
        },
        BotAvatarType::Star => BotAvatarPreset {
            label: "Star",
            color: "#EFDB8E",
            face: BotAvatarFace::Eyes,
            face_x: 50.0,
            face_y: 52.0,
            face_scale: 0.82,
            brightness: Some(1.1),
            saturation: Some(1.84),
        },
        BotAvatarType::Droid => BotAvatarPreset {
            label: "Droid",
            color: "#D5DBEA",
            face: BotAvatarFace::Eyes,
            face_x: 50.0,
            face_y: 60.0,
            face_scale: 0.95,
            brightness: None,
            saturation: None,
        },
        BotAvatarType::Mech => BotAvatarPreset {
            label: "Mech",
            color: "#95A6C4",
            face: BotAvatarFace::Eyes,
            face_x: 50.0,
            face_y: 59.0,
            face_scale: 1.0,
            brightness: None,
            saturation: None,
        },
        BotAvatarType::Alien => BotAvatarPreset {
            label: "Alien",
            color: "#9CD66A",
            face: BotAvatarFace::Eyes,
            face_x: 50.0,
            face_y: 45.0,
            face_scale: 1.05,
            brightness: None,
            saturation: None,
        },
        BotAvatarType::Hexagon => BotAvatarPreset {
            label: "Hexagon",
            color: "#EB7575",
            face: BotAvatarFace::Eyes,
            face_x: 50.0,
            face_y: 50.0,
            face_scale: 0.95,
            brightness: None,
            saturation: None,
        },
        BotAvatarType::Cat => BotAvatarPreset {
            label: "Cat",
            color: "#EAA06F",
            face: BotAvatarFace::Eyes,
            face_x: 50.0,
            face_y: 58.0,
            face_scale: 1.0,
            brightness: Some(1.22),
            saturation: Some(1.88),
        },
        BotAvatarType::Cloud => BotAvatarPreset {
            label: "Cloud",
            color: "#CFD5E4",
            face: BotAvatarFace::Eyes,
            face_x: 50.0,
            face_y: 58.0,
            face_scale: 0.95,
            brightness: None,
            saturation: None,
        },
        BotAvatarType::Pill => BotAvatarPreset {
            label: "Pill",
            color: "#ACAAF3",
            face: BotAvatarFace::Eyes,
            face_x: 50.0,
            face_y: 50.0,
            face_scale: 0.9,
            brightness: None,
            saturation: None,
        },
        BotAvatarType::Pebble => BotAvatarPreset {
            label: "Pebble",
            color: "#ABC793",
            face: BotAvatarFace::Eyes,
            face_x: 50.0,
            face_y: 50.0,
            face_scale: 0.95,
            brightness: None,
            saturation: None,
        },
        BotAvatarType::Puddle => BotAvatarPreset {
            label: "Puddle",
            color: "#EE8BDB",
            face: BotAvatarFace::Eyes,
            face_x: 50.0,
            face_y: 50.0,
            face_scale: 0.95,
            brightness: None,
            saturation: None,
        },
    }
}

/// The body outline for a type, as SVG path data.
#[must_use]
pub const fn bot_avatar_shape(type_: BotAvatarType) -> &'static str {
    match type_ {
        BotAvatarType::Clover => SHAPE_CLOVER,
        BotAvatarType::Flower => SHAPE_FLOWER,
        BotAvatarType::Triangle => SHAPE_TRIANGLE,
        BotAvatarType::Square => SHAPE_SQUARE,
        BotAvatarType::Blob => SHAPE_BLOB,
        BotAvatarType::Ghost => SHAPE_GHOST,
        BotAvatarType::Circle => SHAPE_CIRCLE,
        BotAvatarType::Drop => SHAPE_DROP,
        BotAvatarType::Star => SHAPE_STAR,
        BotAvatarType::Droid => SHAPE_DROID,
        BotAvatarType::Mech => SHAPE_MECH,
        BotAvatarType::Alien => SHAPE_ALIEN,
        BotAvatarType::Hexagon => SHAPE_HEXAGON,
        BotAvatarType::Cat => SHAPE_CAT,
        BotAvatarType::Cloud => SHAPE_CLOUD,
        BotAvatarType::Pill => SHAPE_PILL,
        BotAvatarType::Pebble => SHAPE_PEBBLE,
        BotAvatarType::Puddle => SHAPE_PUDDLE,
    }
}

/// Thin parts (antennae) drawn with a fraction of the body's depth.
///
/// Only `droid` and `mech` have these.
#[must_use]
pub const fn bot_avatar_parts(type_: BotAvatarType) -> Option<&'static str> {
    match type_ {
        BotAvatarType::Droid => Some(SHAPE_PARTS_DROID),
        BotAvatarType::Mech => Some(SHAPE_PARTS_MECH),
        _ => None,
    }
}

/// Body colour by type — the palette on its own.
#[must_use]
pub const fn bot_avatar_palette(type_: BotAvatarType) -> &'static str {
    bot_avatar_preset(type_).color
}

/// The web package's display label for a state (`default` is "idle").
#[must_use]
pub const fn bot_avatar_state_label(state: BotAvatarState) -> &'static str {
    match state {
        BotAvatarState::Idle => "idle",
        BotAvatarState::Working => "working",
        BotAvatarState::Sleeping => "sleeping",
    }
}

impl BotAvatarType {
    /// All eighteen body types, in the web package's order.
    pub const ALL: [Self; 18] = BOT_AVATAR_TYPES;

    /// The type's default preset.
    #[must_use]
    pub const fn preset(self) -> BotAvatarPreset {
        bot_avatar_preset(self)
    }

    /// The type's display label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        bot_avatar_preset(self).label
    }
}

impl BotAvatarState {
    /// The web package's display label for the state.
    #[must_use]
    pub const fn label(self) -> &'static str {
        bot_avatar_state_label(self)
    }
}

// Body outlines and thin antenna parts, copied verbatim from `shapes.ts`.

const SHAPE_CLOVER: &str = "M26.53 22.38A25 25 0 0 1 73.47 22.38A7 7 0 0 0 77.62 26.53A25 25 0 0 1 77.62 73.47A7 7 0 0 0 73.47 77.62A25 25 0 0 1 26.53 77.62A7 7 0 0 0 22.38 73.47A25 25 0 0 1 22.38 26.53A7 7 0 0 0 26.53 22.38Z";
const SHAPE_FLOWER: &str = "M31.2 19.82A20.5 20.5 0 0 1 68.8 19.82A5 5 0 0 0 72.9 22.8A20.5 20.5 0 0 1 84.51 58.55A5 5 0 0 0 82.95 63.37A20.5 20.5 0 0 1 52.54 85.47A5 5 0 0 0 47.46 85.47A20.5 20.5 0 0 1 17.05 63.37A5 5 0 0 0 15.49 58.55A20.5 20.5 0 0 1 27.1 22.8A5 5 0 0 0 31.2 19.82Z";
const SHAPE_TRIANGLE: &str = "M38.75 27.43A13 13 0 0 1 61.25 27.43L82.7 64.49A13 13 0 0 1 71.45 84L28.55 84A13 13 0 0 1 17.3 64.49L38.75 27.43Z";
const SHAPE_SQUARE: &str = "M93 50C93 53.66 92.96 58.23 92.88 60.97C92.8 63.71 92.67 64.81 92.51 66.44C92.35 68.08 92.15 69.44 91.9 70.77C91.66 72.1 91.37 73.3 91.04 74.44C90.72 75.58 90.35 76.63 89.94 77.63C89.53 78.63 89.07 79.55 88.58 80.43C88.08 81.31 87.54 82.13 86.96 82.9C86.37 83.67 85.75 84.39 85.07 85.07C84.39 85.75 83.67 86.37 82.9 86.96C82.13 87.54 81.31 88.08 80.43 88.58C79.55 89.07 78.63 89.53 77.63 89.94C76.63 90.35 75.58 90.72 74.44 91.04C73.3 91.37 72.1 91.66 70.77 91.9C69.44 92.15 68.08 92.35 66.44 92.51C64.81 92.67 63.71 92.8 60.97 92.88C58.23 92.96 53.66 93 50 93C46.34 93 41.77 92.96 39.03 92.88C36.29 92.8 35.19 92.67 33.56 92.51C31.92 92.35 30.56 92.15 29.23 91.9C27.9 91.66 26.7 91.37 25.56 91.04C24.42 90.72 23.37 90.35 22.37 89.94C21.37 89.53 20.45 89.07 19.57 88.58C18.69 88.08 17.87 87.54 17.1 86.96C16.33 86.37 15.61 85.75 14.93 85.07C14.25 84.39 13.63 83.67 13.04 82.9C12.46 82.13 11.92 81.31 11.42 80.43C10.93 79.55 10.47 78.63 10.06 77.63C9.65 76.63 9.28 75.58 8.96 74.44C8.63 73.3 8.34 72.1 8.1 70.77C7.85 69.44 7.65 68.08 7.49 66.44C7.33 64.81 7.2 63.71 7.12 60.97C7.04 58.23 7 53.66 7 50C7 46.34 7.04 41.77 7.12 39.03C7.2 36.29 7.33 35.19 7.49 33.56C7.65 31.92 7.85 30.56 8.1 29.23C8.34 27.9 8.63 26.7 8.96 25.56C9.28 24.42 9.65 23.37 10.06 22.37C10.47 21.37 10.93 20.45 11.42 19.57C11.92 18.69 12.46 17.87 13.04 17.1C13.63 16.33 14.25 15.61 14.93 14.93C15.61 14.25 16.33 13.63 17.1 13.04C17.87 12.46 18.69 11.92 19.57 11.42C20.45 10.93 21.37 10.47 22.37 10.06C23.37 9.65 24.42 9.28 25.56 8.96C26.7 8.63 27.9 8.34 29.23 8.1C30.56 7.85 31.92 7.65 33.56 7.49C35.19 7.33 36.29 7.2 39.03 7.12C41.77 7.04 46.34 7 50 7C53.66 7 58.23 7.04 60.97 7.12C63.71 7.2 64.81 7.33 66.44 7.49C68.08 7.65 69.44 7.85 70.77 8.1C72.1 8.34 73.3 8.63 74.44 8.96C75.58 9.28 76.63 9.65 77.63 10.06C78.63 10.47 79.55 10.93 80.43 11.42C81.31 11.92 82.13 12.46 82.9 13.04C83.67 13.63 84.39 14.25 85.07 14.93C85.75 15.61 86.37 16.33 86.96 17.1C87.54 17.87 88.08 18.69 88.58 19.57C89.07 20.45 89.53 21.37 89.94 22.37C90.35 23.37 90.72 24.42 91.04 25.56C91.37 26.7 91.66 27.9 91.9 29.23C92.15 30.56 92.35 31.92 92.51 33.56C92.67 35.19 92.8 36.29 92.88 39.03C92.96 41.77 93 46.34 93 50Z";
const SHAPE_BLOB: &str = "M93.92 50C94.18 51.9 94.24 53.9 93.99 55.79C93.73 57.68 93.18 59.61 92.39 61.36C91.6 63.1 90.48 64.78 89.27 66.27C88.07 67.76 86.58 69.08 85.14 70.29C83.7 71.49 82.1 72.5 80.62 73.5C79.14 74.49 77.64 75.34 76.24 76.24C74.84 77.15 73.51 78 72.2 78.93C70.88 79.86 69.66 80.83 68.36 81.8C67.06 82.76 65.78 83.82 64.39 84.74C63 85.67 61.55 86.63 60.01 87.35C58.47 88.08 56.82 88.73 55.15 89.1C53.48 89.47 51.71 89.64 50 89.57C48.29 89.5 46.55 89.16 44.91 88.68C43.27 88.21 41.67 87.47 40.16 86.72C38.65 85.96 37.24 85.02 35.86 84.13C34.48 83.24 33.19 82.29 31.89 81.37C30.58 80.46 29.32 79.56 28.03 78.63C26.74 77.71 25.44 76.82 24.16 75.84C22.88 74.86 21.57 73.87 20.36 72.74C19.15 71.62 17.95 70.42 16.91 69.1C15.88 67.79 14.92 66.35 14.15 64.85C13.38 63.35 12.76 61.73 12.29 60.1C11.81 58.48 11.52 56.78 11.29 55.1C11.06 53.41 10.98 51.71 10.91 50C10.83 48.29 10.83 46.59 10.83 44.84C10.84 43.1 10.85 41.34 10.93 39.53C11.02 37.72 11.09 35.86 11.34 33.99C11.59 32.12 11.88 30.16 12.43 28.31C12.99 26.46 13.69 24.56 14.66 22.88C15.64 21.21 16.86 19.58 18.27 18.27C19.69 16.96 21.4 15.84 23.17 15.03C24.93 14.22 26.95 13.71 28.89 13.43C30.83 13.15 32.9 13.21 34.82 13.36C36.74 13.5 38.66 13.92 40.43 14.28C42.2 14.65 43.87 15.16 45.46 15.54C47.06 15.93 48.52 16.31 50 16.58C51.48 16.85 52.87 17.02 54.32 17.17C55.77 17.32 57.22 17.36 58.71 17.49C60.2 17.62 61.74 17.7 63.28 17.95C64.81 18.19 66.39 18.5 67.91 18.98C69.43 19.46 70.95 20.08 72.38 20.83C73.81 21.58 75.19 22.5 76.5 23.5C77.81 24.5 79.03 25.63 80.22 26.81C81.41 27.99 82.54 29.25 83.65 30.57C84.76 31.89 85.85 33.26 86.89 34.72C87.93 36.18 88.98 37.69 89.9 39.31C90.82 40.92 91.73 42.64 92.4 44.42C93.07 46.2 93.66 48.1 93.92 50Z";
const SHAPE_GHOST: &str = "M17 50C17 31.78 31.78 17 50 17C68.22 17 83 31.78 83 50V81.5Q72 93.5 61 81.5Q50 93.5 39 81.5Q28 93.5 17 81.5Z";
const SHAPE_CIRCLE: &str = "M50 8A42 42 0 1 1 50 92A42 42 0 1 1 50 8Z";
const SHAPE_DROP: &str = "M50 8.5C52.2 8.5 53.4 10.3 56.4 15.2C62.9 25.5 84 44.6 84 61.5C84 80.3 68.8 92 50 92C31.2 92 16 80.3 16 61.5C16 44.6 37.1 25.5 43.6 15.2C46.6 10.3 47.8 8.5 50 8.5Z";
const SHAPE_STAR: &str = "M45.77 9.7A5 5 0 0 1 54.23 9.7L65.02 26.81A4 4 0 0 0 67.42 28.55L87.02 33.53A5 5 0 0 1 89.63 41.57L76.7 57.12A4 4 0 0 0 75.78 59.94L77.11 80.12A5 5 0 0 1 70.26 85.09L51.48 77.59A4 4 0 0 0 48.52 77.59L29.74 85.09A5 5 0 0 1 22.89 80.12L24.22 59.94A4 4 0 0 0 23.3 57.12L10.37 41.57A5 5 0 0 1 12.98 33.53L32.58 28.55A4 4 0 0 0 34.98 26.81L45.77 9.7Z";
const SHAPE_DROID: &str = "M16 50C16 38.95 24.95 30 36 30H64C75.05 30 84 38.95 84 50V70C84 81.05 75.05 90 64 90H36C24.95 90 16 81.05 16 70ZM4 62A7 7 0 1 1 18 62A7 7 0 1 1 4 62ZM82 62A7 7 0 1 1 96 62A7 7 0 1 1 82 62Z";
const SHAPE_MECH: &str = "M10 48C10 38.06 18.06 30 28 30H72C81.94 30 90 38.06 90 48V70C90 79.94 81.94 88 72 88H28C18.06 88 10 79.94 10 70ZM3 54C3 51.79 4.79 50 7 50H11V72H7C4.79 72 3 70.21 3 68ZM89 50H93C95.21 50 97 51.79 97 54V68C97 70.21 95.21 72 93 72H89Z";
const SHAPE_ALIEN: &str =
    "M50 10C70 10 83 27 83 46C83 65 64 92 50 92C36 92 17 65 17 46C17 27 30 10 50 10Z";
const SHAPE_HEXAGON: &str = "M91.4 45.5A9 9 0 0 1 91.4 54.5L74.6 83.61A9 9 0 0 1 66.8 88.11L33.2 88.11A9 9 0 0 1 25.4 83.61L8.6 54.5A9 9 0 0 1 8.6 45.5L25.4 16.39A9 9 0 0 1 33.2 11.89L66.8 11.89A9 9 0 0 1 74.6 16.39L91.4 45.5Z";
const SHAPE_CAT: &str = "M50 20A36 36 0 1 1 50 92A36 36 0 1 1 50 20ZM24.72 44.64A4.5 4.5 0 0 1 17.35 40.67L20.21 16.66A4.5 4.5 0 0 1 26.86 13.26L42.3 21.83A4.5 4.5 0 0 1 43.02 29.21L24.72 44.64ZM82.65 40.67A4.5 4.5 0 0 1 75.28 44.64L56.98 29.21A4.5 4.5 0 0 1 57.7 21.83L73.14 13.26A4.5 4.5 0 0 1 79.79 16.66L82.65 40.67Z";
const SHAPE_CLOUD: &str = "M19 44A25 25 0 1 1 69 44A25 25 0 1 1 19 44ZM47 50A21 21 0 1 1 89 50A21 21 0 1 1 47 50ZM7 68A17 17 0 1 1 41 68A17 17 0 1 1 7 68ZM32 73A18 18 0 1 1 68 73A18 18 0 1 1 32 73ZM60 70A16 16 0 1 1 92 70A16 16 0 1 1 60 70Z";
const SHAPE_PILL: &str = "M28 28H72C84.15 28 94 37.85 94 50C94 62.15 84.15 72 72 72H28C15.85 72 6 62.15 6 50C6 37.85 15.85 28 28 28Z";
const SHAPE_PEBBLE: &str = "M94.6 50C94.71 51.92 94.56 53.93 94.17 55.82C93.79 57.7 93.13 59.6 92.28 61.33C91.44 63.06 90.32 64.72 89.1 66.19C87.87 67.67 86.41 69.01 84.92 70.16C83.42 71.31 81.75 72.29 80.11 73.1C78.47 73.92 76.73 74.55 75.06 75.06C73.39 75.58 71.7 75.91 70.1 76.2C68.5 76.48 66.93 76.62 65.45 76.76C63.96 76.9 62.55 76.94 61.2 77.03C59.84 77.11 58.57 77.16 57.31 77.26C56.05 77.36 54.86 77.48 53.64 77.63C52.42 77.79 51.24 77.98 50 78.18C48.76 78.39 47.52 78.64 46.2 78.85C44.89 79.06 43.53 79.3 42.11 79.46C40.69 79.61 39.2 79.75 37.67 79.77C36.14 79.78 34.54 79.74 32.95 79.54C31.35 79.33 29.7 79.03 28.1 78.55C26.49 78.07 24.86 77.45 23.33 76.67C21.8 75.88 20.29 74.94 18.92 73.85C17.54 72.77 16.23 71.51 15.08 70.16C13.94 68.8 12.9 67.29 12.04 65.72C11.17 64.16 10.45 62.46 9.91 60.74C9.36 59.03 8.98 57.22 8.76 55.43C8.55 53.64 8.5 51.8 8.6 50C8.71 48.2 8.98 46.39 9.38 44.65C9.79 42.91 10.35 41.19 11.02 39.56C11.7 37.92 12.52 36.34 13.43 34.85C14.34 33.37 15.39 31.96 16.5 30.66C17.61 29.36 18.84 28.15 20.11 27.07C21.39 25.98 22.76 25 24.15 24.15C25.54 23.3 27.01 22.56 28.48 21.95C29.94 21.33 31.46 20.85 32.95 20.46C34.44 20.08 35.95 19.83 37.43 19.66C38.91 19.48 40.38 19.43 41.81 19.43C43.24 19.43 44.64 19.53 46.01 19.66C47.37 19.78 48.7 19.98 50 20.18C51.3 20.39 52.57 20.63 53.83 20.87C55.1 21.11 56.34 21.37 57.6 21.62C58.87 21.87 60.13 22.12 61.43 22.39C62.74 22.66 64.07 22.93 65.45 23.24C66.83 23.56 68.25 23.88 69.72 24.3C71.19 24.72 72.71 25.17 74.25 25.75C75.78 26.34 77.37 27 78.91 27.81C80.46 28.63 82.04 29.56 83.5 30.66C84.97 31.75 86.43 33 87.7 34.38C88.98 35.77 90.19 37.32 91.17 38.97C92.14 40.62 92.98 42.43 93.56 44.27C94.13 46.1 94.5 48.08 94.6 50Z";
const SHAPE_PUDDLE: &str = "M85.34 50C85.65 51.54 85.9 53.13 86.01 54.74C86.11 56.35 86.11 58 85.95 59.63C85.8 61.26 85.51 62.93 85.07 64.52C84.62 66.12 84.03 67.72 83.3 69.23C82.58 70.74 81.7 72.21 80.72 73.57C79.74 74.94 78.62 76.23 77.43 77.43C76.24 78.62 74.93 79.72 73.58 80.73C72.23 81.74 70.79 82.65 69.33 83.48C67.86 84.31 66.34 85.04 64.79 85.71C63.24 86.37 61.65 86.95 60.04 87.46C58.42 87.96 56.77 88.4 55.1 88.75C53.43 89.09 51.72 89.37 50 89.54C48.28 89.71 46.52 89.81 44.76 89.76C43.01 89.72 41.22 89.58 39.47 89.29C37.72 89 35.95 88.58 34.26 88C32.57 87.42 30.89 86.69 29.33 85.8C27.77 84.92 26.26 83.87 24.91 82.7C23.56 81.53 22.3 80.19 21.23 78.77C20.16 77.35 19.22 75.79 18.47 74.2C17.71 72.61 17.13 70.91 16.7 69.23C16.26 67.55 16.02 65.81 15.88 64.13C15.75 62.45 15.78 60.76 15.88 59.14C15.97 57.52 16.21 55.94 16.45 54.42C16.7 52.89 17.03 51.43 17.34 50C17.65 48.57 18 47.2 18.32 45.83C18.63 44.46 18.94 43.13 19.24 41.76C19.53 40.39 19.79 39.03 20.08 37.61C20.36 36.18 20.62 34.74 20.95 33.23C21.28 31.72 21.61 30.16 22.07 28.57C22.52 26.98 23.02 25.32 23.69 23.69C24.35 22.06 25.11 20.38 26.05 18.79C26.99 17.21 28.08 15.62 29.33 14.2C30.58 12.78 32 11.41 33.54 10.26C35.08 9.12 36.8 8.11 38.57 7.35C40.34 6.6 42.27 6.04 44.17 5.73C46.08 5.43 48.08 5.37 50 5.54C51.92 5.71 53.87 6.15 55.69 6.75C57.52 7.36 59.3 8.22 60.94 9.19C62.57 10.15 64.11 11.33 65.51 12.56C66.91 13.78 68.17 15.15 69.33 16.52C70.48 17.89 71.5 19.34 72.44 20.76C73.38 22.18 74.19 23.62 74.97 25.03C75.75 26.43 76.44 27.83 77.12 29.19C77.8 30.56 78.42 31.89 79.05 33.23C79.67 34.57 80.28 35.87 80.87 37.21C81.46 38.55 82.05 39.88 82.6 41.27C83.14 42.65 83.68 44.05 84.14 45.51C84.6 46.96 85.03 48.46 85.34 50Z";

const SHAPE_PARTS_DROID: &str = "M47.5 14H52.5V32H47.5ZM43 11A7 7 0 1 1 57 11A7 7 0 1 1 43 11Z";
const SHAPE_PARTS_MECH: &str = "M19.5 32L24.5 32L17 13L12 13ZM75.5 32L80.5 32L88 13L83 13ZM10 11.5A4.5 4.5 0 1 1 19 11.5A4.5 4.5 0 1 1 10 11.5ZM81 11.5A4.5 4.5 0 1 1 90 11.5A4.5 4.5 0 1 1 81 11.5Z";

#[cfg(test)]
mod tests {
    use super::*;

    /// `(label, colour, face, faceX, faceY, faceScale, brightness, saturation)`
    /// from `presets.ts`.
    const EXPECTED: [(
        &str,
        &str,
        BotAvatarFace,
        f32,
        f32,
        f32,
        Option<f32>,
        Option<f32>,
    ); 18] = [
        (
            "Clover",
            "#41C4FF",
            BotAvatarFace::Eyes,
            50.0,
            50.0,
            1.0,
            Some(1.2),
            Some(1.59),
        ),
        (
            "Flower",
            "#2FCB7A",
            BotAvatarFace::Eyes,
            50.0,
            51.0,
            0.95,
            None,
            None,
        ),
        (
            "Triangle",
            "#DC48FF",
            BotAvatarFace::Eyes,
            50.0,
            61.0,
            0.9,
            None,
            None,
        ),
        (
            "Square",
            "#35B8FF",
            BotAvatarFace::Eyes,
            50.0,
            50.0,
            1.0,
            None,
            None,
        ),
        (
            "Blob",
            "#2FCB7A",
            BotAvatarFace::Eyes,
            49.5,
            50.0,
            1.0,
            None,
            None,
        ),
        (
            "Ghost",
            "#F4F2FA",
            BotAvatarFace::Eyes,
            50.0,
            48.0,
            0.95,
            None,
            None,
        ),
        (
            "Circle",
            "#9A62FF",
            BotAvatarFace::Eyes,
            50.0,
            50.0,
            1.0,
            None,
            None,
        ),
        (
            "Drop",
            "#1ED3C6",
            BotAvatarFace::Eyes,
            50.0,
            62.0,
            0.9,
            None,
            None,
        ),
        (
            "Star",
            "#EFDB8E",
            BotAvatarFace::Eyes,
            50.0,
            52.0,
            0.82,
            Some(1.1),
            Some(1.84),
        ),
        (
            "Droid",
            "#D5DBEA",
            BotAvatarFace::Eyes,
            50.0,
            60.0,
            0.95,
            None,
            None,
        ),
        (
            "Mech",
            "#95A6C4",
            BotAvatarFace::Eyes,
            50.0,
            59.0,
            1.0,
            None,
            None,
        ),
        (
            "Alien",
            "#9CD66A",
            BotAvatarFace::Eyes,
            50.0,
            45.0,
            1.05,
            None,
            None,
        ),
        (
            "Hexagon",
            "#EB7575",
            BotAvatarFace::Eyes,
            50.0,
            50.0,
            0.95,
            None,
            None,
        ),
        (
            "Cat",
            "#EAA06F",
            BotAvatarFace::Eyes,
            50.0,
            58.0,
            1.0,
            Some(1.22),
            Some(1.88),
        ),
        (
            "Cloud",
            "#CFD5E4",
            BotAvatarFace::Eyes,
            50.0,
            58.0,
            0.95,
            None,
            None,
        ),
        (
            "Pill",
            "#ACAAF3",
            BotAvatarFace::Eyes,
            50.0,
            50.0,
            0.9,
            None,
            None,
        ),
        (
            "Pebble",
            "#ABC793",
            BotAvatarFace::Eyes,
            50.0,
            50.0,
            0.95,
            None,
            None,
        ),
        (
            "Puddle",
            "#EE8BDB",
            BotAvatarFace::Eyes,
            50.0,
            50.0,
            0.95,
            None,
            None,
        ),
    ];

    #[test]
    fn all_types_are_present_in_web_order() {
        assert_eq!(BOT_AVATAR_TYPES.len(), 18);
        assert_eq!(BOT_AVATAR_TYPES, BotAvatarType::ALL);
        assert_eq!(
            BOT_AVATAR_FACES,
            [BotAvatarFace::Eyes, BotAvatarFace::Mouth]
        );
        assert_eq!(
            BOT_AVATAR_STATES,
            [
                BotAvatarState::Idle,
                BotAvatarState::Working,
                BotAvatarState::Sleeping
            ]
        );

        for (index, type_) in BOT_AVATAR_TYPES.iter().enumerate() {
            assert_eq!(EXPECTED[index].0, type_.label());
        }
    }

    #[test]
    fn presets_match_source_table() {
        for (index, type_) in BOT_AVATAR_TYPES.iter().enumerate() {
            let preset = bot_avatar_preset(*type_);
            let expected = EXPECTED[index];

            assert_eq!(preset.label, expected.0);
            assert_eq!(preset.color, expected.1);
            assert_eq!(preset.face, expected.2);
            assert_eq!(preset.face_x, expected.3);
            assert_eq!(preset.face_y, expected.4);
            assert_eq!(preset.face_scale, expected.5);
            assert_eq!(preset.brightness, expected.6);
            assert_eq!(preset.saturation, expected.7);
            assert_eq!(bot_avatar_palette(*type_), preset.color);
            assert_eq!(type_.preset(), preset);
        }
    }

    #[test]
    fn every_shape_is_a_complete_closed_path() {
        for type_ in BOT_AVATAR_TYPES {
            let path = bot_avatar_shape(type_);
            assert!(!path.is_empty(), "{type_:?} has no outline");
            assert!(path.starts_with('M'), "{type_:?} does not start with M");
            assert!(path.ends_with('Z'), "{type_:?} does not end with Z");
            assert!(path.len() > 20, "{type_:?} outline looks truncated");
            assert!(
                path.bytes().all(|byte| byte.is_ascii()),
                "{type_:?} outline has non-ASCII text"
            );
        }
    }

    #[test]
    fn only_droid_and_mech_have_parts() {
        for type_ in BOT_AVATAR_TYPES {
            let expected = matches!(type_, BotAvatarType::Droid | BotAvatarType::Mech);
            let parts = bot_avatar_parts(type_);
            assert_eq!(parts.is_some(), expected, "{type_:?}");
            if let Some(parts) = parts {
                assert!(parts.starts_with('M'));
                assert!(parts.ends_with('Z'));
            }
        }
    }

    #[test]
    fn state_labels_match_source_table() {
        assert_eq!(bot_avatar_state_label(BotAvatarState::Idle), "idle");
        assert_eq!(bot_avatar_state_label(BotAvatarState::Working), "working");
        assert_eq!(bot_avatar_state_label(BotAvatarState::Sleeping), "sleeping");
        assert_eq!(BotAvatarState::Idle.label(), "idle");
        assert_eq!(BotAvatarState::Working.label(), "working");
        assert_eq!(BotAvatarState::Sleeping.label(), "sleeping");
    }

    #[test]
    fn accessors_are_const_usable() {
        const PRESET: BotAvatarPreset = bot_avatar_preset(BotAvatarType::Star);
        const SHAPE: &str = bot_avatar_shape(BotAvatarType::Star);
        const PARTS: Option<&str> = bot_avatar_parts(BotAvatarType::Droid);
        const PALETTE: &str = bot_avatar_palette(BotAvatarType::Star);

        assert_eq!(PRESET.label, "Star");
        assert!(SHAPE.starts_with('M'));
        assert!(PARTS.is_some());
        assert_eq!(PALETTE, "#EFDB8E");
    }
}
