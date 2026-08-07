use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use iced::widget::svg;
use iced::{ContentFit, Element, Length};
use iced_shadcn::{AspectRatioProps, Theme, aspect_ratio};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MockupFrameKind {
    Safari,
    Iphone,
    Android,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MockupFrameMetrics {
    pub width: f32,
    pub height: f32,
    pub inset_left: f32,
    pub inset_top: f32,
    pub inset_right: f32,
    pub inset_bottom: f32,
    pub screen_radius: f32,
    pub chrome_height: f32,
}

impl MockupFrameMetrics {
    pub fn aspect_ratio(self) -> f32 {
        self.width / self.height
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridCell {
    pub row: u16,
    pub col: u16,
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct MockupFrameProps {
    pub chrome_label: Option<String>,
    pub safari_simple: bool,
    pub screen_media: bool,
    pub media_bytes: Option<&'static [u8]>,
    pub media_mime: Option<&'static str>,
}

impl MockupFrameProps {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn chrome_label(mut self, chrome_label: impl Into<String>) -> Self {
        self.chrome_label = Some(chrome_label.into());
        self
    }

    pub fn safari_simple(mut self, safari_simple: bool) -> Self {
        self.safari_simple = safari_simple;
        self
    }

    pub fn screen_media(mut self, screen_media: bool) -> Self {
        self.screen_media = screen_media;
        self
    }

    pub fn media(mut self, bytes: &'static [u8], mime: &'static str) -> Self {
        self.media_bytes = Some(bytes);
        self.media_mime = Some(mime);
        self
    }
}

pub fn mockup_metrics(kind: MockupFrameKind) -> MockupFrameMetrics {
    match kind {
        MockupFrameKind::Safari => MockupFrameMetrics {
            width: 1203.0,
            height: 753.0,
            inset_left: 1.0,
            inset_top: 52.0,
            inset_right: 2.0,
            inset_bottom: 1.0,
            screen_radius: 11.0,
            chrome_height: 52.0,
        },
        MockupFrameKind::Iphone => MockupFrameMetrics {
            width: 433.0,
            height: 882.0,
            inset_left: 21.25,
            inset_top: 19.25,
            inset_right: 22.25,
            inset_bottom: 19.25,
            screen_radius: 55.75,
            chrome_height: 74.0,
        },
        MockupFrameKind::Android => MockupFrameMetrics {
            width: 433.0,
            height: 882.0,
            inset_left: 9.0,
            inset_top: 14.0,
            inset_right: 64.0,
            inset_bottom: 68.0,
            screen_radius: 33.0,
            chrome_height: 48.0,
        },
    }
}

pub fn mockup_frame<'a, Message: Clone + 'a + 'static>(
    kind: MockupFrameKind,
    content: impl Into<Element<'a, Message>>,
    theme: &Theme,
) -> Element<'a, Message> {
    mockup_frame_with_props(kind, content, &MockupFrameProps::new(), theme)
}

const ANDROID_DEVICE_LIGHT: &[u8] = include_bytes!("../../assets/android-device.svg");
const ANDROID_DEVICE_DARK: &[u8] = include_bytes!("../../assets/android-device-dark.svg");
const IPHONE_DEVICE_LIGHT: &[u8] = include_bytes!("../../assets/iphone-device.svg");
const IPHONE_DEVICE_DARK: &[u8] = include_bytes!("../../assets/iphone-device-dark.svg");
const SAFARI_DEVICE_LIGHT: &[u8] = include_bytes!("../../assets/safari-device.svg");
const SAFARI_OVERLAY_LIGHT: &[u8] = include_bytes!("../../assets/safari-overlay.svg");

fn frame_svg<'a, Message: 'a>(bytes: &'static [u8]) -> Element<'a, Message> {
    svg(svg::Handle::from_memory(bytes))
        .width(Length::Fill)
        .height(Length::Fill)
        .content_fit(ContentFit::Fill)
        .into()
}

fn dynamic_svg<'a, Message: 'a>(bytes: Vec<u8>) -> Element<'a, Message> {
    svg(svg::Handle::from_memory(bytes))
        .width(Length::Fill)
        .height(Length::Fill)
        .content_fit(ContentFit::Fill)
        .into()
}

fn safari_svg<'a, Message: 'a>(bytes: &'static [u8], url: &str) -> Element<'a, Message> {
    let svg_str = String::from_utf8_lossy(bytes).replace("magicui.design", url);
    svg(svg::Handle::from_memory(svg_str.into_bytes()))
        .width(Length::Fill)
        .height(Length::Fill)
        .content_fit(ContentFit::Fill)
        .into()
}

fn chrome_url(props: &MockupFrameProps) -> String {
    props
        .chrome_label
        .clone()
        .unwrap_or_else(|| "magicui.design".to_string())
}

fn data_url(bytes: &[u8], mime: &str) -> String {
    format!("data:{mime};base64,{}", BASE64.encode(bytes))
}

/// Android with embedded `<image>` — matches `magicui/android.tsx`.
fn android_media_svg(bytes: &[u8], mime: &str, dark: bool) -> Vec<u8> {
    let href = data_url(bytes, mime);
    let (shell, inner, screen) = if dark {
        ("#404040", "#262626", "#404040")
    } else {
        ("#E5E5E5", "#FFFFFF", "#E5E5E5")
    };

    format!(
        r#"<svg width="433" height="882" viewBox="0 0 433 882" fill="none" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">
<path d="M376 153H378C379.105 153 380 153.895 380 155V249C380 250.105 379.105 251 378 251H376V153Z" fill="{shell}"/>
<path d="M376 301H378C379.105 301 380 301.895 380 303V351C380 352.105 379.105 353 378 353H376V301Z" fill="{shell}"/>
<path d="M0 42C0 18.8041 18.804 0 42 0H336C359.196 0 378 18.804 378 42V788C378 811.196 359.196 830 336 830H42C18.804 830 0 811.196 0 788V42Z" fill="{shell}"/>
<path d="M2 43C2 22.0132 19.0132 5 40 5H338C358.987 5 376 22.0132 376 43V787C376 807.987 358.987 825 338 825H40C19.0132 825 2 807.987 2 787V43Z" fill="{inner}"/>
<g clip-path="url(#androidClip)">
<path d="M9.25 48C9.25 29.3604 24.3604 14.25 43 14.25H335C353.64 14.25 368.75 29.3604 368.75 48V780C368.75 798.64 353.64 813.75 335 813.75H43C24.3604 813.75 9.25 798.64 9.25 780V48Z" fill="{screen}" stroke="{screen}" stroke-width="0.5"/>
</g>
<image href="{href}" x="9" y="14" width="360" height="800" preserveAspectRatio="xMidYMid slice" clip-path="url(#androidClip)"/>
<circle cx="189" cy="28" r="9" fill="{inner}"/>
<circle cx="189" cy="28" r="4" fill="{shell}"/>
<defs>
<clipPath id="androidClip">
<rect width="360" height="800" rx="33" ry="25" transform="translate(9 14)"/>
</clipPath>
</defs>
</svg>"#
    )
    .into_bytes()
}

/// iPhone with embedded image + masked frame — matches `magicui/iphone.tsx` hasMedia path.
fn iphone_media_svg(bytes: &[u8], mime: &str, dark: bool) -> Vec<u8> {
    let href = data_url(bytes, mime);
    let (shell, inner, bezel, notch, camera) = if dark {
        ("#404040", "#262626", "#404040", "#262626", "#404040")
    } else {
        ("#E5E5E5", "#FFFFFF", "#E5E5E5", "#F5F5F5", "#E5E5E5")
    };

    format!(
        r#"<svg width="433" height="882" viewBox="0 0 433 882" fill="none" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">
<defs>
<clipPath id="iphoneScreen">
<rect x="21.25" y="19.25" width="389.5" height="843.5" rx="55.75" ry="55.75"/>
</clipPath>
<mask id="screenPunch" maskUnits="userSpaceOnUse">
<rect x="0" y="0" width="433" height="882" fill="white"/>
<rect x="21.25" y="19.25" width="389.5" height="843.5" rx="55.75" ry="55.75" fill="black"/>
</mask>
</defs>
<image href="{href}" x="21.25" y="19.25" width="389.5" height="843.5" preserveAspectRatio="xMidYMid slice" clip-path="url(#iphoneScreen)"/>
<g mask="url(#screenPunch)">
<path d="M2 73C2 32.6832 34.6832 0 75 0H357C397.317 0 430 32.6832 430 73V809C430 849.317 397.317 882 357 882H75C34.6832 882 2 849.317 2 809V73Z" fill="{shell}"/>
<path d="M0 171C0 170.448 0.447715 170 1 170H3V204H1C0.447715 204 0 203.552 0 203V171Z" fill="{shell}"/>
<path d="M1 234C1 233.448 1.44772 233 2 233H3.5V300H2C1.44772 300 1 299.552 1 299V234Z" fill="{shell}"/>
<path d="M1 319C1 318.448 1.44772 318 2 318H3.5V385H2C1.44772 385 1 384.552 1 384V319Z" fill="{shell}"/>
<path d="M430 279H432C432.552 279 433 279.448 433 280V384C433 384.552 432.552 385 432 385H430V279Z" fill="{shell}"/>
<path d="M6 74C6 35.3401 37.3401 4 76 4H356C394.66 4 426 35.3401 426 74V808C426 846.66 394.66 878 356 878H76C37.3401 878 6 846.66 6 808V74Z" fill="{inner}"/>
<path d="M21.25 75C21.25 44.2101 46.2101 19.25 77 19.25H355C385.79 19.25 410.75 44.2101 410.75 75V807C410.75 837.79 385.79 862.75 355 862.75H77C46.2101 862.75 21.25 837.79 21.25 807V75Z" fill="{bezel}" stroke="{bezel}" stroke-width="0.5"/>
</g>
<path opacity="0.5" d="M174 5H258V5.5C258 6.60457 257.105 7.5 256 7.5H176C174.895 7.5 174 6.60457 174 5.5V5Z" fill="{shell}"/>
<path d="M154 48.5C154 38.2827 162.283 30 172.5 30H259.5C269.717 30 278 38.2827 278 48.5C278 58.7173 269.717 67 259.5 67H172.5C162.283 67 154 58.7173 154 48.5Z" fill="{notch}"/>
<path d="M249 48.5C249 42.701 253.701 38 259.5 38C265.299 38 270 42.701 270 48.5C270 54.299 265.299 59 259.5 59C253.701 59 249 54.299 249 48.5Z" fill="{notch}"/>
<path d="M254 48.5C254 45.4624 256.462 43 259.5 43C262.538 43 265 45.4624 265 48.5C265 51.5376 262.538 54 259.5 54C256.462 54 254 51.5376 254 48.5Z" fill="{camera}"/>
</svg>"#
    )
    .into_bytes()
}

/// Safari with embedded image + masked chrome — matches `magicui/safari.tsx` hasMedia path.
fn safari_media_svg(bytes: &[u8], mime: &str, url: &str, _simple: bool) -> Vec<u8> {
    let href = data_url(bytes, mime);
    let mut svg = String::from_utf8_lossy(SAFARI_OVERLAY_LIGHT).replace("magicui.design", url);
    let clip = r#"<clipPath id="safariScreen"><path d="M1 52H1201V741C1201 747.075 1196.08 752 1190 752H12C5.92486 752 1 747.075 1 741V52Z"/></clipPath>"#;
    if let Some(pos) = svg.find("</defs>") {
        svg.insert_str(pos, clip);
    }
    let image = format!(
        r#"<image href="{href}" x="1" y="52" width="1200" height="700" preserveAspectRatio="xMidYMid slice" clip-path="url(#safariScreen)"/>"#
    );
    if let Some(pos) = svg.find("</defs>") {
        svg.insert_str(pos + "</defs>".len(), &image);
    }
    svg.into_bytes()
}

#[cfg(test)]
fn resvg_parse_ok(bytes: &[u8]) -> bool {
    let options = resvg::usvg::Options::default();
    resvg::usvg::Tree::from_data(bytes, &options).is_ok()
}

pub fn mockup_frame_with_props<'a, Message: Clone + 'a + 'static>(
    kind: MockupFrameKind,
    _content: impl Into<Element<'a, Message>>,
    props: &MockupFrameProps,
    theme: &Theme,
) -> Element<'a, Message> {
    let metrics = mockup_metrics(kind);
    let is_dark = theme.palette.background.r < 0.5;
    let url = chrome_url(props);

    let frame_body: Element<'a, Message> = match kind {
        MockupFrameKind::Iphone if props.screen_media => {
            let (bytes, mime) = props
                .media_bytes
                .zip(props.media_mime)
                .expect("screen_media requires media bytes");
            let svg_bytes = iphone_media_svg(bytes, mime, is_dark);
            dynamic_svg(svg_bytes)
        }
        MockupFrameKind::Android if props.screen_media => {
            let (bytes, mime) = props
                .media_bytes
                .zip(props.media_mime)
                .expect("screen_media requires media bytes");
            let svg_bytes = android_media_svg(bytes, mime, is_dark);
            dynamic_svg(svg_bytes)
        }
        MockupFrameKind::Safari if props.screen_media => {
            let (bytes, mime) = props
                .media_bytes
                .zip(props.media_mime)
                .expect("screen_media requires media bytes");
            let svg_bytes = safari_media_svg(bytes, mime, &url, props.safari_simple);
            dynamic_svg(svg_bytes)
        }
        MockupFrameKind::Iphone => {
            let bytes = if is_dark {
                IPHONE_DEVICE_DARK
            } else {
                IPHONE_DEVICE_LIGHT
            };
            frame_svg(bytes)
        }
        MockupFrameKind::Android => {
            let bytes = if is_dark {
                ANDROID_DEVICE_DARK
            } else {
                ANDROID_DEVICE_LIGHT
            };
            frame_svg(bytes)
        }
        MockupFrameKind::Safari => safari_svg(SAFARI_DEVICE_LIGHT, &url),
    };

    aspect_ratio(
        frame_body,
        AspectRatioProps::new().ratio(metrics.aspect_ratio()),
    )
    .into()
}

pub fn image_grid_cells(rows: u16, cols: u16) -> Box<[GridCell]> {
    if rows == 0 || cols == 0 {
        return Box::new([]);
    }

    let width = 1.0 / cols as f32;
    let height = 1.0 / rows as f32;
    let mut cells = Vec::with_capacity(rows as usize * cols as usize);

    for row in 0..rows {
        for col in 0..cols {
            cells.push(GridCell {
                row,
                col,
                left: col as f32 * width,
                top: row as f32 * height,
                width,
                height,
            });
        }
    }

    cells.into_boxed_slice()
}

#[cfg(test)]
mod tests {
    use super::*;
    use resvg::tiny_skia::Pixmap;

    #[test]
    fn mockup_metrics_match_phase0_reference_geometry() {
        let safari = mockup_metrics(MockupFrameKind::Safari);
        let iphone = mockup_metrics(MockupFrameKind::Iphone);
        let android = mockup_metrics(MockupFrameKind::Android);

        assert_eq!(safari.inset_top, 52.0);
        assert_eq!(safari.screen_radius, 11.0);
        assert_eq!(iphone.screen_radius, 55.75);
        assert_eq!(android.inset_left, 9.0);
        assert_eq!(android.screen_radius, 33.0);
    }

    #[test]
    fn composed_android_media_svg_renders_opaque_frame_pixels() {
        let png = include_bytes!("../../examples/assets/hero-light.png");
        let svg = android_media_svg(png, "image/png", false);
        assert!(resvg_parse_ok(&svg));

        let tree = resvg::usvg::Tree::from_data(&svg, &resvg::usvg::Options::default()).unwrap();
        let mut pixmap = Pixmap::new(433, 882).unwrap();
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::identity(),
            &mut pixmap.as_mut(),
        );

        let opaque = pixmap.pixels().iter().filter(|px| px.alpha() > 10).count();
        assert!(
            opaque > 50_000,
            "expected visible frame chrome, got {opaque}"
        );
    }

    #[test]
    fn mockup_frame_props_can_toggle_safari_simple_mode() {
        let props = MockupFrameProps::new();
        let simple = MockupFrameProps::new().safari_simple(true);

        assert!(!props.safari_simple);
        assert!(simple.safari_simple);
    }

    #[test]
    fn image_grid_cells_are_deterministic_and_cover_grid() {
        let cells = image_grid_cells(2, 3);

        assert_eq!(cells.len(), 6);
        assert_eq!(
            cells[0],
            GridCell {
                row: 0,
                col: 0,
                left: 0.0,
                top: 0.0,
                width: 1.0 / 3.0,
                height: 0.5,
            }
        );
        assert_eq!(cells[5].row, 1);
        assert_eq!(cells[5].col, 2);
        assert!((cells[5].left - (2.0 / 3.0)).abs() < f32::EPSILON);
        assert!((cells[5].top - 0.5).abs() < f32::EPSILON);
    }
}
