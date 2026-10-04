//! Translate measured calibration values and preserve exact backend errors.
use crate::i18n::{format, tr, translate};
use cadiswave_core::model::CalibrationPhase;
pub fn body(phase: &CalibrationPhase) -> String {
    match phase {
        CalibrationPhase::NoiseReady => translate(
            "Stay quiet for three seconds after pressing Record. Only the raw microphone is measured. Current effects and hardware gain stay unchanged; proposed settings require explicit confirmation.",
        ),
        CalibrationPhase::RecordingNoise => translate("Please stay quiet."),
        CalibrationPhase::SpeechReady => {
            translate("Speak normally for five seconds after pressing Record.")
        }
        CalibrationPhase::RecordingSpeech => translate("Speak normally."),
        CalibrationPhase::Expired(error) => {
            format!("{error}\n\n{}", tr("calibration-expired-help"))
        }
        CalibrationPhase::Review { proposal, summary } => {
            let mut lines = vec![
                format(
                    "calibration-noise",
                    &[("value", &format!("{:.1}", summary.noise_floor_db))],
                ),
                format(
                    "calibration-speech",
                    &[
                        ("quiet", &format!("{:.1}", summary.quiet_voice_db)),
                        ("loud", &format!("{:.1}", summary.loud_voice_db)),
                    ],
                ),
                String::new(),
                format(
                    "calibration-gate",
                    &[("value", &format!("{:.1}", proposal.gate_thresh))],
                ),
                format(
                    "calibration-compressor",
                    &[
                        ("threshold", &format!("{:.1}", proposal.comp_thresh)),
                        ("ratio", &format!("{:.1}", proposal.comp_ratio)),
                    ],
                ),
                format(
                    "calibration-low-cut",
                    &[("frequency", &proposal.lowcut.to_string())],
                ),
                format(
                    "calibration-high-shelf",
                    &[("value", &format!("{:+.0}", proposal.eq_high))],
                ),
            ];
            if proposal.mono {
                lines.push(tr(if summary.quiet_channel {
                    "calibration-mono-quiet"
                } else {
                    "calibration-mono-enabled"
                }));
            }
            lines.join("\n")
        }
    }
}

pub fn refresh(dialog: &adw::AlertDialog, phase: &CalibrationPhase) {
    use adw::prelude::*;
    let heading = match phase {
        CalibrationPhase::NoiseReady => "Measure room noise",
        CalibrationPhase::RecordingNoise => "Recording room noise",
        CalibrationPhase::SpeechReady => "Measure speech",
        CalibrationPhase::RecordingSpeech => "Recording speech",
        CalibrationPhase::Review { .. } => "Review calibration",
        CalibrationPhase::Expired(_) => "Calibration expired",
    };
    dialog.set_heading(Some(&translate(heading)));
    dialog.set_body(&body(phase));
    dialog.set_response_label(
        "cancel",
        &translate(if matches!(phase, CalibrationPhase::Review { .. }) {
            "Keep current settings"
        } else {
            "Cancel"
        }),
    );
    if matches!(
        phase,
        CalibrationPhase::NoiseReady | CalibrationPhase::SpeechReady
    ) {
        dialog.set_response_label("record", &translate("Record"));
    }
    if matches!(phase, CalibrationPhase::Review { .. }) {
        dialog.set_response_label("apply", &translate("Apply"));
    }
}
