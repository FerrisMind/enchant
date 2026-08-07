import os

widgets_dir = r'h:\comps\enchant-iced\crates\enchant-iced\src\widgets'
examples_dir = r'h:\comps\enchant-iced\crates\enchant-iced\examples'

if not os.path.exists(examples_dir):
    os.makedirs(examples_dir)

widgets = [f[:-3] for f in os.listdir(widgets_dir) if f.endswith('.rs') and f != 'mod.rs' and f != 'text_element.rs' and f != 'mockups.rs' and f != 'marquee.rs']

template = """use iced::widget::{column, text, container};
use iced::{Element, Length, Task, Size};

pub fn main() -> iced::Result {
    iced::application(
        {widget_camel}Example::new,
        {widget_camel}Example::update,
        {widget_camel}Example::view,
    )
    .title("{widget_camel} Example")
    .window(iced::window::Settings {
        size: Size::new(800.0, 600.0),
        ..Default::default()
    })
    .run()
}

#[derive(Default)]
struct {widget_camel}Example {}

#[derive(Debug, Clone)]
enum Message {}

impl {widget_camel}Example {
    fn new() -> (Self, Task<Message>) {
        (Self::default(), Task::none())
    }

    fn update(&mut self, _message: Message) -> Task<Message> {
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        container(
            column![
                text("{widget_camel} Example").size(24)
            ]
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
    }
}
"""

def to_camel(s):
    return ''.join(word.capitalize() for word in s.split('_'))

for w in widgets:
    content = template.replace('{widget_name}', w).replace('{widget_camel}', to_camel(w))
    filepath = os.path.join(examples_dir, f'{w}.rs')
    with open(filepath, 'w') as f:
        f.write(content)

print(f'Generated examples for {len(widgets)} widgets.')
