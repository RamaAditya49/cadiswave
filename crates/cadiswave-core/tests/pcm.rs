use cadiswave_core::pcm::{ChannelPeaks, PcmPeakDecoder};
#[test]
fn stereo_levels_do_not_cancel_each_other() {
    let mut decoder = PcmPeakDecoder::new(2).unwrap();
    assert_eq!(
        decoder.push(&[0x00, 0x40, 0x00, 0xc0]),
        Some(ChannelPeaks::Stereo {
            left: 0.5,
            right: 0.5
        })
    );
    assert_eq!(decoder.scalar_peak(), 0.0);
}
#[test]
fn partial_frames_wait_for_the_remaining_channel() {
    let mut decoder = PcmPeakDecoder::new(2).unwrap();
    assert_eq!(decoder.push(&[0x00, 0x40]), None);
    assert_eq!(
        decoder.push(&[0x00, 0x20]),
        Some(ChannelPeaks::Stereo {
            left: 0.5,
            right: 0.25
        })
    );
}
#[test]
fn fragments_extrema_and_silence_remain_measured() {
    let mut decoder = PcmPeakDecoder::new(1).unwrap();
    assert_eq!(decoder.push(&[0]), None);
    assert_eq!(decoder.push(&[0x80]), Some(ChannelPeaks::Mono(1.0)));
    assert_eq!(decoder.push(&[0, 0]), Some(ChannelPeaks::Mono(0.0)));
    assert_eq!(decoder.push(&[]), None);
    for channels in [0, 3, u32::MAX] {
        assert!(PcmPeakDecoder::new(channels).is_err());
    }
}
