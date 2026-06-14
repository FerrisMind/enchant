#![allow(dead_code)]

use super::beam_icons;
use super::beam_scene::{self, BeamSceneKind, CONTAINER};
use iced::widget::{column, container, row, space, stack, svg};
use iced::{Alignment, Background, Border, Color, Element, Length, Point, Rectangle, Shadow, Size, Vector};
use magic_iced::{anchor, AnchorBounds, AnchorMeasurements, AnimatedBeamProps, animated_beam};
use std::time::Duration;

pub const SCENE_WIDTH: f32 = 512.0;
pub const SCENE_HEIGHT: f32 = 300.0;

const SCENE_PAD: f32 = 40.0;
const NODE: f32 = 48.0;
const NODE_LG: f32 = 64.0;
const ICON_PAD: f32 = 12.0;

const LEFT_X: f32 = SCENE_PAD + NODE / 2.0;
const RIGHT_X: f32 = SCENE_WIDTH - SCENE_PAD - NODE / 2.0;
const CENTER_X: f32 = SCENE_WIDTH / 2.0;
const CENTER_Y: f32 = SCENE_HEIGHT / 2.0;

const ROW1_Y: f32 = 64.0;
const ROW2_Y: f32 = 150.0;
const ROW3_Y: f32 = 236.0;

const MULTI_SCENE_HEIGHT: f32 = 500.0;
const MULTI_COL_GAP: f32 = 8.0;
const MULTI_OUTPUT_BEAM_DURATION: f32 = 3.0;

fn multi_output_beam_props() -> AnimatedBeamProps {
    AnimatedBeamProps {
        duration_secs: MULTI_OUTPUT_BEAM_DURATION,
        ..AnimatedBeamProps::new()
    }
}

pub fn anchor_at(x: f32, y: f32, size: f32) -> AnchorBounds {
    AnchorBounds::from_bounds(Rectangle::new(
        Point::new(x - size / 2.0, y - size / 2.0),
        Size::new(size, size),
    ))
}

fn local_anchor(container: AnchorBounds, node: AnchorBounds) -> AnchorBounds {
    AnchorBounds::from_bounds(Rectangle::new(
        Point::new(
            node.bounds.x - container.bounds.x,
            node.bounds.y - container.bounds.y,
        ),
        node.bounds.size(),
    ))
}

fn measured_anchor(
    measurements: Option<&AnchorMeasurements>,
    container_index: usize,
    node_index: usize,
) -> Option<AnchorBounds> {
    let measurements = measurements?;
    let container = measurements.get(container_index)?;
    let node = measurements.get(node_index)?;
    Some(local_anchor(container, node))
}

pub fn beam_circle<'a, Message: 'a>(icon: svg::Handle, size: f32) -> Element<'a, Message> {
    beam_avatar(icon, size, ICON_PAD)
}

pub fn beam_avatar<'a, Message: 'a>(
    icon: svg::Handle,
    size: f32,
    padding: f32,
) -> Element<'a, Message> {
    let inner = (size - padding * 2.0).max(0.0);
    container(
        container(
            svg(icon)
                .width(Length::Fixed(inner))
                .height(Length::Fixed(inner)),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill),
    )
    .width(Length::Fixed(size))
    .height(Length::Fixed(size))
    .style(move |_| iced::widget::container::Style {
        background: Some(Background::Color(Color::WHITE)),
        border: Border {
            width: 2.0,
            radius: (size / 2.0).into(),
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.12),
        },
        // Reference: shadow-[0_0_20px_-12px_rgba(0,0,0,0.8)] — centered halo, no drop offset.
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.06),
            offset: Vector::new(0.0, 0.0),
            blur_radius: 8.0,
        },
        ..iced::widget::container::Style::default()
    })
    .into()
}

fn tagged_circle<'a, Message: 'a>(
    id: &'static str,
    icon: svg::Handle,
    size: f32,
) -> Element<'a, Message> {
    anchor(id, beam_circle(icon, size)).into()
}

fn beam_row<'a, Message: 'a>(
    left: Option<(&'static str, svg::Handle, f32)>,
    center: Option<(&'static str, svg::Handle, f32)>,
    right: Option<(&'static str, svg::Handle, f32)>,
) -> Element<'a, Message> {
    match (left, center, right) {
        (Some((lid, l, ls)), None, Some((rid, r, rs))) => row![
            tagged_circle(lid, l, ls),
            space().width(Length::Fill),
            tagged_circle(rid, r, rs),
        ]
        .align_y(Alignment::Center)
        .width(Length::Fill)
        .into(),
        (Some((lid, l, ls)), Some((cid, c, cs)), Some((rid, r, rs))) => row![
            tagged_circle(lid, l, ls),
            space().width(Length::Fill),
            tagged_circle(cid, c, cs),
            space().width(Length::Fill),
            tagged_circle(rid, r, rs),
        ]
        .align_y(Alignment::Center)
        .width(Length::Fill)
        .into(),
        _ => space().height(Length::Fixed(NODE)).into(),
    }
}

fn spaced_column<Message: 'static>(
    rows: Vec<Element<'static, Message>>,
) -> Element<'static, Message> {
    let mut items: Vec<Element<'static, Message>> = Vec::with_capacity(rows.len() * 2 - 1);
    for (index, row) in rows.into_iter().enumerate() {
        if index > 0 {
            items.push(space().height(Length::Fill).into());
        }
        items.push(row);
    }

    container(
        column(items)
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .padding(SCENE_PAD)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn centered_row<Message: 'static>(left: &'static str, right: &'static str) -> Element<'static, Message> {
    container(
        column![
            space().height(Length::Fill),
            row![
                tagged_circle(left, beam_icons::user(), NODE),
                space().width(Length::Fill),
                tagged_circle(right, beam_icons::openai(), NODE),
            ]
            .align_y(Alignment::Center)
            .width(Length::Fill),
            space().height(Length::Fill),
        ]
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .padding(SCENE_PAD)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn beam<Message: 'static>(
    elapsed: Duration,
    from: AnchorBounds,
    to: AnchorBounds,
    props: AnimatedBeamProps,
) -> Element<'static, Message> {
    animated_beam(from, to, elapsed, props)
}

fn scene<Message: 'static>(
    layout: Element<'static, Message>,
    beams: Vec<Element<'static, Message>>,
    height: f32,
) -> Element<'static, Message> {
    // Reference: Circle nodes use z-10; AnimatedBeam sits underneath (no z-index).
    let mut layers: Vec<Element<'static, Message>> = beams
        .into_iter()
        .map(|b| container(b).width(Length::Fill).height(Length::Fill).into())
        .collect();
    layers.push(layout);

    anchor(
        CONTAINER,
        container(
            stack(layers)
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .width(Length::Fixed(SCENE_WIDTH))
        .height(Length::Fixed(height)),
    )
    .into()
}

fn left_column_ys(count: usize, scene_height: f32) -> Vec<f32> {
    let inner = scene_height - SCENE_PAD * 2.0;
    let total = count as f32 * NODE + (count.saturating_sub(1) as f32) * MULTI_COL_GAP;
    let start = SCENE_PAD + (inner - total) / 2.0 + NODE / 2.0;
    (0..count)
        .map(|i| start + i as f32 * (NODE + MULTI_COL_GAP))
        .collect()
}

pub fn measure_ids(kind: BeamSceneKind) -> Vec<iced::widget::Id> {
    beam_scene::measure_ids(kind)
}

/// Main demo: 3 left + center + 3 right service nodes with converging beams.
pub fn demo_scene<Message: 'static>(
    elapsed: Duration,
    measurements: Option<&AnchorMeasurements>,
) -> Element<'static, Message> {
    let layout = spaced_column(vec![
        beam_row(
            Some(("beam-n1", beam_icons::google_drive(), NODE)),
            None,
            Some(("beam-n5", beam_icons::google_docs(), NODE)),
        ),
        beam_row(
            Some(("beam-n2", beam_icons::notion(), NODE)),
            Some(("beam-n4", beam_icons::openai(), NODE_LG)),
            Some(("beam-n6", beam_icons::zapier(), NODE)),
        ),
        beam_row(
            Some(("beam-n3", beam_icons::whatsapp(), NODE)),
            None,
            Some(("beam-n7", beam_icons::messenger(), NODE)),
        ),
    ]);

    let beams = {
        let center = measured_anchor(measurements, 0, 4)
            .unwrap_or_else(|| anchor_at(CENTER_X, ROW2_Y, NODE_LG));
        vec![
            beam(
                elapsed,
                measured_anchor(measurements, 0, 1).unwrap_or_else(|| anchor_at(LEFT_X, ROW1_Y, NODE)),
                center,
                AnimatedBeamProps {
                    curvature: -75.0,
                    end_y_offset: -10.0,
                    ..AnimatedBeamProps::new()
                },
            ),
            beam(
                elapsed,
                measured_anchor(measurements, 0, 2).unwrap_or_else(|| anchor_at(LEFT_X, ROW2_Y, NODE)),
                center,
                AnimatedBeamProps::new(),
            ),
            beam(
                elapsed,
                measured_anchor(measurements, 0, 3).unwrap_or_else(|| anchor_at(LEFT_X, ROW3_Y, NODE)),
                center,
                AnimatedBeamProps {
                    curvature: 75.0,
                    end_y_offset: 10.0,
                    ..AnimatedBeamProps::new()
                },
            ),
            beam(
                elapsed,
                measured_anchor(measurements, 0, 5).unwrap_or_else(|| anchor_at(RIGHT_X, ROW1_Y, NODE)),
                center,
                AnimatedBeamProps {
                    curvature: -75.0,
                    end_y_offset: -10.0,
                    reverse: true,
                    ..AnimatedBeamProps::new()
                },
            ),
            beam(
                elapsed,
                measured_anchor(measurements, 0, 6).unwrap_or_else(|| anchor_at(RIGHT_X, ROW2_Y, NODE)),
                center,
                AnimatedBeamProps {
                    reverse: true,
                    ..AnimatedBeamProps::new()
                },
            ),
            beam(
                elapsed,
                measured_anchor(measurements, 0, 7).unwrap_or_else(|| anchor_at(RIGHT_X, ROW3_Y, NODE)),
                center,
                AnimatedBeamProps {
                    curvature: 75.0,
                    end_y_offset: 10.0,
                    reverse: true,
                    ..AnimatedBeamProps::new()
                },
            ),
        ]
    };

    scene(layout, beams, SCENE_HEIGHT).into()
}

/// Left user → right OpenAI, single beam.
pub fn unidirectional_scene<Message: 'static>(
    elapsed: Duration,
    measurements: Option<&AnchorMeasurements>,
) -> Element<'static, Message> {
    let layout = centered_row("beam-left", "beam-right");

    let beams = vec![beam(
        elapsed,
        measured_anchor(measurements, 0, 1).unwrap_or_else(|| anchor_at(LEFT_X, CENTER_Y, NODE)),
        measured_anchor(measurements, 0, 2).unwrap_or_else(|| anchor_at(RIGHT_X, CENTER_Y, NODE)),
        AnimatedBeamProps {
            duration_secs: 3.0,
            ..AnimatedBeamProps::new()
        },
    )];

    scene(layout, beams, SCENE_HEIGHT).into()
}

/// Two parallel beams between the same endpoints (opposite directions).
pub fn bidirectional_scene<Message: 'static>(
    elapsed: Duration,
    measurements: Option<&AnchorMeasurements>,
) -> Element<'static, Message> {
    let layout = centered_row("beam-left", "beam-right");

    let beams = {
        let from = measured_anchor(measurements, 0, 1).unwrap_or_else(|| anchor_at(LEFT_X, CENTER_Y, NODE));
        let to = measured_anchor(measurements, 0, 2).unwrap_or_else(|| anchor_at(RIGHT_X, CENTER_Y, NODE));
        vec![
            beam(
                elapsed,
                from,
                to,
                AnimatedBeamProps {
                    start_y_offset: 10.0,
                    end_y_offset: 10.0,
                    curvature: -20.0,
                    ..AnimatedBeamProps::new()
                },
            ),
            beam(
                elapsed,
                from,
                to,
                AnimatedBeamProps {
                    start_y_offset: -10.0,
                    end_y_offset: -10.0,
                    curvature: 20.0,
                    reverse: true,
                    ..AnimatedBeamProps::new()
                },
            ),
        ]
    };

    scene(layout, beams, SCENE_HEIGHT).into()
}

fn multi_io_layout<Message: 'static>(
    left_nodes: Vec<Element<'static, Message>>,
    center_id: &'static str,
    center_icon: svg::Handle,
    right_id: &'static str,
    right_icon: svg::Handle,
    scene_height: f32,
) -> Element<'static, Message> {
    let left_col = column(left_nodes)
        .spacing(MULTI_COL_GAP)
        .align_x(Alignment::Center);

    container(
        row![
            container(left_col).center_y(Length::Fill),
            space().width(Length::Fill),
            container(tagged_circle(center_id, center_icon, NODE_LG)).center_y(Length::Fill),
            space().width(Length::Fill),
            container(tagged_circle(right_id, right_icon, NODE)).center_y(Length::Fill),
        ]
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .padding(SCENE_PAD)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

/// Five left inputs → center OpenAI → right user (all beams in sync, like the reference).
pub fn multiple_inputs_scene<Message: 'static>(
    elapsed: Duration,
    measurements: Option<&AnchorMeasurements>,
) -> Element<'static, Message> {
    let icons = [
        ("beam-l1", beam_icons::google_drive()),
        ("beam-l2", beam_icons::google_docs()),
        ("beam-l3", beam_icons::whatsapp()),
        ("beam-l4", beam_icons::messenger()),
        ("beam-l5", beam_icons::notion()),
    ];

    let left_nodes = icons
        .iter()
        .map(|(id, icon)| tagged_circle(id, icon.clone(), NODE))
        .collect::<Vec<_>>();
    let layout = multi_io_layout(
        left_nodes,
        "beam-center",
        beam_icons::openai(),
        "beam-user",
        beam_icons::user(),
        MULTI_SCENE_HEIGHT,
    );

    let beams = {
        let center = measured_anchor(measurements, 0, 6)
            .unwrap_or_else(|| anchor_at(CENTER_X, MULTI_SCENE_HEIGHT / 2.0, NODE_LG));
        let user = measured_anchor(measurements, 0, 7)
            .unwrap_or_else(|| anchor_at(RIGHT_X, MULTI_SCENE_HEIGHT / 2.0, NODE));
        let mut beams = (1..=5)
            .map(|index| {
                let fallback_y = left_column_ys(5, MULTI_SCENE_HEIGHT)[index - 1];
                beam(
                    elapsed,
                    measured_anchor(measurements, 0, index)
                        .unwrap_or_else(|| anchor_at(LEFT_X, fallback_y, NODE)),
                    center,
                    AnimatedBeamProps::new(),
                )
            })
            .collect::<Vec<_>>();
        beams.push(beam(
            elapsed,
            center,
            user,
            AnimatedBeamProps::new(),
        ));
        beams
    };

    scene(layout, beams, MULTI_SCENE_HEIGHT).into()
}

/// User left, center OpenAI, five right outputs — all beams in sync (`duration=3` in reference).
pub fn multiple_outputs_scene<Message: 'static>(
    elapsed: Duration,
    measurements: Option<&AnchorMeasurements>,
) -> Element<'static, Message> {
    let icons = [
        ("beam-r1", beam_icons::google_drive()),
        ("beam-r2", beam_icons::google_docs()),
        ("beam-r3", beam_icons::whatsapp()),
        ("beam-r4", beam_icons::messenger()),
        ("beam-r5", beam_icons::notion()),
    ];

    let right_nodes = icons
        .iter()
        .map(|(id, icon)| tagged_circle(id, icon.clone(), NODE))
        .collect::<Vec<_>>();

    let layout = container(
        row![
            container(tagged_circle("beam-user", beam_icons::user(), NODE)).center_y(Length::Fill),
            space().width(Length::Fill),
            container(tagged_circle("beam-center", beam_icons::openai(), NODE_LG))
                .center_y(Length::Fill),
            space().width(Length::Fill),
            container(
                column(right_nodes)
                    .spacing(MULTI_COL_GAP)
                    .align_x(Alignment::Center),
            )
            .center_y(Length::Fill),
        ]
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .padding(SCENE_PAD)
    .width(Length::Fill)
    .height(Length::Fill)
    .into();

    let beams = {
        let user = measured_anchor(measurements, 0, 1)
            .unwrap_or_else(|| anchor_at(LEFT_X, MULTI_SCENE_HEIGHT / 2.0, NODE));
        let center = measured_anchor(measurements, 0, 2)
            .unwrap_or_else(|| anchor_at(CENTER_X, MULTI_SCENE_HEIGHT / 2.0, NODE_LG));
        let right_ys = left_column_ys(5, MULTI_SCENE_HEIGHT);
        let props = multi_output_beam_props();
        let mut beams = (0..5)
            .map(|index| {
                beam(
                    elapsed,
                    measured_anchor(measurements, 0, 3 + index).unwrap_or_else(|| {
                        anchor_at(RIGHT_X, right_ys[index], NODE)
                    }),
                    center,
                    props,
                )
            })
            .collect::<Vec<_>>();
        beams.push(beam(elapsed, center, user, props));
        beams
    };

    scene(layout, beams, MULTI_SCENE_HEIGHT).into()
}
