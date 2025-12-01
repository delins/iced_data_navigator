use crate::core::scroll_area::{
    Catalog as ScrollCatalog, TrackSide, ScrollArea, HorizontalScrollbar, VerticalScrollbar,
    ScrollAreaResult, ScrollResult, Viewport as ScrollViewport, State as ScrollAreaState
};
use crate::core::util::Timer;

use bitflags::bitflags;
use encoding_rs;
use iced_core::alignment;
use iced_core::keyboard;
use iced_core::layout::{self, Limits};
use iced_core::mouse::{self, Cursor};
use iced_core::renderer::{self, Quad};
use iced_core::text;
use iced_core::keyboard::key;
use iced_core::widget::tree::{self, Tree};
use iced_core::{
    Background, Border, Clipboard, Color, Element, Event, Font, Length, Padding, Pixels, Point,
    Rectangle, Renderer, Shell, Size, Text, Theme, Widget
};
use iced_widget::text::Wrapping;
use std::fmt::Debug;
use std::cmp::{PartialEq, Ordering};
use std::time::{Instant};
use std::ops::Range;
use std::sync::atomic;

static CONTENT_COUNTER: atomic::AtomicU64 = atomic::AtomicU64::new(0);

/// A widget for viewing and interacting with binary data of virtually any size.
pub struct HexViewer<'a, Message, Theme>
where
    Theme: Catalog
{
    content: &'a Content,
    cursor: i64,
    width: Length,
    height: Length,
    font: Option<Font>,
    font_size: Option<Pixels>,
    virtual_columns: i64,
    horizontal_step: Step,
    layout_settings: PaddingSettings,
    horizontal_navigation: Navigation,
    vertical_navigation: Navigation,
    content_styler: Option<&'a ContentStyler>,
    on_cursor_moved: Option<Box<dyn Fn(u64) -> Message + 'a>>,
    on_scrolled: Option<Box<dyn Fn(Viewport) -> Message + 'a>>,
    on_logical_viewport_size_changed: Option<Box<dyn Fn(Viewport) -> Message + 'a>>,
    on_selection: Option<Box<dyn Fn(Option<Selection>) -> Message + 'a>>,
    class: Theme::Class<'a>,
    scroll_area: ScrollArea<'a, Theme>,
}

impl<'a, Message, Theme> HexViewer<'a, Message, Theme>
where
    Theme: Catalog
{
    /// Creates a new HexViewer given the provided [`Content`].
    pub fn new(
        content: &'a Content,
    ) -> Self {
        Self {
            content,
            cursor: 0,
            width: Length::Shrink,
            height: Length::Fill,
            font: None,
            font_size: None,
            virtual_columns: 32,
            horizontal_step: Step::default(),
            layout_settings: PaddingSettings::default(),
            horizontal_navigation: Navigation::Lazy,
            vertical_navigation: Navigation::Lazy,
            content_styler: None,
            on_cursor_moved: None,
            on_scrolled: None,
            on_logical_viewport_size_changed: None,
            on_selection: None,
            class: Theme::default(),
            scroll_area: ScrollArea::default()
                .horizontal_scrollbar(HorizontalScrollbar::new())
                .vertical_scrollbar(VerticalScrollbar::new()),
        }
    }

    /// Sets the width.
    pub fn width(mut self, width: impl Into<Pixels>) -> Self {
        self.width = Length::from(width.into());
        self
    }

    /// Sets the height.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Sets the cursor, as an absolute offset into the [`Content`].
    pub fn cursor(mut self, cursor: u64) -> Self {
        self.cursor = cursor as i64;
        self
    }

    /// Sets the font to render with. If unset, the [`Renderer`]'s default monospaced font is used.
    pub fn font(mut self, font: impl Into<Font>) -> Self {
        self.font = Some(font.into());
        self
    }

    /// Sets the font to render with. If unset or None is set, the [`Renderer`]'s default monospaced
    /// font is used.
    pub fn font_maybe(mut self, font: Option<Font>) -> Self {
        self.font = font;
        self
    }

    /// Sets the font size to render with. If unset, the [`Renderer`]'s default font size is used.
    pub fn font_size(mut self, size: impl Into<Pixels>) -> Self {
        self.font_size = Some(size.into());
        self
    }

    /// Sets the font size to render with. If unset or None is set, the [`Renderer`]'s default font
    /// size is used.
    pub fn font_size_maybe(mut self, size: Option<Pixels>) -> Self {
        self.font_size = size;
        self
    }

    /// Sets the virtual number of columns. If this makes the content too wide horizontal scrollbars
    /// are displayed to scroll through the content.
    pub fn virtual_columns(mut self, columns: u64) -> Self {
        self.virtual_columns = columns.max(1) as i64;
        self
    }

    /// Sets the horizontal [`Step`] that controls whether a horizontal scroll movement moves per
    /// column or per pixel.
    pub fn horizontal_step(mut self, step: Step) -> Self {
        self.horizontal_step = step;
        self
    }

    /// Sets the padding settings.
    pub fn padding_settings(mut self, settings: PaddingSettings) -> Self {
        self.layout_settings = settings;
        self
    }

    /// Controls whether implicit horizontal scrolls, such as the cursor moving horizontally and the
    /// viewport following to keep it in view, scroll lazily or keep the target aligned.
    pub fn horizontal_navigation(mut self, navigation: Navigation) -> Self {
        self.horizontal_navigation = navigation;
        self
    }

    /// Controls whether implicit horizontal scrolls, such as the cursor moving horizontally and the
    /// viewport following to keep it in view, scroll lazily or keep the target aligned.
    pub fn horizontal_navigation_maybe(mut self, navigation_maybe: Option<Navigation>) -> Self {
        if let Some(navigation) = navigation_maybe {
            self.horizontal_navigation = navigation;
        }
        self
    }

    /// Controls whether implicit vertical scrolls, such as the cursor moving vertically and the
    /// viewport following to keep it in view, scroll lazily or keep the target aligned.
    pub fn vertical_navigation(mut self, navigation: Navigation) -> Self {
        self.vertical_navigation = navigation;
        self
    }

    /// Controls whether implicit vertical scrolls, such as the cursor moving vertically and the
    /// viewport following to keep it in view, scroll lazily or keep the target aligned.
    pub fn vertical_navigation_maybe(mut self, navigation_maybe: Option<Navigation>) -> Self {
        if let Some(navigation) = navigation_maybe {
            self.vertical_navigation = navigation;
        }
        self
    }

    /// Sets the [`ContentStyler`], which is used to color of the bytes/chars.
    pub fn content_styler(mut self, content_style: &'a ContentStyler) -> Self {
        self.content_styler = Some(content_style);
        self
    }

    /// Sets the message that should be produced when the cursor is moved.
    pub fn on_cursor_moved(mut self, func: impl Fn(u64) -> Message + 'a) -> Self {
        self.on_cursor_moved = Some(Box::new(func));
        self
    }

    /// Sets the message that should be produced when the viewport is scrolled.
    pub fn on_scrolled(mut self, func: impl Fn(Viewport) -> Message + 'a) -> Self {
        self.on_scrolled = Some(Box::new(func));
        self
    }

    /// Sets the message that should be produced when the logical viewport size has changed.
    /// This is typically caused by setting a different column count with
    /// [`HexViewer::virtual_columns`], or the application as a whole resizing.
    pub fn on_logical_viewport_resized(
        mut self, func: impl Fn(Viewport) -> Message + 'a) -> Self {
        self.on_logical_viewport_size_changed = Some(Box::new(func));
        self
    }

    /// Sets the message that should be produced when the selection as changed. The message is
    /// published with every change to the selection. When the selection is ended, the value is
    /// `None`. If the selection is made by mouse, the message set by [`HexViewer::on_cursor_moved`]
    /// isn't published. If you want to set the cursor while the selection is going, use either
    /// [`Selection::last`] or [`Selection::last_contained`].
    pub fn on_selection(mut self, func: impl Fn(Option<Selection>) -> Message + 'a) -> Self {
        self.on_selection = Some(Box::new(func));
        self
    }

    /// Sets the style of the [`HexViewer`].
    pub fn style(mut self, style: impl Fn(&Theme, Status) -> Style + 'a) -> Self
    where
        Theme::Class<'a>: From<StyleFn<'a, Theme>>,
    {
        self.class = (Box::new(style) as StyleFn<'a, Theme>).into();
        self
    }

    /// Calculates the number of chars needed to address the highest offset.
    fn address_area_horizontal_char_count(&self) -> usize {
        let highest_address = format!("{}", self.content.source_size);
        highest_address.chars().count()
    }

    fn cursor_can_decrease(&self) -> bool {
        self.cursor > 0
    }

    fn cursor_can_increase(&self) -> bool {
        self.cursor + 1 < self.content.source_size
    }

    /// Finds the new cursor position if the move is possible and None otherwise.
    fn move_cursor_left(&self) -> Option<i64> {
        self.cursor_can_decrease().then(|| self.cursor - 1)
    }

    /// Finds the new cursor position if the move is possible and None otherwise.
    fn move_cursor_right(&self) -> Option<i64> {
        self.cursor_can_increase().then(|| self.cursor + 1)
    }

    /// Finds the new cursor position if the move is possible and None otherwise.
    fn move_cursor_up(&self) -> Option<i64> {
        self.cursor_can_decrease().then(|| (self.cursor - self.virtual_columns).max(0))
    }

    /// Finds the new cursor position if the move is possible and None otherwise.
    fn move_cursor_down(&self) -> Option<i64> {
        self.cursor_can_increase()
            .then(|| (self.cursor + self.virtual_columns).min(self.content.source_size.max(1) - 1))
    }

    /// Finds the new cursor position if the move is possible and None otherwise.
    fn move_cursor_page_up(&self, page_size: i64) -> Option<i64> {
        self.cursor_can_decrease().then(|| {
            (self.cursor - page_size * self.virtual_columns).max(0)
        })
    }

    /// Finds the new cursor position if the move is possible and None otherwise.
    fn move_cursor_page_down(&self, page_size: i64) -> Option<i64> {
        self.cursor_can_increase().then(|| {
            (self.cursor + page_size * self.virtual_columns).min(self.content.source_size.max(1) - 1)
        })
    }

    /// Finds the new cursor position if the move is possible and None otherwise.
    fn move_cursor_top(&self) -> Option<i64> {
        self.cursor_can_decrease().then_some(0)
    }

    /// Finds the new cursor position if the move is possible and None otherwise.
    fn move_cursor_bottom(&self) -> Option<i64> {
        self.cursor_can_increase().then(|| {
            (self.content.source_size - 1).max(0)
        })
    }

    /// Scrolls to the target offset, which is the absolute offset into the source from 0.
    fn scroll_viewport(
        &mut self,
        target_offset: i64,
        layout: &Layout,
        horizontal: Scroll,
        vertical: Scroll,
    ) -> Option<Viewport> {
        // for horizontal, we make a lazy closure and static closure. in case of adaptive we check
        // first whether the thing is in the viewpport and decide on that

        let target_column = target_offset % self.virtual_columns;
        let target_row = target_offset / self.virtual_columns;

        let col_in_view = self.column_fully_in_viewport(target_column, layout).is_some();
        let row_in_view = self.row_fully_in_viewport(target_row, layout).is_some();

        let mut percentage_x = 0.0;

        let column = match horizontal {
            Scroll::Lazy(alignment) => {
                if col_in_view {
                    percentage_x = self.content.viewport.percentage_x;
                    self.content.viewport.x
                } else {
                    match alignment {
                        LazyAlignment::Start => {
                            target_column
                        }
                        LazyAlignment::End => {
                            target_column - layout.viewport_column_count_floor() + 1
                        }
                    }
                }
            }
            Scroll::Aligned(alignment) => {
                match alignment {
                    Alignment::Start => {
                        target_column
                    }
                    Alignment::Center => {
                        target_column - (layout.viewport_column_count_floor() + 1) / 2
                    }
                    Alignment::End => {
                        target_column - layout.viewport_column_count_floor() + 1
                    }
                }
            }
        }.min(layout.max_viewport_x_offset()).max(0);

        let row = match vertical {
            Scroll::Lazy(alignment) => {
                if row_in_view {
                    self.content.viewport.y
                } else {
                    match alignment {
                        LazyAlignment::Start => {
                            target_row
                        }
                        LazyAlignment::End => {
                            target_row - layout.viewport_row_count_floor() + 1
                        }
                    }
                }
            }
            Scroll::Aligned(alignment) => {
                match alignment {
                    Alignment::Start => {
                        target_row
                    }
                    Alignment::Center => {
                        target_row - (layout.viewport_row_count_floor() + 1) / 2
                    }
                    Alignment::End => {
                        target_row - layout.viewport_row_count_floor() + 1
                    }
                }
            }
        }.min(layout.max_viewport_y_offset()).max(0);

        (column != self.content.viewport.x || percentage_x != self.content.viewport.percentage_x || row != self.content.viewport.y)
            .then_some(self.create_viewport(layout, column, row, percentage_x))
    }

    /// Determines what selection can be made between the two indices, if any. The order in which
    /// the indices are supplied doesn't matter.
    fn selection(
        &self,
        a: Index,
        b: Index,
        current_cursor: i64,
    ) -> Option<Selection> {
        let (left, right) = if a < b {(a, b) } else {(b, a)};

        let start = left.offset + (left.side == Side::Right) as i64;
        let length = right.offset - left.offset - 1
            + (left.side == Side::Left || left.side == Side::None) as i64
            + (right.side == Side::Right || right.side == Side::None) as i64;

        (length > 0).then(|| Selection::new(start as u64, length as u64, current_cursor as u64))
    }

    fn create_layout(&self, metrics: HexMetrics, bounds: Rectangle, shift_x: f32) -> Layout {
        let (dimensions, settings) =
            self.create_layout_dimensions(metrics, bounds.size());

        Layout::new(
            dimensions,
            settings,
            self.content.source_size,
            self.virtual_columns,
            metrics,
            shift_x,
            bounds,
        )
    }

    fn create_layout_dimensions(&self, metrics: HexMetrics, bounds_size: Size) -> (LayoutDimensions, HexPadding) {
        let settings = HexPadding::new(&self.layout_settings, metrics);

        let dimensions = LayoutDimensions::new(
            &settings,
            self.virtual_columns,
            metrics,
            self.scroll_area.horizontal_scrollbar_height(),
            self.scroll_area.vertical_scrollbar_width(),
            self.content.source_size,
            bounds_size,
            self.height,
        );

        (dimensions, settings)
    }

    /// Create the [`VirtualState`].
    fn x_viewport(&self, layout: &Layout) -> ScrollViewport {
        match self.horizontal_step {
            Step::Cell => {
                ScrollViewport::new(
                    self.content.viewport.x,
                    self.virtual_columns,
                    layout.byte_cell_width,
                    layout.byte_area_content().width.ceil(),
                )
            }
            Step::Pixel => {
                ScrollViewport::new(
                    // Note that this automatically adjusts for sudden Step changes: going from Cell
                    // to Pixel just assumes shift was 0, which is ok. Going from Pixel to Cell
                    // silently drops the small shift and aligns the first (partially) visible byte
                    // to the cell grid. Also, we round here instead of ceil since the percentage
                    // should originate from the actual offset we're on.
                    (self.content.viewport.x as f64
                        * layout.byte_cell_width as f64
                        + layout.byte_shift as f64)
                        .round() as i64,
                    (layout.dim.byte_area_width
                        - layout.padding.byte_area_padding().x())
                        .ceil() as i64,
                    1.0,
                    layout.byte_area_content().width.ceil(),
                )
            }
        }
    }


    /// Create the [`VirtualState`].
    fn y_viewport(&self, layout: &Layout) -> ScrollViewport {
        ScrollViewport::new(
            self.content.viewport.y,
            layout.virtual_rows_ceil(),
            layout.row_height(),
            layout.byte_area_content().height.ceil(),
        )
    }

    fn viewport_offset_x(&self, scroll_offset: ScrollOffset, layout: &Layout) -> (i64, f32) {
        match self.horizontal_step {
            Step::Cell => {
                (scroll_offset.x, 0.0)
            }
            Step::Pixel => {
                (
                    (scroll_offset.x as f64 / layout.byte_cell_width as f64) as i64,
                    (scroll_offset.x as f64 % layout.byte_cell_width as f64) as f32 / layout.byte_cell_width
                )
            }
        }
    }

    fn create_viewport_from_scroll_offset(&self, layout: &Layout, scroll_offset: ScrollOffset) -> Viewport {
        let (x, shift_x) = self.viewport_offset_x(scroll_offset, layout);
        let columns = (self.virtual_columns - x)
            .min(layout.viewport_column_count_ceil() + 1)
            .max(1);

        let rows = ((self.content.source_size + self.virtual_columns - 1)
            / self.virtual_columns - scroll_offset.y)
            .min(layout.viewport_row_count_ceil())
            .max(0);

        Viewport {
            x,
            y: scroll_offset.y,
            columns,
            rows,
            percentage_x: shift_x,
            virtual_columns: self.virtual_columns
        }
    }

    fn create_viewport(&self, layout: &Layout, x: i64, y: i64, shift_x: f32) -> Viewport {
        let columns = (self.virtual_columns - x)
            .min(layout.viewport_column_count_ceil() + 1)
            .max(1);

        let rows = ((self.content.source_size + self.virtual_columns - 1)
            / self.virtual_columns - y)
            .min(layout.viewport_row_count_ceil())
            .max(0);

        Viewport {
            x,
            y,
            columns,
            rows,
            percentage_x: shift_x,
            virtual_columns: self.virtual_columns
        }
    }

    fn cell_to_absolute(&self, cell: &Cell) -> Index {
        let offset = (self.content.viewport.y + cell.row) * self.virtual_columns
            + self.content.viewport.x + cell.col;

        if offset < self.content.source_size {
            Index::new(offset, cell.side)
        } else {
            Index::new((self.content.source_size - 1).max(1), Side::Right)
        }
    }

    fn index(&self, layout: &Layout, location: Location) -> Option<Index> {
        location.approximate_cell(self.virtual_columns, layout.viewport_row_count_ceil())
            .map(|cell_location| {
                self.cell_to_absolute(&cell_location)
            })
    }

    /// Determines if, and if so where, the offset is visible in the current viewport. The returned
    /// offset is the byte offset within the current viewport, so it starts at 0 at the top left
    /// cell. Note: may return Some if the offset is just outside the viewport, need to fix viewport
    /// calculation.
    fn offset_in_viewport(&self, offset: i64) -> Option<(i64, i64)> {
        self.content.viewport.contains(offset as u64).map(|(col, row)| {
            (col as i64, row as i64)
        })
    }

    fn row_fully_in_viewport(&self, row: i64, layout: &Layout) -> Option<i64> {
        // We ignore and percent stuff for now, just focusx on x, y col, and row.

        let &vp = &self.content.viewport;

        let y_end = vp.y + vp.rows.min(layout.viewport_row_count_floor());

        (row >= vp.y && row < y_end).then(|| row - vp.y)
    }

    fn column_fully_in_viewport(&self, column: i64, layout: &Layout) -> Option<i64> {
        // We ignore and percent stuff for now, just focusx on x, y col, and row.

        let &vp = &self.content.viewport;

        let x_end = vp.x + vp.columns.min(layout.viewport_column_count_floor());

        (column >= vp.x && column < x_end && !(column == vp.x && vp.percentage_x > 0.0))
            .then(|| column - vp.x)
    }

    fn handle_scroll_result<R>(
        &self,
        state: &mut State<R>,
        shell: &mut Shell<'_, Message>,
        result: ScrollAreaResult,
        layout: &Layout,
        x_viewport: ScrollViewport,
        y_viewport: ScrollViewport
    ) -> Option<ScrollOffset>
    where
        R: text::Renderer<Font = Font> + 'static,
        R::Paragraph: Clone,
    {
        let horizontal_track = |
            kind: mouse::click::Kind,
            side: TrackSide,
            offset: i64,
        | {
            if kind == mouse::click::Kind::Double {
                offset
            } else {
                let page = x_viewport.viewport_steps_floor();
                match side {
                    TrackSide::Before => {
                        x_viewport - page
                    }
                    TrackSide::After => {
                        x_viewport + page
                    }
                }
            }
        };

        let vertical_track = |
            kind: mouse::click::Kind,
            side: TrackSide,
            offset: i64,
        | {
            if kind == mouse::click::Kind::Double {
                offset
            } else {
                let page = layout.viewport_row_count_floor();
                match side {
                    TrackSide::Before => {
                        (y_viewport.offset - page).max(0)
                    }
                    TrackSide::After => {
                        (y_viewport.offset + page).min(y_viewport.virtual_max_offset())
                    }
                }
            }
        };

        let mut track_held = |
            side: TrackSide,
            offset: i64,
            viewport: ScrollViewport,
            f: Box<dyn FnOnce() -> Option<ScrollOffset>>,
        | {
            let mut result = None;
            if let Some(last_track_scroll) = &mut state.track_timer {
                let now = Instant::now();
                let (finished, _) = last_track_scroll.test(&now);

                if side == TrackSide::Before && offset < viewport.offset
                    || side == TrackSide::After && offset > viewport.offset
                {
                    if finished {
                        last_track_scroll.set_at_interval(&now);
                        result = f();
                    }
                    shell.request_redraw_at(last_track_scroll.target());
                }
            }

            result
        };

        match result {
            ScrollAreaResult::Horizontal(result) => {
                match result {
                    ScrollResult::ThumbDragged(offset) => {
                        shell.request_redraw();
                        Some(ScrollOffset::new(offset, y_viewport.offset))
                    }
                    ScrollResult::TrackClicked(kind, side, offset) => {
                        shell.request_redraw();
                        state.track_timer = Some(Timer::new(Instant::now(), 100));
                        let x = horizontal_track(kind, side, offset);
                        Some(ScrollOffset::new(x, y_viewport.offset))
                    }
                    ScrollResult::TrackHeld(kind, side, offset) => {
                        track_held(side, offset, x_viewport, Box::new(|| {
                            let x = horizontal_track(kind, side, offset);
                            Some(ScrollOffset::new(x, y_viewport.offset))
                        }))
                    }
                    ScrollResult::ThumbGrabbed(_)
                    | ScrollResult::AppearanceChanged => {
                        shell.request_redraw();
                        None
                    }
                    ScrollResult::None => {
                        None
                    }
                }
            }
            ScrollAreaResult::Vertical(result) => {
                match result {
                    ScrollResult::ThumbDragged(offset) => {
                        shell.request_redraw();
                        Some(ScrollOffset::new(x_viewport.offset, offset))
                    }
                    ScrollResult::TrackClicked(kind, side, offset) => {
                        shell.request_redraw();
                        state.track_timer = Some(Timer::new(Instant::now(), 100));
                        let y = vertical_track(kind, side, offset);
                        Some(ScrollOffset::new(x_viewport.offset, y))
                    }
                    ScrollResult::TrackHeld(kind, side, offset) => {
                        track_held(side, offset, y_viewport, Box::new(|| {
                            let y = vertical_track(kind, side, offset);
                            Some(ScrollOffset::new(x_viewport.offset, y))
                        }))
                    }
                    ScrollResult::ThumbGrabbed(_)
                    | ScrollResult::AppearanceChanged => {
                        shell.request_redraw();
                        None
                    }
                    ScrollResult::None => {
                        None
                    }
                }
            }
            ScrollAreaResult::WheelScroll{x, y } => {
                shell.request_redraw();
                Some(ScrollOffset::new(x, y))
            }
            ScrollAreaResult::None => {
                None
            }
        }
    }

    fn check_state<R>(
        &mut self,
        state: &mut State<R>,
        shell: &mut Shell<'_, Message>,
        metrics: HexMetrics,
        bounds: Rectangle) -> Layout
    where
        R: text::Renderer<Font = Font> + 'static,
        R::Paragraph: Clone,
    {
        // If we used to horizontally step pixel-wise, but we just switched to cell-wise, drop any
        // additional sub-cell offset.
        let percentage_x = if self.horizontal_step == Step::Pixel {
            self.content.viewport.percentage_x
        } else {
            0.0
        };

        let layout = self.create_layout(metrics, bounds, percentage_x);

        let scroll_offset = ScrollOffset::new(
            self.x_viewport(&layout).fitted_scroll_offset(),
            self.y_viewport(&layout).fitted_scroll_offset(),
        );

        let viewport = self.create_viewport_from_scroll_offset(&layout, scroll_offset);

        if viewport != self.content.viewport
            && Some((viewport, self.content.id)) != state.last_reported_viewport
            && let Some(func) = &self.on_logical_viewport_size_changed
        {
            let message = (func)(viewport);
            shell.publish(message);
            shell.request_redraw();
            state.last_reported_viewport = Some((viewport, self.content.id));
        }

        layout
    }

    fn publish_scrolled<R>(
        &mut self,
        state: &mut State<R>,
        shell: &mut Shell<'_, Message>,
        viewport: Viewport)
    where
        R: text::Renderer<Font = Font> + 'static,
        R::Paragraph: Clone,
    {
        if let Some(on_scrolled) = &self.on_scrolled
            && viewport != self.content.viewport
            && Some((viewport, self.content.id)) != state.last_reported_viewport
        {
            let message = (on_scrolled)(viewport);
            shell.publish(message);
            shell.request_redraw();
            state.last_reported_viewport = Some((viewport, self.content.id));
        };
    }

    fn publish_on_selection<R>(
        &self,
        state: &mut State<R>,
        shell: &mut Shell<'_, Message>,
        selection: Option<Selection>)
    where
        R: text::Renderer<Font = Font> + 'static,
        R::Paragraph: Clone,
    {
        if state.last_reported_selection != selection {
            if let Some(func) = &self.on_selection {
                let message = (func)(selection);
                shell.publish(message);
                shell.request_redraw();
            }
            state.last_reported_selection = selection;
        }
    }

    fn publish_cursor_moved(
        &self,
        shell: &mut Shell<'_, Message>,
        cursor: i64)
    {
        if let Some(on_cursor_moved) = &self.on_cursor_moved {
            let message = (on_cursor_moved)(cursor as u64);
            shell.publish(message);
            shell.capture_event();
            shell.request_redraw();
        }
    }
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer> for HexViewer<'a, Message, Theme>
where
    Renderer: text::Renderer<Font = Font> + 'static,
    Renderer::Paragraph: Clone,
    Theme: Catalog,
{
    fn size(&self) -> Size<Length> {
        Size::new(self.width, self.height)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &Limits,
    ) -> layout::Node {
        let state = tree.state.downcast_mut::<State<Renderer>>();

        state.text_cache.set(&self.font, self.font_size, renderer);
        let metrics = state.text_cache.metrics();
        let dim = self.create_layout_dimensions(metrics, Size::INFINITE).0;

        layout::Node::new(limits.resolve(dim.width(), dim.height(), Size::ZERO))
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: layout::Layout<'_>,
        _cursor: Cursor,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State<Renderer>>();

        let bounds = layout.bounds();

        let metrics = state.text_cache.metrics();
        let layout = self.create_layout(metrics, bounds, self.content.viewport.percentage_x);
        
        let style = theme.style(&self.class, Status::Active);

        let x_viewport = self.x_viewport(&layout);
        let y_viewport = self.y_viewport(&layout);

        // Draw background of all headers.
        renderer.fill_quad(
            Quad {
                bounds: layout.headers_background(),
                ..Quad::default()
            },
            style.header_background
        );

        // Draw the background of the address area.
        renderer.fill_quad(
            Quad {
                bounds: layout.address_area,
                ..Quad::default()
            },
            style.header_background
        );

        // Draw the byte area headers.
        renderer.with_layer(layout.byte_area_header, |renderer| {
            if let Some(hovered_column) = state.hovered_column {
                renderer.fill_quad(
                    Quad {
                        bounds: layout.byte_header_cell(hovered_column),
                        ..Quad::default()
                    },
                    style.header_hover
                );
            }

            for col in 0 .. self.content.viewport.columns {
                let col_val = (self.content.viewport.x + col) % 256;

                let paragraph = if col_val < 0x10 {
                    state.text_cache.hex_digit(col_val as u8).raw()
                } else {
                    state.text_cache.byte(col_val as u8).raw()
                };

                renderer.fill_paragraph(
                    paragraph,
                    layout.byte_header_text_position(col, col_val),
                    style.header_text,
                    layout.byte_area_header
                );
            }
        });

        // Draw the char area headers.
        renderer.with_layer(layout.char_area_header, |renderer| {
            if let Some(hovered_column) = state.hovered_column {
                renderer.fill_quad(
                    Quad {
                        bounds: layout.char_header_cell(hovered_column),
                        ..Quad::default()
                    },
                    style.header_hover
                );
            }

            for col in 0 .. self.content.viewport.columns {
                // We only have space for one char, so we draw just the last hex digit.
                let col_val = (self.content.viewport.x + col) % 16;

                renderer.fill_paragraph(
                    state.text_cache.hex_digit(col_val as u8).raw(),
                    layout.char_header_text_position(col),
                    style.header_text,
                    layout.char_area_header
                );
            }
        });

        // Draw the address area.
        renderer.with_layer(layout.address_area, |renderer| {
            if let Some(hovered_row) = state.hovered_row
                && y_viewport.offset + hovered_row < y_viewport.size
            {
                renderer.fill_quad(
                    Quad {
                        bounds: layout.address_area_cell(hovered_row),
                        ..Quad::default()
                    },
                    style.header_hover
                );
            }
            let first_address = self.content.viewport.y * self.virtual_columns;
            let fill = self.address_area_horizontal_char_count();
            let content_bounds = layout.address_area_content();

            for row in 0..self.content.viewport.rows {
                let address = first_address + row * self.virtual_columns;
                let address_str = format!("{:0fill$X}", address, fill = fill);

                for (char_num, char_value) in address_str.chars().enumerate() {
                    renderer.fill_paragraph(
                        state.text_cache.char(char_value as u8).raw(),
                        layout.address_area_digit_position(char_num as i64, row),
                        style.header_text,
                        content_bounds
                    );
                }
            }
        });

        // Closure to draw the byte area and char area
        let mut draw_content = |
            bounds: Rectangle,
            content_bounds: Rectangle,
            cell: fn(&Layout, col: i64, row: i64) -> Rectangle,
            text_position: fn(&Layout, col: i64, row: i64) -> Point,
            paragraph: fn(&TextCache<Renderer>, u8) -> &text::paragraph::Plain<Renderer::Paragraph>|{

            // Draw background of the content area.
            renderer.fill_quad(
                Quad {
                    bounds,
                    ..Quad::default()
                },
                style.background
            );

            renderer.start_layer(content_bounds);

            // Draw the bytes/chars.
            for item in self.content.iter() {
                if let Some(styler) = self.content_styler
                    && let Some(color) = styler.background_color(item.viewport_offset as usize)
                {
                    renderer.fill_quad(
                        Quad {
                            bounds: cell(&layout, item.column, item.row),
                            ..Quad::default()
                        },
                        color,
                    )
                }

                let color = if let Some(styler) = self.content_styler {

                    styler.text_color(item.viewport_offset as usize).unwrap_or(style.text)
                } else {
                    style.text
                };

                renderer.fill_paragraph(
                    paragraph(&state.text_cache, item.value).raw(),
                    text_position(&layout, item.column, item.row),
                    color,
                    content_bounds
                );
            };

            // Draw the cursor
            if let Some((col, row)) = self.offset_in_viewport( self.cursor) {
                let quad = Quad {
                    bounds: cell(&layout, col, row),
                    border: Border {
                        color: style.text,
                        width: 1.0,
                        ..Border::default()
                    },
                    ..Quad::default()
                };

                renderer.fill_quad(
                    quad,
                    Color::TRANSPARENT,
                )
            }

            renderer.end_layer();
        };

        if self.content.viewport.virtual_columns != 0 {
            // Draw the entire byte area.
            draw_content(
                layout.byte_area,
                layout.byte_area_content(),
                Layout::byte_cell,
                Layout::byte_text_position,
                TextCache::<Renderer>::byte,
            );

            // Draw the entire char area.
            draw_content(
                layout.char_area,
                layout.char_area_content(),
                Layout::char_cell,
                Layout::char_text_position,
                TextCache::<Renderer>::char,
            );
        }

        // The scrollbars are drawn next to the content as opposed to hovering over it (and
        // therefore obstructing it), but this might become configurable in the future. Either way
        // it makes most sense draw the scrollbars last.
        self.scroll_area.draw(
            renderer,
            theme,
            layout.scroll_area_bounds(),
            Some(x_viewport),
            Some(y_viewport),
        );

        // Draw a border around the widget.
        renderer.fill_quad(
            Quad {
                bounds,
                border: style.border,
                ..Quad::default()
            },
            Color::TRANSPARENT,
        );
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State<Renderer>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::<Renderer>::new())
    }

    // We assume this may get called multiple times in between two HexViewer::update() calls
    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: layout::Layout<'_>,
        cursor: Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<State<Renderer>>();

        let bounds = layout.bounds();
        let cursor_over_abs = cursor.position_over(bounds);
        let metrics = state.text_cache.metrics();

        let layout = self.check_state(state, shell, metrics, bounds);
        let x_viewport = self.x_viewport(&layout);
        let y_viewport = self.y_viewport(&layout);

        let result = self.scroll_area.update(
            &mut state.scroll_area_state,
            event,
            layout.scroll_area_bounds(),
            Some(x_viewport),
            Some(y_viewport),
            cursor,
        );

        if let Some(scroll_offset) = self.handle_scroll_result(
            state, shell, result, &layout, x_viewport, y_viewport)
        {
            self.publish_scrolled(
                state, shell, self.create_viewport_from_scroll_offset(&layout, scroll_offset));
            return;
        }

        // The event wasn't handled by ScrollArea; do our own processing.
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(mouse_pos) = cursor_over_abs {
                    state.focussed = true;

                    let location = layout.pointer_location(mouse_pos);

                    // Handle a cell being clicked, or close to it.
                    if let Some(index) = self.index(&layout, location) {

                        // If shift is held we try to continue a previously created selection, from
                        // its starting point.
                        if state.keyboard_modifiers.shift() {
                            let start = state.start_index_or_set(self.cursor);

                            self.publish_on_selection(
                                state,
                                shell,
                                self.selection(start, index, index.offset)
                            );
                        } else {
                            if index.offset != self.cursor {
                                self.publish_cursor_moved(shell, index.offset);
                            }

                            self.cursor = index.offset;

                            // Start a drag interaction, even though the user may not intend to
                            // drag. We'll cancel the drag later in that case.
                            state.start_index = Some(index);
                        }

                        state.dragging = true;
                    }
                } else {
                    // We lose focus if the button is pressed anywhere outside our widget, but
                    // within the bounds of the drawable area of the main window.
                    if cursor.position_over(*_viewport).is_some() {
                        state.focussed = false;
                    }
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                // Note that we're not resetting state.start_index here, that's on purpose: if we were
                // actually dragging a selection we want to preserve where we started in case we
                // want to continue using the SHIFT button. Even if there was just a click, we'll
                // store the side of the byte/char the click happened, for now. This will
                // influence the offset at which the SHIFT aided selection will start. May change it
                // later if necessary.
                state.dragging = false;
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if let Some(mouse_pos) = cursor_over_abs {
                    let location = layout.pointer_location(mouse_pos);

                    if state.dragging
                        && let Some(selection) = state.start_index
                        && let Some(loc) = self.index(&layout, location)
                    {
                        self.publish_on_selection(
                            state, shell, self.selection(selection, loc, loc.offset));
                    }

                    let column = location.column();
                    if column != state.hovered_column {
                        state.hovered_column = column;
                        shell.request_redraw();
                    }

                    let row = location.row();
                    if row != state.hovered_row {
                        state.hovered_row = row;
                        shell.request_redraw();
                    }
                }
            }
            Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) => {
                if !state.focussed {
                    return;
                }

                let maybe_new_cursor = match key.as_ref() {
                    keyboard::Key::Named(key::Named::ArrowLeft) => {
                        self.move_cursor_left()
                    }
                    keyboard::Key::Named(key::Named::ArrowRight) => {
                        self.move_cursor_right()
                    }
                    keyboard::Key::Named(key::Named::ArrowUp) => {
                        self.move_cursor_up()
                    }
                    keyboard::Key::Named(key::Named::ArrowDown) => {
                        self.move_cursor_down()
                    }
                    keyboard::Key::Named(key::Named::PageUp) => {
                        self.move_cursor_page_up(layout.viewport_row_count_floor())
                    }
                    keyboard::Key::Named(key::Named::PageDown) => {
                        self.move_cursor_page_down(layout.viewport_row_count_floor())
                    }
                    keyboard::Key::Named(key::Named::Home) => {
                        self.move_cursor_top()
                    }
                    keyboard::Key::Named(key::Named::End) => {
                        self.move_cursor_bottom()
                    }
                    _ => {
                        // Hitting the escape key cancels the selection without the need for moving
                        // the cursor.
                        if matches!(key, keyboard::Key::Named(key::Named::Escape)) {
                            state.start_index = None;

                            self.publish_on_selection(state, shell, None);
                        }

                        // Any other keys are simply ignored.
                        return
                    }
                };

                // Check whether we're creating/modifying a selection by keyboard.
                if modifiers.shift() {
                    if let Some(new_cursor) = maybe_new_cursor {
                        let selection = state.start_index_or_set(self.cursor);
                        let new_index = Index::new(new_cursor, Side::None);

                        self.publish_on_selection(
                            state, shell, self.selection(selection, new_index, new_cursor));

                        self.cursor = new_cursor;
                    }
                } else if let Some(new_cursor) = maybe_new_cursor {
                        state.start_index = None;
                        self.publish_cursor_moved(shell, new_cursor);
                        self.cursor = new_cursor;
                } else {
                    // Applies when the cursor is alread at the start/end of the document and
                    // can't be moved further, yet a movement key was pressed without shift.
                    state.start_index = None;
                    self.publish_on_selection(state, shell, None);
                }

                let get_scroll = |navigation: Navigation| {
                    match navigation {
                        Navigation::Lazy => {
                            if matches!(key.as_ref(),
                                keyboard::Key::Named(key::Named::ArrowLeft)
                                | keyboard::Key::Named(key::Named::ArrowUp)
                                | keyboard::Key::Named(key::Named::PageUp))
                            {
                                Scroll::Lazy(LazyAlignment::Start)
                            } else {
                                Scroll::Lazy(LazyAlignment::End)
                            }
                        }
                        Navigation::Aligned(alignment) => {
                            Scroll::Aligned(alignment)
                        }
                    }
                };

                if let Some(viewport) = self.scroll_viewport(
                    self.cursor,
                    &layout,
                    get_scroll(self.horizontal_navigation),
                    get_scroll(self.vertical_navigation),
                ) {
                    self.publish_scrolled(state, shell, viewport);
                }
            }
            Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
                state.keyboard_modifiers = *modifiers;
            }
            _ => {}
        }
    }
}

/// The content that is displayed and interacted with by the [`HexViewer`].
///
/// This should be instantiated and stored in the application's state, and passed to `HexViewer` in
/// the application's view method. Two of `HexViewer`'s messages may give reason for your
/// application to call [`Content::update`]:
/// - [`HexViewer::on_scrolled`] requests the viewport to be moved, which means `Content` needs to be
///   updated.
/// - [`HexViewer::on_logical_viewport_resized`] notifies that the `HexViewer`'s viewport has
///   resized, which means the number of columns and/or rows that can be displayed has changed, and
///   `Content` needs to be updated.
// /// new viewport and reads the corresponding data.
#[derive(Debug)]
pub struct Content {
    source: Box<dyn Source>,
    source_size: i64,
    data: Vec<u8>,
    viewport: Viewport,
    id: u64,
}

impl Default for Content {
    fn default() -> Self {
        Self::new(Empty::default())
    }
}

impl Content {
    /// Creates a new `Content`.
    pub fn new<S: Source + 'static>(mut source: S) -> Self {
        let source_size = source.size() as i64;

        Self {
            source: Box::new(source),
            source_size,
            data: vec![],
            viewport: Viewport::default(),
            id: CONTENT_COUNTER.fetch_add(1, atomic::Ordering::SeqCst)
        }
    }

    /// Updates the contents based on the [`Viewport`].
    pub fn update(&mut self, viewport: Viewport) {
        self.viewport = viewport;
        if self.viewport.virtual_columns == 0 {
            return;
        }

        self.source_size = self.source.size() as i64;

        if self.data.len() != viewport.size() {
            self.data.resize(viewport.size(), 0);
        }

        for r in 0..viewport.rows {
            let source_offset = (viewport.y + r) * viewport.virtual_columns + viewport.x;

            let dst_offset = r * viewport.columns;
            let dst_size = viewport.columns
                .min(self.source_size - source_offset)
                .max(0);
            let dst_end = (dst_offset + dst_size) as usize;

            if dst_size == 0 {
                break;
            }

            self.source.read(source_offset as u64, &mut self.data[dst_offset as usize..dst_end]);
        }
    }

    fn iter(&self) -> impl Iterator<Item = ContentItem> {
        if self.viewport.virtual_columns == 0 {
            panic!("Virtual column count not set");
        };

        self.data.iter().enumerate().map(move |(i, v)| {

            let row = i as i64 / self.viewport.columns;
            let col = i as i64 % self.viewport.columns;

            let offset = (self.viewport.y + row) * self.viewport.virtual_columns + self.viewport.x + col;

            ContentItem::new(offset, i as i64, col, row, *v)
        }).take_while(|item| item.offset < self.source_size)
    }
}

#[derive(Debug, Default)]
pub struct Empty {}

impl Source for Empty {
    fn read(&mut self, _: u64, _: &mut [u8]) -> usize {
        0
    }

    fn size(&mut self) -> u64 {
        0
    }
}

#[derive(Clone, Debug)]
struct ContentItem {
    offset: i64,
    viewport_offset: i64,
    column: i64,
    row: i64,
    value: u8,
}

impl ContentItem {
    fn new(offset: i64, viewport_offset: i64, column: i64, row: i64, byte: u8) -> Self {
        Self {
            offset,
            viewport_offset,
            column,
            row,
            value: byte
        }
    }
}

/// The source of [`Content`]. Must not change its size. In other words, it's expected to be a
/// static source of bytes such as a file that isn't modified as long as the `Source` is in use.
pub trait Source: Debug {
    /// Read as many bytes as necessary to fill `buf`, starting from `offset` in the source file.
    /// [`Content`]'s read pattern is to issue one read per row. Therefore one call to its
    /// [`Content::update`] method can result in a lot of very small reads. Depending on how well
    /// the OS caches the file it may be prudent to implement some form of caching in the
    /// implementation of this `Source` trait.
    /// TODO: the return type should be `Result`.
    fn read(&mut self, offset: u64, buf: &mut [u8]) -> usize;

    /// Gets the file size. `self` is mut so that the file size can be lazily loaded and cachved.
    /// TODO: the return type should be `Result`.
    fn size(&mut self) -> u64;
}

impl<'a, Message, Theme, Renderer> From<HexViewer<'a, Message, Theme>>
for Element<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Renderer: text::Renderer<Font = Font> + 'static,
    Renderer::Paragraph: Clone,
    Theme: Catalog + 'static,
{
    fn from(
        hex_viewer_widget: HexViewer<'a, Message, Theme>,
    ) -> Element<'a, Message, Theme, Renderer> {
        Self::new(hex_viewer_widget)
    }
}

pub fn hex_viewer_widget<Message, Theme>(content: &Content) -> HexViewer<'_, Message, Theme>
where
    Theme: Catalog
{
    HexViewer::new(content)
}

#[derive(Default)]
struct State<R: Renderer>
where
    R: text::Renderer<Font = Font> + 'static,
    R::Paragraph: Clone,
{
    text_cache: TextCache<R>,
    keyboard_modifiers: keyboard::Modifiers,
    /// State of the [`ScrollArea`].
    scroll_area_state: ScrollAreaState,
    /// The last reported selection.
    last_reported_selection: Option<Selection>,
    /// The last reported viewport, and the last reported-to Content.
    last_reported_viewport: Option<(Viewport, u64)>,
    /// Whether we're making a selection by left click + dragging the mouse.
    dragging: bool,
    /// Absolute start index for a current or potential selection.
    start_index: Option<Index>,
    /// Whether this widget is focussed, and should accept keyboard input.
    focussed: bool,
    /// Tracks time between scrollbar jumps when the track is being pressed, for both the horizontal
    /// and vertical scrollbar.
    track_timer: Option<Timer>,
    /// Used for highlighting the byte/char header cell above the cursor.
    hovered_column: Option<i64>,
    /// Used for highlighting the address area cell left of the cursor.
    hovered_row: Option<i64>,
}

impl<R: Renderer> State<R>
where
    R: text::Renderer<Font = Font>,
    R::Paragraph: Clone,
{
    fn new() -> Self {
        Self {
            text_cache: TextCache::new(),
            keyboard_modifiers: keyboard::Modifiers::default(),
            scroll_area_state: ScrollAreaState::default(),
            last_reported_selection: None,
            last_reported_viewport: None,
            dragging: false,
            start_index: None,
            focussed: false,
            track_timer: None,
            hovered_column: None,
            hovered_row: None,
        }
    }

    fn start_index_or_set(&mut self, cursor: i64) -> Index {
        if let Some(index) = self.start_index {
            index
        } else {
            let index = Index::new(cursor, Side::None);
            self.start_index = Some(index);
            index
        }
    }
}

/// Caches the byte and char texts.
#[derive(Default)]
struct TextCache<R: Renderer>
where
    R: text::Renderer<Font = Font> + 'static,
{
    font: Option<Font>,
    font_size: Option<Pixels>,
    uninitialized: bool,
    byte_paragraphs: Vec<text::paragraph::Plain<R::Paragraph>>,
    char_paragraphs: Vec<text::paragraph::Plain<R::Paragraph>>,
}

impl<R: Renderer> TextCache<R>
where
    R: text::Renderer<Font = Font>,
    R::Paragraph: Clone + Default,
{
    fn new() -> Self {
        Self {
            font: None,
            font_size: None,
            uninitialized: true,
            byte_paragraphs: vec![Default::default(); 256],
            char_paragraphs: vec![Default::default(); 256],
        }
    }

    fn set(&mut self, font: &Option<Font>, font_size: Option<Pixels>, renderer: &R) {
        // self.uninitialize is necessary because if we're given only None's then no initialization
        // will ever happen.
        if self.uninitialized || self.font != *font || self.font_size != font_size {
            self.font = *font;
            self.font_size = font_size;

            let font = self.font.unwrap_or(Font::MONOSPACE);
            let font_size = self.font_size.unwrap_or_else(|| renderer.default_size());

            for (byte, paragraph) in self.byte_paragraphs.iter_mut().enumerate() {
                let byte_string = format!("{:02X}", byte);
                let text = Self::create_text(byte_string, &font, font_size);
                paragraph.update(text.as_ref());
            }

            for (byte, paragraph) in self.char_paragraphs.iter_mut().enumerate() {
                //let byte_string = format!("{:02X}", byte);

                let byte_string = Self::byte_to_decoded_char(byte as u8);
                let text = Self::create_text(byte_string, &font, font_size);
                paragraph.update(text.as_ref());
            }

            self.uninitialized = false;
        }
    }

    /// Gets the cached paragraph for a byte value, ready for drawing.
    fn byte(&self, byte: u8) -> &text::paragraph::Plain<R::Paragraph> {
        &self.byte_paragraphs[byte as usize]
    }

    /// Gets the cached paragraph for a char value in the current encoding, ready for drawing.
    fn char(&self, byte: u8) -> &text::paragraph::Plain<R::Paragraph> {
        &self.char_paragraphs[byte as usize]
    }

    /// Gets the cached paragraph for a hex digit value (0-F), ready for drawing.
    fn hex_digit(&self, hex_digit: u8) -> &text::paragraph::Plain<R::Paragraph> {
        if hex_digit <= 9 {
            &self.char_paragraphs[(hex_digit + 0x30) as usize]
        } else if (0xA..0x10).contains(&hex_digit) {
            &self.char_paragraphs[(hex_digit + 0x37) as usize]
        } else {
            panic!("hex digit out of range");
        }
    }

    /// The width of rendered bytes (e.g. "00") and rendered characters (e.g. "0"), and their height
    fn metrics(&self) -> HexMetrics {
        let byte_size = self.byte_paragraphs[0].min_bounds();
        let char_size = self.char_paragraphs[0].min_bounds();

        HexMetrics::new(
            byte_size.width,
            char_size.width,
            char_size.height,
        )
    }

    fn create_text(text: String, font: &Font, font_size: Pixels) -> Text {
        Text {
            content: text,
            bounds: Size::INFINITE,
            size: font_size,
            line_height: text::LineHeight::Relative(1.0),
            font: *font,
            align_x: text::Alignment::Left,
            align_y: alignment::Vertical::Center,
            shaping: text::Shaping::Basic, // Barely makes a difference with Basic in terms of processing performance
            wrapping: Wrapping::None,
        }
    }

    fn byte_to_decoded_char(byte: u8) -> String {
        if (0x20..0x80).contains(&byte) {
            let b = byte.to_le_bytes();
            let (cow, _, had_errors) = encoding_rs::WINDOWS_1252.decode(&b);
            if !had_errors {
                cow.to_string()
            } else {
                String::from(".")
            }
        } else {
            String::from(".")
        }
    }
}

/// The amount of space the byte and char paragraphs occupy.
#[derive(Clone, Copy, Debug, Default)]
struct HexMetrics {
    byte_width: f32,
    char_width: f32,
    height: f32,
}

impl HexMetrics {
    fn new(byte_width: f32, char_width: f32, height: f32) -> Self {
        HexMetrics {
            byte_width,
            char_width,
            height,
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Alignment {
    Start,
    Center,
    End,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum Scroll {
    Lazy(LazyAlignment),
    Aligned(Alignment),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum LazyAlignment {
    Start,
    End,
}

/// How movement of the cursor should affect the viewport.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Navigation {
    // TODO: maybe add an `Ignore` variant that makes the viewport ignore cursor movement.
    /// The viewport should move as little as possible, as long as it contains the new cursor
    /// position. That means that if the new cursor position is visible in the current viewport, the
    /// viewport remains the same. If the new cursor position is not currently visible, the viewport
    /// moves as little as needed to bring it into view.
    Lazy,
    /// The cursor should always be aligned to a certain place in the viewport, denoted by the
    /// [`Alignment`].
    Aligned(Alignment),
}

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq)]
struct ScrollOffset {
    pub x: i64,
    pub y: i64,
}

impl ScrollOffset {
    pub fn new(x: i64, y: i64) -> Self {
        ScrollOffset { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    /// The first column in our viewport. In case of Step::Pixel this column might be only partially
    /// visible.
    x: i64,
    /// The first row in our viewport. Always visible as long as the viewport is high enough to fit
    /// at least one row, since vertical scrolling always happens per row.
    y: i64,
    /// The number of columns (partially) visible.
    columns: i64,
    /// The number of rows (partially) visible.
    rows: i64,
    /// Percentage of a cell we're scrolled beyond our x. Always 0 in case of Step::Cell.
    percentage_x: f32,
    virtual_columns: i64,
}

impl Default for Viewport {
    fn default() -> Self {
        Viewport {
            x: 0,
            y: 0,
            columns: 0,
            rows: 0,
            percentage_x: 0.0,
            virtual_columns: 0
        }
    }
}

impl Viewport {
    /// The first column that is visible in the viewport.
    pub fn x(&self) -> u64 {
        self.x as u64
    }

    /// The first row that is visible in the viewport.
    pub fn y(&self) -> u64 {
        self.y as u64
    }

    /// The number of columns that would (partially) fit in the viewport.
    pub fn columns(&self) -> u64 {
        self.columns as u64
    }

    /// The number of rows that would (partially) fit in the viewport.
    pub fn rows(&self) -> u64 {
        self.rows as u64
    }

    /// The absolute offset of the byte in the top left corner of the viewport.
    pub fn offset(&self) -> u64 {
        (self.virtual_columns * self.y + self.x) as u64
    }

    /// Total number of bytes that would (partially) fit in the viewport.
    pub fn size(&self) -> usize {
        (self.columns * self.rows) as usize
    }

    /// Iterator that yields the absolute start and end (not inclusive) offsets of each row.
    /// With x=2, y=1, columns=8 and virtual_columns = 16, this would yield:
    ///   [18, 26),
    ///   [34, 42).
    ///   ...
    pub fn iter_rows(&self) -> impl Iterator<Item = Range<u64>> {
        (0..self.rows).into_iter()
            .map(|row| {
                let start = (self.y + row) * self.virtual_columns + self.x;
                let end = start + self.columns;
                Range {start: start as u64, end: end as u64}
            })
    }

    /// Determines if, and if so, at which column and row in the viewport, the absolute `offset`
    /// into the source is visible.
    pub fn contains(&self, offset: u64) -> Option<(u64, u64)> {
        let col = offset as i64 % self.virtual_columns;
        let row = offset as i64 / self.virtual_columns;

        if col < self.x || col >= self.x + self.columns
            || row < self.y || row >= self.y + self.rows
        {
            None
        } else {
            Some(
                (
                     (col - self.x) as u64,
                     (row - self.y) as u64
                )
            )
        }
    }
}

/// Contains all paddings for the [`HexViewer`] relative to the font size.
#[derive(Clone, Copy, Debug)]
pub struct PaddingSettings {
    /// Padding above the text in the byte area header and char area header.
    pub header_top: f32,
    /// Padding below the text in the byte area header and char area header.
    pub header_bottom: f32,
    /// Padding above the data cells in the byte area and char area.
    pub content_top: f32,
    /// Padding below the data cells in the byte area and char area.
    pub content_bottom: f32,
    /// Padding left of the cells in the address area.
    pub address_area_left: f32,
    /// Padding right of the cells in the address area.
    pub address_area_right: f32,
    /// Padding left of the cells in the byte area.
    pub byte_area_left: f32,
    /// Padding right of the cells in the byte area.
    pub byte_area_right: f32,
    /// Padding left of the cells in the char area.
    pub char_area_left: f32,
    /// Padding right of the cells in the char area.
    pub char_area_right: f32,
    /// Padding above and below the byte/char cell text.
    pub data_cell_vertical: f32,
    /// Padding left of the byte/char cell text.
    pub byte_cell_horizontal: f32,
    /// Padding right of the byte/char cell text.
    pub char_cell_horizontal: f32,
}

impl Default for PaddingSettings {
    fn default() -> Self {
        Self::compact()
    }
}

impl PaddingSettings {
    pub fn compact() -> Self {
        Self {
            header_top: 0.3,
            header_bottom: 0.3,
            // To match visual horizontal whitespace of 0.6 these should be 0.4, but 0.4 doesn't
            // look good. Maybe it has to do with the ascent/descent.
            content_top: 0.3,
            content_bottom: 0.3,
            address_area_left: 0.6,
            address_area_right: 0.6,
            byte_area_left: 0.3,
            byte_area_right: 0.3,
            char_area_left: 0.25,
            char_area_right: 0.55,
            data_cell_vertical: 0.2,
            byte_cell_horizontal: 0.3,
            char_cell_horizontal: 0.05,
        }
    }

    pub fn spacious() -> Self {
        Self {
            data_cell_vertical: 0.3,
            byte_cell_horizontal: 0.4,
            char_cell_horizontal: 0.1,
            address_area_left: 0.8,
            address_area_right: 0.8,
            byte_area_left: 0.4,
            byte_area_right: 0.4,
            char_area_left: 0.3,
            char_area_right: 0.7,
            header_top: 0.6,
            header_bottom: 0.6,
            // To match visual horizontal whitespace of 0.8 these should be 0.6, but 0.6 doesn't
            // look good. Maybe it has to do with the line height.
            content_top: 0.4,
            content_bottom: 0.4,
        }
    }

    pub fn header_top(mut self, value: f32) -> Self {
        self.header_top = value;
        self
    }

    pub fn header_bottom(mut self, value: f32) -> Self {
        self.header_bottom = value;
        self
    }

    pub fn content_top(mut self, value: f32) -> Self {
        self.content_top = value;
        self
    }

    pub fn content_bottom(mut self, value: f32) -> Self {
        self.content_bottom = value;
        self
    }

    pub fn address_area_left(mut self, value: f32) -> Self {
        self.address_area_left = value;
        self
    }

    pub fn address_area_right(mut self, value: f32) -> Self {
        self.address_area_right = value;
        self
    }

    pub fn byte_area_left(mut self, value: f32) -> Self {
        self.byte_area_left = value;
        self
    }

    pub fn byte_area_right(mut self, value: f32) -> Self {
        self.byte_area_right = value;
        self
    }

    pub fn char_area_left(mut self, value: f32) -> Self {
        self.char_area_left = value;
        self
    }

    pub fn char_area_right(mut self, value: f32) -> Self {
        self.char_area_right = value;
        self
    }

    pub fn data_vertical(mut self, value: f32) -> Self {
        self.data_cell_vertical = value;
        self
    }

    pub fn byte_horizontal(mut self, value: f32) -> Self {
        self.byte_cell_horizontal = value;
        self
    }

    pub fn char_horizontal(mut self, value: f32) -> Self {
        self.char_cell_horizontal = value;
        self
    }
}

/// Contains all paddings for the [`HexViewer`] in pixels.
#[derive(Clone, Copy, Debug)]
struct HexPadding {
    header_top: f32,
    header_bottom: f32,
    content_top: f32,
    content_bottom: f32,
    address_area_left: f32,
    address_area_right: f32,
    byte_area_left: f32,
    byte_area_right: f32,
    char_area_left: f32,
    char_area_right: f32,
    data_vertical: f32,
    byte_horizontal: f32,
    char_horizontal: f32,
}

impl HexPadding {
    fn new(settings: &PaddingSettings, metrics: HexMetrics) -> Self {
        let abs = |
            value: f32
        | {
            (value * metrics.height).round()  // without rounding to full pixels text doesn't always look good.
        };

        Self {
            header_top: abs(settings.header_top),
            header_bottom: abs(settings.header_bottom),
            content_top: abs(settings.content_top),
            content_bottom: abs(settings.content_bottom),
            address_area_left: abs(settings.address_area_left),
            address_area_right: abs(settings.address_area_right),
            byte_area_left: abs(settings.byte_area_left),
            byte_area_right: abs(settings.byte_area_right),
            char_area_left: abs(settings.char_area_left),
            char_area_right: abs(settings.char_area_right),
            data_vertical: abs(settings.data_cell_vertical),
            byte_horizontal: abs(settings.byte_cell_horizontal),
            char_horizontal: abs(settings.char_cell_horizontal),
        }
    }

    pub fn address_area_padding(&self) -> Padding {
        Padding::default()
            .top(self.content_top)
            .bottom(self.content_bottom)
            .left(self.address_area_left)
            .right(self.address_area_right)
    }

    pub fn byte_area_padding(&self) -> Padding {
        Padding::default()
            .top(self.content_top)
            .bottom(self.content_bottom)
            .left(self.byte_area_left)
            .right(self.byte_area_right)
    }

    pub fn char_area_padding(&self) -> Padding {
        Padding::default()
            .top(self.content_top)
            .bottom(self.content_bottom)
            .left(self.char_area_left)
            .right(self.char_area_right)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HorizontalScrollStrategy {
    Normal,
    Split
}

#[derive(Clone, Debug)]
struct Layout {
    dim: LayoutDimensions,
    padding: HexPadding,
    source_size: i64,
    virtual_columns: i64,
    metrics: HexMetrics,
    byte_cell_width: f32,
    char_cell_width: f32,
    byte_shift: f32,
    char_shift: f32,
    top_left: Rectangle,
    byte_area_header: Rectangle,
    char_area_header: Rectangle,
    top_right: Rectangle,
    address_area: Rectangle,
    byte_area: Rectangle,
    char_area: Rectangle,
}

impl Layout {
    fn new(
        dim: LayoutDimensions,
        padding: HexPadding,
        source_size: i64,
        virtual_columns: i64,
        metrics: HexMetrics,
        percentage_x: f32,
        bounds: Rectangle,
    ) -> Self {
        let header_height = dim.bounded_header_height(bounds.size());
        let content_height = dim.bounded_content_height(bounds.size());
        let address_area_width = dim.bounded_address_area_width(bounds.size());

        let (byte_area_width, char_area_width) = if dim.width() == bounds.width {
            (dim.byte_area_width, dim.char_area_width)
        } else {
            // Divide the available horizontal space between the byte area and char area as fairly
            // as possible. Scrolling happens based on the byte area's content width, i.e. the
            // area's width without padding. Division happens based on that first. Then, If there's
            // too little space to even fit the paddings, the space is divided based on their
            // ratios as well.
            let full_content_width = dim.bounded_content_width(bounds.size());

            let all_paddings = padding.byte_area_padding().x()
                + padding.char_area_padding().x();

            let content_width = (full_content_width - all_paddings).max(0.0);

            let byte_padding = padding.byte_area_padding().x();
            let char_padding = padding.char_area_padding().x();

            let byte_content = dim.byte_area_width - byte_padding;
            let char_content = dim.char_area_width - char_padding;

            let remaining_space = full_content_width - content_width;

            let content_ratio = byte_content / (byte_content + char_content);
            let padding_ratio = byte_padding / (byte_padding + char_padding);

            (
                content_width * content_ratio + padding_ratio * remaining_space,
                content_width * (1.0 - content_ratio) + (1.0 - padding_ratio) * remaining_space,
            )
        };

        let top_left = Rectangle::new(
            Point::new(
                bounds.x,
                bounds.y
            ),
            Size::new(address_area_width, header_height)
        );

        let byte_area_header = Rectangle::new(
            Point::new(
                top_left.x + top_left.width,
                bounds.y
            ),
            Size::new(byte_area_width, header_height)
        );

        let char_area_header = Rectangle::new(
            Point::new(
                byte_area_header.x + byte_area_header.width,
                bounds.y
            ),
            Size::new(char_area_width, header_height)
        );

        let top_right = Rectangle::new(
            Point::new(
                char_area_header.x + char_area_header.width,
                bounds.y
            ),
            Size::new(dim.vertical_scrollbar_width, header_height)
        );

        let address_area = Rectangle::new(
            Point::new(
                bounds.x,
                top_left.y + top_left.height
            ),
            Size::new(address_area_width, content_height)
        );

        let byte_area = Rectangle::new(
            Point::new(
                address_area.x + address_area.width,
                byte_area_header.y + byte_area_header.height
            ),
            Size::new(byte_area_width, content_height)
        );

        let char_area = Rectangle::new(
            Point::new(
                byte_area.x + byte_area.width,
                char_area_header.y + char_area_header.height
            ),
            Size::new(char_area_width, content_height)
        );

        let byte_cell_width = metrics.byte_width + 2.0 * padding.byte_horizontal;
        let char_cell_width = metrics.char_width + 2.0 * padding.char_horizontal;
        let byte_shift = percentage_x * byte_cell_width;
        let char_shift = percentage_x * char_cell_width;

        Layout {
            dim,
            padding,
            source_size,
            virtual_columns,
            metrics,
            byte_cell_width,
            char_cell_width,
            byte_shift,
            char_shift,
            top_left,
            byte_area_header,
            char_area_header,
            top_right,
            address_area,
            byte_area,
            char_area,
        }
    }

    fn width(&self) -> f32 {
        self.address_area.width + self.byte_area.width + self.char_area.width + self.top_right.width
    }

    fn address_area_content(&self) -> Rectangle {
        self.address_area.shrink(self.padding.address_area_padding())
    }

    fn byte_area_content(&self) -> Rectangle {
        self.byte_area.shrink(self.padding.byte_area_padding())
    }

    fn char_area_content(&self) -> Rectangle {
        self.char_area.shrink(self.padding.char_area_padding())
    }

    fn headers_background(&self) -> Rectangle {
        Rectangle::new(self.top_left.position(), Size::new(self.width(), self.top_left.height))
    }

    /// The bounding box of the byte header cell for `col`.
    fn byte_header_cell(&self, col: i64) -> Rectangle {
        Rectangle::new(
            Point::new(self.byte_cell_x_offset(col), self.byte_area_header.y),
            Size::new(self.byte_cell_width, self.byte_area_header.height)
        )
    }

    /// The top left point of the byte header text for `col`.
    fn byte_header_text_position(&self, col: i64, col_val: i64) -> Point {
        let rect = self.byte_header_cell(col);

        Point::new(
            rect.x + self.padding.byte_horizontal
                + (if col_val < 0x10 {self.metrics.byte_width * 0.25} else {0.0}),
            rect.y + self.padding.header_top
        )
    }

    /// The bounding box of the char header cell for `col`.
    fn char_header_cell(&self, col: i64) -> Rectangle {
        Rectangle::new(
            Point::new(self.char_cell_x_offset(col), self.char_area_header.y),
            Size::new(self.char_cell_width, self.char_area_header.height)
        )
    }

    /// The top left point of the char header text for `col`.
    fn char_header_text_position(&self, col: i64) -> Point {
        let rect = self.char_header_cell(col);

        Point::new(
            rect.x + self.padding.char_horizontal,
            rect.y + self.padding.header_top
        )
    }

    /// The bounding box of the address area cell for `row`.
    fn address_area_cell(&self, row: i64) -> Rectangle {
        Rectangle::new(
            Point::new(self.address_area.x, self.cell_y_offset(row)),
            Size::new(self.address_area.width, self.row_height())
        )
    }

    /// The top left point of the address area's col'nth digit, for `row`.
    fn address_area_digit_position(&self, col: i64, row: i64) -> Point {
        let rect = self.address_area_cell(row);

        Point::new(
            rect.x
                + self.padding.address_area_left
                + col as f32 * self.metrics.char_width,
            rect.y
                + self.padding.data_vertical
        )
    }

    /// Calculates the bounding box for the byte cell. `col` and `row` are relative to the current
    /// viewport. The position of the bounding box is absolute.
    fn byte_cell(&self, col: i64, row: i64) -> Rectangle {
        Rectangle::new(
            Point::new(self.byte_cell_x_offset(col), self.cell_y_offset(row)),
            Size::new(
                self.metrics.byte_width + 2.0 * self.padding.byte_horizontal,
                self.row_height(),
            )
        )
    }

    /// Calculates the bounding box for the byte text. `col` and `row` are relative to the current
    /// viewport. The position of the bounding box is absolute.
    fn byte_text_position(&self, col: i64, row: i64) -> Point {
        let rect = self.byte_cell(col, row);

        Point::new(
            rect.x + self.padding.byte_horizontal,
            rect.y + self.padding.data_vertical
        )
    }

    /// Calculates the bounding box for the char cell. `col` and `row` are relative to the current
    /// viewport. The position of the bounding box is absolute.
    fn char_cell(&self, col: i64, row: i64) -> Rectangle {
        Rectangle::new(
            Point::new(self.char_cell_x_offset(col), self.cell_y_offset(row)),
            Size::new(
                self.metrics.char_width + 2.0 * self.padding.char_horizontal,
                self.row_height(),
            )
        )
    }

    /// Calculates the bounding box for the char text at offset `offset`. The position of the
    /// bounding box is absolute.
    fn char_text_position(&self, col: i64, row: i64) -> Point {
        let rect = self.char_cell(col, row);

        Point::new(
            rect.x + self.padding.char_horizontal,
            rect.y + self.padding.data_vertical
        )
    }

    /// The height of each row.
    fn row_height(&self) -> f32 {
        self.metrics.height + 2.0 * self.padding.data_vertical
    }

    fn byte_cell_x_offset(&self, col: i64) -> f32 {
        self.byte_area.x
            + col as f32 * (self.metrics.byte_width + 2.0 * self.padding.byte_horizontal)
            + self.padding.byte_area_left
            - self.byte_shift
    }

    fn char_cell_x_offset(&self, col: i64) -> f32 {
        self.char_area.x
            + col as f32 * (self.metrics.char_width + 2.0 * self.padding.char_horizontal)
            + self.padding.char_area_left
            - self.char_shift
    }

    fn cell_y_offset(&self, row: i64) -> f32 {
        self.address_area.y // Address, byte and char area all have the same y offset.
            + row as f32 * self.row_height()
            + self.padding.content_top
    }

    /// Gives the maximum number of columns that could (partially) fit in the viewport. Doesn't take
    /// current offsets into account.
    fn viewport_column_count_ceil(&self) -> i64 {
        (self.byte_area_content().width / self.byte_cell_width).ceil() as i64
    }

    /// Gives the maximum number of columns that could (partially) fit in the viewport. Doesn't take
    /// current offsets into account.
    fn viewport_column_count_floor(&self) -> i64 {
        let count = self.byte_area_content().width / self.byte_cell_width;

        if self.virtual_columns as f32 - count < 0.01 {
            self.virtual_columns
        } else {
            count.floor() as i64
        }
    }

    fn viewport_row_count_ceil(&self) -> i64 {
        (self.byte_area_content().height / self.row_height()).ceil() as i64
    }

    fn viewport_row_count_floor(&self) -> i64 {
        let count = self.byte_area_content().height / self.row_height();

        if self.virtual_rows_ceil() as f32 - count < 0.01 {
            self.virtual_rows_ceil()
        } else {
            count.floor() as i64
        }
    }

    fn virtual_rows_ceil(&self) -> i64 {
        (self.source_size + self.virtual_columns - 1) / self.virtual_columns
    }

    fn max_viewport_x_offset(&self) -> i64 {
        (self.virtual_columns - self.viewport_column_count_floor()).max(0)
    }

    fn max_viewport_y_offset(&self) -> i64 {
        (self.virtual_rows_ceil() - self.viewport_row_count_floor()).max(0)
    }

    /// The bounds that the [`ScrollArea`] has to its disposal to draw scrollbars. This is primarily
    /// cosmetic, but it also influences how much scroll rail, and therefore precision, we have.
    fn scroll_area_bounds(&self) -> Rectangle {
        Rectangle::new(
            self.address_area.position(),
            Size::new(
                self.width(),
                self.address_area.height + self.dim.horizontal_scrollbar_height
            )
        )
    }

    /// Translation the mouse pointer's location to a logical location. `point` is absolute.
    fn pointer_location(&self, point: Point) -> Location {
        if self.byte_area_header.contains(point) {
            Location::ByteHeader
        } else if self.char_area_header.contains(point) {
            Location::CharHeader
        } else if self.address_area.contains(point) {
            Location::AddressArea
        } else if self.byte_area.contains(point) {
            Location::ByteArea(self.pointer_location_in_byte_area(point))
        } else if self.char_area.contains(point) {
            Location::CharArea(self.pointer_location_in_char_area(point))
        } else {
            Location::Other
        }
    }

    /// Translation the mouse pointer's location to a logical location assuming the mouse pointer is
    /// in the byte area. `point` is absolute.
    fn pointer_location_in_byte_area(&self, point: Point) -> DataLocation {
        self.pointer_location_in_data_area(
            point,
            self.byte_area_content(),
            self.byte_cell_width,
            self.byte_shift
        )
    }

    /// Translation the mouse pointer's location to a logical location assuming the mouse pointer is
    /// in the char area. `point` is absolute.
    fn pointer_location_in_char_area(&self, point: Point) -> DataLocation {
        self.pointer_location_in_data_area(
            point,
            self.char_area_content(),
            self.char_cell_width,
            self.char_shift
        )
    }

    /// Helper function that consolidates the translation of the mouse pointer's location for both
    /// the byte and char area. `point` is absolute.
    fn pointer_location_in_data_area(
        &self,
        point: Point,
        content: Rectangle,
        cell_width: f32,
        shift: f32,
    ) -> DataLocation {

        let cell_row = ((point.y - content.y) / self.row_height()).floor() as i64;

        // Click happened in a cell.
        if content.contains(point) {
            let cell_col = ((point.x - (content.x - shift)) / cell_width).floor() as i64;
            let in_cell_offset = (point.x - (content.x - shift)) % cell_width;
            let side = if in_cell_offset < cell_width / 2.0 {Side::Left} else {Side::Right};
            return DataLocation::Cell(Cell::new(cell_col, cell_row, side))
        }

        let cell_col = ((point.x - content.x) / cell_width).floor() as i64;

        // Click happened in the top padding.
        if point.y < content.y {
            if point.x < content.x {
                return DataLocation::CornerTopLeft
            } else if point.x < content.x + content.width {
                return DataLocation::PaddingTop(cell_col)
            } else {
                return DataLocation::CornerTopRight
            }
        }

        // Click happened in the bottom padding.
        if point.y > content.y + content.height {
            if point.x < content.x {
                return DataLocation::CornerBottomLeft
            } else if point.x < content.x + content.width {
                return DataLocation::PaddingBottom(cell_col)
            } else {
                return DataLocation::CornerBottomRight
            }
        }

        // Point is in the left padding.
        if point.x < content.x {
            return DataLocation::PaddingLeft(cell_row);
        }

        // Point is in the right padding.
        DataLocation::PaddingRight(cell_row)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Step {
    Cell,
    Pixel
}

impl Default for Step {
    fn default() -> Self {
        Self::Cell
    }
}

#[derive(Clone, Debug)]
struct LayoutDimensions {
    header_height: f32,
    content_height: f32,
    address_area_width: f32,
    byte_area_width: f32,
    char_area_width: f32,
    horizontal_scrollbar_height: f32,
    vertical_scrollbar_width: f32,
}

impl LayoutDimensions {
    fn new(
        settings: &HexPadding,
        columns: i64,
        metrics: HexMetrics,
        horizontal_scrollbar_height: f32,
        vertical_scrollbar_width: f32,
        source_size: i64,
        bounds_size: Size,
        height: Length,
    ) -> LayoutDimensions {
        let header_height = metrics.height
            + settings.header_top
            + settings.header_bottom;

        let virtual_rows_ceil = (source_size + columns - 1) / columns;

        let row_height = metrics.height + 2.0 * settings.data_vertical;

        let content_height = if height == Length::Shrink {
            (virtual_rows_ceil as f64 * row_height as f64
                + settings.content_top as f64
                + settings.content_bottom as f64) as f32
        } else {
            (bounds_size.height - horizontal_scrollbar_height).max(0.0)
        };

        let address_area_char_count = format!("{}", source_size).chars().count() as f32;

        let address_area_width =  address_area_char_count * metrics.char_width
            + settings.address_area_left
            + settings.address_area_right;

        let byte_area_width = columns as f32
            * (metrics.byte_width + 2.0 * settings.byte_horizontal)
            + settings.byte_area_left
            + settings.byte_area_right;

        let char_area_width = columns as f32
            * (metrics.char_width + 2.0 * settings.char_horizontal)
            + settings.char_area_left
            + settings.char_area_right;

        LayoutDimensions {
            header_height,
            content_height,
            address_area_width,
            byte_area_width,
            char_area_width,
            horizontal_scrollbar_height,
            vertical_scrollbar_width,
        }
    }

    fn width(&self) -> f32 {
        self.address_area_width + self.byte_area_width + self.char_area_width + self.vertical_scrollbar_width
    }

    fn height(&self) -> f32 {
        self.header_height + self.content_height + self.horizontal_scrollbar_height
    }

    fn content_width(&self) -> f32 {
        self.byte_area_width + self.char_area_width
    }

    fn bounded_header_height(&self, bounds: Size) -> f32 {
        self.header_height.min(bounds.height)
    }

    fn bounded_content_height(&self, bounds: Size) -> f32 {
        self.content_height.min(bounds.height - self.header_height - self.horizontal_scrollbar_height)
            .max(0.0)
    }

    fn bounded_address_area_width(&self, bounds: Size) -> f32 {
        self.address_area_width
            .min(bounds.width)
    }

    fn bounded_content_width(&self, bounds: Size) -> f32 {
        self.content_width()
            .min(bounds.width - self.address_area_width - self.vertical_scrollbar_width)
            .max(0.0)
    }
}

/// A logical location within the [`HexViewer`].
#[derive(Clone, Copy, Debug)]
enum Location {
    ByteHeader,
    CharHeader,
    AddressArea,
    ByteArea(DataLocation),
    CharArea(DataLocation),
    Other,
}

impl Location {
    /// Decides what cell we should consider "clicked" so that we can put the cursor there. If the
    /// user clicked a cell directly, the decision is obvious. But clicks in padding may result in
    /// the cursor being moved as well. The returned [`Cell`] is relative to the current viewport.
    fn approximate_cell(&self, cols: i64, rows: i64) -> Option<Cell> {
        match self {
            Location::ByteArea(location)
            | Location::CharArea(location) => {
                Some(location.approximate_cell(cols, rows))
            }
            Location::ByteHeader
            | Location::CharHeader
            | Location::AddressArea
            | Location::Other => None,
        }
    }

    /// The column this Location applies to. No approximation is done, only strict matches are
    /// returned.
    fn column(&self) -> Option<i64> {
        match self {
            Location::ByteArea(location)
            | Location::CharArea(location) => {
                location.column()
            }
            Location::ByteHeader
            | Location::CharHeader
            | Location::AddressArea
            | Location::Other => None,
        }
    }

    /// The row this Location applies to. No approximation is done, only strict matches are
    /// returned.
    fn row(&self) -> Option<i64> {
        match self {
            Location::ByteArea(location)
            | Location::CharArea(location) => {
                location.row()
            }
            Location::ByteHeader
            | Location::CharHeader
            | Location::AddressArea
            | Location::Other => None,
        }
    }
}

/// A logical location within the [`HexViewer`]'s byte/char area.
#[derive(Clone, Copy, Debug)]
enum DataLocation {
    /// A ByteLocation
    Cell(Cell),
    /// Padding left, containing the row.
    PaddingLeft(i64),
    /// Padding right, containing the row.
    PaddingRight(i64),
    /// Padding top, containing the column.
    PaddingTop(i64),
    /// Padding bottom, containing the column.
    PaddingBottom(i64),
    /// Top left corner.
    CornerTopLeft,
    /// Top right corner.
    CornerTopRight,
    /// Bottom left corner.
    CornerBottomLeft,
    /// Bottom right corner.
    CornerBottomRight,
}

impl DataLocation {
    fn approximate_cell(&self, cols: i64, rows: i64) -> Cell {
        match self {
            DataLocation::Cell(location) => {
                *location
            }
            DataLocation::PaddingLeft(row) => {
                Cell::new(0, *row, Side::Left)
            }
            DataLocation::PaddingRight(row) => {
                Cell::new(cols - 1, *row, Side::Right)
            }
            DataLocation::PaddingTop(_)
            | DataLocation::CornerTopLeft
            | DataLocation::CornerTopRight => {
                Cell::new(0, 0, Side::Left)
            }
            DataLocation::PaddingBottom(_)
            | DataLocation::CornerBottomLeft
            | DataLocation::CornerBottomRight => {
                Cell::new(0, rows, Side::Right)
            }
        }
    }

    /// The column this DataLocation applies to. No approximation is done, only strict matches are
    /// returned.
    fn column(&self) -> Option<i64> {
        match self {
            DataLocation::Cell(location) => {
                Some(location.col)
            }
            DataLocation::PaddingTop(row)
            | DataLocation::PaddingBottom(row) => {
                Some(*row)
            }
            DataLocation::PaddingLeft(_)
            | DataLocation::CornerTopLeft
            | DataLocation::CornerTopRight
            | DataLocation::PaddingRight(_)
            | DataLocation::CornerBottomLeft
            | DataLocation::CornerBottomRight => None
        }
    }

    /// The row this DataLocation applies to. No approximation is done, only strict matches are
    /// returned.
    fn row(&self) -> Option<i64> {
        match self {
            DataLocation::Cell(location) => {
                Some(location.row)
            }
            DataLocation::PaddingLeft(row)
            | DataLocation::PaddingRight(row) => {
                Some(*row)
            }
            DataLocation::PaddingTop(_)
            | DataLocation::CornerTopLeft
            | DataLocation::CornerTopRight
            | DataLocation::PaddingBottom(_)
            | DataLocation::CornerBottomLeft
            | DataLocation::CornerBottomRight => None
        }
    }
}

/// Describes the location of a cell relative to the current viewport, and a side within that cell.
/// The cell in the top left corner has col 0 and row 0.
#[derive(Clone, Copy, Debug)]
struct Cell {
    /// Column in the viewport.
    col: i64,
    /// Row in the viewport.
    row: i64,
    /// Side of the cell that was clicked, meaning left from center or right from center. Side::None
    /// when the side isn't chosen, or is irrelevant.
    side: Side,
}

impl Cell {
    fn new(col: i64, row: i64, side: Side) -> Self {
        Self {col, row, side}
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd)]
enum Side {
    Left = 0,
    None = 1,
    Right = 2,
}

/// The counterpart of [`Cell`], which stores an absolute offset into the source, and the same
/// side information as the [`Cell`]
#[derive(Clone, Copy, Debug, PartialEq)]
struct Index {
    offset: i64,
    side: Side,
}

impl PartialOrd for Index {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.offset < other.offset {
            Some(Ordering::Less)
        } else if self.offset > other.offset {
            Some(Ordering::Greater)
        } else {
            self.side.partial_cmp(&other.side)
        }
    }
}

impl Index {
    fn new(offset: i64, side: Side) -> Self {
        Self {
            offset,
            side
        }
    }
}

/// Holds the selection info, made for instance by mouse or by holding shift and using the arrow
/// buttons.
///
/// The byte and char cells each have a `left` side and a `ride` side, which play a role in
/// selections. When a selection is started by clicking the left side of the cell at offset `2` and
/// the cursor is moved to the right side of the cell at offset `2`, this offset is now selected.
/// Continuing the movement to the left side of the cell at offset `3`, the selection doesn't
/// change. Only when the selection progresses to its right side is the cell at offset `3` selected.
///
/// This same principle may also play a role in selection made by keyboard, if the cursor at the
/// start was set by mouse, and hence side information is retained.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Selection {
    /// The leftmost byte in the selection.
    pub offset: u64,
    /// The number of bytes that are selected.
    pub length: u64,
    /// The last byte that was interacted with to create the selection. Whether this byte is part
    /// of the selection depends on whether the selection was made by keyboard or by mouse, and in
    /// the latter case, the side of it that was clicked and the direction of the selection. If you
    /// want the last contained byte, see [`Selection::last_contained`].
    pub last: u64,
}

impl Selection {
    /// Creates a new selection.
    pub fn new(offset: u64, length: u64, last: u64) -> Self {
        Self { offset, length, last }
    }

    /// The last byte that was interacted with to create the selection, that's also contained in the
    /// selection.
    pub fn last_contained(&self) -> u64 {
        if self.last < self.offset {
            self.offset
        } else if self.last >= self.offset + self.length {
            self.offset + self.length - 1
        } else {
            self.last
        }
    }
}

/// Controls the text color and background color of byte/char cells.
///
///
pub struct ContentStyler {
    styles: Vec<CellStyle>,
    is_clear: bool
}

impl Default for ContentStyler {
    fn default() -> Self {
        ContentStyler::new(0)
    }
}

impl ContentStyler {
    // TODO maybe change some return types to Result
    
    pub fn new(size: usize) -> Self {
        Self {
            styles: vec![Default::default(); size],
            is_clear: true
        }
    }

    pub fn set_text(&mut self, index: usize, color: Color) {
        if index < self.styles.len() {
            self.styles[index].text = Some(color);
        }
        self.is_clear = false;
    }

    pub fn set_background(&mut self, index: usize, background: Color) {
        if index < self.styles.len() {
            self.styles[index].background = Some(background);
        }
        self.is_clear = false;
    }

    /// Resets the ContentStyler for reuse, and makes sure it has the required `size`.
    pub fn clear(&mut self, size: usize) {
        if !self.is_clear || self.styles.len() != size {
            if self.styles.len() == size {
                self.styles.fill(Default::default());
            } else {
                self.styles = vec![Default::default(); size];
            }
            self.is_clear = true;
        }
    }

    fn text_color(&self, index: usize) -> Option<Color> {
        self.styles.get(index)?.text
    }

    fn background_color(&self, index: usize) -> Option<Color> {
        self.styles.get(index)?.background
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct CellStyle {
    text: Option<Color>,
    background: Option<Color>,
    /// This is currently a placeholder, borders aren't drawn yet.
    border: Option<CellBorder>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct CellBorder {
    border: Border,
    sides: BorderFlags
}

impl CellBorder {
    /* Unused at the moment, avoid warnings.
    pub fn new(border: Border) -> Self {
        Self {
            border,
            sides: BorderFlags::all()
        }
    }

    pub fn empty(border: Border) -> Self {
        Self {
            border,
            sides: BorderFlags::empty()
        }
    }

    pub fn top(mut self) -> Self {
        self.sides |= BorderFlags::TOP;
        self
    }

    pub fn left(mut self) -> Self {
        self.sides |= BorderFlags::LEFT;
        self
    }

    pub fn bottom(mut self) -> Self {
        self.sides |= BorderFlags::BOTTOM;
        self
    }

    pub fn right(mut self) -> Self {
        self.sides |= BorderFlags::RIGHT;
        self
    }
    */
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    struct BorderFlags: u32 {
        const TOP = 0b0001;
        const LEFT = 0b0010;
        const BOTTOM = 0b0100;
        const RIGHT = 0b1000;
    }
}

/// The possible status of a [`HexViewer`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Status {
    /// The [`TextInput`] can be interacted with.
    Active,
    /// The [`TextInput`] is being hovered.
    Hovered,
    /// The [`TextInput`] is focused.
    Focused {
        /// Whether the [`TextInput`] is hovered, while focused.
        is_hovered: bool,
    },
    /// The [`TextInput`] cannot be interacted with.
    Disabled,
}

/// The appearance of a [`HexViewer`].
#[derive(Debug, Clone, Copy)]
pub struct Style {
    /// The [`Background`] of the byte/char area.
    pub background: Background,
    /// The [`Color`] of the byte/char text.
    pub text: Color,
    /// The [`Background`] of the byte/char header area.
    pub header_background: Background,
    /// The [`Background`] of the byte/char header area when hovered.
    pub header_hover: Background,
    /// The [`Color`] of the byte/char header text.
    pub header_text: Color,
    /// The [`Border`] around the whole widget.
    pub border: Border,
}

/// The theme catalog of a [`HexViewer`].
pub trait Catalog: ScrollCatalog + Sized {
    /// The item class of the [`Catalog`].
    type Class<'a>;

    /// The default class produced by the [`Catalog`].
    fn default<'a>() -> Self::Class<'a>;

    /// The [`Style`] of a class with the given status.
    fn style(&self, class: &Self::Class<'_>, status: Status) -> Style;
}

/// A styling function for a [`HexViewer`].
///
/// This is just a boxed closure: `Fn(&Theme, Status) -> Style`.
pub type StyleFn<'a, Theme> = Box<dyn Fn(&Theme, Status) -> Style + 'a>;

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(default)
    }

    fn style(&self, class: &Self::Class<'_>, status: Status) -> Style {
        class(self, status)
    }
}

/// The default style of a [`HexViewer`].
pub fn default(theme: &Theme, status: Status) -> Style {
    let palette = theme.extended_palette();

    let active = Style {
        background: Background::Color(palette.background.base.color),
        text: palette.background.base.text,
        header_background: Background::Color(palette.background.weaker.color),
        header_hover: Background::Color(palette.background.strong.color),
        header_text: palette.background.weaker.text,
        border: Border {
            radius: 2.0.into(),
            width: 1.0,
            color: palette.background.strong.color,
        }
    };

    match status {
        Status::Active => active,
        Status::Hovered => Style {
            ..active
        },
        Status::Focused { .. } => Style {
            ..active
        },
        Status::Disabled => Style {
            background: Background::Color(palette.background.weaker.color),
            header_background: Background::Color(palette.background.strong.color),
            ..active
        },
    }
}
