use adw::prelude::*;
use openwave_core::model::UnitId;

use super::{scale::SnapshotScale, value_row};

/// A hardware value row and slider, with snapshot-safe local edit feedback.
/// Selection, profile capabilities and command submission stay in the sidebar.
pub(crate) struct DeviceSlider {
    pub(crate) value_row: adw::ActionRow,
    slider_row: adw::PreferencesRow,
    label: gtk::Label,
    scale: SnapshotScale<UnitId>,
    format: fn(UnitId, f64) -> String,
}

impl DeviceSlider {
    pub(crate) fn new(
        title: &str,
        width: i32,
        name: &str,
        adjustment: &gtk::Adjustment,
        format: fn(UnitId, f64) -> String,
    ) -> Self {
        let (value_row, label) = value_row(title, width);
        let widget = gtk::Scale::new(gtk::Orientation::Horizontal, Some(adjustment));
        widget.set_hexpand(true);
        widget.set_draw_value(false);
        widget.update_property(&[gtk::accessible::Property::Label(name)]);
        widget.set_margin_start(12);
        widget.set_margin_end(12);
        widget.set_margin_top(2);
        widget.set_margin_bottom(6);
        let slider_row = adw::PreferencesRow::builder()
            .activatable(false)
            .selectable(false)
            .build();
        slider_row.set_child(Some(&widget));
        Self {
            value_row,
            slider_row,
            label,
            scale: SnapshotScale::new(widget),
            format,
        }
    }

    pub(crate) fn widget(&self) -> &gtk::Scale {
        &self.scale.widget
    }

    pub(crate) fn add_to(&self, group: &adw::PreferencesGroup) {
        group.add(&self.value_row);
        group.add(&self.slider_row);
    }

    pub(crate) fn set_visible(&self, visible: bool) {
        self.value_row.set_visible(visible);
        self.slider_row.set_visible(visible);
    }

    pub(crate) fn render(&self, value: Option<(UnitId, f64)>, enabled: bool) {
        let value = value.filter(|(_, value)| value.is_finite());
        self.widget().set_sensitive(enabled && value.is_some());
        if self.scale.render(value) {
            if let Some((unit, value)) = value {
                self.label.set_label(&(self.format)(unit, value));
            } else {
                self.label.set_label("—");
            }
        }
    }

    /// The sidebar returns an accepted, normalized value for immediate feedback.
    pub(crate) fn connect_changed(
        &self,
        callback: impl Fn(f64) -> Option<(UnitId, f64)> + 'static,
    ) {
        let widget = self.widget().downgrade();
        let label = self.label.clone();
        let format = self.format;
        self.scale.connect_changed(move |value| {
            if !value.is_finite() || !widget.upgrade().is_some_and(|widget| widget.is_sensitive()) {
                return;
            }
            if let Some((unit, value)) = callback(value) {
                label.set_label(&format(unit, value));
            }
        });
    }
}
