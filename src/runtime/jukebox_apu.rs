use crate::project::NgsProject;
use nes_emu_rust::ui::audio::AudioOutput;
use std::fs::File;
use std::io::Write;
use std::path::Path;

#[cfg(target_os = "windows")]
pub mod winmm {
    use std::ffi::CString;

    #[link(name = "winmm")]
    extern "system" {
        fn mciSendStringA(
            lpstrCommand: *const i8,
            lpstrReturnString: *mut i8,
            uReturnLength: u32,
            hwndCallback: usize,
        ) -> u32;
    }

    pub fn play_file(path: &str) {
        stop_file();
        let open_cmd = format!("open \"{}\" type mpegvideo alias ngs_bgm", path);
        if let Ok(c_open) = CString::new(open_cmd) {
            unsafe {
                mciSendStringA(c_open.as_ptr(), std::ptr::null_mut(), 0, 0);
            }
        }
        if let Ok(c_play) = CString::new("play ngs_bgm from 0 repeat") {
            unsafe {
                mciSendStringA(c_play.as_ptr(), std::ptr::null_mut(), 0, 0);
            }
        }
    }

    pub fn stop_file() {
        if let Ok(c_stop) = CString::new("stop ngs_bgm") {
            unsafe {
                mciSendStringA(c_stop.as_ptr(), std::ptr::null_mut(), 0, 0);
            }
        }
        if let Ok(c_close) = CString::new("close ngs_bgm") {
            unsafe {
                mciSendStringA(c_close.as_ptr(), std::ptr::null_mut(), 0, 0);
            }
        }
    }
}

pub struct JukeboxEngine {
    pub audio_out: AudioOutput,
    pub playing: bool,
    pub current_track_id: u8,
    pub mci_playing_file: Option<String>,
    pub mute: [bool; 5],
    pub solo: [bool; 5],
    pub oscilloscope: Vec<f32>,
    temp_samples: Vec<f32>,
}

impl Default for JukeboxEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for JukeboxEngine {
    fn drop(&mut self) {
        self.stop_external_file();
    }
}

impl JukeboxEngine {
    pub fn new() -> Self {
        let audio_out = AudioOutput::new();
        Self {
            audio_out,
            playing: false,
            current_track_id: 1,
            mci_playing_file: None,
            mute: [false; 5],
            solo: [false; 5],
            oscilloscope: vec![0.0; 256],
            temp_samples: Vec::with_capacity(4096),
        }
    }

    pub fn play_external_file(&mut self, path: &str) {
        #[cfg(target_os = "windows")]
        {
            winmm::play_file(path);
        }
        self.mci_playing_file = Some(path.to_string());
    }

    pub fn stop_external_file(&mut self) {
        if self.mci_playing_file.is_some() {
            #[cfg(target_os = "windows")]
            {
                winmm::stop_file();
            }
            self.mci_playing_file = None;
        }
    }

    pub fn sync_channel_masks(&self, project: &mut NgsProject) {
        let any_solo = self.solo.iter().any(|&s| s);
        for ch in 0..5 {
            let enabled = if any_solo {
                self.solo[ch]
            } else {
                !self.mute[ch]
            };
            project.nes_live.bus.apu.channel_enabled[ch] = enabled;
        }
    }

    /// Dispara la reproducción de un tema o efecto. Si está vinculado a un archivo .MP3/.WAV lo reproduce con MCI,
    /// o de lo contrario ejecuta el motor sonoro APU del juego cargado.
    pub fn trigger_game_sound_id(&mut self, project: &mut NgsProject, sound_id: u8) {
        self.current_track_id = sound_id;

        // 1. Si la pista tiene reemplazo .MP3/.WAV, reproducirla directamente
        if let Some(rep) = project.audio_replacements.get(&sound_id) {
            self.play_external_file(&rep.external_file_path);
            self.playing = true;
            return;
        }

        // 2. Si no, detener cualquier audio externo y usar la APU de la NES
        self.stop_external_file();

        // Si tenemos un estado guardado específicamente para esta pista, restaurarlo
        if let Some(saved_state) = project.audio_track_states.get(&sound_id) {
            let _ = project.nes_live.from_save_state_bytes(saved_state);
        } else if sound_id <= 0x01 && !project.scenes.is_empty() {
            if let Some(sc) = project.scenes.first() {
                if !sc.save_state.is_empty() {
                    let _ = project.nes_live.from_save_state_bytes(&sc.save_state);
                }
            }
        } else if project.scenes.len() >= 3 {
            let _ = project.nes_live.from_save_state_bytes(&project.scenes[2].save_state);
        } else if let Some(sc) = project.scenes.first() {
            if !sc.save_state.is_empty() {
                let _ = project.nes_live.from_save_state_bytes(&sc.save_state);
            }
        }

        project
            .nes_live
            .bus
            .apu
            .set_sample_rate(self.audio_out.sample_rate());

        // Comandos dirigidos y limpios según el motor de juego detectado
        let title_lower = project.game_title.to_lowercase();
        if title_lower.contains("contra") {
            if sound_id < 0x10 {
                project.nes_live.bus.cpu_ram[0x0030] = sound_id;
            } else {
                project.nes_live.bus.cpu_ram[0x0031] = sound_id;
            }
        } else if title_lower.contains("mario") {
            if sound_id < 0x10 {
                project.nes_live.bus.cpu_ram[0x0FB0 & 0x7FF] = sound_id;
            } else {
                project.nes_live.bus.cpu_ram[0x0FEE & 0x7FF] = sound_id;
            }
        } else if title_lower.contains("castlevania") {
            project.nes_live.bus.cpu_ram[0x002A & 0x7FF] = sound_id;
        } else if project.sound_cmd_ram_addrs.len() == 1 {
            let target_addr = (project.sound_cmd_ram_addrs[0] as usize) & 0x7FF;
            project.nes_live.bus.cpu_ram[target_addr] = sound_id;
        }

        // Avanzar 3 frames para que el controlador APU absorba el comando
        for _ in 0..3 {
            project.nes_live.step_frame();
            project.nes_live.bus.apu.audio_samples.clear();
        }

        self.playing = true;
    }

    /// Ejecuta 1 frame de la CPU 6502 + APU real del juego si la rockola está en reproducción
    pub fn step_real_audio_frame(&mut self, project: &mut NgsProject) {
        if !self.playing || !project.loaded {
            return;
        }

        if self.mci_playing_file.is_some() {
            // Animación suave del osciloscopio mientras suena un MP3/WAV externo
            let t = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as f32 * 0.015)
                .unwrap_or(0.0);
            for (i, val) in self.oscilloscope.iter_mut().enumerate() {
                *val = ((i as f32 * 0.12 + t).sin() * 0.28).clamp(-1.0, 1.0);
            }
            return;
        }

        project
            .nes_live
            .bus
            .apu
            .set_sample_rate(self.audio_out.sample_rate());
        self.sync_channel_masks(project);

        // Si la cola de audio tiene menos de ~3 frames, avanzar la emulación 6502 + APU
        if self.audio_out.queued_samples() < 3000 {
            project.nes_live.step_frame();
            self.temp_samples.clear();
            project.nes_live.take_audio_samples(&mut self.temp_samples);

            // Actualizar osciloscopio con el canal izquierdo
            for (i, chunk) in self.temp_samples.chunks_exact(2).take(self.oscilloscope.len()).enumerate() {
                self.oscilloscope[i] = chunk[0];
            }

            self.audio_out.push_samples(&self.temp_samples);
        }
    }

    /// Graba 5 segundos (300 frames) del audio real generado por la APU del juego a un archivo .WAV
    pub fn export_real_nes_audio_to_wav(&mut self, project: &mut NgsProject, path: &Path) -> Result<(), String> {
        if !project.loaded {
            return Err("Primero abre un juego desde el menú Archivo.".to_string());
        }
        let sample_rate = 44_100u32;
        project.nes_live.bus.apu.set_sample_rate(sample_rate);
        self.sync_channel_masks(project);

        let mut recorded_mono = Vec::with_capacity(sample_rate as usize * 5);
        let mut frame_buf = Vec::new();
        for _ in 0..300 {
            project.nes_live.step_frame();
            frame_buf.clear();
            project.nes_live.take_audio_samples(&mut frame_buf);
            for pair in frame_buf.chunks_exact(2) {
                recorded_mono.push((pair[0] + pair[1]) * 0.5);
            }
        }

        let data_bytes = (recorded_mono.len() * 2) as u32;
        let mut f = File::create(path).map_err(|e| e.to_string())?;
        f.write_all(b"RIFF").map_err(|e| e.to_string())?;
        f.write_all(&(36 + data_bytes).to_le_bytes()).map_err(|e| e.to_string())?;
        f.write_all(b"WAVEfmt ").map_err(|e| e.to_string())?;
        f.write_all(&16u32.to_le_bytes()).map_err(|e| e.to_string())?;
        f.write_all(&1u16.to_le_bytes()).map_err(|e| e.to_string())?;
        f.write_all(&1u16.to_le_bytes()).map_err(|e| e.to_string())?;
        f.write_all(&sample_rate.to_le_bytes()).map_err(|e| e.to_string())?;
        f.write_all(&(sample_rate * 2).to_le_bytes()).map_err(|e| e.to_string())?;
        f.write_all(&2u16.to_le_bytes()).map_err(|e| e.to_string())?;
        f.write_all(&16u16.to_le_bytes()).map_err(|e| e.to_string())?;
        f.write_all(b"data").map_err(|e| e.to_string())?;
        f.write_all(&data_bytes.to_le_bytes()).map_err(|e| e.to_string())?;

        for s in recorded_mono {
            let pcm = (s.clamp(-0.98, 0.98) * 31000.0) as i16;
            f.write_all(&pcm.to_le_bytes()).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}
