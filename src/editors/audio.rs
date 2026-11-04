use crate::project::{AudioReplacement, NgsProject};
use crate::runtime::jukebox_apu::JukeboxEngine;
use eframe::egui::{self, Color32, Pos2, Sense, Stroke, Vec2};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct TrackEntry {
    pub id: u8,
    pub title: String,
    pub is_music: bool,
}

pub fn get_tracks_for_game(game_title: &str) -> Vec<TrackEntry> {
    let lower = game_title.to_lowercase();
    if lower.contains("contra") {
        vec![
            TrackEntry { id: 0x01, title: "Pista 01 — Pantalla de Título & Menú".into(), is_music: true },
            TrackEntry { id: 0x02, title: "Pista 02 — Nivel 1: Selva / Jungla".into(), is_music: true },
            TrackEntry { id: 0x03, title: "Pista 03 — Nivel 2: Base 3D Militar".into(), is_music: true },
            TrackEntry { id: 0x04, title: "Pista 04 — Nivel 3: Cascada".into(), is_music: true },
            TrackEntry { id: 0x05, title: "Pista 05 — Nivel 5: Campo de Nieve".into(), is_music: true },
            TrackEntry { id: 0x06, title: "Pista 06 — Nivel 6: Zona Energética".into(), is_music: true },
            TrackEntry { id: 0x07, title: "Pista 07 — Nivel 7: Hangar".into(), is_music: true },
            TrackEntry { id: 0x08, title: "Pista 08 — Nivel 8: Guarida Alienígena".into(), is_music: true },
            TrackEntry { id: 0x09, title: "Pista 09 — Batalla de Jefe (Boss Fight)".into(), is_music: true },
            TrackEntry { id: 0x0A, title: "Pista 0A — Nivel Completado (Fanfarria)".into(), is_music: true },
            TrackEntry { id: 0x0B, title: "Pista 0B — Final y Créditos".into(), is_music: true },
            TrackEntry { id: 0x0C, title: "Pista 0C — Game Over".into(), is_music: true },
            TrackEntry { id: 0x10, title: "SFX 10 — Disparo de Rifle Normal".into(), is_music: false },
            TrackEntry { id: 0x11, title: "SFX 11 — Disparo Spread Gun (Escopeta)".into(), is_music: false },
            TrackEntry { id: 0x12, title: "SFX 12 — Disparo de Rayo Láser".into(), is_music: false },
            TrackEntry { id: 0x13, title: "SFX 13 — Disparo de Fuego / Lanzallamas".into(), is_music: false },
            TrackEntry { id: 0x14, title: "SFX 14 — Salto del Soldado".into(), is_music: false },
            TrackEntry { id: 0x15, title: "SFX 15 — Explosión Pequeña".into(), is_music: false },
            TrackEntry { id: 0x16, title: "SFX 16 — Explosión Grande / Jefe".into(), is_music: false },
            TrackEntry { id: 0x17, title: "SFX 17 — Daño / Caída de Soldado".into(), is_music: false },
            TrackEntry { id: 0x18, title: "SFX 18 — Halcón de Arma en Vuelo".into(), is_music: false },
            TrackEntry { id: 0x19, title: "SFX 19 — Alarma / Sirena de la Base".into(), is_music: false },
        ]
    } else if lower.contains("mario") {
        vec![
            TrackEntry { id: 0x01, title: "Pista 01 — Superficie / Overworld".into(), is_music: true },
            TrackEntry { id: 0x02, title: "Pista 02 — Subterráneo / Underground".into(), is_music: true },
            TrackEntry { id: 0x03, title: "Pista 03 — Acuático / Underwater".into(), is_music: true },
            TrackEntry { id: 0x04, title: "Pista 04 — Castillo / Castle".into(), is_music: true },
            TrackEntry { id: 0x05, title: "Pista 05 — Estrella / Starman".into(), is_music: true },
            TrackEntry { id: 0x06, title: "Pista 06 — Nivel Completado (Fanfarria)".into(), is_music: true },
            TrackEntry { id: 0x07, title: "Pista 07 — Muerte / Caída".into(), is_music: true },
            TrackEntry { id: 0x08, title: "Pista 08 — Game Over".into(), is_music: true },
            TrackEntry { id: 0x10, title: "SFX 10 — Salto Pequeño".into(), is_music: false },
            TrackEntry { id: 0x11, title: "SFX 11 — Salto Super Mario".into(), is_music: false },
            TrackEntry { id: 0x12, title: "SFX 12 — Moneda".into(), is_music: false },
            TrackEntry { id: 0x13, title: "SFX 13 — Champiñón / Power Up".into(), is_music: false },
            TrackEntry { id: 0x14, title: "SFX 14 — Bola de Fuego".into(), is_music: false },
            TrackEntry { id: 0x15, title: "SFX 15 — Pisotón a Enemigo".into(), is_music: false },
            TrackEntry { id: 0x16, title: "SFX 16 — Caída por Tubería".into(), is_music: false },
        ]
    } else if lower.contains("castlevania") {
        vec![
            TrackEntry { id: 0x01, title: "Pista 01 — Vampire Killer (Nivel 1)".into(), is_music: true },
            TrackEntry { id: 0x02, title: "Pista 02 — Stalker (Nivel 2)".into(), is_music: true },
            TrackEntry { id: 0x03, title: "Pista 03 — Wicked Child (Nivel 3)".into(), is_music: true },
            TrackEntry { id: 0x04, title: "Pista 04 — Walking on the Edge (Nivel 4)".into(), is_music: true },
            TrackEntry { id: 0x05, title: "Pista 05 — Heart of Fire (Nivel 5)".into(), is_music: true },
            TrackEntry { id: 0x06, title: "Pista 06 — Out of Time (Nivel 6)".into(), is_music: true },
            TrackEntry { id: 0x07, title: "Pista 07 — Poison Mind (Jefe)".into(), is_music: true },
            TrackEntry { id: 0x08, title: "Pista 08 — Black Night (Final)".into(), is_music: true },
            TrackEntry { id: 0x10, title: "SFX 10 — Latigazo".into(), is_music: false },
            TrackEntry { id: 0x11, title: "SFX 11 — Salto".into(), is_music: false },
            TrackEntry { id: 0x12, title: "SFX 12 — Recoger Corazón".into(), is_music: false },
            TrackEntry { id: 0x13, title: "SFX 13 — Golpe / Daño".into(), is_music: false },
        ]
    } else {
        // Genérico para CUALQUIER otro juego de NES
        let mut list = Vec::new();
        for id in 1..=14 {
            list.push(TrackEntry {
                id,
                title: format!("Pista {:02X} — Tema / Música #{:02X}", id, id),
                is_music: true,
            });
        }
        for id in 0x10..=0x26 {
            list.push(TrackEntry {
                id,
                title: format!("SFX {:02X} — Efecto de Sonido #{:02X}", id, id),
                is_music: false,
            });
        }
        list
    }
}

pub struct AudioEditorState {
    pub selected_track: u8,
    pub custom_test_id: u8,
    pub rename_buf: String,
    pub is_renaming: bool,
    pub user_track_names: HashMap<u8, String>,
    pub status_msg: String,
}

impl Default for AudioEditorState {
    fn default() -> Self {
        Self {
            selected_track: 1,
            custom_test_id: 1,
            rename_buf: String::new(),
            is_renaming: false,
            user_track_names: HashMap::new(),
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
             Puedes aislar instrumentos con Mute/Solo, probar IDs de sonido personalizados o reemplazar la música por tu propio archivo .MP3 / .WAV.",
        );

        let tracks = get_tracks_for_game(&project.game_title);

        ui.columns(2, |cols| {
            // COLUMNA IZQUIERDA: Lista Completa de Pistas del Juego (BGM y SFX)
            cols[0].group(|ui| {
                ui.horizontal(|ui| {
                    ui.heading("🎵 Lista de Pistas del Juego");
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

                ui.add_space(4.0);
                // Control para probar cualquier ID de sonido directo (0x00 .. 0xFF)
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Probar ID libre:").small());
                    ui.add(egui::DragValue::new(&mut self.custom_test_id).range(0..=255).hexadecimal(2, false, true));
                    if ui.button("▶ Probar ID").clicked() {
                        self.selected_track = self.custom_test_id;
                        jukebox.trigger_game_sound_id(project, self.custom_test_id);
                        self.status_msg = format!("Reproduciendo comando ID #{:02X}", self.custom_test_id);
                    }
                });

                ui.add_space(4.0);
                egui::ScrollArea::vertical()
                    .max_height(340.0)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("🎶 TEMAS MUSICALES (BGM):").strong().color(Color32::from_rgb(180, 220, 255)));
                        for track in tracks.iter().filter(|t| t.is_music) {
                            let is_sel = self.selected_track == track.id;
                            let has_rep = project.audio_replacements.contains_key(&track.id);
                            let title = self.user_track_names.get(&track.id).unwrap_or(&track.title);

                            let label = if has_rep {
                                format!("🔁 {} [REEMPLAZADA POR MP3/WAV]", title)
                            } else {
                                title.clone()
                            };

                            let resp = ui.selectable_label(is_sel, label);
                            if resp.clicked() {
                                self.selected_track = track.id;
                                jukebox.trigger_game_sound_id(project, track.id);
                                self.status_msg = format!("Reproduciendo: {}", title);
                            }
                        }

                        ui.add_space(8.0);
                        ui.label(egui::RichText::new("💥 EFECTOS DE SONIDO (SFX):").strong().color(Color32::from_rgb(255, 200, 140)));
                        for track in tracks.iter().filter(|t| !t.is_music) {
                            let is_sel = self.selected_track == track.id;
                            let has_rep = project.audio_replacements.contains_key(&track.id);
                            let title = self.user_track_names.get(&track.id).unwrap_or(&track.title);

                            let label = if has_rep {
                                format!("🔁 {} [REEMPLAZADO]", title)
                            } else {
                                title.clone()
                            };

                            let resp = ui.selectable_label(is_sel, label);
                            if resp.clicked() {
                                self.selected_track = track.id;
                                jukebox.trigger_game_sound_id(project, track.id);
                                self.status_msg = format!("Reproduciendo: {}", title);
                            }
                        }
                    });

                ui.separator();
                // Renombrar la pista seleccionada
                let current_title = self.user_track_names.get(&self.selected_track)
                    .cloned()
                    .unwrap_or_else(|| {
                        tracks.iter()
                            .find(|t| t.id == self.selected_track)
                            .map(|t| t.title.clone())
                            .unwrap_or_else(|| format!("Pista #{:02X}", self.selected_track))
                    });

                ui.horizontal(|ui| {
                    if self.is_renaming {
                        ui.text_edit_singleline(&mut self.rename_buf);
                        if ui.button("✔ Guardar Nombre").clicked() {
                            self.user_track_names.insert(self.selected_track, self.rename_buf.clone());
                            self.is_renaming = false;
                        }
                        if ui.button("✖ Cancelar").clicked() {
                            self.is_renaming = false;
                        }
                    } else {
                        if ui.button("✏️ Renombrar esta Pista").clicked() {
                            self.is_renaming = true;
                            self.rename_buf = current_title.clone();
                        }
                    }
                });

                ui.add_space(4.0);
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

                            let track_name = self.user_track_names.get(&self.selected_track)
                                .cloned()
                                .unwrap_or_else(|| {
                                    tracks.iter()
                                        .find(|t| t.id == self.selected_track)
                                        .map(|t| t.title.clone())
                                        .unwrap_or_else(|| format!("Pista #{:02X}", self.selected_track))
                                });

                            project.audio_replacements.insert(
                                self.selected_track,
                                AudioReplacement {
                                    track_id: self.selected_track,
                                    label: track_name,
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
