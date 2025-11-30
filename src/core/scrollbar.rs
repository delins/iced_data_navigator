use iced_core::border;
use iced_core::mouse;
use iced_core::renderer;
use iced_core::touch;
use iced_core::{
    self, Background, Color, Event, Pixels, Point, Rectangle, Theme,
};

use std::ops;

// TODO add general explenation about scrollbars.

#[derive(Clone, Debug)]
/// Horizontal scrollbar utility struct for virtual scrolling. Can be used inside custom widgets
/// (structs that implement the [`Widget`] trait) to add horizontal scrolling functionality.
/// TODO: look into disabling the scrollbar.
pub struct HorizontalScrollbar<'a, Theme>
where
    Theme: Catalog
{
    track_height: f32,
    thumb_height: f32,
    status: Status,
    class: Theme::ScrollClass<'a>,
}


impl<'a, Theme> HorizontalScrollbar<'a, Theme>
where
    Theme: Catalog
{
    /// Creates a new `HorizontalScrollbar`.
    pub fn new() -> Self {
        HorizontalScrollbar::default()
    }

    /// Sets the track height.
    pub fn track_height(mut self, height: impl Into<Pixels>) -> Self {
        self.track_height = height.into().0.max(0.0);
        self
    }

    /// Sets the thumb height.
    pub fn thumb_height(mut self, height: impl Into<Pixels>) -> Self {
        self.thumb_height = height.into().0.max(0.0);
        self
    }

    /// The height that the scrollbar wants to have.
    pub fn height(&self) -> f32 {
        self.track_height.max(self.thumb_height)
    }

    /// Updates the state of the scrollbar, to be called in the widget's `update` method.
    pub fn update(
        &mut self,
        state: &mut State,
        event: &Event,
        bounds: Rectangle,
        scroll_state: Option<Viewport>,
        cursor: mouse::Cursor,
    ) -> ScrollResult {
        let (mut result, status) = update(
            self, self.status, state, event, bounds, scroll_state, cursor);

        if status != self.status && result == ScrollResult::None {
            result = ScrollResult::AppearanceChanged;
        }

        self.status = status;

        result
    }

    /// Draws the scrollbar, to be called in the widget's `draw` method.
    pub fn draw<Renderer>(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        bounds: Rectangle,
        viewport: Option<Viewport>,
    )
    where
        Renderer: iced_core::Renderer,
        Theme: Catalog,
    {
        draw(self, self.status, &self.class, renderer, theme, bounds, viewport)
    }
}

impl<'a, Theme> Default for HorizontalScrollbar<'a, Theme>
where
    Theme: Catalog
{
    fn default() -> Self {
        HorizontalScrollbar {
            track_height: 10.0,
            thumb_height: 10.0,
            status: Status::Enabled(BarStatus::Active),
            class: Theme::scroll_default(),
        }
    }
}

impl<'a, Theme> Scrollbar for HorizontalScrollbar<'a, Theme>
where
    Theme: Catalog
{
    fn layout(&self, bounds: Rectangle, viewport: Viewport) -> Option<Layout> {
        if bounds.width == 0.0 || bounds.height == 0.0 {
            return None
        }

        // If the provided bound height isn't our requested height, we vertically center.
        let center = bounds.y + bounds.height / 2.0;
        let max_offset = self.height().min(bounds.height) / 2.0;

        let track_bounds = Rectangle {
            x: bounds.x,
            y: center - (self.track_height / 2.0).min(max_offset),
            width: bounds.width,
            height: self.track_height.min(bounds.height),
        };

        let thumb_width = (bounds.width * viewport.viewport_ratio())
            .min(bounds.width)
            .max(10.0);

        let offset = self.thumb_offset_from_viewport(viewport, bounds.width, thumb_width);

        let thumb_bounds = Rectangle {
            x: bounds.x + offset,
            y: center - (self.thumb_height / 2.0).min(max_offset),
            width: thumb_width,
            height: self.thumb_height.min(bounds.height),
        };

        Some(Layout {
            track: track_bounds,
            thumb: thumb_bounds,
        })
    }

    fn region(&self, scrollbar: &Layout, cursor_position: Point) -> ScrollbarRegion {
        if cursor_position.x < scrollbar.thumb.x {
            ScrollbarRegion::TrackBeforeThumb(cursor_position.x - scrollbar.track.x)
        } else if cursor_position.x < scrollbar.thumb.x + scrollbar.thumb.width {
            ScrollbarRegion::Thumb(cursor_position.x - scrollbar.thumb.x)
        } else {
            ScrollbarRegion::TrackAfterThumb(cursor_position.x - scrollbar.track.x)
        }
    }

    fn max_visual_range(&self, scrollbar: &Layout) -> f32 {
        (scrollbar.track.width - scrollbar.thumb.width).max(0.0)
    }

    fn thumb_offset_from_grab(&self, scrollbar: &Layout, cursor: Point, grab_offset: f32) -> f32 {
        (cursor.x - scrollbar.track.x - grab_offset)
            .min(self.max_visual_range(scrollbar))
            .max(0.0)
    }

    fn track_click_offset(&self, layout: &Layout, cursor: Point) -> f32 {
        (cursor.x - layout.track.x)
            .min(layout.track.width - 1.0)
            .max(0.0)
    }
}

/// Vertical scrollbar utility struct for virtual scrolling. Can be used inside custom widgets
/// (structs that implement the [`Widget`] trait) to add vertical scrolling functionality.
/// TODO: look into disabling the scrollbar.
#[derive(Clone, Debug)]
pub struct VerticalScrollbar<'a, Theme>
where
    Theme: Catalog
{
    track_width: f32,
    thumb_width: f32,
    status: Status,
    class: Theme::ScrollClass<'a>,
}

impl<'a, Theme> VerticalScrollbar<'a, Theme>
where
    Theme: Catalog
{
    /// Creates a new `VerticalScrollbar`.
    pub fn new() -> Self {
        VerticalScrollbar::default()
    }

    /// Sets the track width.
    pub fn track_width(mut self, width: impl Into<Pixels>) -> Self {
        self.track_width = width.into().0.max(0.0);
        self
    }

    /// Sets the thumb width.
    pub fn thumb_width(mut self, width: impl Into<Pixels>) -> Self {
        self.thumb_width = width.into().0.max(0.0);
        self
    }

    /// The width that the scrollbar wants to have.
    pub fn width(&self) -> f32 {
        self.track_width.max(self.thumb_width)
    }

    /// Updates the state of the scrollbar, to be called in the widget's `update` method.
    pub fn update(
        &mut self,
        state: &mut State,
        event: &Event,
        bounds: Rectangle,
        scroll_state: Option<Viewport>,
        cursor: mouse::Cursor,
    ) -> ScrollResult {
        let (mut result, status) = update(
            self, self.status, state, event, bounds, scroll_state, cursor);

        if status != self.status && result == ScrollResult::None {
            result = ScrollResult::AppearanceChanged;
        }

        self.status = status;

        result
    }

    /// Draws the scrollbar, to be called in the widget's `draw` method. If `viewport` is `None`,
    /// the scrollbar is drawn as disabled.
    pub fn draw<Renderer>(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        bounds: Rectangle,
        scroll_state: Option<Viewport>,
    )
    where
        Renderer: iced_core::Renderer,
        Theme: Catalog,
    {
        draw(self, self.status, &self.class, renderer, theme, bounds, scroll_state,)
    }
}

impl<'a, Theme> Default for VerticalScrollbar<'a, Theme>
where
    Theme: Catalog
{
    fn default() -> Self {
        VerticalScrollbar {
            track_width: 10.0,
            thumb_width: 10.0,
            status: Status::Enabled(BarStatus::Active),
            class: Theme::scroll_default(),
        }
    }
}

impl<'a, Theme> Scrollbar for VerticalScrollbar<'a, Theme>
where
    Theme: Catalog
{
    fn layout(&self, bounds: Rectangle, viewport: Viewport) -> Option<Layout> {
        if bounds.width == 0.0 || bounds.height == 0.0 {
            return None
        }

        // If the provided bound width isn't our requested height, we horizontally center.
        let center = bounds.x + bounds.width / 2.0;
        let max_offset = self.width().min(bounds.width) / 2.0;

        let track_bounds = Rectangle {
            x: center - (self.track_width / 2.0).min(max_offset),
            y: bounds.y,
            width: self.track_width.min(bounds.width),
            height: bounds.height,
        };

        let thumb_height = (bounds.height * viewport.viewport_ratio())
            .min(bounds.height)
            .max(10.0);

        let offset = self.thumb_offset_from_viewport(viewport, bounds.height, thumb_height);

        let thumb_bounds = Rectangle {
            x: center - (self.thumb_width / 2.0).min(max_offset),
            y: bounds.y + offset,
            width: self.thumb_width.min(bounds.width),
            height: thumb_height,
        };

        Some(Layout {
            track: track_bounds,
            thumb: thumb_bounds,
        })
    }

    fn region(&self, layout: &Layout, cursor_position: Point) -> ScrollbarRegion {
        if cursor_position.y < layout.thumb.y {
            ScrollbarRegion::TrackBeforeThumb(cursor_position.y - layout.track.y)
        } else if cursor_position.y < layout.thumb.y + layout.thumb.height {
            ScrollbarRegion::Thumb(cursor_position.y - layout.thumb.y)
        } else {
            ScrollbarRegion::TrackAfterThumb(cursor_position.y - layout.track.y)
        }
    }

    fn max_visual_range(&self, layout: &Layout) -> f32 {
        (layout.track.height - layout.thumb.height).max(0.0)
    }

    fn thumb_offset_from_grab(&self, layout: &Layout, cursor: Point, grab_offset: f32) -> f32 {
        (cursor.y - layout.track.y - grab_offset)
            .min(self.max_visual_range(layout))
            .max(0.0)
    }

    fn track_click_offset(&self, layout: &Layout, cursor: Point) -> f32 {
        (cursor.y - layout.track.y)
            .min(layout.track.height - 1.0)
            .max(0.0)
    }
}

trait Scrollbar {
    fn layout(&self, bounds: Rectangle, scroll_state: Viewport) -> Option<Layout>;

    /// Find the region that the cursor is in. The region isn't limited to the scrollbar itself:
    /// for the [`HorizontalScrollbar`] the y-axis is irrelevant and for the [`VerticalScrollbar`]
    /// the x-axis is irrelevant.
    fn region(&self, scrollbar: &Layout, cursor_position: Point) -> ScrollbarRegion;

    /// The amount of space the thumb has to move around.
    fn max_visual_range(&self, scrollbar: &Layout) -> f32;

    /// Calculates the offset of the thumb (which corresponds with its top/left bound) in the
    /// scrollbar as pixels, calculated from where it was grabbed.
    fn thumb_offset_from_grab(&self, scrollbar: &Layout, cursor: Point, grab_offset: f32) -> f32;

    fn track_click_offset(&self, layout: &Layout, cursor: Point) -> f32;

    fn virtual_offset_from_visual(
        &self,
        scrollbar: &Layout,
        visual_offset: f32,
        scroll_state: Viewport,
    ) -> i64 {
        let scroll_max = scroll_state.virtual_max_offset();

        // We use integers here to avoid rounding errors due to floating point arithmetic.
        (scroll_max * visual_offset as i64 / self.max_visual_range(scrollbar).max(1.0) as i64)
            .min(scroll_state.virtual_max_offset())
    }

    fn thumb_offset_from_viewport(&self, viewport: Viewport, bounds_length: f32, thumb_length: f32) -> f32 {
        let virtual_max_offset = viewport.virtual_max_offset();
        let visual_max_offset = (bounds_length - thumb_length).max(0.0);

        if virtual_max_offset == 0 {
            0.0
        } else {
            viewport.offset as f32
                / virtual_max_offset as f32
                * visual_max_offset
        }
    }
}

/// Contains the state of the [`HorizontalScrollbar`] or [`VerticalScrollbar`] and serves a similar 
/// role as the state of [`Widget`]s. Widgets using the scrollbars should call `State::default()`
/// and store the result in their own state. It should be passed to the scrollbars in the `update`
/// and `draw` methods.
#[derive(Debug, Clone, Copy, Default)]
pub struct State {
    last_region: Option<ScrollbarRegion>,
    last_click: Option<mouse::Click>,
}

fn update<S>(
    scrollbar: &S,
    status: Status,
    state: &mut State,
    event: &Event,
    bounds: Rectangle,
    scroll_state: Option<Viewport>,
    cursor: mouse::Cursor,
) -> (ScrollResult, Status)
where
    S: Scrollbar,
{
    if matches!(event, Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
        | Event::Touch(
            touch::Event::FingerLifted { .. }
            | touch::Event::FingerLost { .. })
        ) {
        state.last_region = None;
    }

    let Some(scroll_state) = scroll_state else {
        return (ScrollResult::None, Status::Disabled)
    };

    let layout = scrollbar.layout(bounds, scroll_state);
    let cursor_position= cursor.position();

    let scrollbar_hovered =
        matches!((&layout, &cursor_position), (Some(layout), &Some(cursor))
            if layout.track.union(&layout.thumb).contains(cursor));

    let update = || {
        let Some(cursor_position) = cursor.position() else {
            return ScrollResult::None;
        };

        let Some(layout) = layout else {
            return ScrollResult::None
        };

        if scrollbar_hovered
            && matches!(event,
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                | Event::Touch(touch::Event::FingerPressed { .. }))
        {
            let region = scrollbar.region(&layout, cursor_position);
            state.last_region = Some(region);

            let click = mouse::Click::new(
                cursor_position,
                mouse::Button::Left,
                state.last_click,
            );

            state.last_click = Some(click);

            return match region {
                ScrollbarRegion::Thumb(_) => {
                    ScrollResult::ThumbGrabbed(click.kind())
                }
                ScrollbarRegion::TrackBeforeThumb(visual_offset) => {
                    ScrollResult::TrackClicked(
                        click.kind(),
                        TrackSide::Before,
                        scrollbar.virtual_offset_from_visual(&layout, visual_offset, scroll_state)
                    )
                }
                ScrollbarRegion::TrackAfterThumb(visual_offset) => {
                    ScrollResult::TrackClicked(
                        click.kind(),
                        TrackSide::After,
                        scrollbar.virtual_offset_from_visual(&layout, visual_offset, scroll_state)
                    )
                }
            }
        }

        if let Some(last_region) = state.last_region {
            let region = scrollbar.region(&layout, cursor_position);

            let track = |
                direction: TrackSide,
            | {
                let new_visual_offset = scrollbar.track_click_offset(
                    &layout, cursor_position
                );

                let virtual_offset = scrollbar.virtual_offset_from_visual(
                    &layout, new_visual_offset, scroll_state);

                let kind = state.last_click
                    .map_or(mouse::click::Kind::Single, |click| {click.kind()});

                ScrollResult::TrackHeld(
                    kind,
                    direction,
                    virtual_offset
                )
            };

            match last_region {
                ScrollbarRegion::Thumb(grab_offset) => {
                    if matches!(event,
                        Event::Mouse(mouse::Event::CursorMoved { .. })
                        | Event::Touch(touch::Event::FingerMoved { .. }))
                    {
                        let visual_offset = scrollbar.thumb_offset_from_grab(
                            &layout, cursor_position, grab_offset,
                        );

                        let virtual_offset = scrollbar.virtual_offset_from_visual(
                            &layout, visual_offset, scroll_state);

                        if virtual_offset != scroll_state.offset {
                            return ScrollResult::ThumbDragged(virtual_offset);
                        }
                    }
                }
                ScrollbarRegion::TrackBeforeThumb(_) => {
                    if matches!(region, ScrollbarRegion::TrackBeforeThumb(_)) {
                        return track(TrackSide::Before);
                    }
                }
                ScrollbarRegion::TrackAfterThumb(_) => {
                    if matches!(region, ScrollbarRegion::TrackAfterThumb(_)) {
                        return track(TrackSide::After);
                    }
                }
            }
        }

        ScrollResult::None
    };

    let result = update();

    let status = if matches!(status, Status::Enabled( .. )) {
        if state.last_region.is_some() {
            Status::Enabled(BarStatus::Dragged)
        } else if scrollbar_hovered {
            Status::Enabled(BarStatus::Hovered)
        } else {
            Status::Enabled(BarStatus::Active)
        }
    } else {
        Status::Disabled
    };

    (result, status)
}

fn draw<'a, Theme, S, Renderer>(
    scrollbar: &S,
    status: Status,
    class: &Theme::ScrollClass<'a>,
    renderer: &mut Renderer,
    theme: &Theme,
    bounds: Rectangle,
    scroll_state: Option<Viewport>,
)
where
    S: Scrollbar,
    Theme: Catalog,
    Renderer: iced_core::Renderer
{
    let Some(scroll_state) = scroll_state else {
        return;
    };

    let Some(layout) = scrollbar.layout(bounds, scroll_state) else {
        return;
    };

    let style = theme.scroll_style(class, status);

    // Draw the track.
    if layout.track.width > 0.0
        && layout.track.height > 0.0
        && (style.background.is_some()
        || (style.border.color != Color::TRANSPARENT
        && style.border.width > 0.0))
    {
        renderer.fill_quad(
            renderer::Quad {
                bounds: layout.track,
                border: style.border,
                ..renderer::Quad::default()
            },
            style.background.unwrap_or(Background::Color(
                Color::TRANSPARENT,
            )),
        );
    }

    // Draw the thumb.
    if !scroll_state.is_fully_visible()
        && layout.thumb.width > 0.0
        && layout.thumb.height > 0.0
        && (style.thumb_style.color != Color::TRANSPARENT
        || (style.thumb_style.border.color != Color::TRANSPARENT
        && style.thumb_style.border.width > 0.0))
    {
        renderer.fill_quad(
            renderer::Quad {
                bounds: layout.thumb,
                //bounds: new_bounds,
                border: style.thumb_style.border,
                ..renderer::Quad::default()
            },
            style.thumb_style.color,
        );
    }
}

/// The result of handling an event.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScrollResult {
    /// The event caused the thumb to be dragged. Contains the virtual viewport offset that
    /// corresponds to the thumb's location.
    ThumbDragged(i64),
    /// The track before or after the thumb was clicked. Stores the type of click (single, double
    /// or triple), which side of the thumb the track was clicked, and the virtual offset that
    /// corresponds to the location of the cursor.
    TrackClicked(mouse::click::Kind, TrackSide, i64),
    /// The track before or after the thumb was clicked in the past and the mouse button was held.
    /// Stores the type of click (single, double or triple), which side of the thumb the track was
    /// clicked, and the virtual offset that corresponds to the location of the current cursor.
    TrackHeld(mouse::click::Kind, TrackSide, i64),
    /// The thumb was grabbed. This in itself doesn't constitute a viewport change.
    ThumbGrabbed(mouse::click::Kind),
    /// No change to the viewport, but Scroller asked for a redraw either way, typically after the
    /// scrollbar was hovered over.
    AppearanceChanged,
    /// The event wasn't handled in any way.
    None,
}

/// The possible status of a [`HorizontalScrollbar`] or [`VerticalScrollbar`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// The scrollbar is enabled.
    Enabled(BarStatus),
    /// The scrollbar is disabled.
    Disabled,
}

/// The possible status of a [`HorizontalScrollbar`] or [`VerticalScrollbar`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarStatus {
    /// The scrollbar is active.
    Active,
    /// The scrollbar is being hovered over.
    Hovered,
    /// The scrollbar is being interacted with in some manner.
    Dragged,
}

/// Denotes whether the track click occurred before or after the thumb.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum TrackSide {
    /// The click happened above (vertical) or to the left (horizontal) of the thumb.
    Before,
    /// The click happened below (vertical) or to the right (horizontal) of the thumb.
    After,
}

/// Properties of the 1-dimensional viewport of a [`HorizontalScrollbar`] and [`VerticalScrollbar`]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Viewport {
    /// Virtual offset in steps.
    pub offset: i64,
    /// Virtual length in steps.
    pub size: i64,
    /// Number of pixels each step occupies.
    pub step_size: f32,
    /// size of the content's viewport in pixels. This may be different from the scrollbar's length,
    /// and is used to determine how much of the content can be displayed at any given time.
    pub content_viewport_size: f32,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            offset: 0,
            size: 0,
            step_size: 1.0,
            content_viewport_size: 0.0,
        }
    }
}

impl Viewport {
    /// Creates a new `Viewport`.
    pub fn new(offset: i64, size: i64, step_size: f32, content_viewport_size: f32) -> Self {
        Self {
            offset,
            size,
            step_size,
            content_viewport_size
        }
    }

    /// Adds the number of steps, clamped to valid values. `Viewport` also implements
    /// `ops::Add<i64>` that returns the new offset without modifying `self`.
    pub fn add_steps(mut self, steps: i64) -> Self {
        self.offset += steps;
        self
    }

    /// Subtracts the number of steps, clamped to valid values. `Viewport` also implements
    /// `ops::Sub<i64>` that returns the new offset without modifying `self`.
    pub fn subtract_steps(mut self, steps: i64) -> Self {
        self.offset -= steps;
        self
    }

    /// Clamps the scroll offset to a valid value.
    pub fn fitted_scroll_offset(&self) -> i64 {
        self.offset
            .min(self.virtual_max_offset())
            .max(0)
    }

    /// Calculates the number of steps that completely or partially fit in the viewport.
    pub fn viewport_steps_ceil(&self) -> i64 {
        (self.content_viewport_size / self.step_size).ceil() as i64
    }

    /// Calculates the number of steps that completely fit in the viewport.
    pub fn viewport_steps_floor(&self) -> i64 {
        (self.content_viewport_size / self.step_size).floor() as i64
    }

    /// The maximum offset we should put the viewport at. The maximum scroll offset will be such
    /// that the last data is in the viewport, and the viewport is completely filled. We don't want
    /// half empty viewports unless the data completely fits inside the viewport already.
    pub fn virtual_max_offset(&self) -> i64 {
        (self.size - self.viewport_steps_floor()).max(0)
    }

    /// The number of pixels the content occupies virtually. Note that for very large virtual sizes
    /// the result may be imprecise due to the limited exactness of floating point notation.
    pub fn virtual_size_in_pixels(&self) -> i64 {
        (self.size as f64 * self.step_size as f64).ceil() as i64
    }

    /// Ratio of how much of the content would be visible in the viewport, all in pixels. Does not
    /// take current scroll offset into account.
    pub fn viewport_ratio(&self) -> f32 {
        self.content_viewport_size / self.virtual_size_in_pixels() as f32
    }

    /// Whether the content is fully visible in the viewport.
    pub fn is_fully_visible(&self) -> bool {
        self.size as f32 * self.step_size <= self.content_viewport_size
    }
}

impl ops::Add<i64> for Viewport {
    type Output = i64;

    /// Calculates the new offset, clamped to valid values.
    fn add(self, steps: i64) -> Self::Output {
        (self.offset + steps)
            .min(self.virtual_max_offset())
            .max(0)
    }
}

impl ops::Sub<i64> for Viewport {
    type Output = i64;

    /// Calculates the new offset, clamped to valid values.
    fn sub(self, steps: i64) -> Self::Output {
        self + -steps
    }
}

/// The regions of a scrollbar.
#[derive(Debug, Clone, Copy)]
enum ScrollbarRegion {
    /// The thumb region and the offset in pixels from the top of the thumb.
    Thumb(f32),
    /// The track region before the thumb, and the offset in pixels from the top of the track.
    TrackBeforeThumb(f32),
    /// The track region after the thumb, and the offset in pixels from the top of the track.
    TrackAfterThumb(f32),
}

#[derive(Debug, Clone)]
struct Layout {
    pub track: Rectangle,
    pub thumb: Rectangle,
}

/// The appearance of a [`HorizontalScrollbar`] and [`VerticalScrollbar`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Style {
    /// The track's [`Background`].
    pub background: Option<Background>,
    /// The track's [`Border`].
    pub border: border::Border,
    /// The thumb's style.
    pub thumb_style: ThumbStyle,
}

/// The appearance of the thumb of a [`HorizontalScrollbar`] and [`VerticalScrollbar`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThumbStyle {
    /// The thumb's [`Color`].
    pub color: Color,
    /// The thumb's [`Border`].
    pub border: border::Border,
}

/// The theme catalog of a [`HorizontalScrollbar`] and [`VerticalScrollbar`].
pub trait Catalog {
    /// The item class of the [`Catalog`].
    type ScrollClass<'a>;

    /// The default class produced by the [`Catalog`].
    fn scroll_default<'a>() -> Self::ScrollClass<'a>;

    /// The [`Style`] of a class with the given status.
    fn scroll_style(&self, class: &Self::ScrollClass<'_>, status: Status) -> Style;
}

/// A styling function for a [`HorizontalScrollbar`] and [`VerticalScrollbar`].
pub type StyleFn<'a, Theme> = Box<dyn Fn(&Theme, Status) -> Style + 'a>;

impl Catalog for Theme {
    type ScrollClass<'a> = StyleFn<'a, Self>;

    fn scroll_default<'a>() -> Self::ScrollClass<'a> {
        Box::new(default)
    }

    fn scroll_style(&self, class: &Self::ScrollClass<'_>, status: Status) -> Style {
        class(self, status)
    }
}

/// The default style of a [`HorizontalScrollbar`] and [`VerticalScrollbar`].
pub fn default(theme: &Theme, status: Status) -> Style {
    let palette = theme.extended_palette();

    let active = Style {
        background: Some(palette.background.weak.color.into()),
        border: border::rounded(2),
        thumb_style: ThumbStyle {
            color: palette.background.strongest.color,
            border: border::rounded(2),
        },
    };

    match status {
        Status::Enabled(enabled_status) => {
            match enabled_status {
                BarStatus::Active => {
                    active
                },
                BarStatus::Hovered => {
                    Style {
                        thumb_style: ThumbStyle {
                            color: palette.primary.strong.color,
                            ..active.thumb_style
                        },
                        ..active
                    }
                }
                BarStatus::Dragged => {
                    Style {
                        thumb_style: ThumbStyle {
                            color: palette.primary.base.color,
                            ..active.thumb_style
                        },
                        ..active
                    }
                }
            }
        }
        Status::Disabled => {
            Style {
                background: Some(palette.background.weakest.color.into()),
                thumb_style: ThumbStyle {
                    color: palette.background.weakest.color,
                    ..active.thumb_style
                },
                ..active
            }
        }
    }
}
