extern crate core;

mod audio;

use console_engine::{pixel, Color, KeyCode};
use rustfft::num_traits::Pow;
use rustfft::{num_complex::Complex, FftPlanner};

fn ra(frequency: f32) -> f32 {
    let denom = (frequency.pow(2.0) + 20.6_f32.pow(2.0))
        * ((frequency.pow(2.0) + 107.7_f32.pow(2.0)) * (frequency.pow(2.0) + 737.9_f32.pow(2.0)))
            .sqrt()
        * (frequency.pow(2.0) + 12194.0_f32.pow(2.0));

    12194_f32.pow(2.0) * frequency.pow(4.0) / denom
}

fn a_weight(frequency: f32) -> f32 {
    20.0f32 * ra(frequency).log10() + 2.0f32
}

fn inv_loudness(value: f32, frequency: f32) -> f32 {
    value / 10.0f32.pow(a_weight(frequency) / 20.0f32)
}

fn slow_to_freq(slow: f32) -> f32 {
    let freq_par = slow as f32 / (audio::SAMPLES_LOW - 1) as f32;

    22050.0f32 - (1.0f32 - freq_par.pow(2.5f32)) * 22030.0f32
}

fn main() {
    let mut data_short = [0i16; audio::WINDOW];

    let mut recorder = audio::Recorder::new();

    let mut engine = console_engine::ConsoleEngine::init(150, 55, 60).unwrap();
    const WINDOW_ITERS: usize = 4;
    const WINDOW_BUF_SIZE: usize = WINDOW_ITERS * audio::SAMPLES;

    let mut i_buffer = [vec![0.0f32; WINDOW_BUF_SIZE], vec![0.0f32; WINDOW_BUF_SIZE]];

    let mut buffer = [
        vec![
            Complex {
                re: 0.0f32,
                im: 0.0f32
            };
            WINDOW_BUF_SIZE
        ],
        vec![
            Complex {
                re: 0.0f32,
                im: 0.0f32
            };
            WINDOW_BUF_SIZE
        ],
    ];

    let mut o_buffer = [
        vec![0.0f32; audio::SAMPLES_LOW],
        vec![0.0f32; audio::SAMPLES_LOW],
    ];

    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(WINDOW_BUF_SIZE);

    let mut frame_i = 0;
    let mut frame = 0;
    let mut fps = 0;
    let mut last_measurement = std::time::Instant::now();

    loop {
        let rec_start = std::time::Instant::now();
        recorder.get_samples(&mut data_short);
        let rec_end = std::time::Instant::now();

        const HEIGHT: i32 = 44;

        engine.wait_frame();
        engine.clear_screen();

        for buf in i_buffer.iter_mut() {
            buf.rotate_left(audio::SAMPLES);
        }

        let offset = WINDOW_BUF_SIZE - audio::SAMPLES;

        for (idx, v) in data_short.iter().enumerate() {
            i_buffer[idx % audio::CHANNELS][offset + idx / audio::CHANNELS] =
                (*v as f32) / (i16::MAX as f32);
        }

        for (buf, o_buf) in i_buffer.iter().zip(buffer.iter_mut()) {
            for (idx, val) in buf.iter().enumerate() {
                let window = 0.5f32
                    * (1.0f32
                        - (2.0f32 * std::f32::consts::PI * (idx as f32)
                            / (WINDOW_BUF_SIZE as f32))
                            .cos());
                o_buf[idx].re = *val * window;
                o_buf[idx].im = 0.0f32;
            }
        }

        for buf in buffer.iter_mut() {
            fft.process(buf);
        }

        for buf in o_buffer.iter_mut() {
            for it in buf.iter_mut() {
                *it *= 0.6;
            }
        }

        let rs_buffer = buffer
            .iter()
            .map(|buf| {
                buf.iter()
                    .take(WINDOW_BUF_SIZE / 2)
                    .map(|c| c.norm())
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        for (buf_idx, buf) in rs_buffer.iter().enumerate() {
            for (idx, val) in buf.iter().enumerate() {
                if idx == 0 {
                    continue;
                }

                let i_freq =
                    (idx as f32 * 44100_f32 / 2.0f32) / ((WINDOW_BUF_SIZE as f32) / 2.0f32);

                let v = val / (buf.len() as f32).sqrt();

                for i in 0..audio::SAMPLES_LOW {
                    let freq = slow_to_freq(i as f32);
                    let freq1 = slow_to_freq((i + 1) as f32);

                    let loudness = v * inv_loudness(1.0f32, freq).abs().sqrt();

                    if (i_freq - freq).abs() < (freq1 - freq).abs() {
                        o_buffer[buf_idx][i] += loudness;
                    }
                }
            }
        }

        let zoom = 0.02f32;

        for idx in 0..o_buffer[0].len() {
            let freq = slow_to_freq(idx as f32) as u32;

            let x_pos: i32 = (idx * (audio::CHANNELS + 1)) as i32;

            for (jdx, digit) in freq.to_string().chars().enumerate() {
                engine.print_fbg(
                    x_pos,
                    HEIGHT + 1 + (jdx as i32),
                    &digit.to_string(),
                    Color::White,
                    Color::Reset,
                );
            }
        }

        let colors = [Color::Blue, Color::Green];
        for (channel, buf) in o_buffer.iter().enumerate() {
            for (idx, val) in buf.iter().enumerate() {
                let x_pos: i32 = (idx * (audio::CHANNELS + 1)) as i32 + channel as i32;

                let loudness = *val;

                let rv: i32 = (loudness * (HEIGHT as f32) * zoom) as i32;
                engine.line(
                    x_pos,
                    HEIGHT - rv.min(HEIGHT),
                    x_pos,
                    HEIGHT,
                    pixel::pxl_fg('*', colors[channel]),
                );
            }
        }

        engine.print(0, 0, format!("Frame: {}", frame).as_str());
        frame += 1;
        frame_i += 1;

        let now = std::time::Instant::now();

        if now.duration_since(last_measurement).as_secs() >= 1 {
            last_measurement = now;
            fps = frame_i;
            frame_i = 0;
        }

        engine.print(0, 1, format!("FPS: {}", fps).as_str());
        engine.print(
            0,
            2,
            format!("Recording time: {}ms", (rec_end - rec_start).as_millis()).as_str(),
        );

        if engine.is_key_pressed(KeyCode::Char('q')) {
            break;
        }

        engine.draw();
    }
}
