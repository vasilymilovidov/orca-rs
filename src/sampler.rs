use std::{
    fs,
    path::Path,
    sync::Arc,
    thread::{self},
};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, FromSample, SizedSample, StreamConfig,
};
use crossbeam::channel::Receiver;
use fundsp::{
    hacker::*,
    hacker::{multipass, pan, reverb_stereo, shared, var},
    prelude::Net64,
    sequencer::Sequencer64,
};

use crate::note_events::Note;
use crate::synth::write_data;

#[derive(Debug, Clone, Copy)]
pub struct SamplerNote {
    pub sample: u8,
    pub velocity: u8,
    pub duration: u64,
    pub started: bool,
    pub speed: u8,
    pub reverb: u8,
}

#[derive(Clone)]
pub struct SamplerState {
    id: Vec<Option<EventId>>,
    sequencer: Sequencer64,
    net: Net64,
    reverb: Shared<f64>,
}

pub fn sampler_out(
    sampler_note_receiver: Receiver<Vec<Note>>,
) {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("failed to find a default output device");
    let config = device.default_output_config().expect("failed to get default output config");

    match config.sample_format() {
        cpal::SampleFormat::F32 => run::<f32>(
            device,
            config.into(),
            sampler_note_receiver,
        ),
        cpal::SampleFormat::F64 => run::<f64>(
            device,
            config.into(),
            sampler_note_receiver,
        ),
        cpal::SampleFormat::I16 => run::<i16>(
            device,
            config.into(),
            sampler_note_receiver,
        ),
        cpal::SampleFormat::U16 => run::<u16>(
            device,
            config.into(),
            sampler_note_receiver,
        ),
        _ => panic!("Unsupported format"),
    }
}

#[allow(clippy::precedence)]
pub fn run<T>(
    device: Device,
    config: StreamConfig,
    sampler_note_receiver: Receiver<Vec<Note>>,
) where
    T: SizedSample + FromSample<f64>,
{
    thread::spawn(move || {
        let sample_rate = config.sample_rate.0 as f64;
        let channels = config.channels as usize;

        let mut sequencer = Sequencer64::new(false, 1);
        let sequencer_backend = sequencer.backend();

        let reverb = shared(0.2);

        let mut net = Net64::wrap(Box::new(sequencer_backend));
        net = net >> pan(0.0);

        net = net
            >> ((1.0 - var(&reverb) >> follow(0.01) >> split()) * multipass()
            & (var(&reverb) >> follow(0.01) >> split()) * reverb_stereo(2.0, 2.0)) >> limiter_stereo((0.005, 0.2));

        net.set_sample_rate(sample_rate);

        let mut backend = BlockRateAdapter64::new(Box::new(net.backend()));

        let mut next_value = move || backend.get_stereo();

        let err_fn = |err| eprintln!("an error occurred on stream: {}", err);
        let mut sampler_state = SamplerState {
            id: Vec::new(),
            sequencer,
            net,
            reverb,
        };
        sampler_state.id.resize(4, None);

        let stream = device
            .build_output_stream(
                &config,
                move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                    write_data(data, channels, &mut next_value)
                },
                err_fn,
                None,
            )
            .expect("failed to build output stream");
        stream.play().expect("failed to play stream");

        let dir_path = Path::new("orca/samples");

        // read the directory
        if !dir_path.exists() {
            fs::create_dir_all(dir_path).expect("Unable to create directory");
        }
        let entries = fs::read_dir(dir_path).expect("Unable to list files in directory");

        // filter for .wav files and load them
        let waves: Vec<Arc<Wave64>> = entries
            .filter_map(Result::ok)
            .filter(|entry| {
                // filter for .wav files
                let path = entry.path();
                path.is_file() && path.extension().map_or(false, |ext| ext == "wav")
            })
            .map(|entry| {
                // load each .wav file
                let path = entry.path();
                let wave =
                    Arc::new(Wave64::load(path.to_str().expect("Failed to load path")).expect("Failed to load track"));
                wave
            })
            .collect();

        let wave_noise = Arc::new(Wave64::render(44100.0, 0.01, &mut (pink())));

        loop {

            let mut notes = sampler_note_receiver.recv().expect("Failed to receive note");
            notes.iter_mut().enumerate().for_each(|(i, note)| {
                if note.started && note.duration == 0 {
                    if let Some(id) = sampler_state.id[i] {
                        sampler_state.sequencer.edit_relative(id, 0.02, 0.02);
                        sampler_state.id[i] = None;
                    }
                }
                if !note.started && sampler_state.id[i].is_none() {
                    note.started = true;
                    sampler_state.reverb.set(note.reverb as f64 * 0.0277);

                    let waveform = match note.slot {
                        0 => play_wave(note, waves.clone(), wave_noise.clone()),
                        1 => play_wave(note, waves.clone(), wave_noise.clone()),
                        2 => play_wave(note, waves.clone(), wave_noise.clone()),
                        3 => play_wave(note, waves.clone(), wave_noise.clone()),
                        4 => play_wave(note, waves.clone(), wave_noise.clone()),
                        _ => play_wave(note, waves.clone(), wave_noise.clone()),
                    };

                    sampler_state.id[i] = Some(sampler_state.sequencer.push_relative(
                        0.0,
                        f64::INFINITY,
                        Fade::Smooth,
                        0.0,
                        0.2,
                        Box::new(waveform),
                    ));
                    if let Some(id) = sampler_state.id[i] {
                        // sampler_state.id[i] = None;
                        sampler_state.sequencer.edit_relative(
                            id,
                            note.duration as f64 * 0.001,
                            0.2,
                        );
                        sampler_state.id[i] = None;
                    }
                }
            });
        }
    });
}

fn play_wave(note: &Note, waves: Vec<Arc<Wave64>>, wave_noise: Arc<Wave64>) -> Net64 {
    Net64::wrap(Box::new(
        (lfo(|t| xerp11(1.0, 1.0, spline_noise(1, t))) * {
            if note.speed as f64 >= 9.0 {
                note.speed as f64 / 100.0
            } else {
                note.speed as f64
            }
        }) >> resample(wave64(
            waves
                .clone()
                .get(note.sample as usize % (waves.len() + 1) % 35)
                .unwrap_or(&wave_noise),
            0,
            None,
        )),
    ))
}

