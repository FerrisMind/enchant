#![allow(dead_code)]

use iced::widget::{column, container, row, text};
use iced::{Alignment, Element, Length};
use iced_shadcn::{
    BadgeProps, BadgeSize, BadgeVariant, ButtonProps, ButtonSize, ButtonVariant, CardProps,
    CardSize, FieldProps, InputProps, InputSize, Theme, badge, button_content, card, field, input,
};

fn login_form_fields<'a, Message: Clone + 'a, FEmail, FPassword>(
    email: &'a str,
    password: &'a str,
    email_placeholder: &'a str,
    on_email: Option<FEmail>,
    on_password: Option<FPassword>,
    on_sign_in: Option<Message>,
    theme: &'a Theme,
) -> Element<'a, Message>
where
    FEmail: Fn(String) -> Message + Clone + 'a,
    FPassword: Fn(String) -> Message + Clone + 'a,
{
    let header = column![
        text("Login").size(20),
        text("Enter your credentials to access your account").size(14),
    ]
    .spacing(6);

    let email_field = field(
        FieldProps::new().label("Email"),
        input(
            email,
            email_placeholder,
            on_email.clone(),
            InputProps::new().size(InputSize::Size2),
            theme,
        )
        .width(Length::Fill),
        theme,
    );

    let password_field = field(
        FieldProps::new().label("Password"),
        input(
            password,
            "",
            on_password,
            InputProps::new().size(InputSize::Size2),
            theme,
        )
        .width(Length::Fill),
        theme,
    );

    let footer = container(
        button_content(
            text("Sign In"),
            on_sign_in,
            ButtonProps::new()
                .variant(ButtonVariant::Default)
                .size(ButtonSize::Size2),
            theme,
        )
        .width(Length::Fill),
    )
    .width(Length::Fill);

    column![header, email_field, password_field, footer]
        .spacing(16)
        .into()
}

/// Login form body for magic-card (no outer card chrome).
pub fn login_form_sign_in<'a, Message: Clone + 'a, FEmail, FPassword>(
    email: &'a str,
    password: &'a str,
    on_email: Option<FEmail>,
    on_password: Option<FPassword>,
    on_sign_in: Option<Message>,
    theme: &'a Theme,
) -> Element<'a, Message>
where
    FEmail: Fn(String) -> Message + Clone + 'a,
    FPassword: Fn(String) -> Message + Clone + 'a,
{
    container(login_form_fields(
        email,
        password,
        "name@example.com",
        on_email,
        on_password,
        on_sign_in,
        theme,
    ))
    .padding(16)
    .into()
}

/// Login card matching magicui shine-border / magic-card demos (Sign In footer).
pub fn login_card_sign_in<'a, Message: Clone + 'a, FEmail, FPassword>(
    email: &'a str,
    password: &'a str,
    on_email: Option<FEmail>,
    on_password: Option<FPassword>,
    on_sign_in: Option<Message>,
    theme: &'a Theme,
) -> Element<'a, Message>
where
    FEmail: Fn(String) -> Message + Clone + 'a,
    FPassword: Fn(String) -> Message + Clone + 'a,
{
    card(
        login_form_fields(
            email,
            password,
            "name@example.com",
            on_email,
            on_password,
            on_sign_in,
            theme,
        ),
        CardProps::new()
            .size(CardSize::Size3)
            .show_shadow(true)
            .radius(12.0),
        theme,
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

/// Login card matching magicui border-beam demo (Register + Login footer).
pub fn login_card_dual_footer<'a, Message: Clone + 'a, FEmail, FPassword>(
    email: &'a str,
    password: &'a str,
    on_email: Option<FEmail>,
    on_password: Option<FPassword>,
    on_register: Option<Message>,
    on_login: Option<Message>,
    theme: &'a Theme,
) -> Element<'a, Message>
where
    FEmail: Fn(String) -> Message + Clone + 'a,
    FPassword: Fn(String) -> Message + Clone + 'a,
{
    let header = column![
        text("Login").size(20),
        text("Enter your credentials to access your account.").size(14),
    ]
    .spacing(6);

    let email_field = field(
        FieldProps::new().label("Email"),
        input(
            email,
            "Enter your email",
            on_email.clone(),
            InputProps::new().size(InputSize::Size2),
            theme,
        )
        .width(Length::Fill),
        theme,
    );

    let password_field = field(
        FieldProps::new().label("Password"),
        input(
            password,
            "Enter your password",
            on_password,
            InputProps::new().size(InputSize::Size2),
            theme,
        )
        .width(Length::Fill),
        theme,
    );

    let footer = row![
        button_content(
            text("Register"),
            on_register,
            ButtonProps::new()
                .variant(ButtonVariant::Outline)
                .size(ButtonSize::Size2),
            theme,
        ),
        button_content(
            text("Login"),
            on_login,
            ButtonProps::new()
                .variant(ButtonVariant::Default)
                .size(ButtonSize::Size2),
            theme,
        ),
    ]
    .spacing(12)
    .align_y(Alignment::Center);

    card(
        column![header, email_field, password_field, footer].spacing(16),
        CardProps::new()
            .size(CardSize::Size3)
            .show_shadow(true)
            .radius(12.0),
        theme,
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn feature_row<'a, Message: 'a>(label: &'static str, muted: bool) -> Element<'a, Message> {
    let marker = if muted { "○" } else { "✓" };
    let size = 14_u32;
    row![
        text(marker).size(size),
        text(label).size(size),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

/// Pro pricing card matching magicui glare-hover demo.
pub fn pricing_card_pro<'a, Message: Clone + 'a>(
    on_get_started: Option<Message>,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let muted = theme.palette.muted_foreground;

    let title_row = row![
        text("Pro").size(20),
        badge(
            "Popular",
            BadgeProps::new()
                .size(BadgeSize::Size1)
                .variant(BadgeVariant::Secondary),
            theme,
        ),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let price_row = row![
        text("$49").size(36),
        text("/mo").size(14).style(move |_| iced::widget::text::Style {
            color: Some(muted),
        }),
    ]
    .spacing(4)
    .align_y(Alignment::End);

    let features = column![
        feature_row::<Message>("Unlimited projects", false),
        feature_row::<Message>("Team collaboration", false),
        feature_row::<Message>("Advanced analytics", false),
        feature_row::<Message>("SSO (coming soon)", true),
    ]
    .spacing(10);

    let footer = container(
        button_content(
            text("Get started"),
            on_get_started,
            ButtonProps::new()
                .variant(ButtonVariant::Default)
                .size(ButtonSize::Size2),
            theme,
        )
        .width(Length::Fill),
    )
    .width(Length::Fill);

    card(
        column![
            title_row,
            text("For teams that need more.").size(14),
            price_row,
            features,
            footer,
        ]
        .spacing(14),
        CardProps::new()
            .size(CardSize::Size3)
            .show_shadow(true)
            .radius(12.0),
        theme,
    )
    .width(Length::Fixed(340.0))
    .into()
}
