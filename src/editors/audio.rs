use crate::project::{AudioEventKey, AudioEventKind, AudioReplacement, NgsProject};
use crate::runtime::jukebox_apu::JukeboxEngine;
use eframe::egui::{self, Color32, Pos2, Sense, Stroke, Vec2};
use nes_emu_rust::controller::JoypadButtons;
use std::collections::HashMap;

pub struct AudioEditorState {
    pub selected_event_key: Option<AudioEventKey>,
    pub selected_track: u8,
    pub custom_test_id: u8,
    pub custom_trigger_idx: usize,
    pub rename_buf: String,
    pub is_renaming: bool,
    pub user_track_names: HashMap<AudioEventKey, String>,
    pub status_msg: String,
}

impl Default for AudioEditorState {
    fn default() -> Self {
        Self {
            selected_event_key: None,
            selected_track: 1,
            custom_test_id: 1,
            custom_trigger_idx: 0,
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
            "Selecciona cualquier pista o efecto de la lista para escucharlo de forma aislada. Cada evento se identifica por su \
             Trigger + Comando + Contexto. Puedes reclasificar eventos, aislar canales con Mute/Solo o vincular archivos .MP3/.WAV.",
        );

        if project.discovered_tracks.is_empty() {
            crate::runtime::jukebox_apu::detect_or_discover_audio_engine(project);
        }
        let valid_sel = self
            .selected_event_key
            .as_ref()
            .map(|k| project.discovered_tracks.iter().any(|t| &t.key == k))
            .unwrap_or(false);
        if !valid_sel {
            if let Some(first) = project.discovered_tracks.first() {
                self.selected_event_key = Some(first.key.clone());
                self.selected_track = first.id;
            }
        }
        let tracks = project.discovered_tracks.clone();
        let active_key = self
            .selected_event_key
            .clone()
            .unwrap_or_else(|| AudioEventKey::new(project.sound_trigger.clone(), self.selected_track));

        ui.columns(2, |cols| {
            // =========================================================================
            // COLUMNA IZQUIERDA: Catálogo de Eventos de Audio (Música, SFX, Fanfarrias, Control, Sin clasificar)
            // =========================================================================
            cols[0].group(|ui| {
                ui.horizontal(|ui| {
                    ui.heading("🎵 Catálogo de Audio (Multitrigger)");
                    ui.separator();
                    if ui
                        .button(if jukebox.playing { "⏸ Pausar" } else { "▶ Reproducir" })
                        .clicked()
                    {
                        jukebox.playing = !jukebox.playing;
                        if jukebox.playing {
                            jukebox.trigger_audio_event(project, &active_key);
                            self.status_msg = format!(
                                "Reproduciendo {} ({})...",
                                active_key.short_label(),
                                if jukebox.last_playback_confirmed_audible {
                                    "Audio APU confirmado"
                                } else {
                                    "Sin actividad inicial"
                                }
                            );
                        } else {
                            jukebox.stop_external_file();
                        }
                    }
                    if ui.button("⏹ Detener").clicked() {
                        jukebox.playing = false;
                        jukebox.stop_external_file();
                        self.status_msg = "Reproducción detenida.".to_string();
                    }
                    if ui.button("💾 Grabar a .WAV").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .set_file_name(&format!(
                                "{}_evento_{:02X}.wav",
                                project.game_title, active_key.command_id
                            ))
                            .add_filter("Audio WAV", &["wav"])
                            .save_file()
                        {
                            match jukebox.export_real_nes_audio_to_wav(project, &path) {
                                Ok(_) => self.status_msg = format!("Evento grabado en {}", path.display()),
                                Err(e) => self.status_msg = format!("Error al exportar: {}", e),
                            }
                        }
                    }
                });

                ui.add_space(4.0);
                // Control para probar cualquier ID de sonido directo sobre cualquiera de los triggers registrados
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Probar comando:").small());
                    if !project.audio_triggers.is_empty() {
                        if self.custom_trigger_idx >= project.audio_triggers.len() {
                            self.custom_trigger_idx = 0;
                        }
                        egui::ComboBox::from_id_salt("trigger_selector_combo")
                            .selected_text(project.audio_triggers[self.custom_trigger_idx].short_label())
                            .show_ui(ui, |ui| {
                                for (idx, trig) in project.audio_triggers.iter().enumerate() {
                                    ui.selectable_value(
                                        &mut self.custom_trigger_idx,
                                        idx,
                                        trig.short_label(),
                                    );
                                }
                            });
                    }
                    ui.add(
                        egui::DragValue::new(&mut self.custom_test_id)
                            .range(0..=255)
                            .hexadecimal(2, false, true),
                    );
                    if ui.button("▶ Probar").clicked() {
                        let trig = project
                            .audio_triggers
                            .get(self.custom_trigger_idx)
                            .cloned()
                            .unwrap_or_else(|| project.sound_trigger.clone());
                        let test_key = AudioEventKey::new(trig, self.custom_test_id);
                        self.selected_event_key = Some(test_key.clone());
                        self.selected_track = self.custom_test_id;
                        jukebox.trigger_audio_event(project, &test_key);
                        self.status_msg = format!(
                            "Probando {} -> {}",
                            test_key.short_label(),
                            if jukebox.last_playback_confirmed_audible {
                                "Audio activo"
                            } else {
                                "Silencioso / Control"
                            }
                        );
                    }
                });

                ui.add_space(4.0);
                let sections = [
                    (
                        AudioEventKind::Music,
                        "🎶 MÚSICA (BGM):",
                        Color32::from_rgb(180, 220, 255),
                    ),
                    (
                        AudioEventKind::Sfx,
                        "💥 EFECTOS DE SONIDO (SFX):",
                        Color32::from_rgb(255, 200, 140),
                    ),
                    (
                        AudioEventKind::Fanfare,
                        "🎺 FANFARRIAS / CORTINILLAS:",
                        Color32::from_rgb(255, 230, 130),
                    ),
                    (
                        AudioEventKind::Control,
                        "🎛 COMANDOS DE CONTROL:",
                        Color32::from_rgb(190, 190, 210),
                    ),
                    (
                        AudioEventKind::Unknown,
                        "❓ SIN CLASIFICAR:",
                        Color32::from_rgb(180, 180, 180),
                    ),
                ];

                egui::ScrollArea::vertical()
                    .max_height(315.0)
                    .show(ui, |ui| {
                        for (section_kind, heading, color) in &sections {
                            let matching: Vec<_> = tracks
                                .iter()
                                .filter(|t| &t.kind == section_kind)
                                .collect();
                            if matching.is_empty() {
                                continue;
                            }
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new(format!("{} ({})", heading, matching.len()))
                                    .strong()
                                    .color(*color),
                            );
                            for track in matching {
                                let is_sel = self.selected_event_key.as_ref() == Some(&track.key);
                                let has_rep = project.get_event_replacement(&track.key).is_some();
                                let has_state = project.get_event_state(&track.key).is_some();
                                let title = self
                                    .user_track_names
                                    .get(&track.key)
                                    .unwrap_or(&track.title);

                                let mut meta_tags = Vec::new();
                                meta_tags.push(track.key.trigger.short_label());
                                meta_tags.push(format!(
                                    "{} {:.0}%",
                                    track.status.label(),
                                    track.confidence * 100.0
                                ));
                                if track.approx_frames > 0 {
                                    meta_tags.push(format!("{:.1}s", track.approx_frames as f32 / 60.0));
                                }
                                let ch_badge = track.channels_badge();
                                if ch_badge != "-" {
                                    meta_tags.push(ch_badge);
                                }

                                let prefix = if has_rep {
                                    "🔁 "
                                } else if has_state {
                                    "📌 "
                                } else {
                                    ""
                                };

                                let label = format!(
                                    "{}{}  [{}]",
                                    prefix,
                                    title,
                                    meta_tags.join(" | ")
                                );

                                let resp = ui.selectable_label(is_sel, label);
                                if resp.clicked() {
                                    self.selected_event_key = Some(track.key.clone());
                                    self.selected_track = track.id;
                                    jukebox.trigger_audio_event(project, &track.key);
                                    self.status_msg = format!(
                                        "Reproduciendo: {} ({}) — {}",
                                        title,
                                        track.key.short_label(),
                                        if jukebox.last_playback_confirmed_audible {
                                            "Audio APU confirmado"
                                        } else {
                                            "Sin salida audible inicial"
                                        }
                                    );
                                }
                            }
                        }
                    });

                ui.separator();
                // Metadatos y reclasificación manual del evento seleccionado
                let current_title = self
                    .user_track_names
                    .get(&active_key)
                    .cloned()
                    .unwrap_or_else(|| {
                        tracks
                            .iter()
                            .find(|t| t.key == active_key)
                            .map(|t| t.title.clone())
                            .unwrap_or_else(|| format!("Evento {}", active_key.short_label()))
                    });

                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("Clave: {}", active_key.short_label()))
                            .small()
                            .color(Color32::from_rgb(160, 210, 255)),
                    );
                    ui.separator();
                    ui.label(egui::RichText::new("Reclasificar como:").small());
                    for target_kind in [
                        AudioEventKind::Music,
                        AudioEventKind::Sfx,
                        AudioEventKind::Fanfare,
                        AudioEventKind::Control,
                    ] {
                        if ui.small_button(target_kind.label()).clicked() {
                            if let Some(t) = project
                                .discovered_tracks
                                .iter_mut()
                                .find(|t| t.key == active_key)
                            {
                                t.set_kind(target_kind.clone());
                                t.status = crate::project::AudioValidationStatus::Confirmed;
                                self.status_msg = format!(
                                    "Evento {} reclasificado como {}.",
                                    active_key.short_label(),
                                    target_kind.label()
                                );
                            }
                        }
                    }
                });

                ui.horizontal(|ui| {
                    if self.is_renaming {
                        ui.text_edit_singleline(&mut self.rename_buf);
                        if ui.button("✔ Guardar Nombre").clicked() {
                            self.user_track_names
                                .insert(active_key.clone(), self.rename_buf.clone());
                            self.is_renaming = false;
                        }
                        if ui.button("✖ Cancelar").clicked() {
                            self.is_renaming = false;
                        }
                    } else if ui.button("✏️ Renombrar este Evento").clicked() {
                        self.is_renaming = true;
                        self.rename_buf = current_title.clone();
                    }
                });

                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(format!(
                        "🔁 Reemplazar Evento ({}) por archivo .MP3 / .WAV:",
                        active_key.short_label()
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

                            project.set_event_replacement(
                                active_key.clone(),
                                AudioReplacement {
                                    event_key: Some(active_key.clone()),
                                    track_id: active_key.command_id,
                                    label: current_title.clone(),
                                    external_file_path: path.display().to_string(),
                                    is_sfx_wav: is_wav,
                                },
                            );
                            self.status_msg = format!(
                                "Evento {} vinculado a '{}'",
                                active_key.short_label(),
                                path.display()
                            );
                        }
                    }
                    if project.get_event_replacement(&active_key).is_some() {
                        if ui.button("🗑 Quitar reemplazo").clicked() {
                            project.remove_event_replacement(&active_key);
                            jukebox.stop_external_file();
                            self.status_msg = "Reemplazo eliminado.".to_string();
                        }
                    }
                });

                if let Some(rep) = project.get_event_replacement(&active_key) {
                    ui.colored_label(
                        Color32::from_rgb(80, 230, 150),
                        format!("✔ Archivo vinculado: {}", rep.external_file_path),
                    );
                }
            });

            // =========================================================================
            // COLUMNA DERECHA: Mezclador 5 Canales APU, Escucha en Vivo y Memorizador
            // =========================================================================
            cols[1].group(|ui| {
                ui.heading("🎛️ Mezclador de Canales de Sonido (Solo / Mute)");

                // Indicador de salida física de hardware y confirmación audible
                let dev_name = jukebox
                    .audio_out
                    .as_ref()
                    .map(|a| a.active_device_name())
                    .unwrap_or_else(|| "Dispositivo de Audio".to_string());
                let rate = jukebox.sample_rate();
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("🔊 Salida:").strong());
                    ui.label(format!("{} ({} Hz)", dev_name, rate));
                    if jukebox.playing {
                        if jukebox.last_playback_confirmed_audible {
                            ui.colored_label(
                                Color32::from_rgb(80, 240, 140),
                                "● Audio APU Confirmado",
                            );
                        } else {
                            ui.colored_label(
                                Color32::from_rgb(255, 200, 90),
                                "● Sin Señal Audible",
                            );
                        }
                    } else {
                        ui.colored_label(Color32::LIGHT_GRAY, "○ Pausado");
                    }
                });
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
                let (o_rect, _) = ui.allocate_exact_size(Vec2::new(380.0, 95.0), Sense::hover());
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

                ui.separator();
                // Panel de Control en Vivo y Memorizador de Canciones
                ui.heading("🎮 Control en Vivo (Navega a cualquier fase o sonido)");
                ui.label(
                    egui::RichText::new("Usa los botones para avanzar la pantalla, jugar y llegar a la música que quieras:")
                        .small()
                        .color(Color32::LIGHT_GRAY),
                );
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    if ui.button("▶ START").clicked() {
                        project.nes_live.set_controller1(JoypadButtons::START);
                        for _ in 0..8 {
                            project.nes_live.step_frame();
                        }
                        project.nes_live.set_controller1(JoypadButtons::empty());
                        jukebox.playing = true;
                    }
                    if ui.button("SELECT").clicked() {
                        project.nes_live.set_controller1(JoypadButtons::SELECT);
                        for _ in 0..8 {
                            project.nes_live.step_frame();
                        }
                        project.nes_live.set_controller1(JoypadButtons::empty());
                        jukebox.playing = true;
                    }
                    if ui.button("A (Saltar)").clicked() {
                        project.nes_live.set_controller1(JoypadButtons::BUTTON_A);
                        for _ in 0..8 {
                            project.nes_live.step_frame();
                        }
                        project.nes_live.set_controller1(JoypadButtons::empty());
                        jukebox.playing = true;
                    }
                    if ui.button("B (Disparo)").clicked() {
                        project.nes_live.set_controller1(JoypadButtons::BUTTON_B);
                        for _ in 0..8 {
                            project.nes_live.step_frame();
                        }
                        project.nes_live.set_controller1(JoypadButtons::empty());
                        jukebox.playing = true;
                    }
                    if ui.button("🔄 Reiniciar").clicked() {
                        project.nes_live.reset();
                        jukebox.playing = true;
                        self.status_msg = "Juego reiniciado a la pantalla de título.".to_string();
                    }
                });

                ui.add_space(6.0);
                if ui
                    .button(
                        egui::RichText::new(format!(
                            "📌 Memorizar este estado exacto en el Evento ({})",
                            active_key.short_label()
                        ))
                        .strong()
                        .color(Color32::from_rgb(180, 240, 255)),
                    )
                    .clicked()
                {
                    let state = project.nes_live.to_save_state_bytes();
                    project.set_event_state(active_key.clone(), state);
                    self.status_msg = format!(
                        "¡Evento {} memorizado con el estado de sonido actual!",
                        active_key.short_label()
                    );
                }

                ui.add_space(6.0);
                if !self.status_msg.is_empty() {
                    ui.colored_label(Color32::from_rgb(130, 230, 255), &self.status_msg);
                }
            });
        });
    }
}
