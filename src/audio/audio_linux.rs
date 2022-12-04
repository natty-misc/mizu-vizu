use crate::audio::WINDOW;
use byteorder::{ByteOrder, NativeEndian};
use libpulse_binding::def::BufferAttr;
use libpulse_binding::sample::{Format, Spec};
use libpulse_binding::stream::Direction;
use libpulse_simple_binding::Simple;

pub struct Recorder(Simple, Vec<u8>);

impl Recorder {
    pub fn new() -> Recorder {
        let spec = Spec {
            format: Format::S16NE,
            channels: 2,
            rate: 44100,
        };
        assert!(spec.is_valid());

        let s = Simple::new(
            None,
            "MizuVizu",
            Direction::Record,
            Some(
                "alsa_output.usb-SteelSeries_SteelSeries_Arctis_5_00000000-00.analog-game.monitor",
            ),
            "Audio vizualizer",
            &spec,
            None,
            Some(&BufferAttr {
                maxlength: (WINDOW * core::mem::size_of::<i16>()) as u32,
                tlength: u32::MAX,
                prebuf: u32::MAX,
                minreq: u32::MAX,
                fragsize: (WINDOW * core::mem::size_of::<i16>()) as u32,
            }),
        )
        .unwrap();

        let mut data_buf = Vec::new();
        data_buf.resize(WINDOW * core::mem::size_of::<i16>(), 0);
        Recorder(s, data_buf)
    }

    pub fn get_samples(&mut self, data_short: &mut [i16; WINDOW]) {
        self.0.read(self.1.as_mut_slice()).unwrap();

        NativeEndian::read_i16_into(&self.1, data_short);
    }
}
