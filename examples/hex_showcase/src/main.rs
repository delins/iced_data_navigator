// Avoids a terminal showing up when double clicking the exe on Windows.
// #![cfg_attr(
//     all(not(debug_assertions), target_os = "windows"),
//     windows_subsystem = "windows"
// )]

use iced_data_navigator::hex::viewer;

use iced::{Element, Font, Function, Length, Settings, Task, Theme, Window};
use iced::{alignment, border, window};
use iced::widget::{Column, Row};
use iced::widget::{
    button, center_x, column, container, pick_list, radio, row, scrollable, slider, space, text,
    toggler, tooltip
};
use std::fmt;
use std::io;
use std::path::PathBuf;

mod component;

const DEFAULT_THEME: Theme = Theme::Dark;
const DEFAULT_FONT_SIZE: f32 = 13.0;
const DEFAULT_HEX_VIEWER_FONT: FontName = FontName::DejaVuSansMono;
const DEFAULT_HEX_VIEWER_TEXT_SIZE: f32 = 12.0;

const LAYOUT_PADDING_SLIDER_RANGE: f32 = 30.0;
const LAYOUT_SLIDER_DIVIDER: f32 = 20.0;

const PADDING_HEADER_TOP: &str = "Header top";
const PADDING_HEADER_BOTTOM: &str = "Header bottom";
const PADDING_CONTENT_TOP: &str = "Content top";
const PADDING_CONTENT_BOTTOM: &str = "Content bottom";
const PADDING_ADDRESS_AREA_LEFT: &str = "Address area left";
const PADDING_ADDRESS_AREA_RIGHT: &str = "Address area right";
const PADDING_BYTE_AREA_LEFT: &str = "Byte area left";
const PADDING_BYTE_AREA_RIGHT: &str = "Byte area right";
const PADDING_CHAR_AREA_LEFT: &str = "Char area left";
const PADDING_CHAR_AREA_RIGHT: &str = "Char area right";
const PADDING_DATA_CELL_VERTICAL: &str = "Data vertical";
const PADDING_BYTE_CELL_HORIZONTAL: &str = "Byte horizontal";
const PADDING_CHAR_CELL_HORIZONTAL: &str = "Char horizontal";

const DROID_SANS_MONO: &str = "Droid Sans Mono";
const COURIER_PRIME: &str = "Courier Prime";
const SOURCE_CODE_PRO: &str = "Source Code Pro";
const UBUNTU_MONO: &str = "Ubuntu Mono";
const DEJAVU_SANS_MONO: &str = "DejaVu Sans Mono";
const CASCADIA_CODE: &str = "Cascadia Code";
const FIRA_MONO: &str = "Fira Mono";
const ROBOTO_MONO: &str = "Roboto Mono";

#[derive(Debug, Clone, PartialEq)]
enum FontName {
    DroidSansMono,
    CourierPrime,
    SourceCodePro,
    UbuntuMono,
    DejaVuSansMono,
    CascadiaCode,
    FiraMono,
    RobotoMono,
}

impl FontName {
    pub const ALL: &'static [Self] = &[
        Self::DroidSansMono,
        Self::CourierPrime,
        Self::SourceCodePro,
        Self::UbuntuMono,
        Self::DejaVuSansMono,
        Self::CascadiaCode,
        Self::FiraMono,
        Self::RobotoMono,
    ];

    fn static_str(&self) -> &'static str {
        match self {
            Self::DroidSansMono => DROID_SANS_MONO,
            Self::CourierPrime => COURIER_PRIME,
            Self::SourceCodePro => SOURCE_CODE_PRO,
            Self::UbuntuMono => UBUNTU_MONO,
            Self::DejaVuSansMono => DEJAVU_SANS_MONO,
            Self::CascadiaCode => CASCADIA_CODE,
            Self::FiraMono => FIRA_MONO,
            Self::RobotoMono => ROBOTO_MONO,
        }
    }
}

impl fmt::Display for FontName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.static_str())
    }
}

#[derive(Clone, Debug)]
struct NavigationSetting {
    navigation: Option<NavigationOption>,
    alignment: Option<viewer::Alignment>,
}

impl Default for NavigationSetting {
    fn default() -> Self {
        Self {
            navigation: Some(NavigationOption::Lazy),
            alignment: Some(viewer::Alignment::Center),
        }
    }
}

impl NavigationSetting {
    fn as_navigation(&self) -> Option<viewer::Navigation> {
        let navigation = self.navigation?;

        match navigation {
            NavigationOption::Lazy => {
                Some(viewer::Navigation::Lazy)
            }
            NavigationOption::Aligned => {
                Some(viewer::Navigation::Aligned(self.alignment?))
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NavigationOption {
    Lazy,
    Aligned
}

pub fn main() -> iced::Result {
    iced::application(Application::default, Application::update, Application::view)
        .settings(Settings {
            default_font: Font::with_name("Fira Sans"),
            default_text_size: DEFAULT_FONT_SIZE.into(),
            ..Default::default()
        })
        .font(include_bytes!("../fonts/droid-sans-mono/DroidSansMono.ttf").as_slice())
        .font(include_bytes!("../fonts/courier_prime/CourierPrime-Regular.ttf").as_slice())
        .font(include_bytes!("../fonts/source_code_pro/SourceCodePro-Regular.ttf").as_slice())
        .font(include_bytes!("../fonts/ubuntu/UbuntuMono-Regular.ttf").as_slice())
        .font(include_bytes!("../fonts/dejavu-sans-mono/DejaVuSansMono.ttf").as_slice())
        .font(include_bytes!("../fonts/cascadia-code/Cascadia.ttf").as_slice())
        .font(include_bytes!("../fonts/Fira_Mono/FiraMono-Regular.ttf").as_slice())
        .font(include_bytes!("../fonts/Roboto_Mono/RobotoMono-Regular.ttf").as_slice())
        .font(include_bytes!("../fonts/icons.ttf").as_slice())
        .theme(Application::theme)
        .run()
}

#[derive(Debug, Clone)]
enum Message {
    ThemeChanged(Theme),
    FontChanged(FontName),
    FontSizeChanged(f32),
    ColumnsChanged(u32),
    PaddingChanged(usize, f32),
    RandomHighlightPressed,
    DebugToggled(bool),
    PresetLayoutCompact,
    PresetLayoutSpacious,
    NavigationOptionChanged(usize, NavigationOption),
    NavigationAlignedChanged(usize, viewer::Alignment),
    HorizontalStepChanged(viewer::Step),
    HexViewer(component::Message),
    OpenFile,
    FileOpened(Result<PathBuf, Error>),
}

struct Application {
    hex_viewer: component::HexComponent,
    theme: Theme,
    font: FontName,
    font_size: f32,
    columns: u32,
    padding_settings: viewer::PaddingSettings,
    debug: bool,
    navigation_settings: [NavigationSetting; 2],
    horizontal_step: Option<viewer::Step>,
    is_loading: bool,
}

impl Default for Application {
    fn default() -> Self {
        let mut application = Self {
            hex_viewer: component::HexComponent::new(DEFAULT_THEME),
            theme: DEFAULT_THEME,
            font: DEFAULT_HEX_VIEWER_FONT,
            font_size: DEFAULT_HEX_VIEWER_TEXT_SIZE,
            columns: 32,
            padding_settings: viewer::PaddingSettings::default(),
            debug: false,
            navigation_settings: [NavigationSetting::default(), NavigationSetting::default()],
            horizontal_step: Some(viewer::Step::Cell),
            is_loading: false,
        };

        application.set_font();
        application.hex_viewer.set_font_size(DEFAULT_HEX_VIEWER_TEXT_SIZE);

        application
    }
}

impl Application {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::HexViewer(message) => {
                let action = self.hex_viewer.update(message);
                
                match action {
                    component::Action::None => {}
                }
                Task::none()
            }
            Message::ThemeChanged(theme) => {
                self.theme = theme.clone();
                self.hex_viewer.set_theme(theme);
                Task::none()
            }
            Message::FontChanged(value) => {
                self.font = value;
                self.set_font();
                Task::none()
            }
            Message::FontSizeChanged(value) => {
                self.font_size = value;
                self.hex_viewer.set_font_size(value);
                Task::none()
            }
            Message::ColumnsChanged(value) => {
                self.columns = value;
                self.hex_viewer.set_columns(value as u64);
                Task::none()
            }
            Message::PaddingChanged(index, value) => {
                self.set_padding(index, value / LAYOUT_SLIDER_DIVIDER);
                self.set_padding_settings();
                Task::none()
            }
            Message::DebugToggled(value) => {
                self.debug = value;
                Task::none()
            }
            Message::PresetLayoutCompact => {
                self.set_preset_layout(viewer::PaddingSettings::compact());
                Task::none()
            }
            Message::PresetLayoutSpacious => {
                self.set_preset_layout(viewer::PaddingSettings::spacious());
                Task::none()
            }
            Message::RandomHighlightPressed => {
                self.hex_viewer.random_highlight();
                Task::none()
            }
            Message::NavigationOptionChanged(index, navigation_option) => {
                let setting = &mut self.navigation_settings[index];
                setting.navigation = Some(navigation_option);
                if let Some(navigation) = setting.as_navigation() {
                    if index == 0 {
                        self.hex_viewer.set_horizontal_navigation(navigation);
                    } else {
                        self.hex_viewer.set_vertical_navigation(navigation);
                    }
                }
                Task::none()
            }
            Message::NavigationAlignedChanged(index, alignment) => {
                let setting = &mut self.navigation_settings[index];
                setting.alignment = Some(alignment);
                if let Some(navigation) = setting.as_navigation() {
                    if index == 0 {
                        self.hex_viewer.set_horizontal_navigation(navigation);
                    } else {
                        self.hex_viewer.set_vertical_navigation(navigation);
                    }
                }
                Task::none()
            }
            Message::HorizontalStepChanged(step) => {
                self.horizontal_step = Some(step);
                self.hex_viewer.set_horizontal_step(step);
                Task::none()
            }
            Message::OpenFile => {
                if self.is_loading {
                    Task::none()
                } else {
                    self.is_loading = true;

                    window::oldest()
                        .and_then(|id| window::run(id, open_file))
                        .then(Task::future)
                        .map(Message::FileOpened)
                }
            }
            Message::FileOpened(result) => {
                if let Ok(path) = result {
                    self.hex_viewer.open_file(&path);
                }
                self.is_loading = false;
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let hex_viewer = self.hex_viewer.view().map(Message::HexViewer);

        let theme = configure_row(row![
            "Theme",
            pick_list(Theme::ALL, Some(&self.theme), Message::ThemeChanged)
                .width(Length::Shrink),
        ]);

        let font = configure_row(row![
            "Font",
            pick_list(FontName::ALL, Some(&self.font), Message::FontChanged)
                .width(Length::Shrink),
        ]);

        let font_size = configure_row(row![
            text("Font size"),
            slider(1.0..=42.0, self.font_size, Message::FontSizeChanged)
                .width(150.0),
            text!("{}", self.font_size),
        ]);

        let columns = configure_row(row![
            text("Columns"),
            slider(1..=128, self.columns, Message::ColumnsChanged)
                .width(300.0),
            text!("{}", self.columns),
        ]);

        let debug = configure_row(row![
            "Debug",
            toggler(self.debug).on_toggle(Message::DebugToggled)
        ]);

        let random_highlight_button = button("Random highlight")
            .on_press(Message::RandomHighlightPressed);

        let horizontal_step_cell = radio(
            "Cell",
            viewer::Step::Cell,
            self.horizontal_step,
            Message::HorizontalStepChanged
        );

        let horizontal_step_pixel= radio(
            "Pixel",
            viewer::Step::Pixel,
            self.horizontal_step,
            Message::HorizontalStepChanged
        );

        let horizontal_step = configure_row(row![
            text("Horizontal step"),
            horizontal_step_cell,
            horizontal_step_pixel,
        ]);

        let settings = column![
            group_settings(
                "General",
                [
                    theme.into(),
                    font.into(),
                    font_size.into(),
                    columns.into(),
                ]
            ),
            group_settings(
                "Padding",
                self.create_padding()
                    .chain(std::iter::once(
                        Element::from(row![
                            button("Compact").on_press(Message::PresetLayoutCompact),
                            button("Spacious").on_press(Message::PresetLayoutSpacious)
                        ].spacing(10.0))
                    ))
            ),
            group_settings(
                "Behavior",
                [
                    self.create_navigation("Horizontal navigation", 0),
                    self.create_navigation("Vertical navigation", 1),
                    horizontal_step.into(),
                ]
            ),
            group_settings(
                "Augmentation",
                [
                    random_highlight_button.into(),
                ]
            ),
            group_settings(
                "Misc",
                [
                    debug.into(),
                ]
            ),
        ].spacing(10)
            .width(450);

        let scrollable_settings = scrollable(
            row![settings, space::horizontal().width(20.)]
        );
        
        let content: Element<'_, Message> = column![
            action(
                open_icon(),
                "Open file",
                (!self.is_loading).then_some(Message::OpenFile)
            ),
            row![
                container(scrollable_settings).align_x(alignment::Horizontal::Left),
                container(hex_viewer),
            ].spacing(10)
        ].padding(10).spacing(10.0).into();

        if self.debug {
            content.explain(self.theme.palette().text)
        } else {
            content
        }
    }

    fn theme(&self) -> Theme {
        self.theme.clone()
    }

    fn set_font(&mut self) {
        self.hex_viewer.set_font(Font::with_name(self.font.static_str()));
    }

    fn set_padding_settings(&mut self) {
        self.hex_viewer.set_layout_settings(self.padding_settings);
    }

    fn set_preset_layout(&mut self, layout_settings: viewer::PaddingSettings) {
        self.padding_settings = layout_settings;
        self.set_padding_settings();
    }

    fn create_navigation<'a>(
        &self,
        label_text: &'a str,
        index: usize,
    ) -> Element<'a, Message> {
        let navigation = configure_row(row![
            text(label_text),
            radio(
                "Lazy",
                NavigationOption::Lazy,
                self.navigation_settings[index].navigation,
                Message::NavigationOptionChanged.with(index)
            ),
            radio(
                "Aligned",
                NavigationOption::Aligned,
                self.navigation_settings[index].navigation,
                Message::NavigationOptionChanged.with(index)
            ),
        ]);

        let alignment = configure_row(row![
            space::horizontal().width(20.0),
            text("Alignment"),
            radio(
                "Start",
                viewer::Alignment::Start,
                self.navigation_settings[index].alignment,
                Message::NavigationAlignedChanged.with(index)
            ),
            radio(
                "Center",
                viewer::Alignment::Center,
                self.navigation_settings[index].alignment,
                Message::NavigationAlignedChanged.with(index)
            ),
            radio(
                "End",
                viewer::Alignment::End,
                self.navigation_settings[index].alignment,
                Message::NavigationAlignedChanged.with(index)
            )
        ]);

        column![
            navigation,
            self.navigation_settings[index].navigation
                .filter(|&nav| nav == NavigationOption::Aligned)
                .map(|_| alignment)
        ].spacing(10).width(450).into()
    }

    fn create_padding(&self) -> impl Iterator<Item = Element<'_, Message, Theme>> {
        (0..=12).map(|index| {
            let (l, v) = self.get_padding(index);
            configure_row(row![
                text(l),
                slider(
                    0.0..=LAYOUT_PADDING_SLIDER_RANGE,
                    v * LAYOUT_SLIDER_DIVIDER,
                    Message::PaddingChanged.with(index)).width(
                        (LAYOUT_PADDING_SLIDER_RANGE) * 5.0),
                text!("{v:.2}x"),
            ]).into()
        })
    }

    fn get_padding(&self, index: usize) -> (&'static str, f32) {
        match index {
            0 => (PADDING_HEADER_TOP, self.padding_settings.header_top),
            1 => (PADDING_HEADER_BOTTOM, self.padding_settings.header_bottom),
            2 => (PADDING_CONTENT_TOP, self.padding_settings.content_top),
            3 => (PADDING_CONTENT_BOTTOM, self.padding_settings.content_bottom),
            4 => (PADDING_ADDRESS_AREA_LEFT, self.padding_settings.address_area_left),
            5 => (PADDING_ADDRESS_AREA_RIGHT, self.padding_settings.address_area_right),
            6 => (PADDING_BYTE_AREA_LEFT, self.padding_settings.byte_area_left),
            7 => (PADDING_BYTE_AREA_RIGHT, self.padding_settings.byte_area_right),
            8 => (PADDING_CHAR_AREA_LEFT, self.padding_settings.char_area_left),
            9 => (PADDING_CHAR_AREA_RIGHT, self.padding_settings.char_area_right),
            10 => (PADDING_DATA_CELL_VERTICAL, self.padding_settings.data_cell_vertical),
            11 => (PADDING_BYTE_CELL_HORIZONTAL, self.padding_settings.byte_cell_horizontal),
            12 => (PADDING_CHAR_CELL_HORIZONTAL, self.padding_settings.char_cell_horizontal),
            _ => panic!()
        }
    }

    fn set_padding(&mut self, index: usize, value: f32) {
        match index {
            0 => self.padding_settings.header_top = value,
            1 => self.padding_settings.header_bottom = value,
            2 => self.padding_settings.content_top = value,
            3 => self.padding_settings.content_bottom = value,
            4 => self.padding_settings.address_area_left = value,
            5 => self.padding_settings.address_area_right = value,
            6 => self.padding_settings.byte_area_left = value,
            7 => self.padding_settings.byte_area_right = value,
            8 => self.padding_settings.char_area_left = value,
            9 => self.padding_settings.char_area_right = value,
            10 => self.padding_settings.data_cell_vertical = value,
            11 => self.padding_settings.byte_cell_horizontal = value,
            12 => self.padding_settings.char_cell_horizontal = value,
            _ => panic!()
        }
    }
}

fn group_settings<'a>(
    heading: &'a str,
    children: impl IntoIterator<Item = Element<'a, Message, Theme>>
) -> Element<'a, Message> {
    container(
        Column::with_children(
            vec![Element::from(text(heading))].into_iter().chain(children)
        )
            .spacing(10).width(450).padding(10)
    )
    .style(|theme| {
        let palette = theme.extended_palette();
        container::Style::default()
            .border(border::color(palette.background.strong.color).width(1))
    })
    .into()
}

fn configure_row(
    row: Row<'_, Message>
) -> Row<'_, Message> {
    row
    .spacing(10.0)
    .align_y(alignment::Vertical::Center)
}

#[derive(Debug, Clone)]
pub enum Error {
    DialogClosed,
    IoError(io::ErrorKind),
}

fn open_file(
    window: &dyn Window,
) -> impl Future<Output = Result<PathBuf, Error>> + use<> {
    let dialog = rfd::AsyncFileDialog::new()
        .set_title("Open a text file...")
        .set_parent(&window);

    async move {
        let picked_file =
            dialog.pick_file().await.ok_or(Error::DialogClosed)?;

        Ok(picked_file.path().to_owned())
    }
}

// Shamelessly taken from the editor example.
fn action<'a, Message: Clone + 'a>(
    content: impl Into<Element<'a, Message>>,
    label: &'a str,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    let action = button(center_x(content).width(30));

    if let Some(on_press) = on_press {
        tooltip(
            action.on_press(on_press),
            label,
            tooltip::Position::FollowCursor,
        )
        .style(container::rounded_box)
        .into()
    } else {
        action.style(button::secondary).into()
    }
}

// Shamelessly taken from the editor example.
fn open_icon<'a, Message>() -> Element<'a, Message> {
    icon('\u{0f115}')
}

// Shamelessly taken from the editor example.
fn icon<'a, Message>(codepoint: char) -> Element<'a, Message> {
    const ICON_FONT: Font = Font::with_name("editor-icons");

    text(codepoint)
        .font(ICON_FONT)
        .shaping(text::Shaping::Basic)
        .into()
}
