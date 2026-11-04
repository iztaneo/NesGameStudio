use crate::project::{AudioReplacement, NgsProject};
use crate::runtime::jukebox_apu::JukeboxEngine;
use eframe::egui::{self, Color32, Pos2, Sense, Stroke, Vec2};

pub struct AudioEditorState {
    pub selected_track: u8,
    pub status_msg: String,
}

impl Default for AudioEditorState {
    fn default() -> Self {
        Self {
            selected_track: 1,
            status_msg: String::new(),
        }
    }
}

impl AudioEditorState {
    pub fn ui(&mut self, ui: &mut egui::Ui, project: &mut NgsProject, jukebox: &mut JukeboxEngine) {
        if !project.loaded {
            super::guide::welcome_ui(ui);
            return;
        }

        super::guide::tab_hint(
            ui,
            "Haz clic en una escena para oír su canción real. Marca Mute para silenciar un instrumento o Solo para \
             oír sólo ese. Los botones #01..#28 prueban otros sonidos del juego.",
        );

        ui.columns(2, |cols| {
            // COLUMNA IZQUIERDA: Pistas Reales Capturadas del Juego + Disparador de IDs del Motor 6502
            cols[0].group(|ui| {
                ui.label(egui::RichText::new("1. Música Real del Juego (Ejecutada por su Motor 6502 + APU)").strong());
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    if ui
                        .button(if jukebox.playing { "⏸ Pausar Audio Real" } else { "▶ Reanudar Audio Real" })
                        .clicked()
                    {
                        jukebox.playing = !jukebox.playing;
                    }
                    if ui.button("⏹ Detener").clicked() {
                        jukebox.playing = false;
                    }
                    if ui.button("💾 Grabar 5s a .WAV Real").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .set_file_name(&format!("{}_audio_real_{:02X}.wav", project.game_title, self.selected_track))
                            .add_filter("WAV Audio", &["wav"])
                            .save_file()
                        {
                            match jukebox.export_real_nes_audio_to_wav(project, &path) {
                                Ok(_) => self.status_msg = format!("Audio real grabado en {}", path.display()),
                                Err(e) => self.status_msg = format!("Error: {}", e),
                            }
                        }
                    }
                });

                ui.add_space(6.0);
                ui.label("🎬 Selecciona una Escena Capturada para escuchar su música real:");
                for idx in 0..project.scenes.len() {
                    let is_active = jukebox.playing && jukebox.active_scene_music == idx;
                    let title = format!("🎵 {}", project.scenes[idx].title);
                    if ui.selectable_label(is_active, title).clicked() {
                        jukebox.play_captured_scene_music(project, idx);
                        self.selected_track = idx as u8;
                        self.status_msg = format!("Reproduciendo música real de '{}'", project.scenes[idx].title);
                    }
                }

                ui.separator();
                ui.label("🎹 Probar IDs de Pista / Efecto en la Cola de Sonido 6502 del Juego (#01..#28):");
                egui::ScrollArea::vertical().max_height(185.0).show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        for snd_id in 1..=40u8 {
                            let has_rep = project.audio_replacements.contains_key(&snd_id);
                            let label = if has_rep {
                                format!("🔁 #{:02X}", snd_id)
                            } else {
                                format!("▶ #{:02X}", snd_id)
                            };
                            if ui
                                .selectable_label(self.selected_track == snd_id, label)
                                .on_hover_text(format!(
                                    "Inyectar comando de sonido #{:02X} en el motor 6502 del juego",
                                    snd_id
                                ))
                                .clicked()
                            {
                                self.selected_track = snd_id;
                                jukebox.trigger_game_sound_id(project, snd_id);
                                self.status_msg = format!(
                                    "Comando de sonido #{:02X} enviado al motor 6502 del juego.",
                                    snd_id
                                );
                            }
                        }
                    });
                });

                ui.separator();
                ui.label(
                    egui::RichText::new(format!(
                        "🔁 Reemplazar Pista / ID #{:02X} por archivo .MP3 o .WAV externo:",
                        self.selected_track
                    ))
                    .strong(),
                );
                ui.horizontal(|ui| {
                    if ui.button("📂 Vincular .MP3 / .WAV a este ID").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Audio MP3 / WAV", &["mp3", "wav"])
                            .pick_file()
                        {
                            let ext_is_wav = path
                                .extension()
                                .and_then(|e| e.to_str())
                                .map(|s| s.eq_ignore_ascii_case("wav"))
                                .unwrap_or(false);
                            project.audio_replacements.insert(
                                self.selected_track,
                                AudioReplacement {
                                    track_id: self.selected_track,
                                    label: format!("Track #{:02X}", self.selected_track),
                                    external_file_path: path.display().to_string(),
                                    is_sfx_wav: ext_is_wav,
                                },
                            );
                            self.status_msg = format!(
                                "ID #{:02X} vinculado a '{}'",
                                self.selected_track,
                                path.display()
                            );
                        }
                    }
                    if project.audio_replacements.contains_key(&self.selected_track) {
                        if ui.button("🗑 Quitar reemplazo").clicked() {
                            project.audio_replacements.remove(&self.selected_track);
                        }
                    }
                });
                if let Some(rep) = project.audio_replacements.get(&self.selected_track) {
                    ui.colored_label(
                        Color32::from_rgb(80, 230, 150),
                        format!("✔ Se inyectará en el .C: {}", rep.external_file_path),
                    );
                }
            });

            // COLUMNA DERECHA: Control Real de Canales APU (Solo / Mute), Osciloscopio y Rutinas [AUDIO / APU]
            cols[1].group(|ui| {
                ui.label(egui::RichText::new("2. Mezclador Real de los 5 Canales APU (Solo / Mute en Vivo)").strong());
                let ch_names = [
                    "Canal 1 — Pulse 1 (Melodía Principal / Sweep)",
                    "Canal 2 — Pulse 2 (Armonía / Segundo Pulso)",
                    "Canal 3 — Triangle (Bajo de 16 escalones)",
                    "Canal 4 — Noise (Percusión / Explosiones LFSR)",
                    "Canal 5 — DMC / DPCM (Muestras de Batería / Voz)",
                ];

                for (i, name) in ch_names.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut jukebox.mute[i], "Mute");
                        ui.checkbox(&mut jukebox.solo[i], "Solo");
                        ui.label(*name);
                    });
                }

                ui.add_space(6.0);
                ui.label(format!(
                    "Osciloscopio de la APU Real ({} — {} Hz):",
                    jukebox.audio_out.active_device_name(),
                    jukebox.audio_out.sample_rate()
                ));
                let (o_rect, _) = ui.allocate_exact_size(Vec2::new(360.0, 95.0), Sense::hover());
                let painter = ui.painter_at(o_rect);
                painter.rect_filled(o_rect, 3.0, Color32::from_rgb(12, 16, 22));

                let n = jukebox.oscilloscope.len().max(2);
                let mut pts = Vec::with_capacity(n);
                for (idx, &val) in jukebox.oscilloscope.iter().enumerate() {
                    let x = o_rect.min.x + (idx as f32 / (n - 1) as f32) * o_rect.width();
                    let y = o_rect.center().y - (val * 2.2).clamp(-1.0, 1.0) * (o_rect.height() * 0.44);
                    pts.push(Pos2::new(x, y));
                }
                painter.add(egui::Shape::line(pts, Stroke::new(1.8, Color32::from_rgb(0, 230, 180))));

                ui.separator();
                let audio_subs: Vec<_> = project
                    .subroutines
                    .iter()
                    .filter(|s| s.category == "[AUDIO / APU]")
                    .collect();
                ui.label(
                    egui::RichText::new(format!(
                        "🔍 Subrutinas [AUDIO / APU] Detectadas en el Código ({})",
                        audio_subs.len()
                    ))
                    .strong(),
                );
                egui::ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
                    for sub in audio_subs {
                        ui.collapsing(
                            format!("{} (${:04X}) — Línea {}", sub.name, sub.addr, sub.line_in_asm),
                            |ui| {
                                for l in &sub.preview_asm {
                                    ui.monospace(l);
                                }
                            },
                        );
                    }
                });

                if !self.status_msg.is_empty() {
                    ui.colored_label(Color32::LIGHT_GREEN, &self.status_msg);
                }
            });
        });
    }
}
