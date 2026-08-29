//! Native Masonry application shell.

use masonry::core::{ErasedAction, NewWidget, StyleProperty, Widget as _, WidgetId};
use masonry::dpi::LogicalSize;
use masonry::layout::Length;
use masonry::parley::style::FontWeight;
use masonry::theme::default_property_set;
use masonry::widgets::{Flex, Label};
use masonry_winit::app::{AppDriver, DriverCtx, NewWindow, WindowId};
use masonry_winit::winit::window::Window;
use std::env;

const SPACING: Length = Length::const_px(8.0);

struct Driver {
    window_id: WindowId,
}

impl AppDriver for Driver {
    fn on_action(
        &mut self,
        window_id: WindowId,
        _ctx: &mut DriverCtx<'_>,
        _widget_id: WidgetId,
        _action: ErasedAction,
    ) {
        debug_assert_eq!(window_id, self.window_id, "unknown editor window");
    }
}

/// Starts the native editor window.
///
/// # Errors
///
/// Returns an error when an unsupported launch argument is supplied or the
/// platform event loop cannot start.
pub fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    if let Some(argument) = args.next() {
        if matches!(argument.as_str(), "--help" | "-h") {
            println!("usage: nuif-editor [--help]");
            return Ok(());
        }
        return Err(format!("unknown native-editor argument {argument:?}"));
    }

    let navigation = Label::new("NUIF")
        .with_style(StyleProperty::FontSize(13.0))
        .with_style(StyleProperty::FontWeight(FontWeight::BOLD));
    let status = Label::new("Native editor shell · profile 0");
    let root = Flex::column()
        .with_fixed(navigation.prepare())
        .with_fixed_spacer(SPACING)
        .with_fixed(status.prepare());

    let window_id = WindowId::next();
    let window_size = LogicalSize::new(1280.0, 800.0);
    let window_attributes = Window::default_attributes()
        .with_title("NUIF Editor")
        .with_resizable(true)
        .with_inner_size(window_size)
        .with_min_inner_size(LogicalSize::new(900.0, 600.0));

    masonry_winit::app::run(
        vec![NewWindow::new_with_id(
            window_id,
            window_attributes,
            NewWidget::new(root).erased(),
        )],
        Driver { window_id },
        default_property_set(),
    )
    .map_err(|error| error.to_string())
}
