use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};

pub struct ApuChannelState {
    pub mute: [bool; 5],
    pub solo: [bool; 5],
    pub playing: bool,
    pub current_track: u8,
    pub step_tick: u64,
    pub phase_p1: f32,
    pub phase_p2: f32,
    pub phase_tri: f32,
    pub lfsr_noise: u16,
    pub dmc_level: f32,
    pub rom_seed: Vec<u8>,
    pub oscilloscope: Vec<f32>,
}

impl Default for ApuChannelState {
    fn default() -> Self {
        Self {
            mute: [false; 5],
            solo: [false; 5],
            playing: false,
            current_track: 0,
            step_tick: 0,
            phase_p1: 0.0,
            phase_p2: 0.0,
            phase_tri: 0.0,
            lfsr_noise: 0x4000,
            dmc_level: 0.0,
            rom_seed: Vec::new(),
            oscilloscope: vec![0.0; 256],
        }
    }
}

impl ApuChannelState {
    pub fn is_channel_audible(&self, ch: usize) -> bool {
        let any_solo = self.solo.iter().any(|&s| s);
        if any_solo {
            self.solo[ch]
        } else {
            !self.mute[ch]
        }
    }

    /// Sintetiza una muestra de audio a 44100 Hz combinando los 5 canales de la APU NES
    pub fn next_sample(&mut self, sample_rate: f32) -> f32 {
        if !self.playing {
            return 0.0;
        }
        self.step_tick = self.step_tick.wrapping_add(1);
        let t_sec = self.step_tick as f32 / sample_rate;
        let track = self.current_track;

        // Escala musical retro (frecuencias pentatónicas / menor armónica + variaciones según PRG_ROM)
        const NOTES: [f32; 16] = [
            130.81, 146.83, 164.81, 174.61, 196.00, 220.00, 246.94, 261.63,
            293.66, 329.63, 349.23, 392.00, 440.00, 493.88, 523.25, 587.33,
        ];

        let is_sfx = track >= 0x10;
        let (f_p1, f_p2, f_tri, noise_trig, dmc_trig, env) = if !is_sfx {
            // Pista Musical (Track #00 .. #0F)
            let tempo_hz = 6.0 + (track % 4) as f32 * 1.5;
            let step_idx = (t_sec * tempo_hz) as usize;
            let seed_byte = if !self.rom_seed.is_empty() {
                self.rom_seed[(step_idx.wrapping_add(track as usize * 37)) % self.rom_seed.len()]
            } else {
                (step_idx * 7 + track as usize * 13) as u8
            };

            let n1 = NOTES[(seed_byte as usize + step_idx) & 15];
            let n2 = NOTES[((seed_byte as usize >> 2) + step_idx + 4) & 15] * 0.5;
            let n_tri = NOTES[(step_idx / 2 + track as usize) & 7] * 0.5;
            let env_decay = 1.0 - ((t_sec * tempo_hz).fract() * 0.65);
            let kick = (step_idx % 4 == 0) && ((t_sec * tempo_hz).fract() < 0.18);
            let snare = (step_idx % 4 == 2) && ((t_sec * tempo_hz).fract() < 0.22);
            (n1, n2, n_tri, snare, kick, env_decay)
        } else {
            // Efectos de Sonido Aislados (Track #10 .. #1F: Disparo, Explosión, Salto, PowerUp, etc.)
            let dur = 0.85;
            let local_t = t_sec % dur;
            let sfx_type = track & 0x07;
            match sfx_type {
                0 => {
                    // Disparo Rápido (Spread Gun / Blaster)
                    let f = (880.0 - local_t * 1900.0).max(90.0);
                    (f, 0.0, f * 0.5, local_t < 0.08, false, (1.0 - local_t * 2.5).max(0.0))
                }
                1 => {
                    // Explosión Pesada (Noise + DMC + Triangle bajo)
                    let f = (160.0 - local_t * 180.0).max(40.0);
                    (0.0, 0.0, f, local_t < 0.65, local_t < 0.35, (1.0 - local_t * 1.3).max(0.0))
                }
                2 => {
                    // Salto Retro (Sweep ascendente)
                    let f = 180.0 + local_t * 950.0;
                    (f, f * 0.5, 0.0, false, false, (1.0 - local_t * 2.2).max(0.0))
                }
                _ => {
                    // Power-Up / Fanfarria de Moneda o Nivel
                    let step = (local_t * 14.0) as usize;
                    let f = NOTES[(step * 2 + track as usize) & 15] * 1.5;
                    (f, f * 0.75, f * 0.5, false, false, (1.0 - local_t * 1.1).max(0.0))
                }
            }
        };

        // 1. Pulse 1 (Duty 25%)
        let mut s_p1 = 0.0;
        if self.is_channel_audible(0) && f_p1 > 20.0 {
            self.phase_p1 = (self.phase_p1 + f_p1 / sample_rate).fract();
            s_p1 = if self.phase_p1 < 0.25 { 1.0 } else { -1.0 } * env * 0.22;
        }

        // 2. Pulse 2 (Duty 50%)
        let mut s_p2 = 0.0;
        if self.is_channel_audible(1) && f_p2 > 20.0 {
            self.phase_p2 = (self.phase_p2 + f_p2 / sample_rate).fract();
            s_p2 = if self.phase_p2 < 0.50 { 1.0 } else { -1.0 } * env * 0.18;
        }

        // 3. Triangle (Bajo de 16 escalones)
        let mut s_tri = 0.0;
        if self.is_channel_audible(2) && f_tri > 20.0 {
            self.phase_tri = (self.phase_tri + f_tri / sample_rate).fract();
            let tri_raw = if self.phase_tri < 0.5 {
                self.phase_tri * 4.0 - 1.0
            } else {
                3.0 - self.phase_tri * 4.0
            };
            s_tri = (tri_raw * 8.0).round() / 8.0 * 0.28;
        }

        // 4. Noise (LFSR de 15 bits)
        let mut s_noise = 0.0;
        if self.is_channel_audible(3) && noise_trig {
            if self.step_tick % 6 == 0 {
                let fb = (self.lfsr_noise & 1) ^ ((self.lfsr_noise >> 1) & 1);
                self.lfsr_noise = (self.lfsr_noise >> 1) | (fb << 14);
            }
            s_noise = if (self.lfsr_noise & 1) != 0 { 0.18 } else { -0.18 } * env;
        }

        // 5. DMC / DPCM (Percusión delta)
        let mut s_dmc = 0.0;
        if self.is_channel_audible(4) && dmc_trig {
            self.dmc_level = ((t_sec * 110.0).sin() * 6.0).round() / 6.0;
            s_dmc = self.dmc_level * 0.22;
        }

        (s_p1 + s_p2 + s_tri + s_noise + s_dmc).clamp(-0.95, 0.95)
    }
}

pub struct JukeboxEngine {
    pub state: Arc<Mutex<ApuChannelState>>,
    _stream: Option<cpal::Stream>,
}

impl JukeboxEngine {
    pub fn new() -> Self {
        let state = Arc::new(Mutex::new(ApuChannelState::default()));
        let stream = Self::init_cpal_stream(state.clone()).ok();
        Self {
            state,
            _stream: stream,
        }
    }

    fn init_cpal_stream(state: Arc<Mutex<ApuChannelState>>) -> Result<cpal::Stream, String> {
        let host = cpal::default_host();
        let device = host.default_output_device().ok_or("Sin dispositivo de audio")?;
        let config = device.default_output_config().map_err(|e| e.to_string())?;
        let sample_rate = config.sample_rate().0 as f32;
        let channels = config.channels() as usize;

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device
                .build_output_stream(
                    &config.into(),
                    move |data: &mut [f32], _| {
                        if let Ok(mut st) = state.lock() {
                            for (i, frame) in data.chunks_mut(channels).enumerate() {
                                let s = st.next_sample(sample_rate);
                                if i < st.oscilloscope.len() {
                                    st.oscilloscope[i] = s;
                                }
                                for ch_sample in frame.iter_mut() {
                                    *ch_sample = s;
                                }
                            }
                        }
                    },
                    |_| {},
                    None,
                )
                .map_err(|e| e.to_string())?,
            _ => return Err("Formato de audio no F32".to_string()),
        };
        stream.play().map_err(|e| e.to_string())?;
        Ok(stream)
    }

    /// Exporta 4 segundos de la pista actual a un archivo .WAV estándar de 44.1 kHz 16-bit
    pub fn export_track_to_wav(&self, track_id: u8, path: &Path) -> Result<(), String> {
        let mut temp_state = ApuChannelState::default();
        if let Ok(curr) = self.state.lock() {
            temp_state.rom_seed = curr.rom_seed.clone();
            temp_state.mute = curr.mute;
            temp_state.solo = curr.solo;
        }
        temp_state.playing = true;
        temp_state.current_track = track_id;

        let sample_rate = 44_100u32;
        let total_samples = sample_rate * 4; // 4 segundos
        let data_bytes = total_samples * 2;
        let mut f = File::create(path).map_err(|e| e.to_string())?;

        // Cabecera RIFF WAV PCM 16-bit Mono
        f.write_all(b"RIFF").map_err(|e| e.to_string())?;
        f.write_all(&(36 + data_bytes).to_le_bytes()).map_err(|e| e.to_string())?;
        f.write_all(b"WAVEfmt ").map_err(|e| e.to_string())?;
        f.write_all(&16u32.to_le_bytes()).map_err(|e| e.to_string())?;
        f.write_all(&1u16.to_le_bytes()).map_err(|e| e.to_string())?; // PCM
        f.write_all(&1u16.to_le_bytes()).map_err(|e| e.to_string())?; // 1 canal
        f.write_all(&sample_rate.to_le_bytes()).map_err(|e| e.to_string())?;
        f.write_all(&(sample_rate * 2).to_le_bytes()).map_err(|e| e.to_string())?;
        f.write_all(&2u16.to_le_bytes()).map_err(|e| e.to_string())?;
        f.write_all(&16u16.to_le_bytes()).map_err(|e| e.to_string())?;
        f.write_all(b"data").map_err(|e| e.to_string())?;
        f.write_all(&data_bytes.to_le_bytes()).map_err(|e| e.to_string())?;

        for _ in 0..total_samples {
            let s = temp_state.next_sample(sample_rate as f32);
            let pcm = (s * 30000.0) as i16;
            f.write_all(&pcm.to_le_bytes()).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}
