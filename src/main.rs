use crate::extensions::deterministic_search::DeterministicSearchEngine;
use crate::fs::config::Configuration;
use crate::ui::search_bar::SearchBar;
use crate::ui::search_engine::GpuiSearchEngine;
use global_hotkey::GlobalHotKeyManager;
use global_hotkey::{GlobalHotKeyEvent, HotKeyState};
use gpui::{
    AppContext, Application, Bounds, Pixels, WindowBackgroundAppearance, WindowBounds, WindowKind,
    WindowOptions, actions,
};
use gpui_component::Root;

pub mod apps;
pub mod extensions;
pub mod fs;
pub mod ui;

const APP_NAME: &str = "Fetch";

actions!(
    fetch_actions,
    [EnterPressed, EscPressed, TabSelectApp, OpenSettings]
);

fn main() {
    let manager = GlobalHotKeyManager::new().unwrap();
    let config = Configuration::read_from_fs();
    let hotkey = config.hotkey_config();

    manager.register(hotkey).unwrap();

    // Attempt to register app to auto-start on login
    if cfg!(target_os = "macos") && config.launch_on_boot {
        use smappservice_rs::{AppService, ServiceStatus, ServiceType};

        let app_service = AppService::new(ServiceType::MainApp);

        match app_service.status() {
            // Either it's already enabled, or user/macOS did not allow
            // Fetch to start, so, leave it as-is.
            ServiceStatus::Enabled | ServiceStatus::RequiresApproval => {}
            ServiceStatus::NotRegistered | ServiceStatus::NotFound => {
                if app_service.register().is_err() {
                    eprintln!("Registering app for auto-start failed");
                }
            }
        }
    }

    let app = Application::new();

    app.run(move |cx| {
        cx.bind_keys([
            gpui::KeyBinding::new("enter", EnterPressed, None),
            gpui::KeyBinding::new("escape", EscPressed, None),
            gpui::KeyBinding::new("tab", TabSelectApp, None),
            gpui::KeyBinding::new("cmd-t", OpenSettings, None),
        ]);

        // This must be called before using any GPUI Component features.
        gpui_component::init(cx);

        cx.set_global(config);

        let display_center = cx
            .primary_display()
            .expect("Display exists")
            .bounds()
            .center();

        cx.spawn(async move |cx| {
            let search_engine = cx
                .read_global::<Configuration, DeterministicSearchEngine>(|config, _| {
                    DeterministicSearchEngine::build(config)
                })
                .unwrap();
            let search_engine_entity = cx
                .new(|_cx| GpuiSearchEngine::new(search_engine))
                .expect("Search engine building is infallible");

            loop {
                // Await hotkey
                if cx
                    .background_executor()
                    .spawn(async move {
                        if let Ok(ev) = GlobalHotKeyEvent::receiver().recv() {
                            return ev.state == HotKeyState::Pressed;
                        }

                        false
                    })
                    .await
                {
                    // Hotkey pressed -> open window
                    let window_options = WindowOptions {
                        window_bounds: Some(WindowBounds::Windowed(Bounds::centered_at(
                            display_center,
                            gpui::Size {
                                width: Pixels::from(500u32),
                                height: Pixels::from(180u32),
                            },
                        ))),
                        focus: true,
                        show: true,
                        kind: WindowKind::PopUp,
                        is_resizable: false,
                        window_decorations: None,
                        titlebar: None,
                        window_background: WindowBackgroundAppearance::Transparent,
                        app_id: Some(APP_NAME.to_string()),
                        tabbing_identifier: None,
                        ..Default::default()
                    };

                    cx.open_window(window_options, |window, cx| {
                        let view =
                            cx.new(|cx| SearchBar::new(window, cx, search_engine_entity.clone()));

                        cx.new(|cx| Root::new(view, window, cx))
                    })
                    .unwrap();
                }
            }
        })
        .detach();
    });
}
