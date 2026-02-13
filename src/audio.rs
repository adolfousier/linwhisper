use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::io::Cursor;
use std::sync::{Arc, Mutex};

pub struct Recorder {
    samples: Arc<Mutex<Vec<f32>>>,
    stream: Option<cpal::Stream>,
    sample_rate: u32,
    channels: u16,
}

impl Recorder {
    pub fn new() -> Result<Self, String> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or("No input device available")?;

        let config = device
            .default_input_config()
            .map_err(|e| format!("No input config: {e}"))?;

        let sample_rate = config.sample_rate().0;
        let channels = config.channels();

        Ok(Self {
            samples: Arc::new(Mutex::new(Vec::new())),
            stream: None,
            sample_rate,
            channels,
        })
    }

    pub fn start(&mut self) -> Result<(), String> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or("No input device available")?;

        let config = device
            .default_input_config()
            .map_err(|e| format!("No input config: {e}"))?;

        let samples = Arc::clone(&self.samples);
        samples.lock().unwrap().clear();

        let err_fn = |err| eprintln!("Audio stream error: {err}");

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => {
                let samples = Arc::clone(&samples);
                device
                    .build_input_stream(
                        &config.into(),
                        move |data: &[f32], _: &_| {
                            samples.lock().unwrap().extend_from_slice(data);
                        },
                        err_fn,
                        None,
                    )
                    .map_err(|e| format!("Failed to build stream: {e}"))?
            }
            cpal::SampleFormat::I16 => {
                let samples = Arc::clone(&samples);
                device
                    .build_input_stream(
                        &config.into(),
                        move |data: &[i16], _: &_| {
                            let floats: Vec<f32> =
                                data.iter().map(|&s| s as f32 / i16::MAX as f32).collect();
                            samples.lock().unwrap().extend_from_slice(&floats);
                        },
                        err_fn,
                        None,
                    )
                    .map_err(|e| format!("Failed to build stream: {e}"))?
            }
            cpal::SampleFormat::U16 => {
                let samples = Arc::clone(&samples);
                device
                    .build_input_stream(
                        &config.into(),
                        move |data: &[u16], _: &_| {
                            let floats: Vec<f32> = data
                                .iter()
                                .map(|&s| (s as f32 / u16::MAX as f32) * 2.0 - 1.0)
                                .collect();
                            samples.lock().unwrap().extend_from_slice(&floats);
                        },
                        err_fn,
                        None,
                    )
                    .map_err(|e| format!("Failed to build stream: {e}"))?
            }
            fmt => return Err(format!("Unsupported sample format: {fmt:?}")),
        };

        stream.play().map_err(|e| format!("Failed to play: {e}"))?;
        self.stream = Some(stream);
        Ok(())
    }

    pub fn stop(&mut self) -> Result<Vec<u8>, String> {
        // Drop the stream to stop recording
        self.stream.take();

        let samples = self.samples.lock().unwrap();
        if samples.is_empty() {
            return Err("No audio recorded".into());
        }

        // Convert to mono if multi-channel
        let mono: Vec<f32> = if self.channels > 1 {
            samples
                .chunks(self.channels as usize)
                .map(|chunk| chunk.iter().sum::<f32>() / chunk.len() as f32)
                .collect()
        } else {
            samples.clone()
        };

        // Encode as WAV
        let mut buf = Cursor::new(Vec::new());
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: self.sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer =
            hound::WavWriter::new(&mut buf, spec).map_err(|e| format!("WAV write error: {e}"))?;

        for &sample in &mono {
            let s = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
            writer
                .write_sample(s)
                .map_err(|e| format!("WAV sample error: {e}"))?;
        }
        writer
            .finalize()
            .map_err(|e| format!("WAV finalize error: {e}"))?;

        Ok(buf.into_inner())
    }
}
