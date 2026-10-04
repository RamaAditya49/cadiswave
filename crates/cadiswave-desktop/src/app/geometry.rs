pub(super) const MIN_SIZE: (i32, i32) = (
    cadiswave_core::model::MIN_WINDOW_WIDTH,
    cadiswave_core::model::MIN_WINDOW_HEIGHT,
);
const DEFAULT_SIZE: (i32, i32) = (1280, 800);

pub(super) fn fit_size(saved: (i32, i32), monitor: Option<(i32, i32)>) -> (i32, i32) {
    let bounds = monitor
        .filter(|(width, height)| *width > 0 && *height > 0)
        .map(|(width, height)| {
            (
                (i64::from(width) * 9 / 10) as i32,
                (i64::from(height) * 9 / 10) as i32,
            )
        })
        .unwrap_or(DEFAULT_SIZE);
    let width = if saved.0 > 0 { saved.0 } else { DEFAULT_SIZE.0 };
    let height = if saved.1 > 0 { saved.1 } else { DEFAULT_SIZE.1 };
    (
        width.clamp(MIN_SIZE.0, bounds.0.max(MIN_SIZE.0)),
        height.clamp(MIN_SIZE.1, bounds.1.max(MIN_SIZE.1)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_size_and_valid_saved_size_fit_large_monitors() {
        assert_eq!(fit_size(DEFAULT_SIZE, Some((1920, 1080))), DEFAULT_SIZE);
        assert_eq!(fit_size((900, 650), Some((1920, 1080))), (900, 650));
        assert_eq!(fit_size(MIN_SIZE, Some((1920, 1080))), MIN_SIZE);
    }

    #[test]
    fn smaller_monitors_bound_initial_and_oversized_saved_dimensions() {
        assert_eq!(fit_size(DEFAULT_SIZE, Some((800, 600))), (720, 540));
        assert_eq!(fit_size(DEFAULT_SIZE, Some((1280, 720))), (1152, 648));
        assert_eq!(fit_size((3840, 2160), Some((1920, 1080))), (1728, 972));
        assert_eq!(fit_size(DEFAULT_SIZE, Some(MIN_SIZE)), MIN_SIZE);
    }

    #[test]
    fn monitor_bounds_use_logical_dimensions_without_device_scale() {
        // A 2560 x 1600 monitor at scale 2 reports 1280 x 800 logical pixels.
        assert_eq!(fit_size(DEFAULT_SIZE, Some((1280, 800))), (1152, 720));
    }

    #[test]
    fn unknown_monitor_and_invalid_preferences_have_bounded_fallbacks() {
        for monitor in [None, Some((0, 1080)), Some((1920, 0)), Some((-1, 1080))] {
            assert_eq!(fit_size((i32::MAX, i32::MAX), monitor), DEFAULT_SIZE);
            assert_eq!(fit_size((900, 650), monitor), (900, 650));
            assert_eq!(fit_size((0, -1), monitor), DEFAULT_SIZE);
        }
        assert_eq!(fit_size((1, 1), Some((1920, 1080))), MIN_SIZE);
    }
}
