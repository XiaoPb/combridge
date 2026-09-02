use std::collections::VecDeque;

use super::threshold_config::ComputeConfig;
use super::types::{ChannelMeasurement, GhFuncFixIdx, GhFuncFrame};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CollectionSpec {
    pub min_number: usize,
    pub skip_number: usize,
    pub timeout_ms: u64,
    pub is_continuous: bool,
    pub sample_rate_hz: f64,
}

impl CollectionSpec {
    pub fn ctr_defaults() -> Self {
        Self {
            min_number: 100,
            skip_number: 0,
            timeout_ms: 10_000,
            is_continuous: false,
            sample_rate_hz: 100.0,
        }
    }

    pub fn noise_defaults() -> Self {
        Self {
            min_number: 100,
            skip_number: 200,
            timeout_ms: 10_000,
            is_continuous: true,
            sample_rate_hz: 100.0,
        }
    }

    pub fn resolve(compute: Option<&ComputeConfig>, test_name: &str) -> Self {
        let is_noise = matches!(test_name, "base_noise" | "ppg_noise");
        let defaults = if is_noise {
            Self::noise_defaults()
        } else {
            Self::ctr_defaults()
        };

        Self {
            min_number: compute.and_then(|c| c.min_number).unwrap_or(defaults.min_number).max(1),
            skip_number: compute.and_then(|c| c.skip_number).unwrap_or(defaults.skip_number),
            timeout_ms: compute
                .and_then(|c| c.timeout_ms)
                .unwrap_or(defaults.timeout_ms),
            is_continuous: compute
                .and_then(|c| c.is_continuous)
                .unwrap_or(defaults.is_continuous),
            sample_rate_hz: compute
                .and_then(|c| c.sample_rate_hz)
                .unwrap_or(defaults.sample_rate_hz),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct CollectedFrames {
    pub channel_count: usize,
    pub frame_cnts: Vec<u32>,
    pub timestamps: Vec<u64>,
    pub rawdata: Vec<Vec<i32>>,
    pub ipd_pa: Vec<Vec<i32>>,
    pub led_drv0: Vec<Vec<u8>>,
    pub led_drv1: Vec<Vec<u8>>,
    pub led_drv_fs: Vec<Vec<u8>>,
}

impl CollectedFrames {
    fn ensure_channels(&mut self, channels: usize) {
        while self.rawdata.len() < channels {
            self.rawdata.push(Vec::new());
            self.ipd_pa.push(Vec::new());
            self.led_drv0.push(Vec::new());
            self.led_drv1.push(Vec::new());
            self.led_drv_fs.push(Vec::new());
        }
        self.channel_count = self.channel_count.max(channels);
    }
}

#[derive(Debug, Clone)]
pub struct FactoryFrameCollector {
    active: bool,
    spec: Option<CollectionSpec>,
    recent_frames: VecDeque<(u32, u64)>,
    samples: CollectedFrames,
}

impl Default for FactoryFrameCollector {
    fn default() -> Self {
        Self {
            active: false,
            spec: None,
            recent_frames: VecDeque::with_capacity(16),
            samples: CollectedFrames::default(),
        }
    }
}

impl FactoryFrameCollector {
    pub fn start(&mut self, spec: CollectionSpec) {
        self.active = true;
        self.spec = Some(spec);
        self.recent_frames.clear();
        self.samples = CollectedFrames::default();
    }

    pub fn push_frame(&mut self, frame: GhFuncFrame) {
        if !self.active || frame.id != GhFuncFixIdx::AlgoMax {
            return;
        }

        let stamp = (frame.frame_cnt, frame.timestamp);
        if self.recent_frames.iter().any(|recent| *recent == stamp) {
            return;
        }

        self.recent_frames.push_back(stamp);
        if self.recent_frames.len() > 16 {
            self.recent_frames.pop_front();
        }

        self.samples.frame_cnts.push(frame.frame_cnt);
        self.samples.timestamps.push(frame.timestamp);
        self.samples.channel_count = self.samples.channel_count.max(frame.data.len());
        self.samples.ensure_channels(frame.data.len());

        for (idx, ch) in frame.data.iter().enumerate() {
            self.samples.rawdata[idx].push(ch.rawdata);
            self.samples.ipd_pa[idx].push(ch.ipd_pa);
            self.samples.led_drv0[idx].push(ch.agc_info.led_drv0);
            self.samples.led_drv1[idx].push(ch.agc_info.led_drv1);
            self.samples.led_drv_fs[idx].push(frame.led_drv_fs[0]);
        }
    }

    pub fn snapshot(&self) -> CollectedFrames {
        self.samples.clone()
    }

    pub fn is_complete(&self) -> bool {
        let Some(spec) = self.spec else {
            return false;
        };

        let distinct = self.samples.frame_cnts.len();
        if distinct < spec.skip_number + spec.min_number {
            return false;
        }

        !spec.is_continuous || is_tail_continuous(&self.samples.frame_cnts, spec.min_number)
    }

    pub fn finish(&mut self) -> CollectedFrames {
        self.active = false;
        self.spec = None;
        self.recent_frames.clear();
        std::mem::take(&mut self.samples)
    }
}

pub fn is_tail_continuous(frame_cnts: &[u32], required: usize) -> bool {
    if required <= 1 {
        return !frame_cnts.is_empty();
    }
    if frame_cnts.len() < required {
        return false;
    }

    frame_cnts
        .windows(2)
        .rev()
        .take(required.saturating_sub(1))
        .all(|pair| pair[1].wrapping_sub(pair[0]) == 1)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdcParams {
    pub full_scale: f64,
    pub offset: f64,
    pub vref: f64,
    pub tia_ratio: f64,
}

impl AdcParams {
    pub fn for_chip(chip: &str) -> Self {
        let offset = match chip.to_ascii_lowercase().as_str() {
            "gh3220" | "gh3300" | "gh3020" | "gh3026" | "gh3228t" | "gh3310" | "gh3030" => {
                8_388_608.0
            }
            _ => 0.0,
        };

        Self {
            full_scale: 8_388_608.0,
            offset,
            vref: 1.8,
            tia_ratio: 2.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChipSeries {
    Gh3036,
    Gh3038,
    Gh3220,
    Other,
}

impl ChipSeries {
    fn from_chip(chip: &str) -> Self {
        match chip.to_ascii_lowercase().as_str() {
            "gh3036" => Self::Gh3036,
            "gh3038" | "gh3038q" => Self::Gh3038,
            "gh3220" | "gh3300" => Self::Gh3220,
            _ => Self::Other,
        }
    }
}

fn mean(values: &[f64]) -> Option<f64> {
    if values.is_empty() || values.iter().any(|v| !v.is_finite()) {
        return None;
    }

    Some(values.iter().sum::<f64>() / values.len() as f64)
}

pub fn population_stddev(values: &[f64]) -> Option<f64> {
    let mean = mean(values)?;
    let variance = values
        .iter()
        .map(|v| (v - mean) * (v - mean))
        .sum::<f64>()
        / values.len() as f64;
    let sigma = variance.sqrt();
    if sigma.is_finite() && sigma > 0.0 {
        Some(sigma)
    } else {
        None
    }
}

#[derive(Clone, Copy)]
struct Section {
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
}

impl Section {
    fn process(&mut self, x: f64) -> f64 {
        let y = self.b0 * x + self.b1 * self.x1 + self.b2 * self.x2 - self.a1 * self.y1 - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}

pub fn butterworth_high_pass_7(
    values: &[f64],
    sample_rate_hz: f64,
    cutoff_hz: f64,
) -> Option<Vec<f64>> {
    if values.is_empty()
        || !sample_rate_hz.is_finite()
        || !cutoff_hz.is_finite()
        || sample_rate_hz <= 0.0
        || cutoff_hz <= 0.0
    {
        return None;
    }
    if values.iter().any(|v| !v.is_finite()) {
        return None;
    }

    let c = 2.0 * sample_rate_hz;
    let omega_a = 2.0 * sample_rate_hz * (std::f64::consts::PI * 0.5 / sample_rate_hz).tan();
    let mean = mean(values)?;

    let first = Section {
        b0: c / (c + omega_a),
        b1: -c / (c + omega_a),
        b2: 0.0,
        a1: (omega_a - c) / (c + omega_a),
        a2: 0.0,
        x1: mean,
        x2: mean,
        y1: 0.0,
        y2: 0.0,
    };

    let mut sections = vec![first];
    for re in [-0.2225209340, -0.6234898019, -0.9009688679] {
        let p = -2.0 * re * omega_a;
        let q = omega_a * omega_a;
        let d = c * c + p * c + q;
        sections.push(Section {
            b0: c * c / d,
            b1: -2.0 * c * c / d,
            b2: c * c / d,
            a1: (-2.0 * c * c + 2.0 * q) / d,
            a2: (c * c - p * c + q) / d,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        });
    }

    let mut output = Vec::with_capacity(values.len());
    for &value in values {
        let mut x = value;
        for section in &mut sections {
            x = section.process(x);
        }
        output.push(x);
    }

    Some(output)
}

fn tail_slice<T>(values: &[T], min: usize) -> Option<&[T]> {
    if values.len() < min {
        None
    } else {
        Some(&values[values.len() - min..])
    }
}

pub fn calculate_noise_uv(raw: &[i32], spec: &CollectionSpec, adc: AdcParams) -> Option<f64> {
    if raw.len() < spec.min_number {
        return None;
    }
    let raw = raw.iter().map(|&v| v as f64).collect::<Vec<_>>();
    let filtered = butterworth_high_pass_7(&raw, spec.sample_rate_hz, 0.5)?;
    let tail = filtered.get(filtered.len().saturating_sub(spec.min_number)..)?;
    let sigma = population_stddev(tail)?;
    let noise = sigma / adc.full_scale * adc.vref * 1_000_000.0;
    if noise.is_finite() && noise > 0.0 {
        Some(noise)
    } else {
        None
    }
}

pub fn calculate_ipd_na(
    ipd_pa: &[i32],
    raw: &[i32],
    min: usize,
    adc: AdcParams,
    gain_k: Option<f64>,
) -> Option<f64> {
    if let Some(values) = tail_slice(ipd_pa, min) {
        let values = values.iter().map(|&v| v as f64).collect::<Vec<_>>();
        let ipd = mean(&values)? / 1000.0;
        if ipd.is_finite() {
            return Some(ipd);
        }
    }

    let gain_k = gain_k?;
    if !(gain_k > 0.0) {
        return None;
    }

    let raw = tail_slice(raw, min)?;
    let raw = raw.iter().map(|&v| v as f64).collect::<Vec<_>>();
    let raw_avg = mean(&raw)?;
    let ipd = (raw_avg - adc.offset) / adc.full_scale * adc.vref * 1_000_000.0
        / (adc.tia_ratio * gain_k);
    if ipd.is_finite() {
        Some(ipd)
    } else {
        None
    }
}

pub fn calculate_ctr_na_per_ma(ipd_na: Option<f64>, led_ma: Option<f64>) -> Option<f64> {
    let ipd_na = ipd_na?;
    let led_ma = led_ma?;
    if !ipd_na.is_finite() || !led_ma.is_finite() || led_ma <= 0.0 {
        return None;
    }
    Some(ipd_na / led_ma)
}

fn led_current_sum_ma(led0: u8, led1: u8, led_fs: u8) -> f64 {
    ((10 * led0 as u32 * led_fs as u32) / 255 + (10 * led1 as u32 * led_fs as u32) / 255) as f64
        / 10.0
}

fn choose_led_current(chip: ChipSeries, decoded: Option<f64>, configured: Option<f64>) -> Option<f64> {
    match chip {
        ChipSeries::Gh3036 | ChipSeries::Gh3038 => decoded.or(configured),
        ChipSeries::Gh3220 => configured.or(decoded),
        ChipSeries::Other => configured.filter(|v| *v > 0.0).or(decoded),
    }
}

pub fn calculate_app_measurements(
    test_name: &str,
    samples: &CollectedFrames,
    chip: &str,
    config: &ComputeConfig,
) -> Vec<ChannelMeasurement> {
    let chip_series = ChipSeries::from_chip(chip);
    let adc = AdcParams::for_chip(chip);
    let spec = CollectionSpec::resolve(Some(config), test_name);

    if !matches!(test_name, "base_noise" | "ppg_noise" | "lpctr" | "lplctr") {
        return (0..samples.channel_count)
            .map(|_| ChannelMeasurement {
                computed_value: None,
                device_value: None,
            })
            .collect();
    }

    let channel_count = samples
        .rawdata
        .len()
        .max(samples.ipd_pa.len())
        .max(samples.led_drv0.len())
        .max(samples.led_drv1.len())
        .max(samples.led_drv_fs.len());

    (0..channel_count)
        .map(|idx| {
            let raw = samples.rawdata.get(idx).map_or(&[][..], |v| v.as_slice());
            let ipd_pa = samples.ipd_pa.get(idx).map_or(&[][..], |v| v.as_slice());
            let led0 = samples.led_drv0.get(idx).and_then(|v| v.last().copied());
            let led1 = samples.led_drv1.get(idx).and_then(|v| v.last().copied());
            let led_fs = samples.led_drv_fs.get(idx).and_then(|v| v.last().copied());
            let decoded_led = match (led0, led1, led_fs) {
                (Some(a), Some(b), Some(fs)) => Some(led_current_sum_ma(a, b, fs)),
                _ => None,
            };
            let led_ma = choose_led_current(chip_series, decoded_led, config.led_current_ma);

            let metric = match test_name {
                "base_noise" | "ppg_noise" => calculate_noise_uv(raw, &spec, adc),
                "lpctr" | "lplctr" => {
                    let ipd_na = calculate_ipd_na(ipd_pa, raw, spec.min_number, adc, config.gain_k);
                    calculate_ctr_na_per_ma(ipd_na, led_ma)
                }
                _ => None,
            };

            ChannelMeasurement {
                computed_value: metric,
                device_value: None,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gh3036::threshold_config::ComputeConfig;
    use crate::gh3036::types::{GhAgcInfo, GhFrameData, GhFuncFixIdx, GhFuncFrame};

    fn frame(frame_cnt: u32, timestamp: u64, raw: &[i32]) -> GhFuncFrame {
        GhFuncFrame {
            frame_cnt,
            timestamp,
            id: GhFuncFixIdx::AlgoMax,
            ch_num: raw.len() as u8,
            led_drv_fs: [255, 255],
            data: raw
                .iter()
                .map(|&rawdata| GhFrameData {
                    ipd_pa: rawdata * 1000,
                    rawdata,
                    agc_info: GhAgcInfo {
                        led_drv0: 10,
                        led_drv1: 20,
                        ..GhAgcInfo::default()
                    },
                    ..GhFrameData::default()
                })
                .collect(),
            ..GhFuncFrame::default()
        }
    }

    fn adc() -> AdcParams {
        AdcParams::for_chip("gh3036")
    }

    fn compute() -> ComputeConfig {
        ComputeConfig {
            sample_rate_hz: Some(100.0),
            min_number: Some(2),
            skip_number: Some(0),
            is_continuous: Some(true),
            timeout_ms: Some(10_000),
            gain_k: Some(2.0),
            led_current_ma: Some(4.0),
        }
    }

    #[test]
    fn collector_deduplicates_frame_cnt_and_timestamp_and_requires_tail_continuity() {
        let mut collector = FactoryFrameCollector::default();
        collector.start(CollectionSpec {
            min_number: 2,
            skip_number: 0,
            is_continuous: true,
            ..CollectionSpec::ctr_defaults()
        });
        collector.push_frame(frame(10, 1000, &[1]));
        collector.push_frame(frame(10, 1000, &[99]));
        collector.push_frame(frame(11, 1010, &[2]));
        assert_eq!(collector.snapshot().frame_cnts, vec![10, 11]);
        assert!(collector.is_complete());
    }

    #[test]
    fn collector_handles_u32_wrap_for_tail_continuity() {
        assert!(is_tail_continuous(&[u32::MAX - 1, u32::MAX, 0, 1], 2));
    }

    #[test]
    fn timeout_defaults_are_ready_for_app_collection() {
        assert_eq!(CollectionSpec::noise_defaults().timeout_ms, 10_000);
        assert_eq!(CollectionSpec::ctr_defaults().timeout_ms, 10_000);
    }

    #[test]
    fn filter_keeps_sine_tail_stddev_near_reference() {
        let values: Vec<f64> = (0..100)
            .map(|i| {
                let t = i as f64 / 100.0;
                (2.0 * std::f64::consts::PI * 5.0 * t).sin() * 1000.0
            })
            .collect();
        let filtered = butterworth_high_pass_7(&values, 100.0, 0.5).unwrap();
        let tail = &filtered[filtered.len() - 100..];
        let sigma = population_stddev(tail).unwrap();
        assert!((sigma - 707.1).abs() / 707.1 < 0.02);
    }

    #[test]
    fn constant_sequence_initializes_to_near_zero_output() {
        let values = vec![8_389_842.0; 128];
        let filtered = butterworth_high_pass_7(&values, 100.0, 0.5).unwrap();
        let tail = &filtered[filtered.len() - 100..];
        let mean = tail.iter().sum::<f64>() / tail.len() as f64;
        let sigma = (tail.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / tail.len() as f64)
            .sqrt();
        assert!(mean.abs() < 1e-6);
        assert!(sigma < 1e-6);
    }

    #[test]
    fn noise_calculation_matches_reference_vector() {
        let raw: Vec<i32> = (0..100)
            .map(|i| {
                let t = i as f64 / 100.0;
                ((2.0 * std::f64::consts::PI * 5.0 * t).sin() * 1000.0).round() as i32
            })
            .collect();
        let spec = CollectionSpec {
            min_number: 100,
            skip_number: 0,
            ..CollectionSpec::noise_defaults()
        };
        let noise = calculate_noise_uv(&raw, &spec, adc()).unwrap();
        assert!((noise - 151.8).abs() < 3.5);
    }

    #[test]
    fn adc_offsets_match_chip_families() {
        assert_eq!(AdcParams::for_chip("gh3036").offset, 0.0);
        assert_eq!(AdcParams::for_chip("gh3220").offset, 8_388_608.0);
    }

    #[test]
    fn ctr_prefers_ipd_pa_and_falls_back_to_rawdata_only_with_gain() {
        assert_eq!(calculate_ipd_na(&[1_200], &[99], 1, adc(), None), Some(1.2));
        assert_eq!(calculate_ipd_na(&[], &[8_388_608], 1, adc(), None), None);
        assert!(calculate_ipd_na(&[], &[8_388_608], 1, adc(), Some(2.0)).is_some());
    }

    #[test]
    fn raw_ctr_requires_positive_gain() {
        assert_eq!(calculate_ipd_na(&[], &[8_388_608], 1, adc(), Some(0.0)), None);
    }

    #[test]
    fn gh3036_led_current_uses_agc_before_config() {
        let mut samples = CollectedFrames::default();
        samples.rawdata = vec![vec![1, 2]];
        samples.ipd_pa = vec![vec![1200, 1300]];
        samples.led_drv0 = vec![vec![10, 10]];
        samples.led_drv1 = vec![vec![20, 20]];
        samples.led_drv_fs = vec![vec![255, 255]];
        let out = calculate_app_measurements("lpctr", &samples, "gh3036", &compute());
        assert_eq!(out.len(), 1);
        assert!(out[0].computed_value.is_some());
    }

    #[test]
    fn gh3036_led_current_none_when_decoded_source_is_nonpositive() {
        let mut samples = CollectedFrames::default();
        samples.rawdata = vec![vec![1, 2]];
        samples.ipd_pa = vec![vec![1200, 1300]];
        samples.led_drv0 = vec![vec![0, 0]];
        samples.led_drv1 = vec![vec![0, 0]];
        samples.led_drv_fs = vec![vec![255, 255]];
        let mut config = compute();
        config.led_current_ma = Some(4.0);
        let out = calculate_app_measurements("lpctr", &samples, "gh3036", &config);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].computed_value, None);
    }

    #[test]
    fn gh3220_led_current_none_when_configured_source_is_nonpositive() {
        let mut samples = CollectedFrames::default();
        samples.rawdata = vec![vec![1, 2]];
        samples.ipd_pa = vec![vec![1200, 1300]];
        samples.led_drv0 = vec![vec![10, 10]];
        samples.led_drv1 = vec![vec![20, 20]];
        samples.led_drv_fs = vec![vec![255, 255]];
        let mut config = compute();
        config.led_current_ma = Some(0.0);
        let out = calculate_app_measurements("lpctr", &samples, "gh3220", &config);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].computed_value, None);
    }

    #[test]
    fn unsupported_test_name_keeps_configured_channel_count_as_none_entries() {
        let samples = CollectedFrames {
            channel_count: 2,
            ..CollectedFrames::default()
        };
        let out = calculate_app_measurements("unknown", &samples, "gh3036", &compute());
        assert_eq!(out.len(), 2);
        assert!(out.iter().all(|m| m.computed_value.is_none() && m.device_value.is_none()));
    }
}
