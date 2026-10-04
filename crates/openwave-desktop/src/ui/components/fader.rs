use super::scale::SnapshotScale;
use gtk::prelude::*;
use std::{cell::Cell, rc::Rc};

#[derive(Clone, Copy)]
pub(crate) enum LevelMute {
    Independent,
    FollowLevel,
}

#[derive(Clone, Copy)]
pub(crate) enum FaderChange {
    Level { level: f64, muted: bool },
    Mute { level: f64, muted: bool },
}

#[derive(Clone)]
pub(crate) struct Fader {
    pub(crate) widget: gtk::Box,
    pub(crate) scale: gtk::Scale,
    pub(crate) mute: gtk::ToggleButton,
    percent: gtk::Label,
    level: Rc<SnapshotScale<()>>,
    rendered_mute: Rc<Cell<Option<bool>>>,
    updating: Rc<Cell<bool>>,
    capture: bool,
}

impl Fader {
    pub(crate) fn new(capture: bool, label: &str) -> Self {
        let widget = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let mute = gtk::ToggleButton::builder()
            .valign(gtk::Align::Center)
            .build();
        mute.add_css_class("flat");
        mute.add_css_class("circular");
        let scale = gtk::Scale::with_range(gtk::Orientation::Horizontal, 0.0, 1.0, 0.01);
        scale.set_draw_value(false);
        scale.set_round_digits(2);
        scale.set_hexpand(true);
        scale.set_valign(gtk::Align::Center);
        scale.add_css_class("openwave-mix-slider");
        scale.set_tooltip_text(Some(label));
        scale.update_property(&[gtk::accessible::Property::Label(label)]);
        let percent = gtk::Label::builder()
            .label("0%")
            .xalign(1.0)
            .width_chars(4)
            .build();
        for class in ["caption", "dim-label", "monospace"] {
            percent.add_css_class(class);
        }
        widget.append(&mute);
        widget.append(&scale);
        widget.append(&percent);
        Self {
            widget,
            level: Rc::new(SnapshotScale::new(scale.clone())),
            scale,
            mute,
            percent,
            rendered_mute: Rc::new(Cell::new(None)),
            updating: Rc::new(Cell::new(false)),
            capture,
        }
    }

    pub(crate) fn render(&self, level: f64, muted: bool) {
        self.updating.set(true);
        if self.level.render(Some(((), level))) {
            self.percent.set_label(&format!("{:.0}%", level * 100.0));
        }
        if self.rendered_mute.replace(Some(muted)) != Some(muted) {
            self.mute.set_active(muted);
        }
        mute_feedback(&self.widget, &self.mute, self.capture);
        self.updating.set(false);
    }

    pub(crate) fn connect_changed(
        &self,
        level_mute: LevelMute,
        callback: impl Fn(FaderChange) + 'static,
    ) {
        let callback: Rc<dyn Fn(FaderChange)> = Rc::new(callback);
        let (widget, mute, percent) = (
            self.widget.downgrade(),
            self.mute.downgrade(),
            self.percent.downgrade(),
        );
        let guard = self.updating.clone();
        let capture = self.capture;
        let changed = callback.clone();
        self.level.connect_changed(move |level| {
            let (Some(widget), Some(mute), Some(percent)) =
                (widget.upgrade(), mute.upgrade(), percent.upgrade())
            else {
                return;
            };
            if guard.get() {
                return;
            }
            percent.set_label(&format!("{:.0}%", level * 100.0));
            if matches!(level_mute, LevelMute::FollowLevel) {
                guard.set(true);
                mute.set_active(level < 0.01);
                guard.set(false);
            }
            mute_feedback(&widget, &mute, capture);
            changed(FaderChange::Level {
                level,
                muted: mute.is_active(),
            });
        });
        let (widget, scale) = (self.widget.downgrade(), self.scale.downgrade());
        let guard = self.updating.clone();
        self.mute.connect_toggled(move |mute| {
            if guard.get() {
                return;
            }
            if let (Some(widget), Some(scale)) = (widget.upgrade(), scale.upgrade()) {
                mute_feedback(&widget, mute, capture);
                callback(FaderChange::Mute {
                    level: scale.value(),
                    muted: mute.is_active(),
                });
            }
        });
    }
}

fn mute_feedback(widget: &gtk::Box, mute: &gtk::ToggleButton, capture: bool) {
    let muted = mute.is_active();
    mute.set_icon_name(if capture {
        if muted {
            "microphone-sensitivity-muted-symbolic"
        } else {
            "audio-input-microphone-symbolic"
        }
    } else if muted {
        "audio-volume-muted-symbolic"
    } else {
        "audio-volume-high-symbolic"
    });
    mute.set_tooltip_text(Some(if muted { "Unmute" } else { "Mute" }));
    if muted {
        widget.add_css_class("openwave-muted");
    } else {
        widget.remove_css_class("openwave-muted");
    }
}
