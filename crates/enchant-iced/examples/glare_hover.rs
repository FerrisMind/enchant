mod _shared;

use enchant_iced::{GlareHoverAction, GlareHoverProps, GlareHoverState, glare_hover};
use iced::time::{Duration, Instant};
use iced::widget::{column, text};
use iced::{Element, Size, Subscription, Task};

pub fn main() -> iced::Result {
    iced::application(
        GlareHoverExample::new,
        GlareHoverExample::update,
        GlareHoverExample::view,
    )
    .title("Glare Hover")
    .window(iced::window::Settings {
        size: Size::new(900.0, 520.0),
        ..Default::default()
    })
    .subscription(GlareHoverExample::subscription)
    .run()
}

struct GlareHoverExample {
    theme_shell: _shared::ThemeShell,
    state: GlareHoverState,
    now: Instant,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    AnchorsMeasured(enchant_iced::AnchorMeasurements),
    Glare(GlareHoverAction),
    Tick(Instant),
    GetStarted,
}

impl GlareHoverExample {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                theme_shell: _shared::ThemeShell::new(),
                state: GlareHoverState::new(),
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

            Message::Glare(action) => self.state.apply_action(&action, self.now),
            Message::Tick(now) => self.now = now,
            Message::GetStarted => {}
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let pricing = _shared::shadcn_forms::pricing_card_pro(
            Some(Message::GetStarted),
            self.theme_shell.theme(),
        );

        let card = glare_hover(
            pricing,
            &self.state,
            self.now,
            GlareHoverProps {
                duration_ms: 600,
                ..GlareHoverProps::new()
            },
            Message::Glare,
            self.theme_shell.theme(),
        );

        _shared::themed_shell(
            "Glare Hover",
            column![text("Pro pricing with glare hover").size(14), card].spacing(24),
            &self.theme_shell,
            Message::ToggleTheme,
        )
    }
}
