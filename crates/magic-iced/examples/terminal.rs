mod _shared;

use iced::time::{Duration, Instant};
use iced::widget::column;
use iced::{Color, Element, Size, Subscription, Task};
use iced_shadcn::Theme;
use magic_iced::{TerminalLine, TerminalProps, TerminalSegment, TerminalState, terminal};

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
    theme: Theme,
    lines: Vec<TerminalLine>,
    props: TerminalProps,
    state: TerminalState,
    now: Instant,
}

#[derive(Debug, Clone)]
enum Message {
    Tick(Instant),
}

fn reference_lines() -> Vec<TerminalLine> {
    let green = Color::from_rgb8(0x22, 0xC5, 0x5E);
    let blue = Color::from_rgb8(0x3B, 0x82, 0xF6);
    let muted = Color::from_rgb8(0xA1, 0xA1, 0xAA);

    vec![
        TerminalLine::typing(vec![TerminalSegment::new("pnpm dlx shadcn@latest init")]),
        TerminalLine::instant(vec![
            TerminalSegment::new("✔ Preflight checks").color(green),
        ]),
        TerminalLine::instant(vec![
            TerminalSegment::new("✔ Verifying framework (Next.js)").color(green),
        ]),
        TerminalLine::instant(vec![
            TerminalSegment::new("✔ Validating Tailwind CSS").color(green),
        ]),
        TerminalLine::instant(vec![
            TerminalSegment::new("✔ Installing dependencies").color(green),
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
                theme: Theme::default(),
                lines,
                props,
                state,
                now: Instant::now(),
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        if let Message::Tick(now) = message {
            self.now = now;
            self.state.tick(now, &self.lines, &self.props);
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let body = terminal(&self.state, &self.lines, &self.props, &self.theme);
        _shared::shell("Terminal", column![body].spacing(24))
    }
}
