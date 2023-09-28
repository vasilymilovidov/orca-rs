use std::{
    thread::{self},
};

use cpal::{
    Device,
    FromSample,
    SizedSample,
    StreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use crossbeam::channel::Receiver;
use fundsp::{
    hacker::*,
    hacker::{midi_hz, multipass, pan, reverb_stereo, shared, var},
    prelude::Net64,
    sequencer::Sequencer64,
};

use crate::note_events::Note;

#[allow(dead_code)]
#[derive(Clone)]
pub struct SynthState {
    id: Vec<Option<EventId>>,
    sequencer: Sequencer64,
    net: Net64,
    reverb: Shared<f64>,
}

pub fn synth_out(
    synth_note_receiver: Receiver<Vec<Note>>,
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
            synth_note_receiver,
        ),
        cpal::SampleFormat::F64 => run::<f64>(
            device,
            config.into(),
            synth_note_receiver,
        ),
        cpal::SampleFormat::I16 => run::<i16>(
            device,
            config.into(),
            synth_note_receiver,
        ),
        cpal::SampleFormat::U16 => run::<u16>(
            device,
            config.into(),
            synth_note_receiver,
        ),
        _ => panic!("Unsupported format"),
    }
}

#[allow(clippy::precedence)]
pub fn run<T>(
    device: Device,
    config: StreamConfig,
    synth_note_receiver: Receiver<Vec<Note>>,
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
            & (var(&reverb) >> follow(0.01) >> split()) * reverb_stereo(2.0, 2.0));
        net = net >> (declick() | declick()) >> (dcblock() | dcblock()) >> (limiter((0.0, 0.1)) | limiter((0.0, 0.1)));
        net.set_sample_rate(sample_rate);

        let mut backend = BlockRateAdapter64::new(Box::new(net.backend()));

        let mut next_value = move || backend.get_stereo();

        let err_fn = |err| eprintln!("an error occurred on stream: {}", err);

        let mut synth_state = SynthState {
            id: Vec::new(),
            sequencer,
            net,
            reverb,
        };
        synth_state.id.resize(36, None);

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


        loop {
            let mut notes = synth_note_receiver.recv().expect("failed to receive note");
            notes.iter_mut().enumerate().for_each(|(i, note)| if !note.started && synth_state.id[i].is_none() {
                let pitch = midi_hz(note.note_number as f64);
                synth_state.reverb.set(note.reverb as f64 * 0.0277);
                let waveform = match note.engine {
                    0 => Net64::wrap(Box::new(oversample(sine_synth(
                        pitch,
                        note.speed as f64,
                        note.velocity as f64 * 0.0076,
                        sine_hz(pitch)
                    )))),
                    1 => Net64::wrap(Box::new(oversample(saw_synth(
                        pitch,
                        note.speed as f64,
                        note.velocity as f64 * 0.0076,
                        sine_hz(pitch)
                    )))),
                    2 => Net64::wrap(Box::new(oversample(tri_synth(
                        pitch,
                        note.speed as f64,
                        note.velocity as f64 * 0.0076,
                        sine_hz(pitch)
                    )))),
                    3 => Net64::wrap(Box::new(oversample(square_synth(
                        pitch,
                        note.speed as f64,
                        note.velocity as f64 * 0.0076,
                        sine_hz(pitch)
                    )))),
                    _ => {
                        Net64::wrap(Box::new(
                            bassdrum2(
                                note.speed as f64 * 0.0076,
                                midi_hz(note.note_number as f64),
                                midi_hz(note.note_number as f64 * 0.5),
                                note.velocity as f64 * 0.0076,
                            )))
                    }
                };

                synth_state.id[i] = Some(synth_state.sequencer.push_relative(
                    0.0,
                    note.duration as f64 * 0.001,
                    Fade::Smooth,
                    0.01,
                    note.duration as f64 * 0.001,
                    Box::new(waveform),
                ));
                if let Some(_id) = synth_state.id[i] {
                    synth_state.id[i] = None;
                }
            });
        }
    });
}

pub fn write_data<T>(output: &mut [T], channels: usize, next_sample: &mut dyn FnMut() -> (f64, f64))
    where
        T: SizedSample + FromSample<f64>,
{
    for frame in output.chunks_mut(channels) {
        let sample = next_sample();
        let left: T = T::from_sample(sample.0);
        let right: T = T::from_sample(sample.1);

        for (channel, sample) in frame.iter_mut().enumerate() {
            if channel & 1 == 0 {
                *sample = left;
            } else {
                *sample = right;
            }
        }
    }
}


pub fn bassdrum2(
    noise_level: f64,
    pitch0: f64,
    pitch1: f64,
    velocity: f64,
) -> An<impl AudioNode<Sample=f64, Inputs=U0, Outputs=U1>> {
    let sweep =
        lfo(move |t| xerp(pitch0, pitch1, clamp01(t * 40.0)) - 10.0 * t) >> sine();
    let noise_env = lfo(|t| exp(-t * 8.0));
    let noise = (noise() * noise_level) >> bandpass_hz(7334.0, 0.5);
    let kick_env = lfo(|t| exp(-t * 10.0));
    (
        (
            (sweep + (noise * noise_env)) * velocity
        ) * kick_env
    ) >> declick_s(xerp(0.002, 0.00002, 0.7))
}

pub fn sine_synth(
    pitch: f64,
    fm: f64,
    velocity: f64,
    waveform: An<Pipe<f64, Constant<U1, f64>, Sine<f64>>>,
) -> An<impl AudioNode<Sample=f64, Inputs=U0, Outputs=U1>> {
    let wave = waveform * ((pitch * 0.75) * fm) * 1.0 >> sine();
    let env = lfo(|t| exp(-t * 10.0));
    (wave * velocity) * env >> limiter((0.0, 0.1)) >> declick_s(xerp(0.002, 0.00002, 0.7))
}

pub fn saw_synth(
    pitch: f64,
    fm: f64,
    velocity: f64,
    waveform: An<Pipe<f64, Constant<U1, f64>, Sine<f64>>>,
) -> An<impl AudioNode<Sample=f64, Inputs=U0, Outputs=U1>> {
    let wave = waveform * ((pitch * 0.75) * fm) * 1.0 >> saw();
    let env = lfo(|t| exp(-t * 10.0));
    (wave * velocity) * env >> limiter((0.0, 0.1)) >> declick_s(xerp(0.002, 0.00002, 0.7))
}

pub fn tri_synth(
    pitch: f64,
    fm: f64,
    velocity: f64,
    waveform: An<Pipe<f64, Constant<U1, f64>, Sine<f64>>>,
) -> An<impl AudioNode<Sample=f64, Inputs=U0, Outputs=U1>> {
    let wave = waveform * ((pitch * 0.75) * fm) * 1.0 >> triangle();
    let env = lfo(|t| exp(-t * 10.0));
    (wave * velocity) * env >> limiter((0.0, 0.1)) >> declick_s(xerp(0.002, 0.00002, 0.7))
}

pub fn square_synth(
    pitch: f64,
    fm: f64,
    velocity: f64,
    waveform: An<Pipe<f64, Constant<U1, f64>, Sine<f64>>>,
) -> An<impl AudioNode<Sample=f64, Inputs=U0, Outputs=U1>> {
    let wave = waveform * ((pitch * 0.75) * fm) * 1.0 >> square();
    let env = lfo(|t| exp(-t * 10.0));
    (wave * velocity) * env >> limiter((0.0, 0.1)) >> declick_s(xerp(0.002, 0.00002, 0.7))
}