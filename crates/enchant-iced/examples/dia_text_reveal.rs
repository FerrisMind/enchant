mod _shared;

use iced::time::Instant;
use iced::widget::column;
use iced::{Color, Element, Size, Subscription, Task};
use enchant_iced::{DiaTextRevealProps, DiaTextRevealState, dia_text_reveal};
use std::time::Duration;

const DEMO_COLORS_1: &[Color] = &[
    Color::from_rgb8(0xA9, 0x7C, 0xF8),
    Color::from_rgb8(0xF3, 0x8C, 0xB8),
    Color::from_rgb8(0xFD, 0xCC, 0x92),
];

const DEMO_COLORS_2: &[Color] = &[
    Color::from_rgb8(0x22, 0xd3, 0xee),
    Color::from_rgb8(0x81, 0x8c, 0xf8),
    Color::from_rgb8(0xf4, 0x72, 0xb6),
    Color::from_rgb8(0x34, 0xd3, 0x99),
];

const DEMO_COLORS_4: &[Color] = &[
    Color::from_rgb8(0xf9, 0x73, 0x16),
    Color::from_rgb8(0xea, 0xb3, 0x08),
    Color::from_rgb8(0x22, 0xc5, 0x5e),
    Color::from_rgb8(0x3b, 0x82, 0xf6),
    Color::from_rgb8(0xa8, 0x55, 0xf7),
];

const ROTATING_TEXTS: &[&str] = &["build faster", "ship smarter", "scale easier"];

pub fn main() -> iced::Result {
    iced::application(
        DiaTextRevealExample::new,
        DiaTextRevealExample::update,
        DiaTextRevealExample::view,
    )
    .title("Dia Text Reveal")
    .window(iced::window::Settings {
        size: Size::new(1100.0, 760.0),
        ..Default::default()
    })
    .subscription(DiaTextRevealExample::subscription)
    .run()
}

struct DiaTextRevealExample {
    theme_shell: _shared::ThemeShell,
    states: [DiaTextRevealState; 4],
    active_tab: usize,
    now: Instant,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    AnchorsMeasured(enchant_iced::AnchorMeasurements),
    Tick(Instant),
    SelectTab(usize),
}

impl DiaTextRevealExample {
    fn new() -> (Self, Task<Message>) {
        let now = Instant::now();
        (
            Self {
                theme_shell: _shared::ThemeShell::new(),
                states: [
                    DiaTextRevealState::new(&Self::demo_props(0)),
                    DiaTextRevealState::new(&Self::demo_props(1)),
                    DiaTextRevealState::new(&Self::demo_props(2)),
                    DiaTextRevealState::new(&Self::demo_props(3)),
                ],
                active_tab: 0,
                now,
            },
            Task::none(),
        )
    }

    fn demo_props(tab: usize) -> DiaTextRevealProps {
        match tab {
            0 => DiaTextRevealProps::from_texts(&["Magic UI"])
                .colors(DEMO_COLORS_1)
                .start_on_view(false)
                .visible(true),
            1 => DiaTextRevealProps::from_texts(&["Design systems"])
                .colors(DEMO_COLORS_2)
                .start_on_view(false)
                .visible(true),
            2 => DiaTextRevealProps::from_texts(ROTATING_TEXTS)
                .prefix("Learn to ")
                .repeat(true)
                .repeat_delay_secs(1.2)
                .start_on_view(false)
                .visible(true)
                .font_size(32),
            _ => DiaTextRevealProps::from_texts(&["Made with care"])
                .colors(DEMO_COLORS_4)
                .delay_secs(0.35)
                .duration_secs(2.4)
                .start_on_view(false)
                .visible(true),
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ToggleTheme => {
                return _shared::ThemeShell::measure_toggle_anchor().map(Message::AnchorsMeasured);
            }
            Message::AnchorsMeasured(measurements) => {
                self.theme_shell.apply_anchor_and_toggle(&measurements);
            }

            Message::Tick(now) => {
                self.theme_shell.tick(now);
                self.now = now;
                // Only advance the visible demo so background tabs do not finish
                // their one-shot sweeps before the user switches to them.
                self.states[self.active_tab].tick(now, &Self::demo_props(self.active_tab));
            }
            Message::SelectTab(index) => {
                self.active_tab = index;
                self.states[index] = DiaTextRevealState::new(&Self::demo_props(index));
            }
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let tabs = _shared::tabs(
            &["Demo", "Demo 2", "Demo 3", "Demo 4"],
            self.active_tab,
            Message::SelectTab,
        );

        let body = dia_text_reveal(
            &self.states[self.active_tab],
            self.now,
            &Self::demo_props(self.active_tab),
            self.theme_shell.theme(),
        );

        _shared::themed_shell(
            "Dia Text Reveal",
            column![tabs, _shared::stage(body)].spacing(24),
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }
}
