//! GTK owns presentation only. Runtime commands and completions delimit every mutation.
use crate::{
    Submit,
    actions::{ActionCallbacks, ActionRegistry},
    icons::Icons,
    tray::{Tray, TrayCallbacks},
    ui::{dialogs, matrix::MatrixView, sidebar::Sidebar, uninstall::UninstallDialog},
};
use adw::prelude::*;
use cadiswave_core::model::*;
use cadiswave_runtime::{
    controller::{AppCommand, RuntimeEvent, RuntimeHandle},
    paths::RuntimePaths,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
    thread::JoinHandle,
    time::Duration,
};

mod geometry;

fn monitor_dimensions(window: Option<&adw::ApplicationWindow>) -> Option<(i32, i32)> {
    let display = window
        .map(gtk::prelude::WidgetExt::display)
        .or_else(gtk::gdk::Display::default)?;
    let monitor = window
        .and_then(|window| window.surface())
        .and_then(|surface| display.monitor_at_surface(&surface))
        .or_else(|| {
            display
                .monitors()
                .item(0)?
                .downcast::<gtk::gdk::Monitor>()
                .ok()
        })?;
    // GDK monitor geometry uses logical pixels. Do not apply the display scale again.
    let geometry = monitor.geometry();
    Some((geometry.width(), geometry.height()))
}

pub fn run(args: Vec<String>) -> i32 {
    if let Err(error) = cadiswave_runtime::process::require_user() {
        eprintln!("cadiswave: {error}");
        return 1;
    }
    let paths = match RuntimePaths::discover() {
        Ok(paths) => paths,
        Err(error) => {
            eprintln!("cadiswave: {error}");
            return 1;
        }
    };
    let _ = env_logger::try_init();
    let application = adw::Application::builder()
        .application_id("io.github.RamaAditya49.CadisWave")
        .flags(gio::ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();
    application.add_main_option(
        "hide",
        glib::Char::from(b'\0'),
        glib::OptionFlags::NONE,
        glib::OptionArg::None,
        "Start hidden when a system tray host is available",
        None,
    );
    let state: Rc<RefCell<Option<Rc<AppUi>>>> = Rc::new(RefCell::new(None));
    let failed = Rc::new(Cell::new(false));
    let state_start = state.clone();
    let failed_start = failed.clone();
    application.connect_startup(move |application| {
        let owner = application
            .dbus_connection()
            .and_then(|connection| connection.unique_name())
            .map(|name| name.to_string());
        let (handle, events) = match RuntimeHandle::launch(paths.clone(), owner) {
            Ok(runtime) => runtime,
            Err(error) => {
                eprintln!("cadiswave: {error}");
                failed_start.set(true);
                application.quit();
                return;
            }
        };
        let ui = AppUi::new(application.clone(), paths.clone(), handle);
        *state_start.borrow_mut() = Some(ui.clone());
        let weak = Rc::downgrade(&ui);
        glib::MainContext::default().spawn_local(async move {
            while let Ok(event) = events.recv().await {
                let Some(ui) = weak.upgrade() else {
                    break;
                };
                ui.event(event);
            }
        });
    });
    let state_command = state.clone();
    application.connect_command_line(move |application, command| {
        if let Some(ui) = state_command.borrow().as_ref() {
            let hide = command
                .options_dict()
                .lookup::<bool>("hide")
                .ok()
                .flatten()
                .unwrap_or(false);
            ui.start_hidden.set(hide && !ui.activated.get());
        }
        application.activate();
        glib::ExitCode::SUCCESS
    });
    let state_activate = state.clone();
    application.connect_activate(move |_| {
        if let Some(ui) = state_activate.borrow().as_ref() {
            ui.activate();
        }
    });
    let code: i32 = application.run_with_args(&args).into();
    if let Some(ui) = state.borrow().as_ref()
        && let Some(mut tail) = ui.event_tail.borrow_mut().take()
        && let Err(error) = tail.stop()
    {
        eprintln!("cadiswave: {error}");
        failed.set(true);
    }
    // Application::shutdown must not join workers. Even unusual loop exits drain here,
    // after GTK has returned, and retain ownership of temporary inspection threads.
    if let Some(ui) = state.borrow_mut().take() {
        if let Some(tray) = ui.tray.borrow().as_ref() {
            tray.shutdown();
        }
        ui.tray_hold.borrow_mut().take();
        if ui.fatal.get() {
            failed.set(true);
        }
        if ui.handle.snapshot().lifecycle != Lifecycle::Stopped {
            let _ = ui.handle.submit(AppCommand::Shutdown);
        }
        let stopped = ui.handle.wait_stopped();
        for worker in ui.inspection_workers.borrow_mut().drain(..) {
            if worker.join().is_err() {
                eprintln!("cadiswave: removal inspection worker panicked");
                failed.set(true);
            }
        }
        if let Err(error) = stopped {
            eprintln!("cadiswave: {error}");
            failed.set(true);
        }
    }
    if failed.get() { 1 } else { code }
}

pub(crate) fn shortcuts_allowed(focus: Option<&gtk::Widget>) -> bool {
    focus.is_none_or(|widget| !widget.is::<gtk::Editable>() && !widget.is::<gtk::TextView>())
}
struct AppUi {
    application: adw::Application,
    paths: RuntimePaths,
    i18n: Rc<RefCell<crate::i18n::I18n>>,
    handle: RuntimeHandle,
    window: adw::ApplicationWindow,
    icons: Rc<Icons>,
    matrix: RefCell<MatrixView>,
    sidebar: RefCell<Sidebar>,
    split: adw::OverlaySplitView,
    pages: gtk::Stack,
    device_page: crate::ui::device::DevicePage,
    device_settings: crate::ui::device::settings::DeviceSettings,
    compact: crate::ui::device::compact::CompactControls,
    compact_window: adw::Window,
    status_icon: Cell<Option<&'static str>>,
    connection_notifier: RefCell<crate::ui::device::compact::ConnectionNotifier>,
    event_tail: RefCell<Option<cadiswave_runtime::events::EventTail>>,
    title: adw::WindowTitle,
    warning: gtk::MenuButton,
    warning_text: gtk::Label,
    scene_button: gtk::MenuButton,
    menu_button: gtk::MenuButton,
    status: gtk::Label,
    submit: Submit,
    registry: RefCell<Option<ActionRegistry>>,
    tray: RefCell<Option<Tray>>,
    tray_hold: RefCell<Option<gio::ApplicationHoldGuard>>,
    latest: RefCell<Arc<AppSnapshot>>,
    rendered_revision: Cell<Option<u64>>,
    render_pending: Cell<bool>,
    activated: Cell<bool>,
    main_presented: Cell<bool>,
    start_hidden: Cell<bool>,
    shutdown_requested: Cell<bool>,
    stopped: Cell<bool>,
    fatal: Cell<bool>,
    geometry_restored: Cell<bool>,
    prepare_command: Cell<Option<CommandId>>,
    uninstall_command: Cell<Option<CommandId>>,
    uninstall_result: Cell<bool>,
    uninstall: RefCell<Option<Rc<UninstallDialog>>>,
    inspection_workers: Rc<RefCell<Vec<JoinHandle<()>>>>,
    setup_phase: RefCell<Option<SetupPhase>>,
    setup_dialog: RefCell<Option<adw::AlertDialog>>,
    setup_generation: Cell<u64>,
    calibration: RefCell<Option<CalibrationSnapshot>>,
    calibration_dialog: RefCell<Option<adw::AlertDialog>>,
    calibration_generation: Cell<u64>,
}

impl AppUi {
    fn new(application: adw::Application, paths: RuntimePaths, handle: RuntimeHandle) -> Rc<Self> {
        adw::StyleManager::default().set_color_scheme(adw::ColorScheme::ForceDark);
        if let Ok(css) = paths.data_file("style.css") {
            let provider = gtk::CssProvider::new();
            provider.load_from_path(css);
            if let Some(display) = gtk::gdk::Display::default() {
                gtk::style_context_add_provider_for_display(
                    &display,
                    &provider,
                    gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
                );
            }
        }
        let icons = Rc::new(Icons::new(paths.clone()));
        let snapshot = handle.snapshot();
        let (width, height) = geometry::fit_size(
            (snapshot.preferences.width, snapshot.preferences.height),
            monitor_dimensions(None),
        );
        let i18n = Rc::new(RefCell::new(
            crate::i18n::I18n::new(snapshot.preferences.language, &crate::i18n::system_locale())
                .expect("validated embedded Fluent catalogs"),
        ));
        crate::i18n::activate(i18n.clone());
        let ui = Rc::new_cyclic(|weak: &std::rc::Weak<Self>| {
            let target = weak.clone();
            let submit: Submit = Rc::new(move |command| {
                if let Some(ui) = target.upgrade() {
                    ui.submit_command(command);
                }
            });
            let window = adw::ApplicationWindow::builder()
                .application(&application)
                .title("CadisWave")
                .default_width(width)
                .default_height(height)
                .build();
            window.set_size_request(geometry::MIN_SIZE.0, geometry::MIN_SIZE.1);
            window.add_css_class("cadiswave");
            if snapshot.preferences.maximized {
                window.maximize();
            }
            let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
            let header = adw::HeaderBar::new();
            let title = adw::WindowTitle::new("CadisWave", "Disconnected");
            header.set_title_widget(Some(&title));
            let warning = gtk::MenuButton::builder()
                .icon_name("dialog-warning-symbolic")
                .visible(false)
                .tooltip_text("Connection, service and routing status")
                .build();
            let warning_text = gtk::Label::builder()
                .xalign(0.0)
                .wrap(true)
                .selectable(true)
                .max_width_chars(48)
                .margin_top(12)
                .margin_bottom(12)
                .margin_start(12)
                .margin_end(12)
                .build();
            let popover = gtk::Popover::new();
            popover.set_child(Some(&warning_text));
            warning.set_popover(Some(&popover));
            header.pack_start(&warning);
            let scene_button = gtk::MenuButton::builder().label("Scenes").build();
            header.pack_start(&scene_button);
            let menu_button = gtk::MenuButton::builder()
                .icon_name("open-menu-symbolic")
                .menu_model(&application_menu())
                .tooltip_text("Application menu")
                .build();
            header.pack_end(&menu_button);
            let settings_button = gtk::Button::from_icon_name("emblem-system-symbolic");
            settings_button.set_action_name(Some("win.settings"));
            crate::i18n::bind(&settings_button, "tooltip-text", "hardware-settings");
            header.pack_end(&settings_button);
            let reconnect = gtk::Button::builder()
                .icon_name("view-refresh-symbolic")
                .tooltip_text("Reconnect")
                .build();
            let send = submit.clone();
            reconnect.connect_clicked(move |_| send(AppCommand::Reconnect));
            header.pack_end(&reconnect);
            let toggle = gtk::ToggleButton::builder()
                .icon_name("sidebar-show-symbolic")
                .tooltip_text("Toggle device panel")
                .build();
            header.pack_end(&toggle);
            content.append(&header);
            let status = gtk::Label::builder()
                .xalign(0.0)
                .wrap(true)
                .visible(false)
                .margin_start(12)
                .margin_end(12)
                .build();
            status.add_css_class("dim-label");
            crate::i18n::protect(&warning_text);
            crate::i18n::protect(&status);
            content.append(&status);
            let split = adw::OverlaySplitView::builder()
                .sidebar_position(gtk::PackType::End)
                .min_sidebar_width(320.0)
                .max_sidebar_width(420.0)
                .sidebar_width_fraction(0.30)
                .vexpand(true)
                .show_sidebar(false)
                .build();
            toggle
                .bind_property("active", &split, "show-sidebar")
                .bidirectional()
                .sync_create()
                .build();
            let matrix = MatrixView::new(icons.clone(), submit.clone());
            let sidebar = Sidebar::new(icons.clone(), submit.clone());
            split.set_content(Some(&matrix.widget));
            split.set_sidebar(Some(&sidebar.widget));
            let pages = gtk::Stack::new();
            pages.set_vexpand(true);
            let device_page = crate::ui::device::DevicePage::new(
                crate::ui::device::controls::DeviceControls::new(handle.clone()),
                i18n.clone(),
            );
            let narrow = device_page.install_breakpoints(&window, &split);
            narrow.add_setter(&title, "visible", Some(&false.to_value()));
            pages.add_titled(
                &device_page.widget,
                Some("device"),
                &crate::i18n::tr("device-page"),
            );
            pages.add_titled(&split, Some("mixer"), &crate::i18n::tr("mixer"));
            let switcher = gtk::StackSwitcher::new();
            switcher.add_css_class("cadiswave-header-switcher");
            switcher.set_stack(Some(&pages));
            header.pack_start(&switcher);
            content.append(&pages);
            window.set_content(Some(&content));
            let compact = crate::ui::device::compact::CompactControls::new(
                crate::ui::device::controls::DeviceControls::new(handle.clone()),
                i18n.clone(),
            );
            let compact_window = adw::Window::builder()
                .title("CadisWave")
                .default_width(400)
                .default_height(620)
                .transient_for(&window)
                .build();
            compact_window.add_css_class("cadiswave");
            let compact_content = gtk::Box::new(gtk::Orientation::Vertical, 0);
            let compact_header = adw::HeaderBar::new();
            let compact_title =
                adw::WindowTitle::new("CadisWave", &crate::i18n::tr("compact-controls"));
            compact_header.set_title_widget(Some(&compact_title));
            crate::i18n::bind(&compact_title, "subtitle", "compact-controls");
            compact_content.append(&compact_header);
            compact_content.append(&compact.widget);
            compact_window.set_content(Some(&compact_content));
            compact_window.connect_close_request(|window| {
                window.set_visible(false);
                glib::Propagation::Stop
            });
            Self {
                application: application.clone(),
                paths: paths.clone(),
                i18n: i18n.clone(),
                handle: handle.clone(),
                window,
                icons: icons.clone(),
                matrix: RefCell::new(matrix),
                sidebar: RefCell::new(sidebar),
                split,
                pages,
                device_settings: crate::ui::device::settings::DeviceSettings::new(
                    crate::ui::device::controls::DeviceControls::new(handle.clone()),
                    i18n.clone(),
                ),
                device_page,
                compact,
                compact_window,
                status_icon: Cell::new(None),
                connection_notifier: RefCell::new(Default::default()),
                event_tail: RefCell::new(cadiswave_runtime::events::EventTail::start().ok()),
                title,
                warning,
                warning_text,
                scene_button,
                menu_button,
                status,
                submit,
                registry: RefCell::new(None),
                tray: RefCell::new(None),
                tray_hold: RefCell::new(None),
                latest: RefCell::new(snapshot.clone()),
                rendered_revision: Cell::new(None),
                render_pending: Cell::new(false),
                activated: Cell::new(false),
                main_presented: Cell::new(false),
                start_hidden: Cell::new(false),
                shutdown_requested: Cell::new(false),
                stopped: Cell::new(false),
                prepare_command: Cell::new(None),
                fatal: Cell::new(false),
                geometry_restored: Cell::new(false),
                uninstall_command: Cell::new(None),
                uninstall_result: Cell::new(false),
                uninstall: RefCell::new(None),
                inspection_workers: Rc::new(RefCell::new(Vec::new())),
                setup_phase: RefCell::new(None),
                setup_dialog: RefCell::new(None),
                setup_generation: Cell::new(0),
                calibration: RefCell::new(None),
                calibration_dialog: RefCell::new(None),
                calibration_generation: Cell::new(0),
            }
        });
        let weak = Rc::downgrade(&ui);
        glib::timeout_add_local(Duration::from_millis(500), move || {
            let Some(ui) = weak.upgrade() else {
                return glib::ControlFlow::Break;
            };
            if ui.stopped.get() {
                return glib::ControlFlow::Break;
            }
            if ui.window.is_visible()
                && let Some(tail) = ui.event_tail.borrow().as_ref()
            {
                ui.device_page.set_events(&tail.latest());
            }
            glib::ControlFlow::Continue
        });
        let weak = Rc::downgrade(&ui);
        ui.window.connect_close_request(move |_| {
            if let Some(ui) = weak.upgrade() {
                if let Some(dialog) = ui
                    .uninstall
                    .borrow()
                    .as_ref()
                    .filter(|dialog| !dialog.is_closed())
                {
                    dialog.present();
                    return glib::Propagation::Stop;
                }
                if ui.tray_active()
                    && ui.latest.borrow().setup_phase == SetupPhase::Ready
                    && !ui.shutdown_requested.get()
                    && matches!(
                        ui.handle.snapshot().lifecycle,
                        Lifecycle::Starting | Lifecycle::Running
                    )
                {
                    ui.save_geometry();
                    ui.window.set_visible(false);
                } else {
                    ui.request_quit();
                }
            }
            glib::Propagation::Stop
        });
        let settings = gio::SimpleAction::new("settings", None);
        let weak = Rc::downgrade(&ui);
        settings.connect_activate(move |_, _| {
            if let Some(ui) = weak.upgrade() {
                ui.present_main_window(false);
                ui.device_settings.render(&ui.handle.snapshot());
                ui.device_settings.dialog.present(Some(&ui.window));
            }
        });
        ui.window.add_action(&settings);
        let compact_action = gio::SimpleAction::new("compact", None);
        let weak = Rc::downgrade(&ui);
        compact_action.connect_activate(move |_, _| {
            if let Some(ui) = weak.upgrade() {
                ui.compact.render(&ui.handle.snapshot());
                ui.compact_window.present();
            }
        });
        ui.window.add_action(&compact_action);
        let quit = gio::SimpleAction::new("quit", None);
        let weak = Rc::downgrade(&ui);
        quit.connect_activate(move |_, _| {
            if let Some(ui) = weak.upgrade() {
                ui.request_quit();
            }
        });
        ui.application.add_action(&quit);
        ui.application
            .set_accels_for_action("app.quit", &["<Primary>q"]);
        let present = gio::SimpleAction::new("present", None);
        let weak = Rc::downgrade(&ui);
        present.connect_activate(move |_, _| {
            if let Some(ui) = weak.upgrade() {
                ui.compact_window.set_visible(false);
                ui.activate();
            }
        });
        ui.application.add_action(&present);
        let panel = gio::SimpleAction::new("device-panel", None);
        let weak = Rc::downgrade(&ui);
        panel.connect_activate(move |_, _| {
            if let Some(ui) = weak.upgrade() {
                ui.device_settings.dialog.close();
                ui.pages.set_visible_child_name("mixer");
                ui.split.set_show_sidebar(true);
            }
        });
        ui.window.add_action(&panel);
        let mixer = gio::SimpleAction::new("mixer", None);
        let weak = Rc::downgrade(&ui);
        mixer.connect_activate(move |_, _| {
            if let Some(ui) = weak.upgrade() {
                ui.device_settings.dialog.close();
                ui.pages.set_visible_child_name("mixer");
            }
        });
        ui.window.add_action(&mixer);
        let about = gio::SimpleAction::new("about", None);
        let weak = Rc::downgrade(&ui);
        about.connect_activate(move |_, _| {
            if let Some(ui) = weak.upgrade() {
                let dialog = adw::AboutDialog::builder()
                    .application_name("CadisWave")
                    .application_icon("cadiswave")
                    .developer_name("Rama Aditya")
                    .developers(["Rama Aditya https://github.com/RamaAditya49"])
                    .version(crate::VERSION)
                    .website("https://github.com/RamaAditya49/cadiswave")
                    .copyright("2026 Rama Aditya and CADIS; 2025 rikkichy")
                    .license_type(gtk::License::MitX11)
                    .build();
                dialog.add_credit_section(Some("OpenWave"), &["rikkichy — upstream author"]);
                dialog.present(Some(&ui.window));
            }
        });
        ui.window.add_action(&about);
        let key = gtk::EventControllerKey::new();
        let weak = Rc::downgrade(&ui);
        key.connect_key_pressed(move |_, key, _, modifiers| {
            let Some(ui) = weak.upgrade() else {
                return glib::Propagation::Proceed;
            };
            if !shortcuts_allowed(gtk::prelude::GtkWindowExt::focus(&ui.window).as_ref()) {
                return glib::Propagation::Proceed;
            }
            if modifiers.contains(gtk::gdk::ModifierType::CONTROL_MASK) && key == gtk::gdk::Key::r {
                ui.submit_command(AppCommand::Reconnect);
                return glib::Propagation::Stop;
            }
            if modifiers.intersects(
                gtk::gdk::ModifierType::CONTROL_MASK
                    | gtk::gdk::ModifierType::ALT_MASK
                    | gtk::gdk::ModifierType::SUPER_MASK,
            ) {
                return glib::Propagation::Proceed;
            }
            if key == gtk::gdk::Key::m || key == gtk::gdk::Key::M {
                let p = crate::ui::device::projection::DeviceProjection::from_snapshot(
                    &ui.handle.snapshot(),
                );
                let _ = crate::ui::device::controls::DeviceControls::new(ui.handle.clone())
                    .toggle_mute(&p);
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        ui.window.add_controller(key);
        let reload = gio::SimpleAction::new("reload-interface", None);
        let weak = Rc::downgrade(&ui);
        reload.connect_activate(move |_, _| {
            if let Some(ui) = weak.upgrade() {
                ui.reload_interface();
            }
        });
        ui.window.add_action(&reload);
        let save = gio::SimpleAction::new("save-scene-as", None);
        let weak = Rc::downgrade(&ui);
        save.connect_activate(move |_, _| {
            if let Some(ui) = weak.upgrade()
                && !ui.shutdown_requested.get()
            {
                dialogs::save_scene(ui.window.upcast_ref(), ui.submit.clone());
            }
        });
        ui.window.add_action(&save);
        let uninstall_target = Rc::downgrade(&ui);
        let prepare_target = Rc::downgrade(&ui);
        *ui.registry.borrow_mut() = Some(ActionRegistry::register(
            &application,
            handle,
            ActionCallbacks {
                uninstall: Rc::new(move || {
                    if let Some(ui) = uninstall_target.upgrade() {
                        ui.show_uninstall();
                    }
                }),
                prepare_uninstall: Rc::new(move |identity| {
                    if let Some(ui) = prepare_target.upgrade() {
                        ui.prepare_uninstall(identity);
                    }
                }),
            },
        ));
        if let Some(connection) = application.dbus_connection() {
            let open = Rc::downgrade(&ui);
            let toggle = Rc::downgrade(&ui);
            let compact = Rc::downgrade(&ui);
            let quit = Rc::downgrade(&ui);
            let host = Rc::downgrade(&ui);
            match Tray::new(
                connection,
                icons,
                TrayCallbacks {
                    open: Rc::new(move || {
                        if let Some(ui) = open.upgrade() {
                            ui.activate();
                        }
                    }),
                    compact: Rc::new(move || {
                        if let Some(ui) = compact.upgrade() {
                            ui.compact.render(&ui.handle.snapshot());
                            ui.compact_window.present();
                        }
                    }),
                    toggle_mute: Rc::new(move |unit| {
                        if let Some(ui) = toggle.upgrade() {
                            ui.submit_command(AppCommand::ToggleDeviceMute { unit });
                        }
                    }),
                    quit: Rc::new(move || {
                        if let Some(ui) = quit.upgrade() {
                            ui.request_quit();
                        }
                    }),
                    host_changed: Rc::new(move |active| {
                        if let Some(ui) = host.upgrade() {
                            ui.host_changed(active);
                        }
                    }),
                },
            ) {
                Ok(tray) => {
                    let active = tray.host_active();
                    *ui.tray.borrow_mut() = Some(tray);
                    ui.host_changed(active);
                }
                Err(error) => log::warn!("Tray unavailable: {error}"),
            }
        }
        ui
    }

    fn activate(self: &Rc<Self>) {
        if self.stopped.get() {
            return;
        }
        if let Some(dialog) = self
            .uninstall
            .borrow()
            .as_ref()
            .filter(|dialog| !dialog.is_closed())
        {
            dialog.present();
            return;
        }
        let first = !self.activated.replace(true);
        if !first {
            self.start_hidden.set(false);
        }
        let snapshot = self.handle.snapshot();
        *self.latest.borrow_mut() = Arc::clone(&snapshot);
        if snapshot.setup_phase == SetupPhase::Ready {
            self.present_main_window(first && self.start_hidden.get());
        } else {
            self.window.present();
            self.render_setup(&snapshot.setup_phase);
        }
        self.refresh(snapshot);
    }

    /// Ordinary activation and first-run Continue converge here: no alternate window lifecycle.
    fn present_main_window(self: &Rc<Self>, hide_requested: bool) {
        if self.latest.borrow().setup_phase != SetupPhase::Ready {
            self.window.present();
            return;
        }
        self.main_presented.set(true);
        self.render_widgets();
        self.start_hidden.set(hide_requested);
        if hide_requested && self.tray_active() {
            self.window.set_visible(false);
            self.start_hidden.set(false);
        } else {
            self.window.present();
            if self.tray_known() {
                self.start_hidden.set(false);
            }
        }
    }

    fn tray_active(&self) -> bool {
        self.tray.borrow().as_ref().is_some_and(Tray::host_active)
    }
    fn tray_known(&self) -> bool {
        self.tray.borrow().as_ref().is_none_or(Tray::host_known)
    }
    fn host_changed(self: &Rc<Self>, active: bool) {
        if active && !self.stopped.get() {
            if self.tray_hold.borrow().is_none() {
                *self.tray_hold.borrow_mut() = Some(self.application.hold());
            }
            if self.start_hidden.get()
                && self.main_presented.get()
                && self.latest.borrow().setup_phase == SetupPhase::Ready
            {
                self.window.set_visible(false);
                self.start_hidden.set(false);
            }
        } else {
            self.start_hidden.set(false);
            // Present before releasing the last application hold.
            if self.activated.get() && !self.stopped.get() && !self.window.is_visible() {
                self.window.present();
                self.render_widgets();
            }
            self.tray_hold.borrow_mut().take();
        }
    }
    fn submit_command(self: &Rc<Self>, command: AppCommand) {
        if let Err(error) = self.handle.submit(command) {
            self.error("Operation was not accepted", &error.to_string());
        }
    }
    fn save_geometry(&self) {
        let snapshot = self.handle.snapshot();
        if !matches!(snapshot.lifecycle, Lifecycle::Running | Lifecycle::Starting) {
            return;
        }
        let maximized = self.window.is_maximized();
        let changes = PreferencesEdit {
            width: (!maximized).then(|| self.window.width().max(geometry::MIN_SIZE.0)),
            height: (!maximized).then(|| self.window.height().max(geometry::MIN_SIZE.1)),
            maximized: Some(maximized),
            ..Default::default()
        };
        if let Err(error) = self.handle.submit(AppCommand::SetPreferences { changes }) {
            log::warn!("Window geometry was not saved: {error}");
        }
    }
    fn request_quit(self: &Rc<Self>) {
        if let Some(dialog) = self
            .uninstall
            .borrow()
            .as_ref()
            .filter(|dialog| dialog.is_busy())
        {
            dialog.present();
            return;
        }
        if self.stopped.get() {
            self.application.quit();
            return;
        }
        if self.shutdown_requested.replace(true) {
            return;
        }
        self.save_geometry();
        match self.handle.submit(AppCommand::Shutdown) {
            Ok(_) => {
                self.set_status("Stopping CadisWave; waiting for owned device and audio workers…")
            }
            Err(error) => {
                self.shutdown_requested.set(false);
                self.error("Cannot stop CadisWave", &error.to_string());
            }
        }
    }
    fn prepare_uninstall(self: &Rc<Self>, identity: String) {
        if identity != self.paths.identity.to_string_lossy() {
            if let Some(registry) = self.registry.borrow().as_ref() {
                registry.set_uninstall_state(
                    "error:Installation identity does not match this running CadisWave",
                );
            }
            return;
        }
        if let Some(dialog) = self
            .uninstall
            .borrow()
            .as_ref()
            .filter(|dialog| !dialog.is_closed())
        {
            if let Some(registry) = self.registry.borrow().as_ref() {
                registry.set_uninstall_state(
                    "error:An interactive removal dialog is open; finish or cancel it before preparing removal",
                );
            }
            dialog.present();
            return;
        }
        self.save_geometry();
        match self.handle.submit(AppCommand::PrepareUninstall {
            canonical_identity: identity,
        }) {
            Ok(id) => {
                self.prepare_command.set(Some(id));
                self.shutdown_requested.set(true);
                if let Some(registry) = self.registry.borrow().as_ref() {
                    registry.set_uninstall_state("stopping");
                }
            }
            Err(error) => {
                if let Some(registry) = self.registry.borrow().as_ref() {
                    registry.set_uninstall_state(&format!("error:{error}"));
                }
            }
        }
    }
    fn show_uninstall(self: &Rc<Self>) {
        if let Some(dialog) = self
            .uninstall
            .borrow()
            .as_ref()
            .filter(|dialog| !dialog.is_closed())
        {
            dialog.present();
            return;
        }
        if self.shutdown_requested.get() {
            return;
        }
        self.setup_generation
            .set(self.setup_generation.get().wrapping_add(1));
        if let Some(dialog) = self.setup_dialog.borrow_mut().take() {
            dialog.force_close();
        }
        let confirm = Rc::downgrade(self);
        let closed = Rc::downgrade(self);
        let dialog = UninstallDialog::new(
            self.window.upcast_ref(),
            self.paths.clone(),
            Rc::new(move |plan, delete_settings| {
                let Some(ui) = confirm.upgrade() else {
                    return Err("CadisWave is no longer available".into());
                };
                ui.save_geometry();
                let id = ui
                    .handle
                    .submit(AppCommand::ConfirmUninstall {
                        plan,
                        delete_settings,
                    })
                    .map_err(|error| error.to_string())?;
                ui.uninstall_command.set(Some(id));
                ui.uninstall_result.set(false);
                ui.shutdown_requested.set(true);
                Ok(())
            }),
            Rc::new(move |confirmed| {
                if let Some(ui) = closed.upgrade() {
                    if ui.stopped.get() {
                        ui.application.quit();
                    } else if confirmed {
                        ui.shutdown_requested.set(false);
                        ui.request_quit();
                    } else {
                        *ui.setup_phase.borrow_mut() = None;
                        ui.render_setup(&ui.handle.snapshot().setup_phase);
                    }
                }
            }),
            self.inspection_workers.clone(),
        );
        *self.uninstall.borrow_mut() = Some(dialog);
    }

    fn event(self: &Rc<Self>, event: RuntimeEvent) {
        match event {
            RuntimeEvent::SnapshotChanged => self.refresh(self.handle.snapshot()),
            RuntimeEvent::UninstallFinished(result) => {
                self.uninstall_result.set(true);
                if let Some(dialog) = self.uninstall.borrow().as_ref() {
                    dialog.completed(&result);
                }
            }
            RuntimeEvent::CommandFinished { id, result } => {
                if self.device_settings.completed(id, &result) {
                    self.refresh(self.handle.snapshot());
                    return;
                }
                if self.uninstall_command.get() == Some(id) {
                    if let CommandOutcome::Rejected(error) = &result
                        && !self.uninstall_result.get()
                        && let Some(dialog) = self.uninstall.borrow().as_ref()
                    {
                        dialog.failed(&error.to_string());
                    }
                    self.uninstall_command.set(None);
                } else if self.prepare_command.get() == Some(id) {
                    if let CommandOutcome::Rejected(error) = &result
                        && let Some(registry) = self.registry.borrow().as_ref()
                    {
                        registry.set_uninstall_state(&format!("error:{error}"));
                    }
                } else {
                    match result {
                        CommandOutcome::Rejected(error) => {
                            if !self.handle.snapshot().scene_pending
                                && self.status.text()
                                    == "Applying scene; waiting for all device operations…"
                            {
                                self.set_status("Scene could not be completed.");
                            }
                            self.error("Operation could not be completed", &error.to_string());
                        }
                        CommandOutcome::SceneFinished {
                            skipped, failed, ..
                        } => {
                            if skipped.is_empty() && failed.is_empty() {
                                self.set_status("Scene applied.");
                            } else {
                                self.set_status("Scene finished with skipped or failed targets.");
                                let mut details = String::new();
                                for (label, issues) in [("Skipped", skipped), ("Failed", failed)] {
                                    for issue in issues {
                                        details.push_str(&format!(
                                            "{label} — {}: {}\n",
                                            issue.target, issue.message
                                        ));
                                    }
                                }
                                self.error("Scene partially applied", details.trim());
                            }
                        }
                        CommandOutcome::Applied { .. } | CommandOutcome::Cancelled => {}
                    }
                }
                self.refresh(self.handle.snapshot());
            }
            RuntimeEvent::ShutdownFinished(result) => match result {
                Ok(()) => {
                    self.stopped.set(true);
                    if let Some(tray) = self.tray.borrow().as_ref() {
                        tray.shutdown();
                    }
                    self.tray_hold.borrow_mut().take();
                    if self
                        .uninstall
                        .borrow()
                        .as_ref()
                        .is_some_and(|dialog| !dialog.is_closed())
                    {
                        self.window.set_visible(false);
                    } else {
                        self.application.quit();
                    }
                }
                Err(error) => {
                    self.shutdown_requested.set(false);
                    if self.handle.snapshot().lifecycle == Lifecycle::Stopped {
                        self.fatal.set(true);
                        self.stopped.set(true);
                        self.setup_generation
                            .set(self.setup_generation.get().wrapping_add(1));
                        if let Some(dialog) = self.setup_dialog.borrow_mut().take() {
                            dialog.close();
                        }
                        self.error("CadisWave could not start", &error.to_string());
                        return;
                    }
                    if let Some(registry) = self.registry.borrow().as_ref()
                        && self.prepare_command.get().is_some()
                    {
                        registry.set_uninstall_state(&format!("error:{error}"));
                    }
                    if let Some(dialog) = self
                        .uninstall
                        .borrow()
                        .as_ref()
                        .filter(|dialog| !dialog.is_closed())
                    {
                        dialog.failed(&error.to_string());
                    } else {
                        self.error("Shutdown incomplete", &format!("{error}\n\nMutations remain frozen. Move any stream named above to a non-CadisWave output, then close the window again to retry stopping owned workers."));
                    }
                }
            },
        }
    }
    fn refresh(self: &Rc<Self>, snapshot: Arc<AppSnapshot>) {
        let icon = crate::icons::status_icon(&snapshot);
        if self.status_icon.replace(Some(icon)) != Some(icon) {
            for window in [
                self.window.upcast_ref::<gtk::Window>(),
                self.compact_window.upcast_ref::<gtk::Window>(),
            ] {
                window.set_icon_name(Some(icon));
            }
        }
        if let Some(registry) = self.registry.borrow().as_ref() {
            registry.refresh(&snapshot);
        }
        if let Some(tray) = self.tray.borrow().as_ref() {
            tray.update(&snapshot);
        }
        *self.latest.borrow_mut() = snapshot.clone();
        if snapshot.revision > 0 && !self.geometry_restored.replace(true) {
            let (width, height) = geometry::fit_size(
                (snapshot.preferences.width, snapshot.preferences.height),
                monitor_dimensions(Some(&self.window)),
            );
            self.window.set_default_size(width, height);
            if snapshot.preferences.maximized {
                self.window.maximize();
            } else {
                self.window.unmaximize();
            }
        }
        if self.activated.get() {
            self.render_setup(&snapshot.setup_phase);
            self.render_calibration(snapshot.calibration.as_ref());
        }
        if snapshot.scene_pending {
            self.set_status("Applying scene; waiting for all device operations…");
        }
        if !self.render_pending.replace(true) {
            let weak = Rc::downgrade(self);
            glib::timeout_add_local_once(Duration::from_millis(50), move || {
                if let Some(ui) = weak.upgrade() {
                    ui.render_pending.set(false);
                    if ui.window.is_visible() {
                        ui.render_widgets();
                    }
                }
            });
        }
    }
    /// Replace the presentation only. The controller, its workers and routing are untouched.
    fn reload_interface(self: &Rc<Self>) {
        if self.stopped.get() {
            return;
        }
        // Fresh views render under their own guards, so the first render submits nothing.
        // Replacing the old views drops their widgets together with every signal closure.
        let matrix = MatrixView::new(self.icons.clone(), self.submit.clone());
        let sidebar = Sidebar::new(self.icons.clone(), self.submit.clone());
        self.split.set_content(Some(&matrix.widget));
        self.split.set_sidebar(Some(&sidebar.widget));
        *self.matrix.borrow_mut() = matrix;
        *self.sidebar.borrow_mut() = sidebar;
        self.rendered_revision.set(None);
        // Setup and calibration dialogs are re-presented; generations fence the old responses.
        *self.setup_phase.borrow_mut() = None;
        *self.calibration.borrow_mut() = None;
        self.refresh(self.handle.snapshot());
        self.render_widgets();
    }
    fn render_widgets(&self) {
        let snapshot = self.latest.borrow().clone();
        let locale = crate::i18n::system_locale();
        if self.i18n.borrow().locale() != snapshot.preferences.language.resolve(&locale) {
            if let Err(error) = self
                .i18n
                .borrow_mut()
                .set_choice(snapshot.preferences.language, &locale)
            {
                log::error!("{error}");
            }
            crate::i18n::retranslate();
            self.device_settings.retranslate();
            if let (Some(calibration), Some(dialog)) = (
                self.calibration.borrow().as_ref(),
                self.calibration_dialog.borrow().as_ref(),
            ) {
                crate::calibration::refresh(dialog, &calibration.phase);
            }
            self.rendered_revision.set(None);
            self.menu_button.set_menu_model(Some(&application_menu()));
        }
        self.device_page.render(&snapshot);
        self.device_settings.render(&snapshot);
        self.compact.render(&snapshot);
        let p = crate::ui::device::projection::DeviceProjection::from_snapshot(&snapshot);
        if let Some(notification) = self.connection_notifier.borrow_mut().observe(&p) {
            if let Ok(path) = self.icons.supplied_path("cadiswave") {
                notification.set_icon(&gio::FileIcon::new(&gio::File::for_path(path)));
            }
            self.application
                .send_notification(Some("cadiswave-device-connection"), &notification);
        }
        self.pages
            .page(&self.device_page.widget)
            .set_title(&crate::i18n::tr("device-page"));
        self.pages
            .page(&self.split)
            .set_title(&crate::i18n::tr("mixer"));
        let (matrix, sidebar) = (self.matrix.borrow(), self.sidebar.borrow());
        matrix.render(snapshot.clone());
        sidebar.render(snapshot.clone());
        let active = matches!(snapshot.lifecycle, Lifecycle::Starting | Lifecycle::Running)
            && snapshot.setup_phase == SetupPhase::Ready;
        matrix.widget.set_sensitive(active);
        sidebar.widget.set_sensitive(active);
        let subtitle = snapshot
            .selected_unit
            .and_then(|id| snapshot.units.iter().find(|unit| unit.id == id))
            .map(|unit| {
                let name = unit.id.profile.profile().display_name;
                if snapshot.units.len() > 1 {
                    crate::i18n::format(
                        "connected-count",
                        &[("name", name), ("count", &snapshot.units.len().to_string())],
                    )
                } else {
                    format!("{name} · {}", crate::i18n::tr("device-connected"))
                }
            })
            .unwrap_or_else(|| crate::i18n::tr("device-disconnected"));
        if self.title.subtitle() != subtitle {
            self.title.set_subtitle(&subtitle);
        }
        let mut warnings = snapshot.service_status.clone();
        for issue in snapshot
            .errors
            .iter()
            .chain(snapshot.units.iter().flat_map(|unit| unit.errors.iter()))
        {
            if !warnings.is_empty() {
                warnings.push_str("\n\n");
            }
            warnings.push_str(&format!("{}: {}", issue.target, issue.message));
        }
        if self.warning_text.text() != warnings {
            self.warning_text.set_label(&warnings);
        }
        self.warning.set_visible(!warnings.is_empty());
        if self.rendered_revision.replace(Some(snapshot.revision)) != Some(snapshot.revision) {
            let menu = gio::Menu::new();
            let recall = gio::Menu::new();
            let delete = gio::Menu::new();
            let mut scenes: Vec<_> = snapshot.desired.scenes.iter().collect();
            scenes.sort_by_key(|(_, scene)| scene.name.to_lowercase());
            for (id, scene) in scenes {
                for (section, action) in
                    [(&recall, "app.apply-scene"), (&delete, "app.delete-scene")]
                {
                    let item = gio::MenuItem::new(Some(&scene.name), None);
                    item.set_action_and_target_value(Some(action), Some(&id.as_str().to_variant()));
                    section.append_item(&item);
                }
            }
            if recall.n_items() > 0 {
                menu.append_section(None, &recall);
            }
            let manage = gio::Menu::new();
            manage.append(
                Some(&crate::i18n::translate("Save current as…")),
                Some("win.save-scene-as"),
            );
            if delete.n_items() > 0 {
                manage.append_submenu(Some(&crate::i18n::translate("Delete scene")), &delete);
            }
            menu.append_section(None, &manage);
            self.scene_button.set_menu_model(Some(&menu));
        }
        crate::i18n::bind_tree(&self.window);
        if let Some(tray) = self.tray.borrow().as_ref() {
            tray.update(&snapshot);
        }
    }
    fn set_status(&self, text: &str) {
        self.status.set_label(&crate::i18n::translate(text));
        self.status.set_visible(!text.is_empty());
    }
    fn error(&self, heading: &str, message: &str) {
        self.window.present();
        dialogs::show_error(self.window.upcast_ref(), heading, message);
    }

    fn render_setup(self: &Rc<Self>, phase: &SetupPhase) {
        log::debug!("Rendering setup phase: {phase:?}");
        if self.stopped.get()
            || self.shutdown_requested.get()
            || self
                .uninstall
                .borrow()
                .as_ref()
                .is_some_and(|dialog| !dialog.is_closed())
        {
            return;
        }
        if self.setup_phase.borrow().as_ref() == Some(phase) {
            return;
        }
        *self.setup_phase.borrow_mut() = Some(phase.clone());
        let generation = self.setup_generation.get().wrapping_add(1);
        self.setup_generation.set(generation);
        if let Some(dialog) = self.setup_dialog.borrow_mut().take() {
            dialog.force_close();
        }
        if phase == &SetupPhase::Ready {
            self.set_status("");
            if !self.main_presented.get() {
                self.present_main_window(self.start_hidden.get());
            }
            return;
        }
        if matches!(
            phase,
            SetupPhase::Checking | SetupPhase::Running | SetupPhase::Starting
        ) {
            self.set_status(match phase {
                SetupPhase::Checking => "Checking CadisWave setup",
                SetupPhase::Running => "Setting Up CadisWave",
                _ => "Starting CadisWave",
            });
            return;
        }
        let (heading, body) = match phase {
            SetupPhase::Checking => ("Checking CadisWave setup", "Inspecting USB permissions, audio configuration and the capture service. No device controls are opened during setup inspection.".to_string()),
            SetupPhase::Required => ("First-Time Setup", "CadisWave needs to configure USB permissions and install the audio service.\n\nYou may be prompted for your password.".to_string()),
            SetupPhase::Running => ("Setting Up CadisWave", "Configuring USB permissions, user audio rules, mixes and the capture service. This window remains responsive.".to_string()),
            SetupPhase::Replug(message) => ("Setup Complete", format!("{message}\n\nPlease replug your Elgato Wave device, then click Continue.")),
            SetupPhase::Failed(message) => ("Setup Failed", message.clone()),
            SetupPhase::Starting => ("Starting CadisWave", "Opening the device and audio workers. No host setup changes are made.".to_string()),
            SetupPhase::ActivationFailed(message) => ("CadisWave could not start", format!("{message}\n\nResolve the problem, then retry starting CadisWave.")),
            SetupPhase::Ready => return,
        };
        let dialog = adw::AlertDialog::builder()
            .heading(crate::i18n::translate(heading))
            .body(&body)
            .build();
        dialog.add_response(
            "cancel",
            if matches!(phase, SetupPhase::Failed(_)) {
                "Close"
            } else {
                "Cancel"
            },
        );
        dialog.set_close_response("cancel");
        if !matches!(
            phase,
            SetupPhase::Checking | SetupPhase::Running | SetupPhase::Starting
        ) {
            dialog.add_response("uninstall", "Uninstall CadisWave…");
        }
        match phase {
            SetupPhase::Required | SetupPhase::Failed(_) => {
                dialog.add_response(
                    "setup",
                    if phase == &SetupPhase::Required {
                        "Set Up"
                    } else {
                        "Retry Setup"
                    },
                );
                dialog.set_response_appearance("setup", adw::ResponseAppearance::Suggested);
                dialog.set_default_response(Some("setup"));
            }
            SetupPhase::Replug(_) | SetupPhase::ActivationFailed(_) => {
                dialog.add_response(
                    "continue",
                    if matches!(phase, SetupPhase::ActivationFailed(_)) {
                        "Retry"
                    } else {
                        "Continue"
                    },
                );
                dialog.set_response_appearance("continue", adw::ResponseAppearance::Suggested);
                dialog.set_default_response(Some("continue"));
            }
            SetupPhase::Checking | SetupPhase::Running | SetupPhase::Starting => {
                let spinner = gtk::Spinner::new();
                spinner.start();
                dialog.set_extra_child(Some(&spinner));
            }
            SetupPhase::Ready => {}
        }
        let weak = Rc::downgrade(self);
        dialog.connect_response(None, move |_, response| {
            let Some(ui) = weak.upgrade() else {
                return;
            };
            if ui.setup_generation.get() != generation {
                return;
            }
            match response {
                "setup" => ui.submit_command(AppCommand::RunSetup),
                "continue" => {
                    // Snapshot notifications may coalesce Starting and an identical failure.
                    // The dismissed dialog is no longer a presentation of that failure.
                    *ui.setup_phase.borrow_mut() = None;
                    ui.submit_command(AppCommand::ContinueSetup);
                }
                "uninstall" => ui.show_uninstall(),
                _ => ui.request_quit(),
            }
        });
        self.window.present();
        dialog.present(Some(&self.window));
        *self.setup_dialog.borrow_mut() = Some(dialog);
    }

    fn render_calibration(self: &Rc<Self>, calibration: Option<&CalibrationSnapshot>) {
        if self.calibration.borrow().as_ref() == calibration {
            return;
        }
        *self.calibration.borrow_mut() = calibration.cloned();
        let generation = self.calibration_generation.get().wrapping_add(1);
        self.calibration_generation.set(generation);
        if let Some(dialog) = self.calibration_dialog.borrow_mut().take() {
            dialog.close();
        }
        let Some(calibration) = calibration else {
            return;
        };
        if self.shutdown_requested.get() {
            return;
        }
        let heading = match &calibration.phase {
            CalibrationPhase::NoiseReady => "Measure room noise",
            CalibrationPhase::RecordingNoise => "Recording room noise",
            CalibrationPhase::SpeechReady => "Measure speech",
            CalibrationPhase::RecordingSpeech => "Recording speech",
            CalibrationPhase::Review { .. } => "Review calibration",
            CalibrationPhase::Expired(_) => "Calibration expired",
        };
        let body = crate::calibration::body(&calibration.phase);
        let dialog = adw::AlertDialog::builder()
            .heading(crate::i18n::translate(heading))
            .body(&body)
            .build();
        dialog.add_response(
            "cancel",
            &crate::i18n::translate(
                if matches!(calibration.phase, CalibrationPhase::Review { .. }) {
                    "Keep current settings"
                } else {
                    "Cancel"
                },
            ),
        );
        dialog.set_close_response("cancel");
        dialog.set_default_response(Some("cancel"));
        match calibration.phase {
            CalibrationPhase::NoiseReady | CalibrationPhase::SpeechReady => {
                dialog.add_response("record", &crate::i18n::translate("Record"));
                dialog.set_response_appearance("record", adw::ResponseAppearance::Suggested);
            }
            CalibrationPhase::Review { .. } => {
                dialog.add_response("apply", &crate::i18n::translate("Apply"));
                dialog.set_response_appearance("apply", adw::ResponseAppearance::Suggested);
            }
            CalibrationPhase::RecordingNoise | CalibrationPhase::RecordingSpeech => {
                let spinner = gtk::Spinner::new();
                spinner.start();
                dialog.set_extra_child(Some(&spinner));
            }
            CalibrationPhase::Expired(_) => {}
        }
        let token = calibration.token.clone();
        let phase = calibration.phase.clone();
        let weak = Rc::downgrade(self);
        dialog.connect_response(None, move |_, response| {
            let Some(ui) = weak.upgrade() else {
                return;
            };
            if ui.calibration_generation.get() != generation {
                return;
            }
            let command = match (response, &phase) {
                ("record", CalibrationPhase::NoiseReady) => AppCommand::RecordNoise {
                    token: token.clone(),
                },
                ("record", CalibrationPhase::SpeechReady) => AppCommand::RecordSpeech {
                    token: token.clone(),
                },
                ("apply", CalibrationPhase::Review { .. }) => AppCommand::AcceptCalibration {
                    token: token.clone(),
                },
                _ => AppCommand::CancelCalibration {
                    token: token.clone(),
                },
            };
            ui.submit_command(command);
        });
        self.window.present();
        dialog.present(Some(&self.window));
        *self.calibration_dialog.borrow_mut() = Some(dialog);
    }
}

fn application_menu() -> gio::Menu {
    let menu = gio::Menu::new();
    menu.append(
        Some(&crate::i18n::tr("compact-controls")),
        Some("win.compact"),
    );
    menu.append(Some(&crate::i18n::tr("about")), Some("win.about"));
    menu.append(
        Some(&crate::i18n::translate("Settings")),
        Some("win.settings"),
    );
    menu.append(
        Some(&crate::i18n::translate("Reload interface")),
        Some("win.reload-interface"),
    );
    menu.append(Some(&crate::i18n::translate("Quit")), Some("app.quit"));
    menu.append(
        Some(&crate::i18n::translate("Uninstall CadisWave…")),
        Some("app.uninstall"),
    );
    menu
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::test_support::{Rig, descendants};
    use std::time::Instant;

    #[test]
    #[ignore = "requires the isolated installed GTK test runner"]
    fn status_icon_updates_windows_and_read_only_action_without_a_revision_change() {
        let (_rig, ui) = fixture();
        let mut snapshot = (*ui.handle.snapshot()).clone();
        let unit = crate::ui::test_support::unit("status fixture", 1, 0.0);
        snapshot.selected_unit = Some(unit.id);
        snapshot.units = Arc::new(vec![unit]);
        Arc::make_mut(&mut snapshot.units)[0].desired_mute = Some(false);
        for (mute, failure, expected) in [
            (false, false, "cadiswave-green"),
            (true, false, "cadiswave-red"),
            (true, true, "cadiswave-orange"),
            (true, false, "cadiswave-red"),
        ] {
            Arc::make_mut(&mut snapshot.units)[0].desired_mute = Some(mute);
            snapshot.errors = Arc::new(if failure {
                vec![OperationIssue {
                    target: "fixture".into(),
                    message: "Effects unavailable".into(),
                }]
            } else {
                vec![]
            });
            ui.refresh(Arc::new(snapshot.clone()));
            assert_eq!(ui.window.icon_name().as_deref(), Some(expected));
            assert_eq!(ui.compact_window.icon_name().as_deref(), Some(expected));
            assert_eq!(
                ui.application
                    .action_state("status-icon")
                    .unwrap()
                    .get::<String>()
                    .as_deref(),
                Some(expected)
            );
            ui.application
                .change_action_state("status-icon", &"cadiswave-green".to_variant());
            assert_eq!(
                ui.application
                    .action_state("status-icon")
                    .unwrap()
                    .get::<String>()
                    .as_deref(),
                Some(expected)
            );
        }
        ui.window.destroy();
    }

    fn assert_main_controls_in_view(ui: &AppUi) {
        let viewport = &ui.device_page.widget;
        let width = viewport.width() as f32;
        let height = viewport.height() as f32;
        for (name, control) in [
            ("mute", ui.device_page.mute.upcast_ref::<gtk::Widget>()),
            (
                "dial slider",
                ui.device_page.knob.scale.upcast_ref::<gtk::Widget>(),
            ),
        ] {
            assert!(control.is_mapped(), "{name} is not mapped");
            let bounds = control.compute_bounds(viewport).unwrap();
            assert!(bounds.width() > 0.0 && bounds.height() > 0.0);
            assert!(
                bounds.x() >= -1.0
                    && bounds.y() >= -1.0
                    && bounds.x() + bounds.width() <= width + 1.0
                    && bounds.y() + bounds.height() <= height + 1.0,
                "{name} bounds {bounds:?} exceed viewport {width} x {height}"
            );
        }
    }

    #[test]
    #[ignore = "requires the isolated installed GTK test runner"]
    fn main_window_geometry_and_controls_respect_minimum_size() {
        adw::init().unwrap();
        let data = crate::ui::test_support::asset_paths().data;
        let css = if data.join("style.css").exists() {
            data.join("style.css")
        } else {
            data.join("data/style.css")
        };
        let provider = gtk::CssProvider::new();
        provider.load_from_path(css);
        gtk::style_context_add_provider_for_display(
            &gtk::gdk::Display::default().unwrap(),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        let rig = Rig::new(
            serde_json::json!({}),
            vec![crate::ui::test_support::unit("Geometry", 2, -12.0)],
        );
        let application = adw::Application::builder()
            .application_id("io.github.RamaAditya49.CadisWave.GeometryTest")
            .build();
        application.register(None::<&gio::Cancellable>).unwrap();
        let ui = AppUi::new(application, rig.paths(), rig.handle());
        let preferences = rig.snapshot().preferences.clone();
        assert_eq!(
            (ui.window.default_width(), ui.window.default_height()),
            geometry::fit_size(
                (preferences.width, preferences.height),
                monitor_dimensions(None)
            )
        );
        assert_eq!(
            (ui.window.width_request(), ui.window.height_request()),
            geometry::MIN_SIZE
        );
        assert!(ui.window.is_resizable());
        let mut snapshot = (*rig.snapshot()).clone();
        let preferences = Arc::make_mut(&mut snapshot.preferences);
        preferences.width = i32::MAX;
        preferences.height = i32::MAX;
        ui.refresh(Arc::new(snapshot));
        assert_eq!(
            (ui.window.default_width(), ui.window.default_height()),
            geometry::fit_size((i32::MAX, i32::MAX), monitor_dimensions(Some(&ui.window)))
        );
        for (width, height) in [(800, 600), (640, 480), geometry::MIN_SIZE] {
            ui.window.set_default_size(width, height);
            ui.window.present();
            let deadline = Instant::now() + Duration::from_millis(300);
            while Instant::now() < deadline {
                while glib::MainContext::default().pending() {
                    glib::MainContext::default().iteration(false);
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            assert_eq!(
                {
                    let surface = ui.window.surface().unwrap();
                    (surface.width(), surface.height())
                },
                (
                    width.max(geometry::MIN_SIZE.0),
                    height.max(geometry::MIN_SIZE.1)
                )
            );
            assert!(ui.title.is_visible());
            assert_main_controls_in_view(&ui);
        }
        assert_eq!(rig.device_command_count(), 0);
        ui.window.set_visible(false);
    }

    fn calibration_fixture() -> (Rig, Rc<AppUi>, CalibrationToken) {
        adw::init().unwrap();
        let rig = Rig::new(serde_json::json!({}), vec![]);
        let application = adw::Application::builder()
            .application_id("io.github.RamaAditya49.CadisWave.CalibrationTest")
            .build();
        application.register(None::<&gio::Cancellable>).unwrap();
        let ui = AppUi::new(application, rig.paths(), rig.handle());
        let token = CalibrationToken {
            session: 1,
            source: SourceId::new("fixture").unwrap(),
            node_name: "fixture".into(),
            identity: NodeIdentity {
                server_cookie: 1,
                object_serial: "1".into(),
            },
            channels: 1,
        };
        (rig, ui, token)
    }
    #[test]
    #[ignore = "Requires the isolated GTK runner"]
    fn calibration_review_translates_measurements_and_proposal() {
        let (_rig, ui, token) = calibration_fixture();
        for (language, expected) in [
            (
                cadiswave_core::locale::LanguageChoice::English,
                "Noise floor:",
            ),
            (
                cadiswave_core::locale::LanguageChoice::Indonesian,
                "Batas kebisingan:",
            ),
        ] {
            ui.i18n.borrow_mut().set_choice(language, "en").unwrap();
            crate::i18n::activate(ui.i18n.clone());
            ui.render_calibration(None);
            ui.render_calibration(Some(&CalibrationSnapshot {
                token: token.clone(),
                phase: CalibrationPhase::Review {
                    proposal: cadiswave_core::effects::FxSettings {
                        mono: true,
                        ..Default::default()
                    },
                    summary: CalibrationMeasurements {
                        noise_floor_db: -60.0,
                        quiet_voice_db: -30.0,
                        loud_voice_db: -12.0,
                        quiet_channel: true,
                    },
                },
            }));
            let dialog = ui.calibration_dialog.borrow();
            let body = dialog.as_ref().unwrap().body();
            assert!(body.contains(expected), "{body}");
            assert!(body.contains("-60.0 dB"));
            assert!(body.contains("-30.0 / -12.0 dB"));
            assert_eq!(
                body.matches(
                    if language == cadiswave_core::locale::LanguageChoice::English {
                        "Gate:"
                    } else {
                        "Ambang gate:"
                    }
                )
                .count(),
                1
            );
            assert!(body.contains(
                if language == cadiswave_core::locale::LanguageChoice::English {
                    "one channel is very quiet"
                } else {
                    "satu kanal sangat pelan"
                }
            ));
            assert_eq!(
                dialog.as_ref().unwrap().heading().unwrap(),
                crate::i18n::tr("ui-review-calibration")
            );
            assert!(body.contains(
                if language == cadiswave_core::locale::LanguageChoice::English {
                    "Compressor:"
                } else {
                    "Kompresor:"
                }
            ));
        }
        let generation = ui.calibration_generation.get();
        let mut snapshot = (*ui.latest.borrow()).as_ref().clone();
        Arc::make_mut(&mut snapshot.preferences).language =
            cadiswave_core::locale::LanguageChoice::English;
        *ui.latest.borrow_mut() = Arc::new(snapshot);
        ui.render_widgets();
        let dialog = ui.calibration_dialog.borrow();
        assert!(dialog.as_ref().unwrap().body().contains("Noise floor:"));
        assert_eq!(
            dialog.as_ref().unwrap().heading().unwrap(),
            "Review calibration"
        );
        assert_eq!(ui.calibration_generation.get(), generation);
    }
    #[test]
    #[ignore = "Requires the isolated GTK runner"]
    fn calibration_expiry_translates_recovery_and_preserves_error() {
        let (_rig, ui, token) = calibration_fixture();
        for (language, expected) in [
            (
                cadiswave_core::locale::LanguageChoice::English,
                "No proposed settings were applied.",
            ),
            (
                cadiswave_core::locale::LanguageChoice::Indonesian,
                "Tidak ada pengaturan usulan yang diterapkan.",
            ),
        ] {
            ui.i18n.borrow_mut().set_choice(language, "en").unwrap();
            crate::i18n::activate(ui.i18n.clone());
            ui.render_calibration(None);
            ui.render_calibration(Some(&CalibrationSnapshot {
                token: token.clone(),
                phase: CalibrationPhase::Expired("EXACT_BACKEND_ERROR: input replaced".into()),
            }));
            let dialog = ui.calibration_dialog.borrow();
            let body = dialog.as_ref().unwrap().body();
            assert!(body.starts_with("EXACT_BACKEND_ERROR: input replaced\n\n"));
            assert!(body.contains(expected), "{body}");
        }
    }
    #[test]
    #[ignore = "requires the isolated installed GTK test runner"]
    fn device_gallery_renders_both_languages_and_small_windows() {
        adw::init().unwrap();
        let mut unit = crate::ui::test_support::unit("Gallery", 2, -12.0);
        if let Observation::Known(state) = &mut unit.state {
            state.gain_raw = 50 * 256;
        }
        let rig = Rig::new(serde_json::json!({}), vec![unit]);
        let application = adw::Application::builder()
            .application_id("io.github.RamaAditya49.CadisWave.Gallery")
            .build();
        application.register(None::<&gio::Cancellable>).unwrap();
        let ui = AppUi::new(application, rig.paths(), rig.handle());
        let data = crate::ui::test_support::asset_paths().data;
        let css = if data.join("style.css").exists() {
            data.join("style.css")
        } else {
            data.join("data/style.css")
        };
        let provider = gtk::CssProvider::new();
        provider.load_from_path(css);
        gtk::style_context_add_provider_for_display(
            &gtk::gdk::Display::default().unwrap(),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        let output = std::env::var_os("CADISWAVE_GALLERY_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::PathBuf::from("/work/evidence/device-gallery"));
        std::fs::create_dir_all(&output).unwrap();
        fn settle() {
            let until = Instant::now() + Duration::from_millis(250);
            while Instant::now() < until {
                while glib::MainContext::default().pending() {
                    glib::MainContext::default().iteration(false);
                }
                std::thread::sleep(Duration::from_millis(5));
            }
        }
        let shot = |name: &str| {
            settle();
            let args = vec![
                "-window".into(),
                "root".into(),
                output
                    .join(format!("{name}.png"))
                    .to_string_lossy()
                    .into_owned(),
            ];
            cadiswave_runtime::process::CommandRunner::default()
                .run("import", &args, Duration::from_secs(3))
                .unwrap();
        };
        ui.window.set_default_size(1280, 800);
        ui.window.present();
        settle();
        for (name, language) in [
            ("en", cadiswave_core::locale::LanguageChoice::English),
            ("id", cadiswave_core::locale::LanguageChoice::Indonesian),
        ] {
            let mut snapshot = (*rig.snapshot()).clone();
            snapshot.service_state = ServiceState::Running;
            let mut source = Source::new("Wave XLR".into(), SourceKind::Device);
            source.id = SourceId::new("gallery").unwrap();
            source.node_name = "fixture_wave".into();
            Arc::make_mut(&mut snapshot.desired)
                .sources
                .insert(source.id.clone(), source);
            Arc::make_mut(&mut snapshot.captures).push(CaptureSnapshot {
                identity: NodeIdentity {
                    server_cookie: 1,
                    object_serial: "1".into(),
                },
                node_id: 1,
                node_name: "fixture_wave".into(),
                name: "Wave XLR".into(),
                muted: Observation::Known(false),
                channels: Some(2),
                properties: serde_json::json!({"device.serial":"Gallery"})
                    .as_object()
                    .unwrap()
                    .clone(),
            });
            Arc::make_mut(&mut snapshot.channel_meters).insert(
                "src:gallery".into(),
                cadiswave_core::pcm::ChannelPeaks::Stereo {
                    left: 0.12,
                    right: 0.08,
                },
            );

            Arc::make_mut(&mut snapshot.preferences).language = language;
            *ui.latest.borrow_mut() = Arc::new(snapshot);
            ui.render_widgets();
            shot(&format!("device-{name}"));
            ui.device_settings.dialog.present(Some(&ui.window));
            for scroll in descendants::<gtk::ScrolledWindow>(&ui.device_settings.dialog) {
                scroll.vadjustment().set_value(0.0);
            }
            shot(&format!("settings-{name}"));
            for scroll in descendants::<gtk::ScrolledWindow>(&ui.device_settings.dialog) {
                if scroll.is_mapped() {
                    let adjustment = scroll.vadjustment();
                    adjustment.set_value((adjustment.upper() - adjustment.page_size()) / 2.0);
                }
            }
            shot(&format!("settings-microphone-{name}"));
            for scroll in descendants::<gtk::ScrolledWindow>(&ui.device_settings.dialog) {
                if scroll.is_mapped() {
                    let adjustment = scroll.vadjustment();
                    adjustment.set_value(adjustment.upper() - adjustment.page_size());
                }
            }
            shot(&format!("settings-footer-{name}"));
            ui.device_settings.dialog.close();
            settle();
            ui.compact_window.present();
            shot(&format!("compact-{name}"));
            ui.compact_window.set_visible(false);
            ui.window.lookup_action("about").unwrap().activate(None);
            shot(&format!("about-{name}"));
            ui.window.visible_dialog().unwrap().close();
            settle();
        }
        ui.window.set_default_size(1024, 768);
        shot("device-1024");
        ui.window
            .set_default_size(geometry::MIN_SIZE.0, geometry::MIN_SIZE.1);
        shot("device-minimum");
        assert_main_controls_in_view(&ui);
        let dialog = adw::AlertDialog::builder()
            .heading(crate::i18n::tr("rate-confirm-title"))
            .body(crate::i18n::tr("rate-confirm-body"))
            .build();
        dialog.add_response("cancel", &crate::i18n::tr("rate-confirm-cancel"));
        dialog.add_response("apply", &crate::i18n::tr("rate-confirm-apply"));
        dialog.set_response_enabled("apply", false);
        dialog.present(Some(&ui.window));
        shot("rate-confirm-controlled");
        dialog.close();
        assert_eq!(rig.device_command_count(), 0);
        ui.window.set_visible(false);
    }

    fn fixture() -> (Rig, Rc<AppUi>) {
        adw::init().expect("private GTK display");
        let rig = Rig::new(serde_json::json!({}), vec![]);
        let application = adw::Application::builder()
            .application_id("io.github.RamaAditya49.CadisWave.WidgetTest")
            .build();
        application.register(None::<&gio::Cancellable>).unwrap();
        let ui = AppUi::new(application, rig.paths(), rig.handle());
        ui.window.present();
        (rig, ui)
    }

    fn until(rig: &Rig, ui: &Rc<AppUi>, condition: impl Fn() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            for event in rig.events() {
                ui.event(event);
            }
            while glib::MainContext::default().pending() {
                glib::MainContext::default().iteration(false);
            }
            rig.snapshot();
            if condition() {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "GTK runtime observation deadline"
            );
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    #[test]
    #[ignore = "requires the isolated installed GTK test runner"]
    fn setup_ready_closes_a_dialog_before_its_open_animation_finishes() {
        let (rig, ui) = fixture();
        ui.activate();
        until(&rig, &ui, || ui.main_presented.get());
        ui.render_setup(&SetupPhase::Checking);
        ui.render_setup(&SetupPhase::Ready);
        let deadline = Instant::now() + Duration::from_millis(500);
        while Instant::now() < deadline {
            while glib::MainContext::default().pending() {
                glib::MainContext::default().iteration(false);
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        assert!(
            ui.window.visible_dialog().is_none(),
            "completed setup leaves a modal dialog"
        );
        ui.window.set_visible(false);
    }

    #[test]
    fn application_menu_offers_interface_reload() {
        let menu = application_menu();
        let actions: Vec<_> = (0..menu.n_items())
            .filter_map(|index| {
                menu.item_attribute_value(index, "action", None)
                    .and_then(|value| value.get::<String>())
            })
            .collect();
        assert_eq!(
            actions,
            [
                "win.compact",
                "win.about",
                "win.settings",
                "win.reload-interface",
                "app.quit",
                "app.uninstall"
            ]
        );
    }

    #[test]
    #[ignore = "requires the isolated installed GTK test runner"]
    fn reload_interface_replaces_views_without_commands() {
        let (rig, ui) = fixture();
        ui.activate();
        until(&rig, &ui, || ui.main_presented.get());
        until(&rig, &ui, || !ui.render_pending.get());
        let before = rig.snapshot();
        let old: Vec<_> = [
            ui.matrix.borrow().widget.clone().upcast::<gtk::Widget>(),
            ui.sidebar.borrow().widget.clone().upcast(),
        ]
        .iter()
        .flat_map(descendants::<gtk::Widget>)
        .map(|widget| widget.downgrade())
        .collect();
        WidgetExt::activate_action(&ui.window, "win.reload-interface", None).unwrap();
        until(&rig, &ui, || !ui.render_pending.get());
        assert_eq!(
            ui.split.content(),
            Some(ui.matrix.borrow().widget.clone().upcast())
        );
        assert_eq!(
            ui.split.sidebar(),
            Some(ui.sidebar.borrow().widget.clone().upcast())
        );
        assert_eq!(ui.rendered_revision.get(), Some(before.revision));
        // Finalized widgets take every signal closure, so no old control can double-submit.
        assert!(old.iter().all(|widget| widget.upgrade().is_none()));
        // Rebuilding renders the snapshot under the new views' guards and submits nothing.
        assert_eq!(rig.snapshot().revision, before.revision);
        assert!(ui.window.visible_dialog().is_none());
        ui.window.destroy();
    }

    #[test]
    #[ignore = "requires the isolated installed GTK test runner"]
    fn open_removal_dialog_reports_immediate_prepare_conflict() {
        let (rig, ui) = fixture();
        ui.application.activate_action("uninstall", None);
        let dialog = ui.uninstall.borrow().as_ref().unwrap().clone();
        assert!(!dialog.is_closed() && !dialog.is_busy());
        let identity = ui.paths.identity.to_string_lossy().to_string().to_variant();
        ui.application
            .activate_action("prepare-uninstall", Some(&identity));
        let state = ui
            .application
            .action_state("prepare-uninstall")
            .unwrap()
            .get::<String>()
            .unwrap();
        assert!(
            state.starts_with("error:"),
            "public caller needs an immediate conflict, got {state}"
        );
        assert!(!ui.shutdown_requested.get());
        assert_eq!(
            rig.shutdown_attempts(),
            0,
            "read-only confirmation must not freeze the runtime"
        );
        assert!(dialog.window.is_visible());
        dialog.window.close();
        until(&rig, &ui, || dialog.is_closed());
        // Cancellation resolves the conflict; the same public request can then finish.
        ui.application
            .activate_action("prepare-uninstall", Some(&identity));
        until(&rig, &ui, || ui.stopped.get());
        assert_eq!(rig.shutdown_attempts(), 1);
        until(&rig, &ui, || {
            ui.inspection_workers
                .borrow()
                .iter()
                .all(JoinHandle::is_finished)
        });
        for worker in ui.inspection_workers.borrow_mut().drain(..) {
            worker.join().expect("read-only inspection worker");
        }
        ui.window.destroy();
    }

    #[test]
    #[ignore = "requires the isolated installed GTK test runner"]
    fn close_retries_failed_shutdown_with_active_tray_host() {
        let (rig, ui) = fixture();
        let bus = ui
            .application
            .dbus_connection()
            .expect("private session bus");
        let xml = gio::DBusNodeInfo::for_xml(
            r#"<node>
            <interface name="org.kde.StatusNotifierWatcher">
                <method name="RegisterStatusNotifierItem"><arg type="s" direction="in"/></method>
                <property name="IsStatusNotifierHostRegistered" type="b" access="read"/>
                <property name="ProtocolVersion" type="i" access="read"/>
            </interface>
        </node>"#,
        )
        .unwrap();
        let interface = xml
            .lookup_interface("org.kde.StatusNotifierWatcher")
            .unwrap();
        let registration = bus
            .register_object("/StatusNotifierWatcher", &interface)
            .method_call(|_, _, _, _, method, parameters, invocation| {
                if method == "RegisterStatusNotifierItem" && parameters.get::<(String,)>().is_some()
                {
                    invocation.return_value(None);
                } else {
                    invocation.return_dbus_error(
                        "org.freedesktop.DBus.Error.InvalidArgs",
                        "Unexpected fixture request",
                    );
                }
            })
            .property(|_, _, _, _, name| match name {
                "IsStatusNotifierHostRegistered" => true.to_variant(),
                "ProtocolVersion" => 0_i32.to_variant(),
                _ => unreachable!("declared watcher properties only"),
            })
            .build()
            .unwrap();
        let owner = gio::bus_own_name_on_connection(
            &bus,
            "org.kde.StatusNotifierWatcher",
            gio::BusNameOwnerFlags::DO_NOT_QUEUE,
            |_, _| {},
            |_, _| {},
        );
        until(&rig, &ui, || ui.tray_active());
        assert_eq!(ui.latest.borrow().setup_phase, SetupPhase::Ready);
        rig.fail_first_shutdown();
        ui.request_quit();
        until(&rig, &ui, || {
            rig.shutdown_attempts() == 1
                && !ui.shutdown_requested.get()
                && ui.window.visible_dialog().is_some()
        });
        assert!(!ui.stopped.get());
        until(&rig, &ui, || {
            if let Some(error) = ui.window.visible_dialog() {
                error.close();
                false
            } else {
                true
            }
        });
        assert!(!matches!(
            rig.snapshot().lifecycle,
            Lifecycle::Starting | Lifecycle::Running
        ));
        assert!(
            ui.tray_active(),
            "the tray host must remain present across the failure"
        );
        ui.window.close();
        until(&rig, &ui, || ui.stopped.get());
        assert_eq!(
            rig.shutdown_attempts(),
            2,
            "Close must retry rather than hide frozen ownership"
        );
        ui.window.destroy();
        gio::bus_unown_name(owner);
        bus.unregister_object(registration).unwrap();
    }
}
