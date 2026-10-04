use cadiswave_core::model::{
    AppSnapshot, Lifecycle, OperationError, Result, ServiceState, SetupPhase,
};
use cadiswave_runtime::paths::RuntimePaths;
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    path::PathBuf,
    rc::Rc,
};

/// Errors take priority over mute. Any muted device keeps the red warning.
pub(crate) fn status_icon(snapshot: &AppSnapshot) -> &'static str {
    if !snapshot.errors.is_empty()
        || snapshot.service_state == ServiceState::Failed
        || matches!(snapshot.lifecycle, Lifecycle::Frozen)
        || matches!(
            snapshot.setup_phase,
            SetupPhase::Failed(_) | SetupPhase::ActivationFailed(_)
        )
        || snapshot
            .units
            .iter()
            .any(|unit| !unit.errors.is_empty() || unit.effective_mute().is_none())
    {
        "cadiswave-orange"
    } else if snapshot
        .units
        .iter()
        .any(|unit| unit.effective_mute() == Some(true))
    {
        "cadiswave-red"
    } else if !snapshot.units.is_empty() {
        "cadiswave-green"
    } else if snapshot.preferences.tray_icon_color == "black" {
        "cadiswave-black"
    } else {
        "cadiswave-white"
    }
}

/// Rendering choices never replace the icon name stored in a source or preference.
#[derive(Clone)]
pub struct Icons {
    supplied: Rc<HashMap<String, Result<PathBuf>>>,
    resolved: Rc<RefCell<HashMap<String, String>>>,
    watched: Rc<Cell<bool>>,
}

impl Icons {
    pub fn new(paths: RuntimePaths) -> Self {
        if let Some(display) = gtk::gdk::Display::default() {
            if let Ok(path) = paths.data_file("icons/cadiswave.svg")
                && let Some(directory) = path.parent()
            {
                let theme = gtk::IconTheme::for_display(&display);
                if !theme.search_path().iter().any(|path| path == directory) {
                    theme.add_search_path(directory);
                }
            }
            gtk::Window::set_default_icon_name("cadiswave");
        }
        let supplied = [
            "cadiswave",
            "cadiswave-white",
            "cadiswave-black",
            "cadiswave-red",
            "cadiswave-green",
            "cadiswave-orange",
        ]
        .into_iter()
        .map(|name| {
            (
                name.to_owned(),
                paths.data_file(&format!("icons/{name}.svg")),
            )
        })
        .collect();
        let resolved = Rc::new(RefCell::new(HashMap::new()));
        Self {
            supplied: Rc::new(supplied),
            resolved,
            watched: Rc::new(Cell::new(false)),
        }
    }

    pub fn supplied_path(&self, name: &str) -> Result<PathBuf> {
        self.supplied
            .get(name.strip_suffix(".svg").unwrap_or(name))
            .cloned()
            .unwrap_or_else(|| Err(OperationError::invalid("Not supplied CadisWave artwork")))
    }

    pub fn image(&self, name: &str, size: i32) -> gtk::Image {
        let image = if let Ok(path) = self.supplied_path(name) {
            gtk::Image::from_file(path)
        } else {
            gtk::Image::from_icon_name(&self.resolve(name))
        };
        image.set_pixel_size(size);
        image
    }

    pub(crate) fn theme_path(&self, name: &str) -> String {
        self.supplied_path(name)
            .ok()
            .and_then(|path| path.parent().map(|p| p.to_string_lossy().into_owned()))
            .unwrap_or_default()
    }

    pub(crate) fn resolve(&self, name: &str) -> String {
        if let Some(chosen) = self.resolved.borrow().get(name) {
            return chosen.clone();
        }
        let Some(display) = gtk::gdk::Display::default() else {
            return name.to_owned();
        };
        let theme = gtk::IconTheme::for_display(&display);
        if !self.watched.replace(true) {
            let cache = Rc::downgrade(&self.resolved);
            theme.connect_changed(move |_| {
                if let Some(cache) = cache.upgrade() {
                    cache.borrow_mut().clear();
                }
            });
        }
        let alternatives: &[&str] = match name {
            "web-browser-symbolic" => &[
                "internet-web-browser-symbolic",
                "applications-internet-symbolic",
                "globe-symbolic",
            ],
            "input-gaming-symbolic" => &["applications-games-symbolic", "input-gamepad-symbolic"],
            "audio-x-generic-symbolic" => {
                &["multimedia-player-symbolic", "media-optical-audio-symbolic"]
            }
            "list-drag-handle-symbolic" => &["view-list-symbolic", "open-menu-symbolic"],
            "network-transmit-symbolic" => &["network-wired-symbolic", "network-connect-symbolic"],
            "preferences-desktop-multimedia-symbolic" => &[
                "multimedia-player-symbolic",
                "applications-multimedia-symbolic",
            ],
            "video-display-symbolic" => {
                &["computer-symbolic", "preferences-desktop-display-symbolic"]
            }
            _ => &[],
        };
        let chosen = if theme.has_icon(name) {
            name
        } else {
            alternatives
                .iter()
                .copied()
                .find(|candidate| theme.has_icon(candidate))
                .unwrap_or(name)
        }
        .to_owned();
        self.resolved
            .borrow_mut()
            .insert(name.to_owned(), chosen.clone());
        chosen
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gio::prelude::FileExt;

    #[test]
    #[ignore = "Requires the isolated GTK runner"]
    fn supplied_artwork_resolves_without_a_global_icon_installation() {
        gtk::init().unwrap();
        let display = gtk::gdk::Display::default().unwrap();
        let theme = gtk::IconTheme::for_display(&display);
        theme.set_search_path(&[] as &[&std::path::Path]);
        assert!(!theme.has_icon("cadiswave"));

        let icons = Icons::new(crate::ui::test_support::asset_paths());
        assert!(theme.has_icon("cadiswave"), "Supplied artwork is missing");
        let icon = theme.lookup_icon(
            "cadiswave",
            &[],
            128,
            1,
            gtk::TextDirection::None,
            gtk::IconLookupFlags::empty(),
        );
        let file = icon.file().unwrap().path().unwrap();
        let supplied = icons.supplied_path("cadiswave").unwrap();
        assert_eq!(file.parent(), supplied.parent());
        assert_eq!(file.file_stem().unwrap(), "cadiswave");
        assert_eq!(
            gtk::Window::default_icon_name().as_deref(),
            Some("cadiswave")
        );
        for name in ["cadiswave-green", "cadiswave-red", "cadiswave-orange"] {
            assert!(theme.has_icon(name), "Missing status artwork: {name}");
            let icon = theme.lookup_icon(
                name,
                &[],
                32,
                1,
                gtk::TextDirection::None,
                gtk::IconLookupFlags::empty(),
            );
            assert_eq!(
                icon.file().unwrap().path().unwrap(),
                icons.supplied_path(name).unwrap()
            );
        }
    }
}
