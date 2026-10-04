use adw::prelude::*;

pub(crate) fn navigation_dialog(
    title: &str,
    width: i32,
    height: i32,
) -> (adw::Dialog, adw::NavigationView) {
    let dialog = adw::Dialog::builder()
        .title(title)
        .content_width(width)
        .content_height(height)
        .build();
    let navigation = adw::NavigationView::new();
    dialog.set_child(Some(&navigation));
    (dialog, navigation)
}

pub(crate) fn page(title: &str) -> (adw::NavigationPage, adw::HeaderBar, gtk::Box) {
    let toolbar = adw::ToolbarView::new();
    let header = adw::HeaderBar::new();
    toolbar.add_top_bar(&header);
    let scroll = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .build();
    let clamp = adw::Clamp::builder()
        .maximum_size(440)
        .margin_start(12)
        .margin_end(12)
        .margin_top(12)
        .margin_bottom(12)
        .build();
    let body = gtk::Box::new(gtk::Orientation::Vertical, 16);
    clamp.set_child(Some(&body));
    scroll.set_child(Some(&clamp));
    toolbar.set_content(Some(&scroll));
    (adw::NavigationPage::new(&toolbar, title), header, body)
}

pub(crate) fn cancel(header: &adw::HeaderBar, dialog: &adw::Dialog) {
    let button = gtk::Button::with_label("Cancel");
    let weak = dialog.downgrade();
    button.connect_clicked(move |_| {
        if let Some(dialog) = weak.upgrade() {
            dialog.close();
        }
    });
    header.pack_start(&button);
}

pub(crate) fn action(header: &adw::HeaderBar, label: &str) -> gtk::Button {
    let button = gtk::Button::with_label(label);
    button.add_css_class("suggested-action");
    header.pack_end(&button);
    button
}

pub(crate) fn require_name(entry: &adw::EntryRow, action: &gtk::Button) {
    action.set_sensitive(!entry.text().trim().is_empty());
    let action = action.downgrade();
    entry.connect_changed(move |entry| {
        if let Some(action) = action.upgrade() {
            action.set_sensitive(!entry.text().trim().is_empty());
        }
    });
}
