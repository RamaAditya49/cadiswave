use gtk::prelude::*;
use std::{cell::Cell, rc::Rc};

/// Reconcile observations without rolling back an input awaiting its snapshot.
/// A different identity always ends the previous control's adjustment animation.
pub(crate) struct SnapshotScale<K> {
    pub(crate) widget: gtk::Scale,
    rendered: Cell<Option<(K, f64)>>,
    updating: Rc<Cell<bool>>,
}

impl<K: Copy + PartialEq> SnapshotScale<K> {
    pub(crate) fn new(widget: gtk::Scale) -> Self {
        Self {
            widget,
            rendered: Cell::new(None),
            updating: Rc::new(Cell::new(false)),
        }
    }

    /// Returns whether the incoming projection changed, including availability.
    /// The caller renders its label and availability; None leaves the range alone.
    pub(crate) fn render(&self, value: Option<(K, f64)>) -> bool {
        let previous = self.rendered.replace(value);
        if previous == value {
            return false;
        }
        if let Some((identity, value)) = value
            && (previous.map(|(identity, _)| identity) != Some(identity)
                || self.widget.value() != value)
        {
            let updating = self.updating.replace(true);
            self.widget.set_value(value);
            self.updating.set(updating);
        }
        true
    }

    pub(crate) fn connect_changed(&self, callback: impl Fn(f64) + 'static) {
        let updating = self.updating.clone();
        self.widget.connect_value_changed(move |scale| {
            if !updating.get() {
                callback(scale.value());
            }
        });
    }
}
