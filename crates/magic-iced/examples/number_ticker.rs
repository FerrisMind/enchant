mod _shared;

use iced::time::{Duration, Instant};
use iced::widget::column;
use iced::{Element, Size, Subscription, Task};
use iced_shadcn::Theme;
use magic_iced::{NumberTickerProps, NumberTickerState, number_ticker};

pub fn main() -> iced::Result {
    iced::application(
        NumberTickerExample::new,
        NumberTickerExample::update,
        NumberTickerExample::view,
    )
    .title("Number Ticker")
    .window(iced::window::Settings {
        size: Size::new(900.0, 420.0),
        ..Default::default()
    })
    .subscription(NumberTickerExample::subscription)
    .run()
}

struct NumberTickerExample {
    theme: Theme,
    state: NumberTickerState,
    props: NumberTickerProps,
    now: Instant,
}

#[derive(Debug, Clone)]
enum Message {
    Tick(Instant),
}

impl NumberTickerExample {
    fn new() -> (Self, Task<Message>) {
        let props = NumberTickerProps::new(100.0).font_size(96);
        let now = Instant::now();
        (
            Self {
                theme: Theme::default(),
                state: NumberTickerState::new(&props),
                props,
                now,
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick(now) => {
                self.now = now;
                self.state.tick(now, &self.props);
            }
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let body = number_ticker(&self.state, self.now, &self.props, &self.theme);
        _shared::shell("Number Ticker", column![body].spacing(24))
    }
}
