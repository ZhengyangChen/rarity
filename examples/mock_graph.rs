use std::{
    net::{SocketAddrV4, UdpSocket},
    str::FromStr,
    time::Instant,
};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, FromSample, SampleFormat, SizedSample, Stream, StreamConfig,
};
use rarity_engine::A_OUT_NODE;
use rosc::{OscPacket, OscType};

use rarity::{
    engine::{
        AudioBuffer, FloatMessage, Graph, Message, MessageCollector, MessageValue, MidiMessage,
        NoteOn, PlayHead,
    },
    node::{DigitalOverDrive, SimpleSaw},
};

fn run<T: SizedSample + FromSample<f32>>(
    device: &Device,
    config: &StreamConfig,
    mut collector: MessageCollector,
    mut graph: Graph,
) -> Stream {
    let channels = config.channels as usize;
    let playhead = PlayHead {
        upper: 4,
        lower: 4,
        div: 4,
        samples_from_last_bar: 0.0,
        samples_per_quarter: 0.0,
    };
    let mut audio_in = AudioBuffer::new(4096);
    let mut audio_out = AudioBuffer::new(4096);
    device
        .build_output_stream(
            config.into(),
            move |data: &mut [T], _| {
                let frames = data.len() / channels;
                let audio_in_ref = audio_in.next_n_frames_ref(frames);
                let audio_out_mut = audio_out.next_n_frames_mut(frames);
                collector.collect();
                let message_in = collector.drain_frames(frames);
                graph.process(&playhead, frames, audio_in_ref, audio_out_mut, &message_in);
                for ((l, r), f) in audio_out
                    .next_n_frames_ref(frames)
                    .into_iter()
                    .zip(data.chunks_mut(channels))
                {
                    f[0] = T::from_sample_(*l as f32);
                    f[1] = T::from_sample_(*r as f32);
                }
                audio_in.next_n_frames_mut(frames).clear();
                audio_out.next_n_frames_mut(frames).clear();
                audio_in.forward(frames);
                audio_out.forward(frames);
            },
            |err| eprintln!("err: {}", err),
            None,
        )
        .unwrap()
}

fn main() {
    let mut graph = Graph::new("mock");
    graph.add_audio_source(SimpleSaw::new("simple_saw", 3));
    graph.add_audio_effect(DigitalOverDrive::new("overdrive"));
    graph.add_audio_link("simple_saw", "overdrive");
    graph.add_audio_link("overdrive", A_OUT_NODE);
    let mut collector = MessageCollector::new();
    let sender = collector.add_port(vec![]);
    let host = cpal::default_host();
    let device = host.default_output_device().unwrap();
    let config = device.default_output_config().unwrap();
    let sample_rate = config.sample_rate().0 as f64;
    println!("Sample rate: {}", sample_rate);
    graph.mock_prepare(sample_rate);

    let stream = match config.sample_format() {
        SampleFormat::I8 => run::<i8>(&device, &config.into(), collector, graph),
        SampleFormat::I16 => run::<i16>(&device, &config.into(), collector, graph),
        SampleFormat::I32 => run::<i32>(&device, &config.into(), collector, graph),
        SampleFormat::I64 => run::<i64>(&device, &config.into(), collector, graph),
        SampleFormat::U8 => run::<u8>(&device, &config.into(), collector, graph),
        SampleFormat::U16 => run::<u16>(&device, &config.into(), collector, graph),
        SampleFormat::U32 => run::<u32>(&device, &config.into(), collector, graph),
        SampleFormat::U64 => run::<u64>(&device, &config.into(), collector, graph),
        SampleFormat::F32 => run::<f32>(&device, &config.into(), collector, graph),
        SampleFormat::F64 => run::<f64>(&device, &config.into(), collector, graph),
        _ => panic!("Unknown SampleFormat"),
    };
    stream.play().unwrap();

    let addr = SocketAddrV4::from_str("169.254.237.244:7001").unwrap();
    let sock = UdpSocket::bind(addr).unwrap();
    let mut buf = [0u8; rosc::decoder::MTU];
    let start_time = Instant::now();
    loop {
        match sock.recv_from(&mut buf) {
            Ok((size, _addr)) => {
                let (_, packet) = rosc::decoder::decode_udp(&buf[..size]).unwrap();
                match packet {
                    OscPacket::Message(msg) => {
                        for value in msg.args {
                            if let OscType::Float(v) = value {
                                let now = Instant::now();
                                let frame = ((now - start_time).as_secs_f64() * sample_rate).floor()
                                    as usize;
                                let mut addr = msg
                                    .addr
                                    .split('/')
                                    .map(|s| s.to_string())
                                    .collect::<Vec<_>>();
                                addr.remove(0);
                                if addr.len() == 1 {
                                    let name = addr.pop().unwrap();
                                    addr.push("simple_saw".to_string());
                                    let msg = match name.parse::<u8>() {
                                        Ok(pitch) => {
                                            MessageValue::Midi(MidiMessage::NoteOn(NoteOn {
                                                pitch,
                                                velocity: (v * 128.0) as u8,
                                            }))
                                        }
                                        Err(_) => MessageValue::Float(FloatMessage {
                                            name,
                                            value: v as f64,
                                        }),
                                    };
                                    sender.send((frame, Message { addr, value: msg })).unwrap();
                                } else if addr.len() == 2 {
                                    let name = addr.pop().unwrap();
                                    let msg = MessageValue::Float(FloatMessage {
                                        name,
                                        value: v as f64,
                                    });
                                    sender.send((frame, Message { addr, value: msg })).unwrap();
                                }
                            }
                        }
                    }
                    OscPacket::Bundle(bundle) => {
                        println!("OSC Bundle: {:?}", bundle);
                    }
                }
            }
            Err(e) => {
                println!("Error receiving from socket: {}", e);
                break;
            }
        }
    }
}
