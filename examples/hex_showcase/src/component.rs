use iced_data_navigator::hex::viewer::{self, ContentStyler};

use rand::prelude::*;
use iced::{Color, Element, Font, Pixels, Theme, Right};
use iced::widget::{column, container, row, text};
use iced_core::Length;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::ops::Range;
use std::path::PathBuf;

pub enum Action {
    None,
}

#[derive(Debug)]
struct Reader {
    reader: BufReader<File>,
    size: u64,
}

impl Reader {
    fn new(path: &PathBuf) -> Self {
        let file = File::open(path).unwrap();
        let mut reader = BufReader::new(file);

        let size = reader.seek(SeekFrom::End(0)).unwrap();
        reader.seek(SeekFrom::Start(0)).unwrap();

        Reader {
            reader,
            size
        }
    }
}

impl viewer::Source for Reader {
    fn read(&mut self, offset: u64, buf: &mut [u8]) -> usize {
        self.reader.seek(SeekFrom::Start(offset)).unwrap();
        self.reader.read(buf).unwrap()
    }

    fn size(&mut self) -> u64 {
        self.size
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Message {
    CursorMoved(u64),
    Scrolled(viewer::Viewport),
    LogicalViewportSizeChanged(viewer::Viewport),
    Selected(Option<viewer::Selection>),
}

pub struct HexComponent {
    theme: Theme,
    content: viewer::Content,
    viewport: viewer::Viewport,
    font: Option<Font>,
    font_size: Option<Pixels>,
    layout_settings: viewer::PaddingSettings,
    columns: u64,
    horizontal_step: viewer::Step,
    cursor: u64,
    selection: Option<viewer::Selection>,
    style: Option<viewer::Style>,
    horizontal_navigation: Option<viewer::Navigation>,
    vertical_navigation: Option<viewer::Navigation>,
    content_styler: ContentStyler,
    rng: ThreadRng,
}

impl HexComponent {
    pub fn new(theme: Theme) -> Self {
        Self {
            theme,
            content: viewer::Content::default(),
            viewport: viewer::Viewport::default(),
            font: None,
            font_size: None,
            layout_settings: viewer::PaddingSettings::default(),
            columns: 32,
            horizontal_step: viewer::Step::default(),
            cursor: 0,
            selection: None,
            style: None,
            horizontal_navigation: None,
            vertical_navigation: None,
            content_styler: ContentStyler::default(),
            rng: rand::rng(),
        }
    }

    pub fn open_file(&mut self, path: &PathBuf) {
        let reader = Reader::new(path);
        self.content = viewer::Content::new(reader);
    }

    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
        self.rebuild_content_styler_cache();
    }

    pub fn set_font(&mut self, font: Font) {
        self.font = Some(font);
    }

    pub fn set_font_size(&mut self, size: impl Into<Pixels>) {
        self.font_size = Some(size.into());
    }

    pub fn set_layout_settings(&mut self, layout_settings: viewer::PaddingSettings) {
        self.layout_settings = layout_settings;
    }

    pub fn set_columns(&mut self, columns: u64) {
        self.columns = columns;
    }

    pub fn set_horizontal_step(&mut self, step: viewer::Step) {
        self.horizontal_step = step;
    }

    pub fn set_horizontal_navigation(&mut self, navigation: viewer::Navigation) {
        self.horizontal_navigation = Some(navigation);
    }

    pub fn set_vertical_navigation(&mut self, navigation: viewer::Navigation) {
        self.vertical_navigation = Some(navigation);
    }

    pub fn random_highlight(&mut self) {
        self.clear_content_styler();

        for _ in 0..self.rng.random_range(5 .. 20) {
            let range = self.get_random_interval();
            let text_color = self.get_random_color();
            let background_color = self.get_random_color();
            for i in range {
                self.content_styler.set_text(i, text_color);
                self.content_styler.set_background(i, background_color);
            }
        }
    }

    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::CursorMoved(cursor) => {
                self.cursor = cursor;
                self.selection = None;
                self.rebuild_content_styler_cache();
            }
            Message::Scrolled(viewport) => {
                self.viewport = viewport;
                self.update_content();
                self.rebuild_content_styler_cache();
            }
            Message::LogicalViewportSizeChanged(viewport) => {
                self.viewport = viewport;
                self.update_content();
                self.rebuild_content_styler_cache();
            }
            Message::Selected(selection_maybe) => {
                self.selection = selection_maybe;
                if let Some(selection) = selection_maybe {
                    self.cursor = selection.last_contained();
                }
                self.rebuild_content_styler_cache();
            }
        }

        Action::None
    }

    pub fn view(&self) -> Element<'_, Message> {
        let mut hex_viewer = viewer::hex_viewer_widget(&self.content)
        .cursor(self.cursor)
        .on_cursor_moved(Message::CursorMoved)
        .on_scrolled(Message::Scrolled)
        .on_logical_viewport_resized(Message::LogicalViewportSizeChanged)
        .on_selection(Message::Selected)
        .font_maybe(self.font)
        .font_size_maybe(self.font_size)
        .virtual_columns(self.columns)
        .horizontal_step(self.horizontal_step)
        .padding_settings(self.layout_settings)
        .horizontal_navigation_maybe(self.horizontal_navigation)
        .vertical_navigation_maybe(self.vertical_navigation)
        .content_styler(&self.content_styler)
            .height(Length::Fill);
        
        if let Some(configured_style) = self.style {
            let style_fn = move |_: &Theme, _: viewer::Status| {
                configured_style
            };

            hex_viewer = hex_viewer.style(style_fn);
        };

        let font = Font::with_name("Fira Mono");

        let status_bar = row![
            if let Some(selection) = &self.selection {
                text!("Sel length: {:#0X} {}", selection.length, selection.length).font(font)
            } else {
                text("")
            },
            text!("Cursor: {:#0X} {}", self.cursor, self.cursor).font(font),
        ]
        .spacing(30);

        container(
            column![
                hex_viewer,
                status_bar,
            ]
            .align_x(Right)
            .spacing(10.0)
        )
        .width(Length::Fill)
        .into()
    }

    fn update_content(&mut self) {
        self.content.update(self.viewport);
    }

    fn clear_content_styler(&mut self) {
        self.content_styler.clear(self.viewport.size());
    }

    fn rebuild_content_styler_cache(&mut self) {
        self.clear_content_styler();
        // The ContentStyler only knows about the current viewport, so potentially only part of the
        // data horizontally and vertically. We need to translate the selection, which is a
        // contiguous range in absolute space to this little viewport window.
        if let Some(selection) = self.selection {
            let (text, background) = highlight_color(&self.theme);
            for (row, range) in self.viewport.iter_rows().enumerate() {
                let start = range.start.max(selection.offset);
                let end = range.end.min(selection.offset + selection.length);
                for index in start..end {
                    let index = (self.viewport.columns() * row as u64 + index - range.start) as usize;

                    self.content_styler.set_text(index, text);
                    self.content_styler.set_background(index, background);
                }
            }

        }
    }

    fn get_random_interval(&mut self) -> Range<usize> {
        let offset = self.rng.random_range(0 .. self.viewport.size().saturating_sub(50));
        let length = self.rng.random_range(0 .. 50);

        Range {start: offset, end: offset + length}
    }

    fn get_random_color(&mut self) -> Color {
        let r = self.rng.random_range(0. .. 1.);
        let g = self.rng.random_range(0. .. 1.);
        let b = self.rng.random_range(0. .. 1.);
        Color::from_rgb(r, g, b)
    }
}

fn highlight_color(theme: &Theme) -> (Color, Color) {
    (
        theme.extended_palette().primary.weak.text,
        theme.extended_palette().primary.weak.color
    )
}
