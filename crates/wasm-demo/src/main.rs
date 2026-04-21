mod app;
mod catalog;
mod reference;
mod route;

use app::WasmDemoApp;
use lucide_icons::LUCIDE_FONT_BYTES;

const GEIST_VARIABLE: &[u8] = include_bytes!("../assets/fonts/Geist/Geist-VariableFont_wght.ttf");
const GEIST_MONO_VARIABLE: &[u8] =
    include_bytes!("../assets/fonts/Geist_Mono/GeistMono-VariableFont_wght.ttf");
const INTER_VARIABLE: &[u8] =
    include_bytes!("../assets/fonts/Inter/Inter-VariableFont_opsz,wght.ttf");
const INTER_ITALIC_VARIABLE: &[u8] =
    include_bytes!("../assets/fonts/Inter/Inter-Italic-VariableFont_opsz,wght.ttf");

pub fn main() -> iced::Result {
    iced::application(WasmDemoApp::boot, WasmDemoApp::update, WasmDemoApp::view)
        .subscription(WasmDemoApp::subscription)
        .font(GEIST_VARIABLE)
        .font(GEIST_MONO_VARIABLE)
        .font(INTER_VARIABLE)
        .font(INTER_ITALIC_VARIABLE)
        .font(LUCIDE_FONT_BYTES)
        .run()
}
