#![allow(dead_code)]

use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Element, Length};

pub fn shell<'a, Message>(
    title: impl Into<String>,
    body: impl Into<Element<'a, Message>>,
) -> Element<'a, Message>
where
    Message: 'a,
{
    container(
        column![text(title.into()).size(28), body.into()]
            .spacing(20)
            .align_x(Alignment::Center),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(24)
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .into()
}

pub fn tabs<'a, Message: Clone + 'a>(
    labels: &[&'a str],
    _active: usize,
    on_select: impl Fn(usize) -> Message + Clone + 'a,
) -> Element<'a, Message> {
    let mut chips = Vec::with_capacity(labels.len());
    for (index, label) in labels.iter().enumerate() {
        let chip = button(text(*label).size(14))
            .padding([10.0, 14.0])
            .on_press(on_select(index));
        chips.push(chip.into());
    }

    row(chips).spacing(10).align_y(Alignment::Center).into()
}

pub fn stage<'a, Message>(body: impl Into<Element<'a, Message>>) -> Element<'a, Message>
where
    Message: 'a,
{
    container(body)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}
