use crate::audio::{CHANNELS, WINDOW};
use rubato::{InterpolationParameters, InterpolationType, Resampler, SincFixedIn, WindowFunction};
use std::collections::VecDeque;
use std::ptr::null_mut;
use winapi::shared::minwindef::{DWORD, LPVOID, PBYTE};
use winapi::shared::mmreg::WAVEFORMATEX;
use winapi::shared::ntdef::NULL;
use winapi::um::audioclient::{IAudioCaptureClient, IAudioClient, AUDCLNT_BUFFERFLAGS_SILENT};
use winapi::um::audiosessiontypes::{AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_LOOPBACK};
use winapi::um::combaseapi::{
    CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINITBASE_MULTITHREADED,
};
use winapi::um::mmdeviceapi::{
    eMultimedia, eRender, IMMDevice, IMMDeviceEnumerator, MMDeviceEnumerator,
};
use winapi::{Class, Interface};

pub struct Recorder {
    audio_client: *mut IAudioClient,
    capture_client: *mut IAudioCaptureClient,
    device_enumerator: *mut IMMDeviceEnumerator,
    audio_device: *mut IMMDevice,
    data_buffer: VecDeque<i16>,
    resampler: SincFixedIn<f32>,
}

impl Recorder {
    pub fn new() -> Recorder {
        unsafe {
            CoInitializeEx(NULL, COINITBASE_MULTITHREADED);
        }

        let device_enumerator = unsafe {
            let mut imm_device_enumerator: LPVOID = NULL;

            let result = CoCreateInstance(
                &MMDeviceEnumerator::uuidof(),
                null_mut(),
                CLSCTX_ALL,
                &IMMDeviceEnumerator::uuidof(),
                &mut imm_device_enumerator,
            );

            if result < 0 || imm_device_enumerator == NULL {
                panic!(
                    "Failed to create IMMDeviceEnumerator. HRESULT: {:X}",
                    result
                );
            }

            imm_device_enumerator as *mut IMMDeviceEnumerator
        };

        let audio_device = unsafe {
            let mut audio_device: *mut IMMDevice = null_mut();

            let result = (*device_enumerator).GetDefaultAudioEndpoint(
                eRender,
                eMultimedia,
                &mut audio_device,
            );

            if result < 0 || audio_device.is_null() {
                panic!(
                    "Failed to get the default audio endpoint. HRESULT: {:X}",
                    result
                );
            }

            audio_device
        };

        let audio_client = unsafe {
            let mut audio_client: LPVOID = NULL;

            let result = (*audio_device).Activate(
                &IAudioClient::uuidof(),
                CLSCTX_ALL,
                null_mut(),
                &mut audio_client,
            );

            if result < 0 || audio_client == NULL {
                panic!("Failed to activate the audio client. HRESULT: {:X}", result);
            }

            audio_client as *mut IAudioClient
        };

        let format = unsafe {
            let mut format: *mut WAVEFORMATEX = null_mut();
            let result = (*audio_client).GetMixFormat(&mut format);

            if result < 0 || format.is_null() {
                panic!("Failed to get the mix format. HRESULT: {:X}", result);
            }

            format as *mut WAVEFORMATEX
        };

        let (samples, align, bits_per_sample, channels, format_tag, cb_size) = unsafe {
            (
                (*format).nSamplesPerSec,
                (*format).nBlockAlign,
                (*format).wBitsPerSample,
                (*format).nChannels,
                (*format).wFormatTag,
                (*format).cbSize,
            )
        };

        assert!(channels == CHANNELS as u16);
        assert!(align == CHANNELS as u16 * std::mem::size_of::<f32>() as u16);

        println!("Bits per sample:    {}", bits_per_sample);
        println!("Block size of data: {}", align);
        println!("Samples:            {}", samples);
        println!("Channels:           {}", channels);
        println!("Size:               {}", cb_size);
        println!("Format tag:         {:X}", format_tag);

        unsafe {
            let result = (*audio_client).Initialize(
                AUDCLNT_SHAREMODE_SHARED,
                AUDCLNT_STREAMFLAGS_LOOPBACK,
                10000000,
                0,
                format,
                null_mut(),
            );

            if result < 0 {
                panic!(
                    "Failed to initialize the audio client. HRESULT: {:X}",
                    result
                );
            }
        }

        let buffer_size = unsafe {
            let mut buffer_size: u32 = 0;

            let result = (*audio_client).GetBufferSize(&mut buffer_size);

            if result < 0 {
                panic!("Failed to get the buffer size.. HRESULT: {:X}", result);
            }

            buffer_size
        };

        let capture_client = unsafe {
            let mut capture_client: LPVOID = NULL;

            let result =
                (*audio_client).GetService(&IAudioCaptureClient::uuidof(), &mut capture_client);

            if result < 0 || capture_client == NULL {
                panic!("Failed to get the capture client. HRESULT: {:X}", result);
            }

            capture_client as *mut IAudioCaptureClient
        };

        unsafe {
            let result = (*audio_client).Start();

            if result < 0 {
                panic!("Failed to start the audio client. HRESULT: {:X}", result);
            }
        }

        let params = InterpolationParameters {
            sinc_len: 256,
            f_cutoff: 0.95,
            interpolation: InterpolationType::Linear,
            oversampling_factor: 256,
            window: WindowFunction::BlackmanHarris2,
        };

        let resampler =
            SincFixedIn::<f32>::new(44100.0f64 / samples as f64, 2.0, params, 480, 2).unwrap();

        Recorder {
            audio_client,
            capture_client,
            device_enumerator,
            audio_device,
            resampler,
            data_buffer: VecDeque::with_capacity(WINDOW),
        }
    }

    pub fn get_samples(&mut self, data_short: &mut [i16; WINDOW]) {
        loop {
            let buffer_size = unsafe {
                let mut buf_size: u32 = 0;

                let result = (*self.capture_client).GetNextPacketSize(&mut buf_size);

                if result < 0 {
                    // Failed to get the next packet size.
                    return;
                }

                buf_size
            };

            if buffer_size == 0 {
                // No packet data.
                break;
            }

            let buffer = unsafe {
                let mut buf: PBYTE = NULL as PBYTE;
                let mut frames_available: u32 = 0;

                let mut flags: DWORD = 0;

                let result = (*self.capture_client).GetBuffer(
                    &mut buf,
                    &mut frames_available,
                    &mut flags,
                    null_mut(),
                    null_mut(),
                );

                if result < 0 || buf.is_null() {
                    // Failed to get the buffer.
                    return;
                }

                if frames_available == 0
                    || flags & AUDCLNT_BUFFERFLAGS_SILENT == AUDCLNT_BUFFERFLAGS_SILENT
                {
                    // No frames available.
                    break;
                }

                std::slice::from_raw_parts_mut(
                    buf as *mut f32,
                    frames_available as usize * CHANNELS as usize,
                )
            };

            // TODO: Shorter frame counts than the buffer size.

            let mut rs_buf = vec![
                Vec::with_capacity(buffer.len() / 2),
                Vec::with_capacity(buffer.len() / 2),
            ];

            for (i, sample) in buffer.iter().enumerate() {
                rs_buf[i % CHANNELS].push(*sample);
            }

            unsafe {
                let result =
                    (*self.capture_client).ReleaseBuffer(buffer.len() as u32 / CHANNELS as u32);
                if result < 0 {
                    // Failed to release the buffer.
                    return;
                }
            }

            let resampled = self.resampler.process(&rs_buf, None).unwrap();

            for i in 0..resampled[0].len() {
                for chan in resampled.iter() {
                    self.data_buffer.push_back((chan[i] * 32768.0) as i16);
                }
            }
        }

        let data_cnt = data_short.len();
        if self.data_buffer.len() >= data_cnt {
            let src = &self.data_buffer.make_contiguous()[..data_cnt];
            data_short.copy_from_slice(src);
            self.data_buffer.drain(..data_cnt);
        }
    }
}

impl Drop for Recorder {
    fn drop(&mut self) {
        unsafe {
            (*(self.audio_device as *mut IMMDevice)).Release();
            (*(self.capture_client as *mut IAudioCaptureClient)).Release();
            (*(self.audio_client as *mut IAudioClient)).Release();
            (*(self.device_enumerator as *mut IMMDeviceEnumerator)).Release();
        }
    }
}
