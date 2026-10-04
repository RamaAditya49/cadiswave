use adw::prelude::*;
use cadiswave_core::pcm::ChannelPeaks;
use std::{cell::Cell, rc::Rc};
pub struct Meters {
    pub widget: gtk::DrawingArea,
    peaks: Rc<Cell<Option<ChannelPeaks>>>,
}
impl Default for Meters {
    fn default() -> Self {
        Self::new()
    }
}
impl Meters {
    pub fn new() -> Self {
        let widget = gtk::DrawingArea::builder()
            .content_width(160)
            .content_height(190)
            .hexpand(true)
            .build();
        let peaks = Rc::new(Cell::new(None));
        let draw = peaks.clone();
        widget.set_draw_func(move |_, cr, w, h| {
            let channels = match draw.get() {
                Some(ChannelPeaks::Mono(p)) => vec![p],
                Some(ChannelPeaks::Stereo { left, right }) => vec![left, right],
                None => vec![0.0, 0.0],
            };
            let width = 32.0;
            let count = channels.len();
            for (channel, peak) in channels.into_iter().enumerate() {
                let db = if peak > 0.0 {
                    20.0 * peak.log10()
                } else {
                    -60.0
                };
                let level = ((db + 60.0) / 60.0).clamp(0.0, 1.0);
                let x = f64::from(w) / 2.0
                    + (channel as f64 - (count as f64 - 1.0) / 2.0) * (width + 12.0)
                    - width / 2.0;
                for segment in 0..24 {
                    let y = f64::from(h)
                        - 14.0
                        - (f64::from(segment) + 1.0) * (f64::from(h) - 24.0) / 24.0;
                    if draw.get().is_none() || f64::from(segment) / 24.0 > level {
                        cr.set_source_rgb(0.13, 0.19, 0.15);
                    } else if segment >= 22 {
                        cr.set_source_rgb(1.0, 0.48, 0.52);
                    } else if segment >= 19 {
                        cr.set_source_rgb(1.0, 0.82, 0.4);
                    } else {
                        cr.set_source_rgb(0.49, 1.0, 0.61);
                    }
                    cr.rectangle(x, y, width, (f64::from(h) - 24.0) / 24.0 - 2.0);
                    let _ = cr.fill();
                }
                cr.set_source_rgb(0.55, 0.6, 0.56);
                cr.move_to(x + 12.0, f64::from(h) - 1.0);
                let _ = cr.show_text(if count == 1 {
                    "M"
                } else if channel == 0 {
                    "L"
                } else {
                    "R"
                });
            }
        });
        Self { widget, peaks }
    }
    pub fn set_peaks(&self, peaks: Option<ChannelPeaks>) {
        let peaks = peaks.filter(|p| p.is_valid());
        if self.peaks.replace(peaks) != peaks {
            self.widget.queue_draw();
        }
        let text = match peaks {
            None => crate::i18n::tr("unknown-reading"),
            Some(ChannelPeaks::Mono(_)) => crate::i18n::tr("mono-reading"),
            _ => crate::i18n::tr("stereo-reading"),
        };
        self.widget.set_tooltip_text(Some(&text));
    }
}
