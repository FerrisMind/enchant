use iced::advanced::Renderer as _;
use iced::advanced::graphics::geometry::Renderer as _;
use iced::advanced::layout;
use iced::advanced::renderer;
use iced::advanced::widget::Tree;
use iced::advanced::widget::tree;
use iced::advanced::{Clipboard, Layout, Shell, Widget};
use iced::mouse;
use iced::widget::canvas;
use iced::{Element, Event, Length, Rectangle, Renderer, Size, Vector};
use std::cell::Cell;

pub trait EffectProgram<Message> {
    type State: Default + 'static;

    fn update(
        &self,
        _state: &mut Self::State,
        _event: &canvas::Event,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) -> Option<canvas::Action<Message>> {
        None
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        bounds: Rectangle,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) -> Vec<canvas::Geometry>;

    fn draw_cached(
        &self,
        _state: &Self::State,
        _frame: &mut canvas::Frame,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) -> bool {
        false
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) -> mouse::Interaction {
        mouse::Interaction::None
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum EffectCanvasCache {
    #[default]
    Off,
    On,
    Grouped(canvas::Group),
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct CachedBounds {
    width: f32,
    height: f32,
}

impl From<Size> for CachedBounds {
    fn from(size: Size) -> Self {
        Self {
            width: size.width,
            height: size.height,
        }
    }
}

struct EffectCanvasCacheState {
    cache: canvas::Cache<Renderer>,
    bounds: Cell<Option<CachedBounds>>,
    revision: Cell<u64>,
}

impl EffectCanvasCacheState {
    fn new(mode: EffectCanvasCache) -> Option<Self> {
        let cache = match mode {
            EffectCanvasCache::Off => return None,
            EffectCanvasCache::On => canvas::Cache::new(),
            EffectCanvasCache::Grouped(group) => canvas::Cache::with_group(group),
        };

        Some(Self {
            cache,
            bounds: Cell::new(None),
            revision: Cell::new(0),
        })
    }
}

fn cache_needs_clear(
    previous_bounds: Option<CachedBounds>,
    previous_revision: u64,
    next_bounds: CachedBounds,
    next_revision: u64,
) -> bool {
    previous_bounds != Some(next_bounds) || previous_revision != next_revision
}

struct EffectCanvasTreeState<S> {
    program: S,
    cache_mode: EffectCanvasCache,
    cache_revision: u64,
    cache: Option<EffectCanvasCacheState>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EffectCanvasProps {
    pub width: Length,
    pub height: Length,
    pub cache: EffectCanvasCache,
    pub cache_revision: u64,
}

impl Default for EffectCanvasProps {
    fn default() -> Self {
        Self {
            width: Length::Fill,
            height: Length::Fixed(220.0),
            cache: EffectCanvasCache::Off,
            cache_revision: 0,
        }
    }
}

impl EffectCanvasProps {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    pub fn cache(mut self, cache: EffectCanvasCache) -> Self {
        self.cache = cache;
        self
    }

    pub fn cache_revision(mut self, cache_revision: u64) -> Self {
        self.cache_revision = cache_revision;
        self
    }
}

pub struct EffectCanvas<P> {
    program: P,
    props: EffectCanvasProps,
}

pub fn effect_canvas<'a, P, Message: 'a>(program: P, props: EffectCanvasProps) -> EffectCanvas<P>
where
    P: EffectProgram<Message> + 'a,
{
    EffectCanvas { program, props }
}

impl<P, Message> Widget<Message, iced::Theme, Renderer> for EffectCanvas<P>
where
    P: EffectProgram<Message>,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<EffectCanvasTreeState<P::State>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(EffectCanvasTreeState {
            program: P::State::default(),
            cache_mode: self.props.cache,
            cache_revision: self.props.cache_revision,
            cache: EffectCanvasCacheState::new(self.props.cache),
        })
    }

    fn diff(&self, tree: &mut Tree) {
        let state = tree.state.downcast_mut::<EffectCanvasTreeState<P::State>>();

        if state.cache_mode != self.props.cache {
            state.cache_mode = self.props.cache;
            state.cache = EffectCanvasCacheState::new(self.props.cache);
            state.cache_revision = self.props.cache_revision;
        }
    }

    fn size(&self) -> Size<Length> {
        Size::new(self.props.width, self.props.height)
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::atomic(limits, self.props.width, self.props.height)
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let state = &mut tree.state.downcast_mut::<EffectCanvasTreeState<P::State>>();

        if let Some(action) =
            self.program
                .update(&mut state.program, event, bounds, cursor, viewport)
        {
            clear_cache_if_needed(state.cache.as_ref());
            let (message, redraw_request, event_status) = action.into_inner();

            shell.request_redraw_at(redraw_request);

            if let Some(message) = message {
                shell.publish(message);
            }

            if event_status == iced::event::Status::Captured {
                shell.capture_event();
            }
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        self.program.mouse_interaction(
            &tree
                .state
                .downcast_ref::<EffectCanvasTreeState<P::State>>()
                .program,
            layout.bounds(),
            cursor,
            viewport,
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        _theme: &iced::Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();

        if bounds.width < 1.0 || bounds.height < 1.0 || !bounds.intersects(viewport) {
            return;
        }

        let state = tree.state.downcast_ref::<EffectCanvasTreeState<P::State>>();

        renderer.with_translation(Vector::new(bounds.x, bounds.y), |renderer| {
            if let Some(cache_state) = state.cache.as_ref() {
                let cached_bounds = CachedBounds::from(bounds.size());
                if cache_needs_clear(
                    cache_state.bounds.get(),
                    cache_state.revision.get(),
                    cached_bounds,
                    self.props.cache_revision,
                ) {
                    cache_state.cache.clear();
                    cache_state.bounds.set(Some(cached_bounds));
                    cache_state.revision.set(self.props.cache_revision);
                }

                let used_cache = Cell::new(false);
                let geometry = cache_state.cache.draw_with_bounds(
                    renderer,
                    Rectangle::with_size(bounds.size()),
                    |frame| {
                        used_cache.set(self.program.draw_cached(
                            &state.program,
                            frame,
                            Rectangle::with_size(bounds.size()),
                            cursor,
                            viewport,
                        ));
                    },
                );

                if used_cache.get() {
                    renderer.draw_geometry(geometry);
                } else {
                    cache_state.cache.clear();
                    for geometry in
                        self.program
                            .draw(&state.program, renderer, bounds, cursor, viewport)
                    {
                        renderer.draw_geometry(geometry);
                    }
                }
            } else {
                for geometry in
                    self.program
                        .draw(&state.program, renderer, bounds, cursor, viewport)
                {
                    renderer.draw_geometry(geometry);
                }
            }
        });
    }
}

fn clear_cache_if_needed(cache: Option<&EffectCanvasCacheState>) {
    if let Some(cache) = cache {
        cache.cache.clear();
        cache.bounds.set(None);
    }
}

impl<'a, P, Message> From<EffectCanvas<P>> for Element<'a, Message>
where
    P: EffectProgram<Message> + 'a,
    Message: 'a,
{
    fn from(widget: EffectCanvas<P>) -> Self {
        Element::new(widget)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct DummyProgram;

    impl EffectProgram<()> for DummyProgram {
        type State = ();

        fn draw(
            &self,
            _state: &Self::State,
            renderer: &Renderer,
            bounds: Rectangle,
            _cursor: mouse::Cursor,
            _viewport: &Rectangle,
        ) -> Vec<canvas::Geometry> {
            let frame = canvas::Frame::new(renderer, bounds.size());
            vec![frame.into_geometry()]
        }

        fn draw_cached(
            &self,
            _state: &Self::State,
            frame: &mut canvas::Frame,
            bounds: Rectangle,
            _cursor: mouse::Cursor,
            _viewport: &Rectangle,
        ) -> bool {
            frame.stroke(
                &canvas::Path::rectangle(iced::Point::ORIGIN, bounds.size()),
                canvas::Stroke::default(),
            );
            true
        }
    }

    #[test]
    fn effect_canvas_props_have_stable_defaults() {
        let props = EffectCanvasProps::new();

        assert_eq!(props.width, Length::Fill);
        assert_eq!(props.height, Length::Fixed(220.0));
        assert_eq!(props.cache, EffectCanvasCache::Off);
        assert_eq!(props.cache_revision, 0);
    }

    #[test]
    fn effect_canvas_builds_from_program_and_props() {
        let widget = effect_canvas(
            DummyProgram,
            EffectCanvasProps::new().height(Length::Fixed(180.0)),
        );

        assert_eq!(widget.props.height, Length::Fixed(180.0));
    }

    #[test]
    fn effect_canvas_props_support_cache_modes_and_revision() {
        let props = EffectCanvasProps::new()
            .cache(EffectCanvasCache::On)
            .cache_revision(7);

        assert_eq!(props.cache, EffectCanvasCache::On);
        assert_eq!(props.cache_revision, 7);
    }

    #[test]
    fn cache_invalidation_tracks_bounds_and_revision() {
        let bounds = CachedBounds {
            width: 120.0,
            height: 60.0,
        };

        assert!(cache_needs_clear(None, 0, bounds, 0));
        assert!(!cache_needs_clear(Some(bounds), 7, bounds, 7));
        assert!(cache_needs_clear(Some(bounds), 7, bounds, 8));
        assert!(cache_needs_clear(
            Some(bounds),
            7,
            CachedBounds {
                width: 160.0,
                height: 60.0,
            },
            7,
        ));
    }
}
