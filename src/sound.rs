use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

pub struct SoundBackEnd {
    buffer: Arc<Mutex<VecDeque<f32>>>,
    pub sample_rate: f64,
    _stream: cpal::Stream,
}

impl SoundBackEnd {
    pub fn new() -> Self {
        let host = cpal::default_host();
        let device = host.default_output_device().expect("No output device.");
        let config = device.default_output_config().unwrap();
        let sample_rate = config.sample_rate() as f64;
        let stream_config = config.config();

        let buffer: Arc<Mutex<VecDeque<f32>>> = Arc::new(Mutex::new(VecDeque::new()));
        let buffer_clone = buffer.clone();

        let stream = device.build_output_stream(
            stream_config,
            move |data: &mut [f32], _| {
                let mut buf = buffer_clone.lock().unwrap();
                for sample in data.iter_mut() {
                    *sample = buf.pop_front().unwrap_or(0.0);
                }
            },
            |err| eprintln!("Audio error: {err}"),
            None,
        ).unwrap();

        stream.play().unwrap();
        Self {
            buffer,
            sample_rate,
            _stream: stream,
        }
    }

    pub fn push(&self, sample: f32) {
        self.buffer.lock().unwrap().push_back(sample);
    }
}