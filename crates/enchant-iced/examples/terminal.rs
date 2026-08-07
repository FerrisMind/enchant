mod _shared;

use iced::time::{Duration, Instant};
use iced::widget::column;
use iced::{Color, Element, Size, Subscription, Task};
use enchant_iced::{TerminalLine, TerminalProps, TerminalSegment, TerminalState, terminal};

pub fn main() -> iced::Result {
    iced::application(
        TerminalExample::new,
        TerminalExample::update,
        TerminalExample::view,
    )
    .title("Terminal")
    .window(iced::window::Settings {
        size: Size::new(900.0, 560.0),
        ..Default::default()
    })
    .subscription(TerminalExample::subscription)
    .run()
}

struct TerminalExample {
    theme_shell: _shared::ThemeShell,
    lines: Vec<TerminalLine>,
    props: TerminalProps,
    state: TerminalState,
    now: Instant,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    AnchorsMeasured(enchant_iced::AnchorMeasurements),
    Tick(Instant),
}

fn reference_lines() -> Vec<TerminalLine> {
    let green = Color::from_rgb8(0x22, 0xC5, 0x5E);
    let blue = Color::from_rgb8(0x3B, 0x82, 0xF6);
    let muted = Color::from_rgb8(0xA1, 0xA1, 0xAA);

    vec![
        TerminalLine::typing(vec![TerminalSegment::new("> pnpm dlx shadcn@latest init")]),
        TerminalLine::instant(vec![
            TerminalSegment::new("✔ Preflight checks.").color(green),
        ]),
        TerminalLine::instant(vec![
            TerminalSegment::new("✔ Verifying framework. Found Next.js.").color(green),
        ]),
        TerminalLine::instant(vec![
            TerminalSegment::new("✔ Validating Tailwind CSS.").color(green),
        ]),
        TerminalLine::instant(vec![
            TerminalSegment::new("✔ Validating import alias.").color(green),
        ]),
        TerminalLine::instant(vec![
            TerminalSegment::new("✔ Writing components.json.").color(green),
        ]),
        TerminalLine::instant(vec![
            TerminalSegment::new("✔ Checking registry.").color(green),
        ]),
        TerminalLine::instant(vec![
            TerminalSegment::new("✔ Updating tailwind.config.ts").color(green),
        ]),
        TerminalLine::instant(vec![
            TerminalSegment::new("✔ Updating app/globals.css").color(green),
        ]),
        TerminalLine::instant(vec![
            TerminalSegment::new("✔ Installing dependencies.").color(green),
        ]),
        TerminalLine::instant(vec![
            TerminalSegment::new("ℹ Updated 1 file:").color(blue),
            TerminalSegment::new(" - lib/utils.ts").color(muted),
        ]),
        TerminalLine::typing(vec![
            TerminalSegment::new("Success! Project initialization completed.").color(muted),
        ]),
        TerminalLine::typing(vec![
            TerminalSegment::new("You may now add components.").color(muted),
        ]),
    ]
}

impl TerminalExample {
    fn new() -> (Self, Task<Message>) {
        let lines = reference_lines();
        let props = TerminalProps::new();
        let state = TerminalState::new(&lines, &props);
        (
            Self {
                theme_shell: _shared::ThemeShell::new(),
                lines,
                props,
                state,
                now: Instant::now(),
            },
            Task::none(),
        )
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
                self.state.tick(now, &self.lines, &self.props);
            }
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let body = terminal(
            &self.state,
            &self.lines,
            &self.props,
            self.theme_shell.theme(),
        );
        _shared::themed_shell(
            "Terminal",
            column![body].spacing(24),
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }
}
