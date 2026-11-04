use crate::project::{AudioReplacement, NgsProject};
use crate::runtime::jukebox_apu::JukeboxEngine;
use eframe::egui::{self, Color32, Pos2, Sense, Stroke, Vec2};

pub struct TrackEntry {
    pub id: u8,
    pub title: &'static str,
    pub is_music: bool,
}

pub const DEFAULT_TRACKS: &[TrackEntry] = &[
    // --- TEMAS MUSICALES ---
    TrackEntry { id: 0x01, title: "Pista 01 — Pantalla de Título & Menú", is_music: true },
    TrackEntry { id: 0x02, title: "Pista 02 — Nivel 1: Selva / Jungla", is_music: true },
    TrackEntry { id: 0x03, title: "Pista 03 — Nivel 2: Base 3D Militar", is_music: true },
    TrackEntry { id: 0x04, title: "Pista 04 — Nivel 3: Cascada", is_music: true },
    TrackEntry { id: 0x05, title: "Pista 05 — Nivel 5: Campo de Nieve", is_music: true },
    TrackEntry { id: 0x06, title: "Pista 06 — Nivel 6: Zona Energética", is_music: true },
    TrackEntry { id: 0x07, title: "Pista 07 — Nivel 7: Hangar", is_music: true },
    TrackEntry { id: 0x08, title: "Pista 08 — Nivel 8: Guarida Alienígena", is_music: true },
    TrackEntry { id: 0x09, title: "Pista 09 — Batalla de Jefe (Boss Fight)", is_music: true },
    TrackEntry { id: 0x0A, title: "Pista 0A — Nivel Completado (Fanfarria)", is_music: true },
    TrackEntry { id: 0x0B, title: "Pista 0B — Final y Créditos", is_music: true },
    TrackEntry { id: 0x0C, title: "Pista 0C — Game Over", is_music: true },
    // --- EFECTOS DE SONIDO ---
    TrackEntry { id: 0x10, title: "SFX 10 — Disparo de Rifle Normal", is_music: false },
    TrackEntry { id: 0x11, title: "SFX 11 — Disparo Spread Gun (Escopeta Múltiple)", is_music: false },
    TrackEntry { id: 0x12, title: "SFX 12 — Disparo de Rayo Láser", is_music: false },
    TrackEntry { id: 0x13, title: "SFX 13 — Disparo de Fuego / Lanzallamas", is_music: false },
    TrackEntry { id: 0x14, title: "SFX 14 — Salto del Soldado", is_music: false },
    TrackEntry { id: 0x15, title: "SFX 15 — Explosión Pequeña", is_music: false },
    TrackEntry { id: 0x16, title: "SFX 16 — Explosión Grande / Jefe", is_music: false },
    TrackEntry { id: 0x17, title: "SFX 17 — Daño / Caída de Soldado", is_music: false },
    TrackEntry { id: 0x18, title: "SFX 18 — Cápsula / Halcón de Arma en Vuelo", is_music: false },
    TrackEntry { id: 0x19, title: "SFX 19 — Alarma / Sirena de la Base", is_music: false },
    TrackEntry { id: 0x1A, title: "SFX 1A — Impacto en Metal / Rebote", is_music: false },
    TrackEntry { id: 0x1B, title: "SFX 1B — Torreta Desplegada", is_music: false },
];

pub struct AudioEditorState {
    pub selected_track: u8,
    pub status_msg: String,
}

impl Default for AudioEditorState {
    fn default() -> Self {
        Self {
            selected_track: 2, // Por defecto Nivel 1 (Jungla)
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
            "Selecciona cualquier pista o efecto de la lista para escucharlo al instante. \
             Puedes aislar instrumentos con Mute/Solo o reemplazar la música por tu propio archivo .MP3 / .WAV.",
        );

        ui.columns(2, |cols| {
            // COLUMNA IZQUIERDA: Lista Completa de Pistas del Juego (BGM y SFX)
            cols[0].group(|ui| {
                ui.horizontal(|ui| {
                    ui.heading("🎵 Lista Completa de Pistas del Juego");
                    ui.separator();
                    if ui
                        .button(if jukebox.playing { "⏸ Pausar" } else { "▶ Reproducir" })
                        .clicked()
                    {
                        jukebox.playing = !jukebox.playing;
                        if jukebox.playing {
                            jukebox.trigger_game_sound_id(project, self.selected_track);
                        }
                    }
                    if ui.button("⏹ Detener").clicked() {
                        jukebox.playing = false;
                    }
                    if ui.button("💾 Grabar a .WAV").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .set_file_name(&format!("{}_pista_{:02X}.wav", project.game_title, self.selected_track))
                            .add_filter("Audio WAV", &["wav"])
                            .save_file()
                        {
                            match jukebox.export_real_nes_audio_to_wav(project, &path) {
                                Ok(_) => self.status_msg = format!("Pista grabada en {}", path.display()),
                                Err(e) => self.status_msg = format!("Error al exportar: {}", e),
                            }
                        }
                    }
                });

                ui.add_space(6.0);
                egui::ScrollArea::vertical()
                    .max_height(380.0)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("🎶 TEMAS MUSICALES (BGM):").strong().color(Color32::from_rgb(180, 220, 255)));
                        for track in DEFAULT_TRACKS.iter().filter(|t| t.is_music) {
                            let is_sel = self.selected_track == track.id;
                            let has_rep = project.audio_replacements.contains_key(&track.id);
                            let label = if has_rep {
                                format!("🔁 {} [REEMPLAZADA POR MP3/WAV]", track.title)
                            } else {
                                track.title.to_string()
                            };

                            let resp = ui.selectable_label(is_sel, label);
                            if resp.clicked() {
                                self.selected_track = track.id;
                                jukebox.trigger_game_sound_id(project, track.id);
                                self.status_msg = format!("Reproduciendo: {}", track.title);
                            }
                        }

                        ui.add_space(8.0);
                        ui.label(egui::RichText::new("💥 EFECTOS DE SONIDO (SFX):").strong().color(Color32::from_rgb(255, 200, 140)));
                        for track in DEFAULT_TRACKS.iter().filter(|t| !t.is_music) {
                            let is_sel = self.selected_track == track.id;
                            let has_rep = project.audio_replacements.contains_key(&track.id);
                            let label = if has_rep {
                                format!("🔁 {} [REEMPLAZADO]", track.title)
                            } else {
                                track.title.to_string()
                            };

                            let resp = ui.selectable_label(is_sel, label);
                            if resp.clicked() {
                                self.selected_track = track.id;
                                jukebox.trigger_game_sound_id(project, track.id);
                                self.status_msg = format!("Reproduciendo: {}", track.title);
                            }
                        }
                    });

                ui.separator();
                ui.label(
                    egui::RichText::new(format!(
                        "🔁 Reemplazar la Pista Seleccionada (#{0:02X}) por música .MP3 o sonido .WAV:",
                        self.selected_track
                    ))
                    .strong(),
                );
                ui.horizontal(|ui| {
                    if ui.button("📂 Vincular archivo .MP3 / .WAV").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Audio MP3 o WAV", &["mp3", "wav"])
                            .pick_file()
                        {
                            let is_wav = path
                                .extension()
                                .and_then(|e| e.to_str())
                                .map(|s| s.eq_ignore_ascii_case("wav"))
                                .unwrap_or(false);

                            let track_name = DEFAULT_TRACKS
                                .iter()
                                .find(|t| t.id == self.selected_track)
                                .map(|t| t.title)
                                .unwrap_or("Pista personalizada");

                            project.audio_replacements.insert(
                                self.selected_track,
                                AudioReplacement {
                                    track_id: self.selected_track,
                                    label: track_name.to_string(),
                                    external_file_path: path.display().to_string(),
                                    is_sfx_wav: is_wav,
                                },
                            );
                            self.status_msg = format!(
                                "Pista #{:02X} vinculada a '{}'",
                                self.selected_track,
                                path.display()
                            );
                        }
                    }
                    if project.audio_replacements.contains_key(&self.selected_track) {
                        if ui.button("🗑 Quitar reemplazo").clicked() {
                            project.audio_replacements.remove(&self.selected_track);
                            self.status_msg = "Reemplazo eliminado.".to_string();
                        }
                    }
                });

                if let Some(rep) = project.audio_replacements.get(&self.selected_track) {
                    ui.colored_label(
                        Color32::from_rgb(80, 230, 150),
                        format!("✔ Archivo vinculado: {}", rep.external_file_path),
                    );
                }
            });

            // COLUMNA DERECHA: Mezclador de 5 Canales APU (Mute / Solo en Vivo) y Osciloscopio
            cols[1].group(|ui| {
                ui.heading("🎛️ Mezclador de Canales de Sonido (Solo / Mute)");
                ui.label("Puedes apagar o aislar canales para escuchar cómo está compuesta la música:");
                ui.add_space(4.0);

                let ch_names = [
                    "Canal 1 — Pulse 1 (Melodía Principal)",
                    "Canal 2 — Pulse 2 (Armonía / Segundo Pulso)",
                    "Canal 3 — Triangle (Bajo Musical)",
                    "Canal 4 — Noise (Batería y Explosiones)",
                    "Canal 5 — DMC / DPCM (Percusión de Muestras)",
                ];

                for (i, name) in ch_names.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut jukebox.mute[i], "Mute");
                        ui.checkbox(&mut jukebox.solo[i], "Solo");
                        ui.label(*name);
                    });
                }

                ui.add_space(8.0);
                ui.label(egui::RichText::new("Osciloscopio en Tiempo Real:").strong());
                let (o_rect, _) = ui.allocate_exact_size(Vec2::new(380.0, 110.0), Sense::hover());
                let painter = ui.painter_at(o_rect);
                painter.rect_filled(o_rect, 3.0, Color32::from_rgb(10, 14, 20));

                let n = jukebox.oscilloscope.len().max(2);
                let mut pts = Vec::with_capacity(n);
                for (idx, &val) in jukebox.oscilloscope.iter().enumerate() {
                    let x = o_rect.min.x + (idx as f32 / (n - 1) as f32) * o_rect.width();
                    let y = o_rect.center().y - (val * 2.2).clamp(-1.0, 1.0) * (o_rect.height() * 0.44);
                    pts.push(Pos2::new(x, y));
                }
                painter.add(egui::Shape::line(pts, Stroke::new(2.0, Color32::from_rgb(0, 230, 180))));

                ui.add_space(6.0);
                if !self.status_msg.is_empty() {
                    ui.colored_label(Color32::from_rgb(130, 230, 255), &self.status_msg);
                }
            });
        });
    }
}
