use adw::prelude::*;

pub(crate) mod device_slider;
pub(crate) mod editor;
pub(crate) mod fader;
pub(crate) mod fx;
pub(crate) mod scale;

pub(crate) fn value_row(title: &str, width: i32) -> (adw::ActionRow, gtk::Label) {
    let row = adw::ActionRow::builder().title(title).build();
    let label = gtk::Label::builder()
        .label("—")
        .width_chars(width)
        .xalign(1.0)
        .build();
    label.add_css_class("monospace");
    row.add_suffix(&label);
    (row, label)
}
