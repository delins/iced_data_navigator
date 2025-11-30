pub use crate::core::scrollbar::{
    Catalog, TrackSide, HorizontalScrollbar, VerticalScrollbar, ScrollResult, Viewport
};
use crate::core::scrollbar::State as ScrollbarState;

use iced_core::keyboard;
use iced_core::mouse;
use iced_core::{self, Event, Rectangle, Vector};

/// Scroll area utility struct for virtual scrolling. Can be used inside custom widgets
/// (structs that implement the [`Widget`] trait) to add horizontal and/or vertical scrolling 
/// functionality, as well as wheel scrolling.
pub struct ScrollArea<'a, Theme>
where
    Theme: Catalog
{
    x_scrollbar: Option<HorizontalScrollbar<'a, Theme>>,
    y_scrollbar: Option<VerticalScrollbar<'a, Theme>>,
}

impl<'a, Theme> Default for ScrollArea<'a, Theme>
where
    Theme: Catalog
{
    fn default() -> Self {
        Self {
            x_scrollbar: None,
            y_scrollbar: None,
        }
    }
}

impl<'a, Theme> ScrollArea<'a, Theme>
where
    Theme: Catalog
{
    /// Creates a default [`ScrollArea`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Enables the horizontal scrollbar. 
    pub fn horizontal_scrollbar(mut self, scrollbar: HorizontalScrollbar<'a, Theme>) -> Self {
        self.x_scrollbar = Some(scrollbar);
        self
    }

    /// Enables the vertical scrollbar. 
    pub fn vertical_scrollbar(mut self, scrollbar: VerticalScrollbar<'a, Theme>) -> Self {
        self.y_scrollbar = Some(scrollbar);
        self
    }

    /// The height that the horizontal scrollbar would like to have. 0 if the horizontal scrollbar
    /// is disabled.
    pub fn horizontal_scrollbar_height(&self) -> f32 {
        self.x_scrollbar
            .as_ref()
            .map_or(0.0, |scrollbar| {scrollbar.height()})
    }

    /// The width that the vertical scrollbar would like to have. 0 if the vertical scrollbar is 
    /// disabled.
    pub fn vertical_scrollbar_width(&self) -> f32 {
        self.y_scrollbar
            .as_ref()
            .map_or(0.0, |scrollbar| {scrollbar.width()})
    }

    /// Updates the state of the scroll area, to be called in the widget's `update` method.
    pub fn update(
        &mut self,
        state: &mut State,
        event: &Event,
        bounds: Rectangle,
        x_viewport: Option<Viewport>,
        y_viewport: Option<Viewport>,
        cursor: mouse::Cursor,
    ) -> ScrollAreaResult {
        match event {
            Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
                state.keyboard_modifiers = *modifiers;
            }
            Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                if cursor.position_over(bounds).is_none() {
                    return ScrollAreaResult::None;
                }

                let delta = match *delta {
                    mouse::ScrollDelta::Lines { x, y } => {
                        let is_shift_pressed =
                            state.keyboard_modifiers.shift();

                        // MacOS automatically inverts the axes when shift is pressed.
                        let (x, y) = if cfg!(target_os = "macos")
                            && is_shift_pressed
                        {
                            (y, x)
                        } else {
                            (x, y)
                        };

                        let movement = if !is_shift_pressed {
                            Vector::<i64>::new(x as i64, y as i64)
                        } else {
                            Vector::<i64>::new(y as i64, x as i64)
                        };

                        // A negative value means scrolling down, and vice versa. So we need to
                        // invert. A single scroll appears to be -1 or +1.
                        -movement
                    },
                    mouse::ScrollDelta::Pixels { x, y } => {
                        // Seems to come straight from winit and might be caused by
                        // touchscreens. We want a scroll expressed in steps, not pixels. So
                        // convert. It probably won't work well with all step sizes.
                        -Vector::new(
                            x_viewport.map_or(0, |s| {
                                (x / s.step_size).max(1.0) as i64
                            }),
                            y_viewport.map_or(0, |s| {
                                (y / s.step_size).max(1.0) as i64
                            }),
                        )
                    }
                };

                let (x_old, x_new) = x_viewport.map_or((0, 0), |x| {
                    (x.offset, x + delta.x)
                });

                let (y_old, y_new) = y_viewport.map_or((0, 0), |y| {
                    (y.offset, y + delta.y)
                });

                if x_old != x_new || y_old != y_new {
                    return ScrollAreaResult::WheelScroll {
                        x: x_new,
                        y: y_new
                    }
                }
            }
            _ => {}
        }

        if let Some(scrollbar) = self.x_scrollbar.as_mut() {
            let bounds = x_bounds(bounds, scrollbar, &self.y_scrollbar);
            let result = scrollbar.update(
                &mut state.x_state, event, bounds, x_viewport, cursor);

            if result != ScrollResult::None {
                return ScrollAreaResult::Horizontal(result);
            }
        }

        if let Some(scrollbar) = self.y_scrollbar.as_mut() {
            let bounds = y_bounds(bounds, scrollbar, &self.x_scrollbar);
            let result = scrollbar.update(
                &mut state.y_state, event, bounds, y_viewport, cursor);

            if result != ScrollResult::None {
                return ScrollAreaResult::Vertical(result);
            }
        }

        ScrollAreaResult::None
    }

    /// Draws the scroll area, to be called in the widget's `draw` method.
    pub fn draw<Renderer>(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        bounds: Rectangle,
        x_viewport: Option<Viewport>,
        y_viewport: Option<Viewport>,
    )
    where
        Renderer: iced_core::Renderer
    {
        if let Some(scrollbar) = &self.x_scrollbar {
            let bounds = x_bounds(bounds, scrollbar, &self.y_scrollbar);
            scrollbar.draw(renderer, theme, bounds, x_viewport);
        }

        if let Some(scrollbar) = &self.y_scrollbar {
            let bounds = y_bounds(bounds, scrollbar, &self.x_scrollbar);
            scrollbar.draw(renderer, theme, bounds, y_viewport);
        }
    }
}

/// Contains the state of the [`ScrollArea`] and serves a similar role as the state of
/// [`Widget`]s. Widgets using ScrollArea should call `State::default()` and store the result in
/// their own state. It should be passed to ScrollArea in the `update` and `draw` methods.
#[derive(Debug, Clone, Copy, Default)]
pub struct State {
    x_state: ScrollbarState,
    y_state: ScrollbarState,
    keyboard_modifiers: keyboard::Modifiers,
}

/// Calculate the bounds of the horizontal scrollbar.
fn x_bounds<Theme>(
    bounds: Rectangle,
    x_scrollbar: &HorizontalScrollbar<Theme>,
    y_scrollbar: &Option<VerticalScrollbar<Theme>>,
) -> Rectangle
where
    Theme: Catalog
{
    let y_scrollbar_width = y_scrollbar
        .as_ref()
        .map_or(0.0, |scrollbar| scrollbar.width());

    Rectangle {
        x: bounds.x,
        y: (bounds.y + bounds.height - x_scrollbar.height()).max(bounds.y),
        width: (bounds.width - y_scrollbar_width).max(0.0),
        height: bounds.height.min(x_scrollbar.height())
    }
}

/// Calculate the bounds of the vertical scrollbar.
fn y_bounds<Theme>(
    bounds: Rectangle,
    y_scrollbar: &VerticalScrollbar<Theme>,
    x_scrollbar: &Option<HorizontalScrollbar<Theme>>,
) -> Rectangle
where
    Theme: Catalog
{
    let x_scrollbar_height = x_scrollbar
        .as_ref()
        .map_or(0.0, |scrollbar| scrollbar.height());

    Rectangle {
        x: (bounds.x + bounds.width - y_scrollbar.width()).max(bounds.x),
        y: bounds.y,
        width: bounds.width.min(y_scrollbar.width()),
        height: (bounds.height - x_scrollbar_height).max(0.0)
    }
}

/// The result of handling an event. The `Horizontal` and `Vertical` variants can be ignored if
/// their respective scrollbars aren't used.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScrollAreaResult {
    /// The horizontal scrollbar was interacted with.
    Horizontal(ScrollResult),
    /// The horizontal scrollbar was interacted with.
    Vertical(ScrollResult),
    /// Wheel was scrolled which resulted in a change in either the x or y offset (or both).
    /// Contains the new virtual viewport offset.
    WheelScroll {
        /// The horizontal offset.
        x: i64,
        /// The vertical offset.
        y: i64,
    },
    /// The event wasn't handled in any way.
    None
}
