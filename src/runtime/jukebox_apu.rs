use crate::project::NgsProject;
use nes_emu_rust::ui::audio::AudioOutput;
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub struct JukeboxEngine {
    pub audio_out: AudioOutput,
    pub playing: bool,
    pub current_track_id: u8,
    pub active_scene_music: usize,
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

impl JukeboxEngine {
    pub fn new() -> Self {
        let audio_out = AudioOutput::new();
        Self {
            audio_out,
            playing: false,
            current_track_id: 1,
            active_scene_music: 2,
            mute: [false; 5],
            solo: [false; 5],
            oscilloscope: vec![0.0; 256],
            temp_samples: Vec::with_capacity(4096),
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

    /// Carga el estado de una escena capturada (ej. Título o Nivel 1 de Contra) y reproduce su música real
    pub fn play_captured_scene_music(&mut self, project: &mut NgsProject, scene_idx: usize) {
        if let Some(sc) = project.scenes.get(scene_idx) {
            if !sc.save_state.is_empty() {
                let _ = project.nes_live.from_save_state_bytes(&sc.save_state);
                project
                    .nes_live
                    .bus
                    .apu
                    .set_sample_rate(self.audio_out.sample_rate() as f32);
                self.active_scene_music = scene_idx;
                self.playing = true;
            }
        }
    }

    /// Envía un ID de pista/efecto (#01..#3F) a las casillas de comando de sonido del juego en RAM 6502
    pub fn trigger_game_sound_id(&mut self, project: &mut NgsProject, sound_id: u8) {
        self.current_track_id = sound_id;
        // Restaurar base desde Escena 0 (Título en reposo) o Escena activa
        if let Some(sc) = project.scenes.first() {
            if !sc.save_state.is_empty() {
                let _ = project.nes_live.from_save_state_bytes(&sc.save_state);
            }
        }
        project
            .nes_live
            .bus
            .apu
            .set_sample_rate(self.audio_out.sample_rate() as f32);

        // Escribir el ID de sonido tanto en las direcciones detectadas como en los registros de cola comunes de Konami/NES
        for &addr in &project.sound_cmd_ram_addrs {
            project.nes_live.bus.cpu_ram[(addr as usize) & 0x7FF] = sound_id;
        }
        // Direcciones de cola de sonido de Konami (Contra / Gradius / Life Force / Castlevania)
        for &konami_snd_addr in &[0x00F0usize, 0x00F1, 0x00F2, 0x00F3, 0x00F4, 0x00E0, 0x07F0] {
            if project.sound_cmd_ram_addrs.is_empty() || konami_snd_addr >= 0x00F0 {
                project.nes_live.bus.cpu_ram[konami_snd_addr] = sound_id;
            }
        }
        self.playing = true;
    }

    /// Ejecuta 1 frame de la CPU 6502 + APU real del juego si la rockola está en reproducción
    pub fn step_real_audio_frame(&mut self, project: &mut NgsProject) {
        if !self.playing || !project.loaded {
            return;
        }

        project
            .nes_live
            .bus
            .apu
            .set_sample_rate(self.audio_out.sample_rate() as f32);
        self.sync_channel_masks(project);

        // Si la cola de audio tiene menos de ~3 frames, avanzar la emulación 6502 + APU
        if self.audio_out.queued_samples() < 2400 {
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
        project.nes_live.bus.apu.set_sample_rate(sample_rate as f32);
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
