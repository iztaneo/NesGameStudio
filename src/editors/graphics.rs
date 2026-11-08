use crate::graphics::animation::build_entity_animation_sequence;
use crate::graphics::metasprite::{reconstruct_scene_metasprites, MetaspriteEntity};
use crate::graphics::replacement::{
    bake_rgba_into_entity_nes_tiles, encode_uncompressed_png_rgba, load_rgba_image_from_path,
    register_extended_hd_asset, DecodedRgbaImage,
};
use crate::graphics::{GraphicId, GraphicSource};
use crate::project::{nes_color_rgb_u8, NgsProject};
use eframe::egui::{self, Color32, ColorImage, Pos2, Rect, RichText, Sense, Stroke, TextureHandle, Vec2};
use nes_emu_rust::controller::JoypadButtons;
use std::fs;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum InspectorTab {
    PaintCanvas,
    Properties,
    ReplaceHd,
}

#[derive(Clone, Debug)]
struct UndoEntry {
    tile_id: GraphicId,
    previous_bytes: [u8; 16],
    new_bytes: [u8; 16],
}

pub struct GraphicsEditorState {
    pub selected_entity_idx: usize,
    pub selected_global_tile_idx: Option<usize>,
    pub editing_single_tile_mode: bool,
    pub active_color_idx: u8,
    pub inspector_tab: InspectorTab,
    pub search_query: String,
    pub live_running: bool,
    pub show_oam_boxes: bool,
    pub show_hd_overlays: bool,
    pub show_canvas_grid: bool,
    pub scene_zoom: f32,
    pub anim_playing: bool,
    pub anim_fps: u8,
    pub anim_loop: bool,
    pub anim_current_frame: usize,
    pub anim_frame_accum: f32,
    pub new_creation_name: String,
    pub new_creation_size: u32,
    pub status_feedback: String,
    undo_stack: Vec<UndoEntry>,
    redo_stack: Vec<UndoEntry>,
    scene_texture: Option<TextureHandle>,
    texture_dirty: bool,
}

impl Default for GraphicsEditorState {
    fn default() -> Self {
        Self {
            selected_entity_idx: 0,
            selected_global_tile_idx: None,
            editing_single_tile_mode: false,
            active_color_idx: 1,
            inspector_tab: InspectorTab::PaintCanvas,
            search_query: String::new(),
            live_running: false,
            show_oam_boxes: true,
            show_hd_overlays: true,
            show_canvas_grid: true,
            scene_zoom: 1.75,
            anim_playing: false,
            anim_fps: 6,
            anim_loop: true,
            anim_current_frame: 0,
            anim_frame_accum: 0.0,
            new_creation_name: "Nuevo_Personaje_HD".to_string(),
            new_creation_size: 16,
            status_feedback: String::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            scene_texture: None,
            texture_dirty: true,
        }
    }
}

impl GraphicsEditorState {
    pub fn invalidate_texture(&mut self) {
        self.texture_dirty = true;
    }

    pub fn ui(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, project: &mut NgsProject) {
        if !project.loaded {
            super::guide::welcome_ui(ui);
            return;
        }

        // Si el modo "Correr en Vivo" está activo, avanzar el emulador 1 frame y sincronizar firmas CHR-RAM
        if self.live_running {
            project.nes_live.step_frame();
            project.sync_live_chr_ram_signatures();
            project.nes_live.bus.apu.audio_samples.clear();
            self.sync_active_scene_from_live(project);
            self.texture_dirty = true;
            ctx.request_repaint();
        }

        // Asegurar que exista al menos una escena activa
        if project.scenes.is_empty() {
            project.capture_current_live_scene("Escena Inicial (VRAM Activa)".to_string());
        }
        let active_sc_idx = project
            .active_scene_idx
            .min(project.scenes.len().saturating_sub(1));
        project.active_scene_idx = active_sc_idx;

        // Reconstruir los personajes y objetos activos desde la OAM de la escena activa (Fase G3)
        let entities = reconstruct_scene_metasprites(
            &project.scenes[active_sc_idx].oam,
            project.scenes[active_sc_idx].ppu_ctrl_bits,
            &project.entity_custom_names,
        );

        if !entities.is_empty() && self.selected_entity_idx >= entities.len() {
            self.selected_entity_idx = 0;
        }

        // =========================================================================
        // PANEL 1 (IZQUIERDO): EXPLORADOR DE RECURSOS EN ÁRBOL (Limpio y Amigable)
        // =========================================================================
        egui::SidePanel::left("ngs_graphics_left_tree")
            .resizable(true)
            .default_width(275.0)
            .min_width(230.0)
            .show_inside(ui, |ui| {
                self.render_left_explorer_panel(ui, project, &entities, active_sc_idx);
            });

        // =========================================================================
        // PANEL 3 (DERECHO): INSPECTOR Y LIENZO UNIFICADO CON 3 PESTAÑAS
        // =========================================================================
        egui::SidePanel::right("ngs_graphics_right_inspector")
            .resizable(true)
            .default_width(365.0)
            .min_width(320.0)
            .show_inside(ui, |ui| {
                self.render_right_inspector_panel(ui, project, &entities, active_sc_idx);
            });

        // =========================================================================
        // PANEL 4 (CENTRAL INFERIOR): TIMELINE DE ANIMACIÓN Y FOTOGRAMAS
        // =========================================================================
        egui::TopBottomPanel::bottom("ngs_graphics_bottom_timeline")
            .resizable(true)
            .default_height(145.0)
            .min_height(120.0)
            .show_inside(ui, |ui| {
                self.render_bottom_animation_timeline(ctx, ui, project, &entities, active_sc_idx);
            });

        // =========================================================================
        // PANEL 2 (CENTRAL SUPERIOR): ESCENA INTERACTIVA (CLIC PARA SELECCIONAR)
        // =========================================================================
        egui::CentralPanel::default().show_inside(ui, |ui| {
            self.render_center_interactive_scene(ctx, ui, project, &entities, active_sc_idx);
        });
    }

    fn render_left_explorer_panel(
        &mut self,
        ui: &mut egui::Ui,
        project: &mut NgsProject,
        entities: &[MetaspriteEntity],
        active_sc_idx: usize,
    ) {
        ui.heading("📂 Explorador Visual");

        // Selector de Escena Activa
        ui.horizontal(|ui| {
            ui.label("📸 Escena:");
            let current_title = project
                .scenes
                .get(active_sc_idx)
                .map(|s| s.title.clone())
                .unwrap_or_else(|| "Escena 1".to_string());
            egui::ComboBox::from_id_salt("scene_selector_combo")
                .selected_text(format!("#{} {}", active_sc_idx + 1, current_title))
                .width(175.0)
                .show_ui(ui, |ui| {
                    for (idx, sc) in project.scenes.iter().enumerate() {
                        if ui
                            .selectable_label(
                                idx == active_sc_idx,
                                format!("#{} — {}", idx + 1, sc.title),
                            )
                            .clicked()
                        {
                            project.active_scene_idx = idx;
                            // Si la escena tiene save_state, cargarlo en nes_live para continuar desde ahí
                            if !sc.save_state.is_empty() {
                                let _ = project.nes_live.from_save_state_bytes(&sc.save_state);
                            }
                            self.texture_dirty = true;
                        }
                    }
                });
        });

        // Buscador rápido
        ui.horizontal(|ui| {
            ui.label("🔍");
            ui.add(
                egui::TextEdit::singleline(&mut self.search_query)
                    .hint_text("Buscar personaje o bloque..."),
            );
            if !self.search_query.is_empty() && ui.small_button("✖").clicked() {
                self.search_query.clear();
            }
        });
        ui.separator();

        let query_lower = self.search_query.trim().to_lowercase();

        egui::ScrollArea::vertical()
            .id_salt("left_tree_scroll")
            .show(ui, |ui| {
                // RAMA 1: PERSONAJES Y OBJETOS ACTIVOS EN PANTALLA (OAM)
                egui::CollapsingHeader::new(
                    RichText::new(format!(
                        "🦸 Personajes en Pantalla ({})",
                        entities.len()
                    ))
                    .strong()
                    .color(Color32::from_rgb(120, 235, 165)),
                )
                .default_open(true)
                .show(ui, |ui| {
                    if entities.is_empty() {
                        ui.weak("No hay sprites OAM activos en este fotograma. Usa [▶ Correr] o [⏭ +15 Frames] en la escena central para avanzar a la partida.");
                    }
                    for (idx, ent) in entities.iter().enumerate() {
                        if !query_lower.is_empty()
                            && !ent.name.to_lowercase().contains(&query_lower)
                            && !ent.signature_key.to_lowercase().contains(&query_lower)
                        {
                            continue;
                        }
                        let is_selected =
                            !self.editing_single_tile_mode && self.selected_entity_idx == idx;

                        ui.horizontal(|ui| {
                            // Miniatura reconstruida del personaje
                            self.draw_entity_mini_preview(ui, project, ent, active_sc_idx, 28.0);
                            let label_txt = format!(
                                "{}\nPos ({},{}) • {} tiles",
                                ent.name,
                                ent.bbox_x,
                                ent.bbox_y,
                                ent.pieces.len()
                            );
                            if ui.selectable_label(is_selected, label_txt).clicked() {
                                self.selected_entity_idx = idx;
                                self.editing_single_tile_mode = false;
                                self.anim_current_frame = 0;
                            }
                        });
                        ui.add_space(2.0);
                    }
                });

                ui.add_space(4.0);

                // RAMA 2: MIS CREACIONES Y GRÁFICOS HD (MEMORIA EXTENDIDA ILIMITADA)
                egui::CollapsingHeader::new(
                    RichText::new(format!(
                        "✨ Memoria Extendida HD ({})",
                        project.extended_assets.len()
                    ))
                    .strong()
                    .color(Color32::from_rgb(255, 210, 110)),
                )
                .default_open(true)
                .show(ui, |ui| {
                    ui.weak("Sin límite de 8 KB ni de 4 colores NES:");
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut self.new_creation_name);
                        egui::ComboBox::from_id_salt("new_creation_size_cb")
                            .selected_text(format!("{}x{}", self.new_creation_size, self.new_creation_size))
                            .width(64.0)
                            .show_ui(ui, |ui| {
                                for sz in [16u32, 24, 32, 48, 64] {
                                    ui.selectable_value(
                                        &mut self.new_creation_size,
                                        sz,
                                        format!("{}x{}", sz, sz),
                                    );
                                }
                            });
                    });
                    if ui
                        .button("➕ Crear Nuevo Gráfico en Memoria Extendida")
                        .clicked()
                    {
                        let sz = self.new_creation_size.max(8);
                        let mut pixels = vec![0u32; (sz * sz) as usize];
                        // Dibujar silueta inicial amigable en el centro
                        for y in 2..(sz - 2) {
                            for x in 2..(sz - 2) {
                                pixels[(y * sz + x) as usize] =
                                    DecodedRgbaImage::pack_rgba(70, 180, 255, 255);
                            }
                        }
                        let img = DecodedRgbaImage {
                            width: sz,
                            height: sz,
                            pixels,
                        };
                        let target_id = entities
                            .get(self.selected_entity_idx)
                            .and_then(|e| e.pieces.first())
                            .map(|p| {
                                GraphicId::new(
                                    GraphicSource::OamSprite,
                                    active_sc_idx,
                                    p.vram_tile_id,
                                )
                            });
                        let new_id = register_extended_hd_asset(
                            project,
                            self.new_creation_name.clone(),
                            target_id,
                            &img,
                            None,
                        );
                        self.status_feedback = format!(
                            "Creado activo HD #{} ({}x{} RGBA) en Memoria Extendida Ilimitada.",
                            new_id, sz, sz
                        );
                        self.texture_dirty = true;
                    }

                    for asset in &project.extended_assets {
                        ui.horizontal(|ui| {
                            ui.colored_label(
                                Color32::from_rgb(255, 220, 130),
                                format!(
                                    "🌟 #{} {} ({}x{} RGBA)",
                                    asset.asset_id, asset.name, asset.width, asset.height
                                ),
                            );
                        });
                    }
                });

                ui.add_space(4.0);

                // RAMA 3: BLOQUES DEL ESCENARIO ACTUAL (Fondo / Tiles no vacíos de la escena activa)
                egui::CollapsingHeader::new(
                    RichText::new("🏞️ Bloques del Escenario Actual")
                        .color(Color32::from_rgb(150, 205, 255)),
                )
                .default_open(false)
                .show(ui, |ui| {
                    ui.weak("Haz clic en un bloque 8x8 para retocar el suelo, muros o HUD:");
                    let scene_tiles: Vec<(usize, String, u16)> = project
                        .global_tiles
                        .iter()
                        .enumerate()
                        .filter(|(_, t)| {
                            matches!(
                                t.id.source,
                                GraphicSource::ChrRamScene
                                    | GraphicSource::OamSprite
                                    | GraphicSource::NametableBg
                            ) && t.id.bank_or_scene == active_sc_idx
                                && t.bytes.iter().any(|&b| b != 0 && b != 0xFF)
                                && (query_lower.is_empty()
                                    || t.label.to_lowercase().contains(&query_lower))
                        })
                        .take(128)
                        .map(|(g_idx, t)| (g_idx, t.label.clone(), t.id.tile_index))
                        .collect();

                    for (g_idx, label, tile_idx) in scene_tiles {
                        let sel = self.editing_single_tile_mode
                            && self.selected_global_tile_idx == Some(g_idx);
                        if ui
                            .selectable_label(sel, format!("🧱 Tile #{:03X} — {}", tile_idx, label))
                            .clicked()
                        {
                            self.selected_global_tile_idx = Some(g_idx);
                            self.editing_single_tile_mode = true;
                        }
                    }
                });

                ui.add_space(4.0);

                // RAMA 4: VISTA TÉCNICA AVANZADA (Oculta por defecto para no abrumar al usuario)
                egui::CollapsingHeader::new(
                    RichText::new("🔧 Avanzado (Bancos CHR / Candidatos PRG)").weak(),
                )
                .default_open(false)
                .show(ui, |ui| {
                    ui.label(format!(
                        "Modo de Memoria: {}",
                        if project.is_chr_ram {
                            "CHR-RAM Dinámica + Auto-Sync PRG"
                        } else {
                            "CHR-ROM por Bancos"
                        }
                    ));
                    ui.label(format!(
                        "Parches activos: {} tiles | Candidatos PRG filtrados: {}",
                        project.tile_patches.len(),
                        project.prg_candidate_tiles.len()
                    ));
                    if ui
                        .checkbox(
                            &mut project.show_advanced_prg_candidates,
                            "Mostrar candidatos heurísticos PRG",
                        )
                        .changed()
                    {
                        project.rebuild_global_tile_catalog();
                    }

                    let chr_rom_tiles: Vec<(usize, String)> = project
                        .global_tiles
                        .iter()
                        .enumerate()
                        .filter(|(_, t)| t.id.source == GraphicSource::ChrRom)
                        .take(96)
                        .map(|(i, t)| (i, t.label.clone()))
                        .collect();
                    if !chr_rom_tiles.is_empty() {
                        ui.separator();
                        ui.weak("Bancos CHR-ROM (primeros 96 tiles):");
                        for (g_idx, lbl) in chr_rom_tiles {
                            let sel = self.editing_single_tile_mode
                                && self.selected_global_tile_idx == Some(g_idx);
                            if ui.selectable_label(sel, lbl).clicked() {
                                self.selected_global_tile_idx = Some(g_idx);
                                self.editing_single_tile_mode = true;
                            }
                        }
                    }
                });
            });
    }

    fn render_center_interactive_scene(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        project: &mut NgsProject,
        entities: &[MetaspriteEntity],
        active_sc_idx: usize,
    ) {
        // Barra de control superior de la Escena Interactiva
        ui.horizontal_wrapped(|ui| {
            let run_label = if self.live_running {
                "⏸ Pausar Juego"
            } else {
                "▶ Correr en Vivo"
            };
            if ui
                .button(RichText::new(run_label).strong().color(if self.live_running {
                    Color32::from_rgb(255, 200, 100)
                } else {
                    Color32::from_rgb(110, 240, 150)
                }))
                .clicked()
            {
                self.live_running = !self.live_running;
            }

            if ui.button("⏭ +1 Frame").clicked() {
                self.live_running = false;
                project.nes_live.step_frame();
                project.sync_live_chr_ram_signatures();
                project.nes_live.bus.apu.audio_samples.clear();
                self.sync_active_scene_from_live(project);
                self.texture_dirty = true;
            }

            if ui.button("⏭ +15 Frames").clicked() {
                self.live_running = false;
                for _ in 0..15 {
                    project.nes_live.step_frame();
                    project.sync_live_chr_ram_signatures();
                    project.nes_live.bus.apu.audio_samples.clear();
                }
                self.sync_active_scene_from_live(project);
                self.texture_dirty = true;
            }

            if ui.button("🎮 Pulsar START").clicked() {
                project.nes_live.set_controller1(JoypadButtons::START);
                for _ in 0..8 {
                    project.nes_live.step_frame();
                    project.sync_live_chr_ram_signatures();
                    project.nes_live.bus.apu.audio_samples.clear();
                }
                project.nes_live.set_controller1(JoypadButtons::empty());
                for _ in 0..20 {
                    project.nes_live.step_frame();
                    project.sync_live_chr_ram_signatures();
                    project.nes_live.bus.apu.audio_samples.clear();
                }
                self.sync_active_scene_from_live(project);
                self.texture_dirty = true;
            }

            if ui.button("📸 Guardar Nueva Escena").clicked() {
                let next_n = project.scenes.len() + 1;
                project.capture_current_live_scene(format!("Escena #{}: Captura", next_n));
                project.auto_sync_workspace();
                self.texture_dirty = true;
            }

            ui.separator();
            ui.checkbox(&mut self.show_oam_boxes, "☑ Cajas de Personajes");
            ui.checkbox(&mut self.show_hd_overlays, "☑ Vista HD");
            ui.label("Zoom:");
            for z in [1.25f32, 1.75, 2.0] {
                if ui
                    .selectable_label((self.scene_zoom - z).abs() < 0.05, format!("{:.2}x", z))
                    .clicked()
                {
                    self.scene_zoom = z;
                }
            }
        });

        ui.weak("💡 Haz clic directamente sobre cualquier personaje u objeto en la pantalla del juego para seleccionarlo y pintarlo en el panel derecho.");

        if !self.status_feedback.is_empty() {
            ui.colored_label(Color32::from_rgb(125, 230, 175), &self.status_feedback);
        }

        // Actualizar textura de la escena componiendo los tiles editados y los reemplazos HD
        if self.texture_dirty || self.scene_texture.is_none() {
            let composed_rgb = self.compose_scene_preview_rgb(project, entities, active_sc_idx);
            let img = ColorImage::from_rgb([256, 240], &composed_rgb);
            self.scene_texture =
                Some(ctx.load_texture("ngs_interactive_scene", img, egui::TextureOptions::NEAREST));
            self.texture_dirty = false;
        }

        let disp_w = 256.0 * self.scene_zoom;
        let disp_h = 240.0 * self.scene_zoom;

        egui::ScrollArea::both()
            .id_salt("center_interactive_scene_scroll")
            .show(ui, |ui| {
                let (rect, response) =
                    ui.allocate_exact_size(Vec2::new(disp_w, disp_h), Sense::click());

                if let Some(tex) = &self.scene_texture {
                    ui.painter().image(
                        tex.id(),
                        rect,
                        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                        Color32::WHITE,
                    );
                }

                // Dibujar cajas OAM interactivas alrededor de cada personaje detectado
                if self.show_oam_boxes {
                    for (idx, ent) in entities.iter().enumerate() {
                        let is_sel =
                            !self.editing_single_tile_mode && idx == self.selected_entity_idx;
                        let min_p = Pos2::new(
                            rect.min.x + (ent.bbox_x as f32) * self.scene_zoom,
                            rect.min.y + (ent.bbox_y as f32) * self.scene_zoom,
                        );
                        let max_p = Pos2::new(
                            min_p.x + (ent.width as f32) * self.scene_zoom,
                            min_p.y + (ent.height as f32) * self.scene_zoom,
                        );
                        let box_rect = Rect::from_min_max(min_p, max_p);
                        let stroke_color = if is_sel {
                            Color32::from_rgb(80, 255, 120)
                        } else {
                            Color32::from_rgba_unmultiplied(90, 200, 255, 190)
                        };
                        let stroke_width = if is_sel { 2.5 } else { 1.2 };
                        ui.painter().rect_stroke(
                            box_rect,
                            2.0,
                            Stroke::new(stroke_width, stroke_color),
                        );

                        if is_sel {
                            ui.painter().text(
                                Pos2::new(min_p.x, (min_p.y - 14.0).max(rect.min.y)),
                                egui::Align2::LEFT_TOP,
                                &ent.name,
                                egui::FontId::proportional(12.0),
                                Color32::from_rgb(80, 255, 120),
                            );
                        }
                    }
                }

                // Selección por clic directo sobre el personaje en pantalla (Hit-Test OAM)
                if response.clicked() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        let nes_x =
                            (((pos.x - rect.min.x) / self.scene_zoom).clamp(0.0, 255.0)) as u16;
                        let nes_y =
                            (((pos.y - rect.min.y) / self.scene_zoom).clamp(0.0, 239.0)) as u16;

                        if let Some((hit_idx, hit_ent)) = entities
                            .iter()
                            .enumerate()
                            .find(|(_, e)| e.contains_screen_point(nes_x, nes_y))
                        {
                            self.selected_entity_idx = hit_idx;
                            self.editing_single_tile_mode = false;
                            self.anim_current_frame = 0;
                            self.status_feedback = format!(
                                "🎯 Seleccionado en escena: '{}' ({}x{} px en X={}, Y={})",
                                hit_ent.name,
                                hit_ent.width,
                                hit_ent.height,
                                hit_ent.bbox_x,
                                hit_ent.bbox_y
                            );
                        }
                    }
                }
            });
    }

    fn render_right_inspector_panel(
        &mut self,
        ui: &mut egui::Ui,
        project: &mut NgsProject,
        entities: &[MetaspriteEntity],
        active_sc_idx: usize,
    ) {
        ui.heading("🔧 Inspector y Editor");

        // Selector de las 3 Pestañas del Inspector Derecho
        ui.horizontal(|ui| {
            ui.selectable_value(
                &mut self.inspector_tab,
                InspectorTab::PaintCanvas,
                "🖌️ Pintar",
            );
            ui.selectable_value(
                &mut self.inspector_tab,
                InspectorTab::Properties,
                "📋 Propiedades",
            );
            ui.selectable_value(
                &mut self.inspector_tab,
                InspectorTab::ReplaceHd,
                "🖼️ Reemplazar HD",
            );
        });
        ui.separator();

        // Si el usuario seleccionó un bloque individual de fondo/avanzado
        if self.editing_single_tile_mode {
            if let Some(g_idx) = self.selected_global_tile_idx {
                self.render_single_tile_editor(ui, project, g_idx);
                return;
            }
        }

        let Some(entity) = entities.get(self.selected_entity_idx) else {
            ui.label("Selecciona un personaje en la Escena Interactiva o elige un bloque del escenario en el árbol izquierdo.");
            return;
        };

        match self.inspector_tab {
            InspectorTab::PaintCanvas => {
                self.render_entity_paint_canvas_tab(ui, project, entity, active_sc_idx);
            }
            InspectorTab::Properties => {
                self.render_entity_properties_tab(ui, project, entity, active_sc_idx);
            }
            InspectorTab::ReplaceHd => {
                self.render_entity_replace_hd_tab(ui, project, entity, active_sc_idx);
            }
        }
    }

    fn render_entity_paint_canvas_tab(
        &mut self,
        ui: &mut egui::Ui,
        project: &mut NgsProject,
        entity: &MetaspriteEntity,
        active_sc_idx: usize,
    ) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(&entity.name)
                    .strong()
                    .color(Color32::from_rgb(120, 240, 160)),
            );
            ui.weak(format!("({}x{} px)", entity.width, entity.height));
        });

        // Botones Deshacer / Rehacer / Restaurar Original
        ui.horizontal(|ui| {
            if ui
                .add_enabled(!self.undo_stack.is_empty(), egui::Button::new("↩ Deshacer"))
                .clicked()
            {
                if let Some(entry) = self.undo_stack.pop() {
                    let orig = project
                        .tile_patches
                        .get(&entry.tile_id)
                        .map(|p| p.original_bytes)
                        .unwrap_or(entry.previous_bytes);
                    project.apply_tile_patch_by_id(
                        entry.tile_id.clone(),
                        orig,
                        entry.previous_bytes,
                    );
                    self.redo_stack.push(entry);
                    project.auto_sync_workspace();
                    self.texture_dirty = true;
                }
            }
            if ui
                .add_enabled(!self.redo_stack.is_empty(), egui::Button::new("↪ Rehacer"))
                .clicked()
            {
                if let Some(entry) = self.redo_stack.pop() {
                    let orig = project
                        .tile_patches
                        .get(&entry.tile_id)
                        .map(|p| p.original_bytes)
                        .unwrap_or(entry.previous_bytes);
                    project.apply_tile_patch_by_id(entry.tile_id.clone(), orig, entry.new_bytes);
                    self.undo_stack.push(entry);
                    project.auto_sync_workspace();
                    self.texture_dirty = true;
                }
            }
            if ui.button("🔄 Restaurar Original").clicked() {
                for tile_vram in entity.unique_vram_tiles() {
                    let id_oam =
                        GraphicId::new(GraphicSource::OamSprite, active_sc_idx, tile_vram);
                    let id_sc =
                        GraphicId::new(GraphicSource::ChrRamScene, active_sc_idx, tile_vram);
                    project.restore_tile_original(&id_oam);
                    project.restore_tile_original(&id_sc);
                }
                project.auto_sync_workspace();
                self.texture_dirty = true;
                self.status_feedback =
                    format!("Restaurados los gráficos originales de '{}'.", entity.name);
            }
        });

        // Selector de Color de la Paleta de Sprite del Personaje
        let palette_table = project.scenes[active_sc_idx].palette_table;
        let pal_base = 16 + ((entity.primary_palette as usize) & 3) * 4;
        ui.separator();
        ui.label(format!(
            "🎨 Paleta de Sprite #{} (Elige color para pintar):",
            entity.primary_palette
        ));
        ui.horizontal(|ui| {
            for c_idx in 0..4u8 {
                let nes_col = palette_table[(pal_base + (c_idx as usize)) & 31];
                let [r, g, b] = nes_color_rgb_u8(nes_col);
                let col32 = if c_idx == 0 {
                    Color32::from_rgb(35, 35, 45)
                } else {
                    Color32::from_rgb(r, g, b)
                };
                let is_active = self.active_color_idx == c_idx;
                let (swatch_rect, resp) =
                    ui.allocate_exact_size(Vec2::new(66.0, 26.0), Sense::click());
                ui.painter().rect_filled(swatch_rect, 4.0, col32);
                ui.painter().rect_stroke(
                    swatch_rect,
                    4.0,
                    Stroke::new(
                        if is_active { 2.5 } else { 1.0 },
                        if is_active {
                            Color32::WHITE
                        } else {
                            Color32::GRAY
                        },
                    ),
                );
                let txt = if c_idx == 0 {
                    "0: Transp".to_string()
                } else {
                    format!("C{} (${:02X})", c_idx, nes_col)
                };
                ui.painter().text(
                    swatch_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    txt,
                    egui::FontId::proportional(11.0),
                    if (r as u16 + g as u16 + b as u16) > 380 && c_idx != 0 {
                        Color32::BLACK
                    } else {
                        Color32::WHITE
                    },
                );
                if resp.clicked() {
                    self.active_color_idx = c_idx;
                }
            }
        });

        ui.checkbox(
            &mut self.show_canvas_grid,
            "Mostrar cuadrícula de píxeles y división 8x8",
        );

        // LIENZO UNIFICADO DEL PERSONAJE COMPLETO (Mapeo inverso automático respetando FlipH / FlipV)
        let canvas_rgba = entity.render_rgba_canvas(
            &project.scenes[active_sc_idx].chr_8kb,
            &project.scenes[active_sc_idx].palette_table,
        );
        let w = (entity.width as usize).max(8);
        let h = (entity.height as usize).max(8);
        let max_dim = w.max(h) as f32;
        let cell_px = (256.0 / max_dim).clamp(6.0, 16.0);
        let canvas_size = Vec2::new((w as f32) * cell_px, (h as f32) * cell_px);

        let (canvas_rect, canvas_resp) =
            ui.allocate_exact_size(canvas_size, Sense::click_and_drag());

        for py in 0..h {
            for px in 0..w {
                let p_min = Pos2::new(
                    canvas_rect.min.x + (px as f32) * cell_px,
                    canvas_rect.min.y + (py as f32) * cell_px,
                );
                let p_rect = Rect::from_min_size(p_min, Vec2::splat(cell_px));
                let [r, g, b, a] = canvas_rgba[py * w + px];
                let fill = if a == 0 {
                    if (px + py) % 2 == 0 {
                        Color32::from_rgb(38, 40, 48)
                    } else {
                        Color32::from_rgb(48, 50, 60)
                    }
                } else {
                    Color32::from_rgb(r, g, b)
                };
                ui.painter().rect_filled(p_rect, 0.0, fill);
                if self.show_canvas_grid {
                    let is_tile_border = px % 8 == 0 || py % 8 == 0;
                    ui.painter().rect_stroke(
                        p_rect,
                        0.0,
                        Stroke::new(
                            if is_tile_border { 1.0 } else { 0.4 },
                            if is_tile_border {
                                Color32::from_rgba_unmultiplied(120, 210, 255, 110)
                            } else {
                                Color32::from_rgba_unmultiplied(255, 255, 255, 28)
                            },
                        ),
                    );
                }
            }
        }

        // Al hacer clic o arrastrar en el lienzo unificado, actualizar automáticamente el sub-tile y el mapa de memoria
        if canvas_resp.clicked() || canvas_resp.dragged() {
            if let Some(pos) = canvas_resp.interact_pointer_pos() {
                let cx = (((pos.x - canvas_rect.min.x) / cell_px).floor() as i32)
                    .clamp(0, (w as i32) - 1) as u16;
                let cy = (((pos.y - canvas_rect.min.y) / cell_px).floor() as i32)
                    .clamp(0, (h as i32) - 1) as u16;

                if let Some((vram_tile, row, col, _pal)) =
                    entity.map_canvas_pixel_to_vram_tile(cx, cy)
                {
                    if let Some(global_idx) = project.global_tiles.iter().position(|t| {
                        matches!(
                            t.id.source,
                            GraphicSource::OamSprite
                                | GraphicSource::ChrRamScene
                                | GraphicSource::NametableBg
                        ) && t.id.bank_or_scene == active_sc_idx
                            && t.id.tile_index == vram_tile
                    }) {
                        let prev_bytes = project.global_tiles[global_idx].bytes;
                        let tile_id = project.global_tiles[global_idx].id.clone();
                        project.set_tile_pixel(global_idx, row, col, self.active_color_idx);
                        let new_bytes = project.global_tiles[global_idx].bytes;
                        if prev_bytes != new_bytes {
                            self.undo_stack.push(UndoEntry {
                                tile_id,
                                previous_bytes: prev_bytes,
                                new_bytes,
                            });
                            self.redo_stack.clear();
                            project.auto_sync_workspace();
                            self.texture_dirty = true;
                        }
                    }
                }
            }
        }

        ui.separator();
        ui.colored_label(
            Color32::from_rgb(130, 225, 165),
            "🧠 Mapeo Automático Activo: Al pintar sobre el personaje, la herramienta actualiza automáticamente CHR-ROM / CHR-RAM y sincroniza los offsets en PRG-ROM.",
        );
    }

    fn render_entity_properties_tab(
        &mut self,
        ui: &mut egui::Ui,
        project: &mut NgsProject,
        entity: &MetaspriteEntity,
        active_sc_idx: usize,
    ) {
        ui.label(RichText::new("📋 Propiedades del Personaje / Entidad").strong());
        let mut custom_name = project
            .entity_custom_names
            .get(&entity.signature_key)
            .cloned()
            .unwrap_or_else(|| entity.name.clone());
        ui.horizontal(|ui| {
            ui.label("Nombre:");
            if ui.text_edit_singleline(&mut custom_name).changed() {
                project
                    .entity_custom_names
                    .insert(entity.signature_key.clone(), custom_name.clone());
                project.auto_sync_workspace();
            }
        });

        ui.add_space(4.0);
        ui.label(format!("• Firma de Entidad: {}", entity.signature_key));
        ui.label(format!(
            "• Posición en Pantalla: X = {}, Y = {}",
            entity.bbox_x, entity.bbox_y
        ));
        ui.label(format!(
            "• Dimensiones Compuestas: {} x {} píxeles",
            entity.width, entity.height
        ));
        ui.label(format!(
            "• Modo de Sprite PPU: {}",
            if entity.is_8x16_mode {
                "Sprites 8x16 (Bit 5 PPUCTRL activo)"
            } else {
                "Sprites 8x8 estándar"
            }
        ));
        ui.label(format!(
            "• Sub-bloques 8x8 ensamblados: {} piezas",
            entity.pieces.len()
        ));

        let vram_list: Vec<String> = entity
            .unique_vram_tiles()
            .iter()
            .map(|t| format!("${:03X}", t))
            .collect();
        ui.label(format!("• Tiles VRAM constitutivos: {}", vram_list.join(", ")));

        // Mostrar cuántos offsets PRG fueron localizados automáticamente por AutoMemoryMapper
        let mut prg_synced_count = 0usize;
        for t_id in entity.unique_vram_tiles() {
            let id_oam = GraphicId::new(GraphicSource::OamSprite, active_sc_idx, t_id);
            if let Some(p) = project.tile_patches.get(&id_oam) {
                prg_synced_count += p.auto_prg_offsets.len();
            }
        }
        ui.colored_label(
            Color32::from_rgb(130, 220, 255),
            format!(
                "• Estado AutoMemoryMapper: {} direcciones PRG-ROM sincronizadas automáticamente",
                prg_synced_count
            ),
        );

        ui.separator();
        if ui
            .button("💾 Exportar Personaje Ensamblado como .PNG Transparente...")
            .clicked()
        {
            if let Some(save_path) = rfd::FileDialog::new()
                .set_file_name(&format!("{}.png", entity.signature_key))
                .add_filter("Imagen PNG", &["png"])
                .save_file()
            {
                let rgba_arr = entity.render_rgba_canvas(
                    &project.scenes[active_sc_idx].chr_8kb,
                    &project.scenes[active_sc_idx].palette_table,
                );
                let packed: Vec<u32> = rgba_arr
                    .into_iter()
                    .map(|[r, g, b, a]| DecodedRgbaImage::pack_rgba(r, g, b, a))
                    .collect();
                let png_bytes = encode_uncompressed_png_rgba(
                    entity.width as u32,
                    entity.height as u32,
                    &packed,
                );
                match fs::write(&save_path, png_bytes) {
                    Ok(_) => {
                        self.status_feedback = format!(
                            "Personaje exportado como PNG en '{}'.",
                            save_path.display()
                        );
                    }
                    Err(e) => {
                        self.status_feedback = format!("Error exportando PNG: {}", e);
                    }
                }
            }
        }
    }

    fn render_entity_replace_hd_tab(
        &mut self,
        ui: &mut egui::Ui,
        project: &mut NgsProject,
        entity: &MetaspriteEntity,
        active_sc_idx: usize,
    ) {
        ui.label(
            RichText::new("🖼️ Reemplazar o Expandir con .PNG / .BMP (Memoria Ilimitada)")
                .strong()
                .color(Color32::from_rgb(255, 215, 115)),
        );
        ui.label("Puedes importar cualquier archivo .PNG o .BMP externo de dos maneras automáticas:");
        ui.add_space(6.0);

        if ui
            .button(
                RichText::new("✨ 1. Vincular .PNG / .BMP en Memoria Extendida HD (Color y Resolución Ilimitados)")
                    .strong()
                    .color(Color32::from_rgb(110, 240, 155)),
            )
            .clicked()
        {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Imagen PNG o BMP", &["png", "bmp"])
                .pick_file()
            {
                match load_rgba_image_from_path(&path) {
                    Ok(decoded) => {
                        let first_tile = entity
                            .pieces
                            .first()
                            .map(|p| p.vram_tile_id)
                            .unwrap_or(0);
                        let target_id = Some(GraphicId::new(
                            GraphicSource::OamSprite,
                            active_sc_idx,
                            first_tile,
                        ));
                        let asset_id = register_extended_hd_asset(
                            project,
                            entity.name.clone(),
                            target_id,
                            &decoded,
                            Some(path.display().to_string()),
                        );
                        self.texture_dirty = true;
                        self.status_feedback = format!(
                            "✔ Imagen '{}' ({}x{} RGBA) vinculada en Memoria Extendida HD (#{}) para '{}'.",
                            path.display(),
                            decoded.width,
                            decoded.height,
                            asset_id,
                            entity.name
                        );
                    }
                    Err(e) => {
                        self.status_feedback = e;
                    }
                }
            }
        }

        ui.add_space(6.0);

        if ui
            .button("🎨 2. Adaptar y Pintar .PNG / .BMP Directamente en los Tiles NES del Personaje")
            .clicked()
        {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Imagen PNG o BMP", &["png", "bmp"])
                .pick_file()
            {
                match load_rgba_image_from_path(&path) {
                    Ok(decoded) => {
                        bake_rgba_into_entity_nes_tiles(project, entity, active_sc_idx, &decoded);
                        self.texture_dirty = true;
                        self.status_feedback = format!(
                            "✔ Imagen '{}' adaptada automáticamente a los tiles y memoria de '{}'.",
                            path.display(),
                            entity.name
                        );
                    }
                    Err(e) => {
                        self.status_feedback = e;
                    }
                }
            }
        }

        ui.separator();
        let first_tile = entity.pieces.first().map(|p| p.vram_tile_id).unwrap_or(0);
        let target_id = GraphicId::new(GraphicSource::OamSprite, active_sc_idx, first_tile);
        if let Some(bound) = project
            .extended_assets
            .iter()
            .find(|a| a.target_graphic_id.as_ref() == Some(&target_id))
            .cloned()
        {
            ui.colored_label(
                Color32::from_rgb(255, 220, 120),
                format!(
                    "🌟 Activo HD vinculado actualmente: #{} '{}' ({}x{} RGBA)",
                    bound.asset_id, bound.name, bound.width, bound.height
                ),
            );
            if ui.button("🗑 Quitar reemplazo HD y volver al sprite NES").clicked() {
                project
                    .extended_assets
                    .retain(|a| a.asset_id != bound.asset_id);
                project.auto_sync_workspace();
                self.texture_dirty = true;
            }
        }
    }

    fn render_single_tile_editor(
        &mut self,
        ui: &mut egui::Ui,
        project: &mut NgsProject,
        global_idx: usize,
    ) {
        let Some(tile_info) = project.global_tiles.get(global_idx).cloned() else {
            self.editing_single_tile_mode = false;
            return;
        };

        ui.horizontal(|ui| {
            if ui.button("⬅ Volver al Personaje").clicked() {
                self.editing_single_tile_mode = false;
            }
            ui.label(RichText::new(&tile_info.label).strong());
        });
        ui.weak(format!("ID Único: {}", tile_info.id));

        if ui.button("🔄 Restaurar Bloque Original").clicked() {
            project.restore_tile_original(&tile_info.id);
            project.auto_sync_workspace();
            self.texture_dirty = true;
        }

        let pixels = project.get_tile_pixels(global_idx);
        let cell_px = 24.0;
        let (rect, resp) = ui.allocate_exact_size(Vec2::splat(8.0 * cell_px), Sense::click_and_drag());
        let pal = project.palettes[project.selected_palette.min(7)];

        for r in 0..8 {
            for c in 0..8 {
                let c_idx = pixels[r][c] as usize;
                let [red, green, blue] = nes_color_rgb_u8(pal[c_idx & 3]);
                let p_min = Pos2::new(rect.min.x + (c as f32) * cell_px, rect.min.y + (r as f32) * cell_px);
                let p_rect = Rect::from_min_size(p_min, Vec2::splat(cell_px));
                ui.painter()
                    .rect_filled(p_rect, 0.0, Color32::from_rgb(red, green, blue));
                ui.painter().rect_stroke(
                    p_rect,
                    0.0,
                    Stroke::new(0.5, Color32::from_rgba_unmultiplied(255, 255, 255, 45)),
                );
            }
        }

        if resp.clicked() || resp.dragged() {
            if let Some(pos) = resp.interact_pointer_pos() {
                let col = (((pos.x - rect.min.x) / cell_px).floor() as i32).clamp(0, 7) as usize;
                let row = (((pos.y - rect.min.y) / cell_px).floor() as i32).clamp(0, 7) as usize;
                project.set_tile_pixel(global_idx, row, col, self.active_color_idx);
                project.auto_sync_workspace();
                self.texture_dirty = true;
            }
        }
    }

    fn render_bottom_animation_timeline(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        project: &mut NgsProject,
        entities: &[MetaspriteEntity],
        active_sc_idx: usize,
    ) {
        let Some(selected_entity) = entities.get(self.selected_entity_idx) else {
            ui.heading("🎞️ Timeline de Animación y Fotogramas");
            ui.weak("Selecciona un personaje en la escena para ver y editar su tira de animación.");
            return;
        };

        let anim_seq = build_entity_animation_sequence(
            selected_entity,
            active_sc_idx,
            &project.scenes,
            &project.entity_custom_names,
        );

        // Avanzar previsualización de animación si está en Play
        if self.anim_playing && anim_seq.frames.len() > 1 {
            let dt = ctx.input(|i| i.stable_dt).min(0.1);
            self.anim_frame_accum += dt * (self.anim_fps.max(1) as f32);
            if self.anim_frame_accum >= 1.0 {
                self.anim_frame_accum = 0.0;
                if self.anim_current_frame + 1 < anim_seq.frames.len() {
                    self.anim_current_frame += 1;
                } else if self.anim_loop {
                    self.anim_current_frame = 0;
                } else {
                    self.anim_playing = false;
                }
            }
            ctx.request_repaint();
        }

        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!(
                    "🎞️ Timeline de '{}' ({} poses detectadas)",
                    selected_entity.name,
                    anim_seq.frames.len()
                ))
                .strong(),
            );
            let play_txt = if self.anim_playing {
                "⏸ Pausar Anim"
            } else {
                "▶ Play Anim"
            };
            if ui.button(play_txt).clicked() {
                self.anim_playing = !self.anim_playing;
            }
            ui.checkbox(&mut self.anim_loop, "🔁 Loop");
            ui.add(egui::Slider::new(&mut self.anim_fps, 1..=20).text("FPS"));

            if ui
                .button("➕ Avanzar +12 Frames y Capturar Pose")
                .clicked()
            {
                for _ in 0..12 {
                    project.nes_live.step_frame();
                    project.sync_live_chr_ram_signatures();
                    project.nes_live.bus.apu.audio_samples.clear();
                }
                let n = project.scenes.len() + 1;
                project.capture_current_live_scene(format!("Pose #{} en Vivo", n));
                project.auto_sync_workspace();
                self.texture_dirty = true;
            }
        });

        ui.separator();

        // Tira horizontal de fotogramas
        egui::ScrollArea::horizontal()
            .id_salt("bottom_timeline_frames_scroll")
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    for (f_idx, frame) in anim_seq.frames.iter().enumerate() {
                        let is_curr = f_idx == self.anim_current_frame;
                        ui.vertical(|ui| {
                            self.draw_entity_mini_preview(
                                ui,
                                project,
                                &frame.entity,
                                frame.scene_idx,
                                46.0,
                            );
                            if ui
                                .selectable_label(is_curr, &frame.label)
                                .clicked()
                            {
                                self.anim_current_frame = f_idx;
                                project.active_scene_idx = frame.scene_idx;
                                self.texture_dirty = true;
                            }
                        });
                        ui.add_space(8.0);
                    }
                });
            });
    }

    fn draw_entity_mini_preview(
        &self,
        ui: &mut egui::Ui,
        project: &NgsProject,
        entity: &MetaspriteEntity,
        scene_idx: usize,
        box_size: f32,
    ) {
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(box_size), Sense::hover());
        ui.painter()
            .rect_filled(rect, 4.0, Color32::from_rgb(28, 30, 38));

        let Some(sc) = project.scenes.get(scene_idx) else {
            return;
        };
        let rgba = entity.render_rgba_canvas(&sc.chr_8kb, &sc.palette_table);
        let w = (entity.width as usize).max(8);
        let h = (entity.height as usize).max(8);
        let scale = (box_size - 4.0) / (w.max(h) as f32);

        let offset_x = rect.min.x + (box_size - (w as f32) * scale) * 0.5;
        let offset_y = rect.min.y + (box_size - (h as f32) * scale) * 0.5;

        for py in 0..h {
            for px in 0..w {
                let [r, g, b, a] = rgba[py * w + px];
                if a == 0 {
                    continue;
                }
                let p_min = Pos2::new(offset_x + (px as f32) * scale, offset_y + (py as f32) * scale);
                let p_rect = Rect::from_min_size(p_min, Vec2::splat(scale.max(1.0)));
                ui.painter()
                    .rect_filled(p_rect, 0.0, Color32::from_rgb(r, g, b));
            }
        }
    }

    /// Compone la imagen RGB `256x240` de la escena combinando el framebuffer base con
    /// las ediciones en vivo de los personajes (`MetaspriteEntity`) y los reemplazos HD de `extended_assets`.
    fn compose_scene_preview_rgb(
        &self,
        project: &NgsProject,
        entities: &[MetaspriteEntity],
        active_sc_idx: usize,
    ) -> Vec<u8> {
        let Some(sc) = project.scenes.get(active_sc_idx) else {
            return vec![0u8; 256 * 240 * 3];
        };

        let mut rgb = if sc.screen_rgb.len() == 256 * 240 * 3 {
            sc.screen_rgb.clone()
        } else {
            vec![20u8; 256 * 240 * 3]
        };

        // Superponer cada entidad con sus tiles editados (y reemplazo HD si está activo)
        for ent in entities {
            let first_tile = ent.pieces.first().map(|p| p.vram_tile_id).unwrap_or(0);
            let target_id = GraphicId::new(GraphicSource::OamSprite, active_sc_idx, first_tile);

            if self.show_hd_overlays {
                if let Some(hd_asset) = project
                    .extended_assets
                    .iter()
                    .find(|a| a.target_graphic_id.as_ref() == Some(&target_id))
                {
                    let draw_w = (ent.width as u32).max(hd_asset.width.min(64));
                    let draw_h = (ent.height as u32).max(hd_asset.height.min(64));
                    for dy in 0..draw_h {
                        for dx in 0..draw_w {
                            let sx = (dx * hd_asset.width) / draw_w.max(1);
                            let sy = (dy * hd_asset.height) / draw_h.max(1);
                            let px = hd_asset
                                .rgba_pixels
                                .get((sy * hd_asset.width + sx) as usize)
                                .copied()
                                .unwrap_or(0);
                            let [r, g, b, a] = DecodedRgbaImage::unpack_rgba(px);
                            if a < 64 {
                                continue;
                            }
                            let screen_x = (ent.bbox_x as usize) + (dx as usize);
                            let screen_y = (ent.bbox_y as usize) + (dy as usize);
                            if screen_x < 256 && screen_y < 240 {
                                let base = (screen_y * 256 + screen_x) * 3;
                                rgb[base] = r;
                                rgb[base + 1] = g;
                                rgb[base + 2] = b;
                            }
                        }
                    }
                    continue;
                }
            }

            // Si alguno de los tiles de esta entidad tiene un parche activo, redibujarla sobre el framebuffer
            let has_patched_tile = ent.pieces.iter().any(|p| {
                let id1 = GraphicId::new(GraphicSource::OamSprite, active_sc_idx, p.vram_tile_id);
                let id2 =
                    GraphicId::new(GraphicSource::ChrRamScene, active_sc_idx, p.vram_tile_id);
                project.tile_patches.contains_key(&id1) || project.tile_patches.contains_key(&id2)
            });

            if has_patched_tile {
                let canvas = ent.render_rgba_canvas(&sc.chr_8kb, &sc.palette_table);
                let w = ent.width as usize;
                let h = ent.height as usize;
                for dy in 0..h {
                    for dx in 0..w {
                        let [r, g, b, a] = canvas[dy * w + dx];
                        if a == 0 {
                            continue;
                        }
                        let screen_x = (ent.bbox_x as usize) + dx;
                        let screen_y = (ent.bbox_y as usize) + dy;
                        if screen_x < 256 && screen_y < 240 {
                            let base = (screen_y * 256 + screen_x) * 3;
                            rgb[base] = r;
                            rgb[base + 1] = g;
                            rgb[base + 2] = b;
                        }
                    }
                }
            }
        }

        rgb
    }

    fn sync_active_scene_from_live(&mut self, project: &mut NgsProject) {
        let active_idx = project.active_scene_idx;
        if let Some(sc) = project.scenes.get_mut(active_idx) {
            if let Some(cart) = project.nes_live.bus.cartridge.as_ref() {
                sc.chr_8kb = cart.chr_mem.clone();
            }
            sc.palette_table = project.nes_live.bus.ppu.palette_table;
            sc.oam = project.nes_live.bus.ppu.oam;
            sc.ppu_ctrl_bits = project.nes_live.bus.ppu.ctrl.bits();
            sc.screen_rgb = project.nes_live.bus.ppu.framebuffer.clone();
            sc.save_state = project.nes_live.to_save_state_bytes();
        }
        project.rebuild_global_tile_catalog();
    }
}
