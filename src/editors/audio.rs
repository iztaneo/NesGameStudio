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
            selected_track: 0,
            status_msg: String::new(),
        }
    }
}

impl AudioEditorState {
    pub fn ui(&mut self, ui: &mut egui::Ui, project: &mut NgsProject, jukebox: &JukeboxEngine) {
        ui.heading("🎵 Rockola Aislada (Sound Test APU de 5 Canales) e Inyector .MP3 / .WAV");
        ui.label("Reproduce aisladamente cada canción y efecto del juego, controla canales individuales y reemplaza pistas por .MP3 / .WAV.");
        ui.separator();

        // Sincronizar semilla de PRG_ROM con la rockola
        if let Ok(mut st) = jukebox.state.lock() {
            if st.rom_seed.is_empty() && !project.prg_rom.is_empty() {
                st.rom_seed = project.prg_rom.iter().take(4096).copied().collect();
            }
        }

        ui.columns(2, |cols| {
            // COLUMNA IZQUIERDA: Lista de Canciones y Efectos + Controles de Reproducción
            cols[0].group(|ui| {
                ui.heading("📀 Selector de Pistas y Efectos Aislados");
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    if let Ok(mut st) = jukebox.state.lock() {
                        if ui.button(if st.playing { "⏸ Pausar" } else { "▶ Reproducir Pista" }).clicked() {
                            st.playing = !st.playing;
                            st.current_track = self.selected_track;
                        }
                        if ui.button("⏹ Detener").clicked() {
                            st.playing = false;
                            st.step_tick = 0;
                        }
                    }
                    if ui.button("💾 Exportar esta Pista a .WAV").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .set_file_name(&format!("{}_track_{:02X}.wav", project.game_title, self.selected_track))
                            .add_filter("WAV Audio", &["wav"])
                            .save_file()
                        {
                            match jukebox.export_track_to_wav(self.selected_track, &path) {
                                Ok(_) => self.status_msg = format!("Pista #{:02X} exportada a {}", self.selected_track, path.display()),
                                Err(e) => self.status_msg = format!("Error exportando WAV: {}", e),
                            }
                        }
                    }
                });

                ui.add_space(6.0);
                egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                    for track_id in 0..24u8 {
                        let default_label = match track_id {
                            0x00 => "Track #00 — Tema Principal / Nivel 1 (BGM)",
                            0x01 => "Track #01 — Base / Nivel 2 (BGM)",
                            0x02 => "Track #02 — Cascada / Nivel Vertical (BGM)",
                            0x03 => "Track #03 — Fortaleza / Jefe de Nivel (BGM)",
                            0x04 => "Track #04 — Batalla Final / Boss Rush (BGM)",
                            0x05 => "Track #05 — Victoria / Stage Clear (Fanfarria)",
                            0x06..=0x0F => "Pista Musical Secuenciada (BGM)",
                            0x10 => "SFX #10 — Disparo Rápido / Spread Gun",
                            0x11 => "SFX #11 — Explosión Pesada (Noise + DMC)",
                            0x12 => "SFX #12 — Salto / Sweep Ascendente",
                            _ => "SFX — Moneda / Ítem / Power-Up",
                        };

                        let has_rep = project.audio_replacements.contains_key(&track_id);
                        let is_sel = self.selected_track == track_id;
                        let text = if has_rep {
                            format!("🔁 [MP3/WAV] #{:02X} — {}", track_id, default_label)
                        } else {
                            format!("🎵 #{:02X} — {}", track_id, default_label)
                        };

                        if ui.selectable_label(is_sel, text).clicked() {
                            self.selected_track = track_id;
                            if let Ok(mut st) = jukebox.state.lock() {
                                st.current_track = track_id;
                                st.step_tick = 0;
                                st.playing = true;
                            }
                        }
                    }
                });

                ui.separator();
                ui.heading("🔁 Reemplazar Pista Seleccionada por .MP3 o .WAV HD");
                ui.horizontal(|ui| {
                    if ui.button("📂 Elegir archivo .MP3 / .WAV para esta pista").clicked() {
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
                                "Pista #{:02X} vinculada a '{}'",
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

            // COLUMNA DERECHA: Mezclador de 5 Canales (Solo / Mute), Osciloscopio y Rutinas [AUDIO / APU]
            cols[1].group(|ui| {
                ui.heading("🎛️ Control por Canal APU (Solo / Mute)");
                let ch_names = [
                    "1. Pulse 1 (Melodía Principal / Sweep)",
                    "2. Pulse 2 (Armonía / Contrapunto)",
                    "3. Triangle (Bajo profundo de 16 pasos)",
                    "4. Noise (Percusión / Explosiones LFSR)",
                    "5. DMC / DPCM (Muestras percusivas 1-bit)",
                ];

                if let Ok(mut st) = jukebox.state.lock() {
                    for (i, name) in ch_names.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut st.mute[i], "Mute");
                            ui.checkbox(&mut st.solo[i], "Solo");
                            ui.label(*name);
                        });
                    }

                    ui.add_space(8.0);
                    ui.label("Osciloscopio de Salida en Tiempo Real (44.1 kHz):");
                    let (o_rect, _) = ui.allocate_exact_size(Vec2::new(340.0, 90.0), Sense::hover());
                    let painter = ui.painter_at(o_rect);
                    painter.rect_filled(o_rect, 3.0, Color32::from_rgb(12, 16, 22));

                    let n = st.oscilloscope.len().max(2);
                    let mut pts = Vec::with_capacity(n);
                    for (idx, &val) in st.oscilloscope.iter().enumerate() {
                        let x = o_rect.min.x + (idx as f32 / (n - 1) as f32) * o_rect.width();
                        let y = o_rect.center().y - val * (o_rect.height() * 0.42);
                        pts.push(Pos2::new(x, y));
                    }
                    painter.add(egui::Shape::line(pts, Stroke::new(1.8, Color32::from_rgb(0, 230, 180))));
                }

                ui.separator();
                let audio_subs: Vec<_> = project
                    .subroutines
                    .iter()
                    .filter(|s| s.category == "[AUDIO / APU]")
                    .collect();
                ui.heading(format!("🔍 Subrutinas [AUDIO / APU] Detectadas en el Juego ({})", audio_subs.len()));
                egui::ScrollArea::vertical().max_height(130.0).show(ui, |ui| {
                    if audio_subs.is_empty() {
                        ui.label("Abre un archivo .c/.asm exportado por NesEmuRust para listar sus rutinas APU.");
                    } else {
                        for sub in audio_subs {
                            ui.collapsing(format!("{} ($ {:04X}) — Línea {}", sub.name, sub.addr, sub.line_in_asm), |ui| {
                                for l in &sub.preview_asm {
                                    ui.monospace(l);
                                }
                            });
                        }
                    }
                });

                if !self.status_msg.is_empty() {
                    ui.colored_label(Color32::LIGHT_GREEN, &self.status_msg);
                }
            });
        });
    }
}
