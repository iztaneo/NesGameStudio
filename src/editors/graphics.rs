use crate::graphics::animation::build_entity_animation_sequence;
use crate::graphics::metasprite::{
    extract_all_loaded_vram_sprite_cards, reconstruct_all_oam_sprites_as_one,
    reconstruct_individual_oam_sprites, reconstruct_scene_metasprites,
    resolve_screen_region_to_entity, MetaspriteEntity,
};
use crate::graphics::replacement::{
    bake_rgba_into_entity_nes_tiles, encode_uncompressed_png_rgba, load_rgba_image_from_path,
    register_extended_hd_asset, DecodedRgbaImage,
};
use crate::graphics::{GraphicId, GraphicSource};
use crate::project::{nes_color_rgb_u8, NgsProject};
use eframe::egui::{
    self, Color32, ColorImage, Pos2, Rect, RichText, Sense, Stroke, TextureHandle, Vec2,
};
use nes_emu_rust::controller::JoypadButtons;
use std::fs;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum InspectorTab {
    PaintCanvas,
    Properties,
    ReplaceHd,
}

/// Modo de selección sobre la pantalla interactiva (Fase G7).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ScreenSelectionMode {
    /// Agrupa los sprites OAM cercanos en personajes completos (ej. soldado 26x48).
    GroupedCharacters,
    /// Permite seleccionar cada sprite OAM individual (8x8 u 8x16) por separado.
    IndividualSprites,
    /// Permite hacer clic o arrastrar un recuadro sobre cualquier parte de la pantalla (fondo, montañas, árboles, rocas o sprites).
    FreeScreenRegion,
}

#[derive(Clone, Debug)]
struct UndoEntry {
    tile_id: GraphicId,
    previous_bytes: [u8; 16],
    new_bytes: [u8; 16],
}

pub struct GraphicsEditorState {
    pub selected_entity_idx: usize,
    /// Entidad personalizada seleccionada libremente en pantalla (región arrastrada, tarjeta VRAM o "Todos los sprites").
    pub custom_selected_entity: Option<MetaspriteEntity>,
    pub selected_global_tile_idx: Option<usize>,
    pub editing_single_tile_mode: bool,
    pub selection_mode: ScreenSelectionMode,
    pub drag_start_nes: Option<(u16, u16)>,
    pub drag_current_nes: Option<(u16, u16)>,
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
            custom_selected_entity: None,
            selected_global_tile_idx: None,
            editing_single_tile_mode: false,
            selection_mode: ScreenSelectionMode::GroupedCharacters,
            drag_start_nes: None,
            drag_current_nes: None,
            active_color_idx: 1,
            inspector_tab: InspectorTab::PaintCanvas,
            search_query: String::new(),
            live_running: false,
            show_oam_boxes: true,
            show_hd_overlays: true,
            show_canvas_grid: true,
            scene_zoom: 1.65,
            anim_playing: false,
            anim_fps: 6,
            anim_loop: true,
            anim_current_frame: 0,
            anim_frame_accum: 0.0,
            new_creation_name: "Nuevo_Personaje_HD".to_string(),
            new_creation_size: 16,
            status_feedback: "Haz clic sobre cualquier personaje, sprite o elemento del escenario en la pantalla central para editarlo.".to_string(),
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

        if project.scenes.is_empty() {
            project.capture_current_live_scene("Escena Inicial (VRAM Activa)".to_string());
        }
        let active_sc_idx = project
            .active_scene_idx
            .min(project.scenes.len().saturating_sub(1));
        project.active_scene_idx = active_sc_idx;

        // Reconstruir entidades OAM según el modo de selección activo (Agrupados vs Individuales)
        let entities = match self.selection_mode {
            ScreenSelectionMode::IndividualSprites => reconstruct_individual_oam_sprites(
                &project.scenes[active_sc_idx].oam,
                project.scenes[active_sc_idx].ppu_ctrl_bits,
                &project.entity_custom_names,
            ),
            _ => reconstruct_scene_metasprites(
                &project.scenes[active_sc_idx].oam,
                project.scenes[active_sc_idx].ppu_ctrl_bits,
                &project.entity_custom_names,
            ),
        };

        // Extraer también todas las tarjetas de sprites y bloques cargadas en la VRAM de esta escena
        let vram_cards = extract_all_loaded_vram_sprite_cards(
            &project.scenes[active_sc_idx].chr_8kb,
            project.scenes[active_sc_idx].ppu_ctrl_bits,
            &project.entity_custom_names,
        );

        if !entities.is_empty() && self.selected_entity_idx >= entities.len() {
            self.selected_entity_idx = 0;
        }

        // Determinar cuál es la entidad activa para el Inspector Derecho y el Timeline
        let active_entity: Option<MetaspriteEntity> =
            if let Some(custom) = &self.custom_selected_entity {
                Some(custom.clone())
            } else {
                entities
                    .get(self.selected_entity_idx)
                    .cloned()
                    .or_else(|| vram_cards.first().cloned())
            };

        // =========================================================================
        // BARRA SUPERIOR DE GALERÍA DE ESCENAS (MINIATURAS VISUALES + AUTO-DESCUBRIDOR SIN JUGAR)
        // =========================================================================
        egui::TopBottomPanel::top("ngs_scenes_visual_strip")
            .resizable(false)
            .show_inside(ui, |ui| {
                self.render_top_scene_gallery_strip(ui, project, active_sc_idx);
            });

        // =========================================================================
        // PANEL 1 (IZQUIERDO): PASO 1 — CATÁLOGO VISUAL DE LA ESCENA
        // =========================================================================
        egui::SidePanel::left("ngs_graphics_left_tree")
            .resizable(true)
            .default_width(285.0)
            .min_width(240.0)
            .show_inside(ui, |ui| {
                self.render_left_explorer_panel(
                    ui,
                    project,
                    &entities,
                    &vram_cards,
                    active_sc_idx,
                );
            });

        // =========================================================================
        // PANEL 3 (DERECHO): PASO 3 — PINTAR O REEMPLAZAR LA SELECCIÓN
        // =========================================================================
        egui::SidePanel::right("ngs_graphics_right_inspector")
            .resizable(true)
            .default_width(370.0)
            .min_width(325.0)
            .show_inside(ui, |ui| {
                self.render_right_inspector_panel(
                    ui,
                    project,
                    active_entity.as_ref(),
                    active_sc_idx,
                );
            });

        // =========================================================================
        // PANEL 4 (CENTRAL INFERIOR): TIMELINE DE ANIMACIÓN Y POSES
        // =========================================================================
        egui::TopBottomPanel::bottom("ngs_graphics_bottom_timeline")
            .resizable(true)
            .default_height(132.0)
            .min_height(110.0)
            .show_inside(ui, |ui| {
                self.render_bottom_animation_timeline(
                    ctx,
                    ui,
                    project,
                    active_entity.as_ref(),
                    active_sc_idx,
                );
            });

        // =========================================================================
        // PANEL 2 (CENTRAL): PASO 2 — PANTALLA INTERACTIVA (SELECCIÓN UNIVERSAL)
        // =========================================================================
        egui::CentralPanel::default().show_inside(ui, |ui| {
            self.render_center_interactive_scene(
                ctx,
                ui,
                project,
                &entities,
                active_entity.as_ref(),
                active_sc_idx,
            );
        });
    }

    /// Tira visual superior con miniaturas de todas las escenas del juego y botón para descubrir más sin jugar.
    fn render_top_scene_gallery_strip(
        &mut self,
        ui: &mut egui::Ui,
        project: &mut NgsProject,
        active_sc_idx: usize,
    ) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("PASO 1 — Escenas del Juego (Haz clic en una miniatura para ver esa pantalla):")
                    .strong()
                    .color(Color32::from_rgb(130, 220, 255)),
            );
            if ui
                .button(
                    RichText::new("+ Auto-Descubrir Mas Escenas y Enemigos (Sin Jugar)")
                        .strong()
                        .color(Color32::from_rgb(100, 245, 150)),
                )
                .clicked()
            {
                let added = project.auto_discover_more_scenes_without_playing();
                self.custom_selected_entity = None;
                self.texture_dirty = true;
                self.status_feedback = format!(
                    "Se descubrieron y agregaron {} nuevas escenas automaticamente sin necesidad de jugar.",
                    added
                );
            }
        });

        egui::ScrollArea::horizontal()
            .id_salt("top_scenes_thumbnails_scroll")
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let scene_count = project.scenes.len();
                    for sc_idx in 0..scene_count {
                        let is_active = sc_idx == active_sc_idx;
                        let title = project.scenes[sc_idx].title.clone();
                        ui.vertical(|ui| {
                            let thumb_w = 72.0;
                            let thumb_h = 54.0;
                            let (rect, resp) =
                                ui.allocate_exact_size(Vec2::new(thumb_w, thumb_h), Sense::click());
                            self.draw_scene_mini_thumbnail(
                                ui,
                                &project.scenes[sc_idx].screen_rgb,
                                rect,
                            );
                            ui.painter().rect_stroke(
                                rect,
                                3.0,
                                Stroke::new(
                                    if is_active { 2.5 } else { 1.0 },
                                    if is_active {
                                        Color32::from_rgb(90, 255, 130)
                                    } else {
                                        Color32::GRAY
                                    },
                                ),
                            );
                            let short_lbl = if title.len() > 22 {
                                format!("#{} {}...", sc_idx + 1, &title[..19])
                            } else {
                                format!("#{} {}", sc_idx + 1, title)
                            };
                            let lbl_clicked = ui.selectable_label(is_active, short_lbl).clicked();
                            if resp.clicked() || lbl_clicked {
                                project.active_scene_idx = sc_idx;
                                if !project.scenes[sc_idx].save_state.is_empty() {
                                    let _ = project
                                        .nes_live
                                        .from_save_state_bytes(&project.scenes[sc_idx].save_state);
                                }
                                self.custom_selected_entity = None;
                                self.selected_entity_idx = 0;
                                self.editing_single_tile_mode = false;
                                self.texture_dirty = true;
                            }
                        });
                        ui.add_space(6.0);
                    }
                });
            });
    }

    fn draw_scene_mini_thumbnail(&self, ui: &mut egui::Ui, screen_rgb: &[u8], rect: Rect) {
        ui.painter()
            .rect_filled(rect, 3.0, Color32::from_rgb(15, 15, 20));
        if screen_rgb.len() < 256 * 240 * 3 {
            return;
        }
        // Muestrear en cuadrícula de 32x24 para dibujar la miniatura instantáneamente
        let cols = 32usize;
        let rows = 24usize;
        let cw = rect.width() / (cols as f32);
        let ch = rect.height() / (rows as f32);
        for ry in 0..rows {
            let sy = (ry * 240) / rows;
            for rx in 0..cols {
                let sx = (rx * 256) / cols;
                let off = (sy * 256 + sx) * 3;
                let r = screen_rgb[off];
                let g = screen_rgb[off + 1];
                let b = screen_rgb[off + 2];
                let p_min = Pos2::new(rect.min.x + (rx as f32) * cw, rect.min.y + (ry as f32) * ch);
                let p_rect = Rect::from_min_size(p_min, Vec2::new(cw.ceil(), ch.ceil()));
                ui.painter()
                    .rect_filled(p_rect, 0.0, Color32::from_rgb(r, g, b));
            }
        }
    }

    fn render_left_explorer_panel(
        &mut self,
        ui: &mut egui::Ui,
        project: &mut NgsProject,
        entities: &[MetaspriteEntity],
        vram_cards: &[MetaspriteEntity],
        active_sc_idx: usize,
    ) {
        ui.label(
            RichText::new("Catalogo Visual de esta Pantalla")
                .strong()
                .size(15.0)
                .color(Color32::from_rgb(130, 220, 255)),
        );

        ui.horizontal(|ui| {
            ui.label("Buscar:");
            ui.add(
                egui::TextEdit::singleline(&mut self.search_query)
                    .hint_text("Filtrar por nombre o tile..."),
            );
            if !self.search_query.is_empty() && ui.small_button("X").clicked() {
                self.search_query.clear();
            }
        });

        // Botón directo para seleccionar TODOS los sprites activos de la pantalla al mismo tiempo
        if ui
            .button(
                RichText::new("[Seleccionar TODOS los Sprites de la Pantalla]")
                    .strong()
                    .color(Color32::from_rgb(255, 220, 110)),
            )
            .clicked()
        {
            if let Some(all_ent) = reconstruct_all_oam_sprites_as_one(
                &project.scenes[active_sc_idx].oam,
                project.scenes[active_sc_idx].ppu_ctrl_bits,
            ) {
                self.editing_single_tile_mode = false;
                self.status_feedback = format!(
                    "Seleccionados todos los sprites activos de la pantalla ({} piezas).",
                    all_ent.pieces.len()
                );
                self.custom_selected_entity = Some(all_ent);
            }
        }

        ui.separator();
        let query_lower = self.search_query.trim().to_lowercase();

        egui::ScrollArea::vertical()
            .id_salt("left_tree_scroll")
            .show(ui, |ui| {
                // RAMA 1: SPRITES ACTIVOS EN PANTALLA (OAM)
                egui::CollapsingHeader::new(
                    RichText::new(format!(
                        "1. Sprites Visibles en Pantalla ({})",
                        entities.len()
                    ))
                    .strong()
                    .color(Color32::from_rgb(115, 240, 160)),
                )
                .default_open(true)
                .show(ui, |ui| {
                    if entities.is_empty() {
                        ui.weak("No hay sprites OAM en este cuadro. Elige otra miniatura de escena arriba o usa los grupos VRAM de abajo.");
                    }
                    for (idx, ent) in entities.iter().enumerate() {
                        if !query_lower.is_empty()
                            && !ent.name.to_lowercase().contains(&query_lower)
                            && !ent.signature_key.to_lowercase().contains(&query_lower)
                        {
                            continue;
                        }
                        let is_selected = !self.editing_single_tile_mode
                            && self.custom_selected_entity.is_none()
                            && self.selected_entity_idx == idx;

                        ui.horizontal(|ui| {
                            self.draw_entity_mini_preview(ui, project, ent, active_sc_idx, 30.0);
                            let label_txt = format!(
                                "{}\nPos ({},{}) • {} bloques",
                                ent.name,
                                ent.bbox_x,
                                ent.bbox_y,
                                ent.pieces.len()
                            );
                            if ui.selectable_label(is_selected, label_txt).clicked() {
                                self.selected_entity_idx = idx;
                                self.custom_selected_entity = None;
                                self.editing_single_tile_mode = false;
                                self.anim_current_frame = 0;
                            }
                        });
                        ui.add_space(2.0);
                    }
                });

                ui.add_space(4.0);

                // RAMA 2: TODOS LOS SPRITES Y BLOQUES CARGADOS EN LA VRAM DE ESTA ESCENA (Con miniaturas a color)
                egui::CollapsingHeader::new(
                    RichText::new(format!(
                        "2. Todos los Sprites y Bloques de la Escena ({})",
                        vram_cards.len()
                    ))
                    .strong()
                    .color(Color32::from_rgb(150, 210, 255)),
                )
                .default_open(true)
                .show(ui, |ui| {
                    ui.weak("Incluye enemigos, proyectiles, poses y bloques de escenario cargados en la memoria de esta pantalla:");
                    for card in vram_cards {
                        if !query_lower.is_empty()
                            && !card.name.to_lowercase().contains(&query_lower)
                            && !card.signature_key.to_lowercase().contains(&query_lower)
                        {
                            continue;
                        }
                        let is_sel = self
                            .custom_selected_entity
                            .as_ref()
                            .map(|c| c.signature_key == card.signature_key)
                            .unwrap_or(false);

                        ui.horizontal(|ui| {
                            self.draw_entity_mini_preview(ui, project, card, active_sc_idx, 30.0);
                            if ui.selectable_label(is_sel, &card.name).clicked() {
                                self.editing_single_tile_mode = false;
                                self.custom_selected_entity = Some(card.clone());
                                self.status_feedback = format!(
                                    "Abierto en el lienzo derecho: '{}'.",
                                    card.name
                                );
                            }
                        });
                        ui.add_space(2.0);
                    }
                });

                ui.add_space(4.0);

                // RAMA 3: MEMORIA EXTENDIDA HD ILIMITADA
                egui::CollapsingHeader::new(
                    RichText::new(format!(
                        "3. Memoria Extendida HD ({})",
                        project.extended_assets.len()
                    ))
                    .strong()
                    .color(Color32::from_rgb(255, 210, 110)),
                )
                .default_open(false)
                .show(ui, |ui| {
                    ui.weak("Crea personajes o gráficos nuevos sin límite de memoria NES:");
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut self.new_creation_name);
                        egui::ComboBox::from_id_salt("new_creation_size_cb")
                            .selected_text(format!(
                                "{}x{}",
                                self.new_creation_size, self.new_creation_size
                            ))
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
                        .button("+ Crear Gráfico en Memoria Extendida")
                        .clicked()
                    {
                        let sz = self.new_creation_size.max(8);
                        let mut pixels = vec![0u32; (sz * sz) as usize];
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
                        ui.colored_label(
                            Color32::from_rgb(255, 220, 130),
                            format!(
                                "* #{} {} ({}x{} RGBA)",
                                asset.asset_id, asset.name, asset.width, asset.height
                            ),
                        );
                    }
                });

                ui.add_space(4.0);

                // RAMA 4: AVANZADO (Bancos CHR / Candidatos PRG)
                egui::CollapsingHeader::new(
                    RichText::new("4. Avanzado (Bancos CHR / Candidatos PRG)").weak(),
                )
                .default_open(false)
                .show(ui, |ui| {
                    ui.label(format!(
                        "Modo: {}",
                        if project.is_chr_ram {
                            "CHR-RAM Dinámica + Auto-Sync PRG"
                        } else {
                            "CHR-ROM por Bancos"
                        }
                    ));
                    ui.label(format!(
                        "Parches activos: {} tiles | Candidatos PRG: {}",
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
                });
            });
    }

    fn render_center_interactive_scene(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        project: &mut NgsProject,
        entities: &[MetaspriteEntity],
        active_entity: Option<&MetaspriteEntity>,
        active_sc_idx: usize,
    ) {
        // Encabezado PASO 2 y selector del Modo de Selección en Pantalla
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new("PASO 2 — Modo de Selección en Pantalla:")
                    .strong()
                    .color(Color32::from_rgb(130, 220, 255)),
            );
            if ui
                .selectable_label(
                    self.selection_mode == ScreenSelectionMode::GroupedCharacters
                        && self.custom_selected_entity.is_none(),
                    "[1. Personaje Completo]",
                )
                .clicked()
            {
                self.selection_mode = ScreenSelectionMode::GroupedCharacters;
                self.custom_selected_entity = None;
                self.editing_single_tile_mode = false;
            }
            if ui
                .selectable_label(
                    self.selection_mode == ScreenSelectionMode::IndividualSprites
                        && self.custom_selected_entity.is_none(),
                    "[2. Cada Sprite Individual]",
                )
                .clicked()
            {
                self.selection_mode = ScreenSelectionMode::IndividualSprites;
                self.custom_selected_entity = None;
                self.editing_single_tile_mode = false;
            }
            if ui
                .selectable_label(
                    self.selection_mode == ScreenSelectionMode::FreeScreenRegion,
                    "[3. Seleccionar Cualquier Zona / Fondo (Clic o Arrastrar)]",
                )
                .clicked()
            {
                self.selection_mode = ScreenSelectionMode::FreeScreenRegion;
                self.editing_single_tile_mode = false;
            }
        });

        // Controles de ejecución opcionales y vista
        ui.horizontal_wrapped(|ui| {
            let run_label = if self.live_running {
                "[Pausar Juego]"
            } else {
                "[Correr en Vivo]"
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

            if ui.button("+1 Frame").clicked() {
                self.live_running = false;
                project.nes_live.step_frame();
                project.sync_live_chr_ram_signatures();
                project.nes_live.bus.apu.audio_samples.clear();
                self.sync_active_scene_from_live(project);
                self.texture_dirty = true;
            }

            if ui.button("+15 Frames").clicked() {
                self.live_running = false;
                for _ in 0..15 {
                    project.nes_live.step_frame();
                    project.sync_live_chr_ram_signatures();
                    project.nes_live.bus.apu.audio_samples.clear();
                }
                self.sync_active_scene_from_live(project);
                self.texture_dirty = true;
            }

            if ui.button("Pulsar START").clicked() {
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

            ui.separator();
            ui.checkbox(&mut self.show_oam_boxes, "Ver Cajas de Sprites");
            ui.checkbox(&mut self.show_hd_overlays, "Ver Reemplazos HD");
            ui.label("Zoom:");
            for z in [1.35f32, 1.65, 2.0] {
                if ui
                    .selectable_label((self.scene_zoom - z).abs() < 0.05, format!("{:.2}x", z))
                    .clicked()
                {
                    self.scene_zoom = z;
                }
            }
        });

        ui.colored_label(Color32::from_rgb(125, 230, 175), &self.status_feedback);

        if self.texture_dirty || self.scene_texture.is_none() {
            let composed_rgb =
                self.compose_scene_preview_rgb(project, entities, active_entity, active_sc_idx);
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
                    ui.allocate_exact_size(Vec2::new(disp_w, disp_h), Sense::click_and_drag());

                if let Some(tex) = &self.scene_texture {
                    ui.painter().image(
                        tex.id(),
                        rect,
                        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                        Color32::WHITE,
                    );
                }

                // Dibujar cajas alrededor de los sprites OAM
                if self.show_oam_boxes {
                    for (idx, ent) in entities.iter().enumerate() {
                        let is_sel = !self.editing_single_tile_mode
                            && self.custom_selected_entity.is_none()
                            && idx == self.selected_entity_idx;
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
                        ui.painter().rect_stroke(
                            box_rect,
                            2.0,
                            Stroke::new(if is_sel { 2.5 } else { 1.2 }, stroke_color),
                        );
                    }
                }

                // Si hay una entidad o región de pantalla personalizada seleccionada, enmarcarla en amarillo/verde
                if let Some(custom_ent) = &self.custom_selected_entity {
                    if custom_ent.width > 0 && custom_ent.height > 0 {
                        let min_p = Pos2::new(
                            rect.min.x + (custom_ent.bbox_x as f32) * self.scene_zoom,
                            rect.min.y + (custom_ent.bbox_y as f32) * self.scene_zoom,
                        );
                        let max_p = Pos2::new(
                            min_p.x + (custom_ent.width as f32) * self.scene_zoom,
                            min_p.y + (custom_ent.height as f32) * self.scene_zoom,
                        );
                        ui.painter().rect_stroke(
                            Rect::from_min_max(min_p, max_p),
                            2.0,
                            Stroke::new(2.5, Color32::from_rgb(255, 225, 80)),
                        );
                    }
                }

                // Dibujar el rectángulo de arrastre en vivo cuando el usuario arrastra sobre la pantalla
                if let (Some((sx, sy)), Some((cx, cy))) =
                    (self.drag_start_nes, self.drag_current_nes)
                {
                    let x0 = sx.min(cx) as f32;
                    let y0 = sy.min(cy) as f32;
                    let x1 = (sx.max(cx) + 8).min(256) as f32;
                    let y1 = (sy.max(cy) + 8).min(240) as f32;
                    let drag_rect = Rect::from_min_max(
                        Pos2::new(
                            rect.min.x + x0 * self.scene_zoom,
                            rect.min.y + y0 * self.scene_zoom,
                        ),
                        Pos2::new(
                            rect.min.x + x1 * self.scene_zoom,
                            rect.min.y + y1 * self.scene_zoom,
                        ),
                    );
                    ui.painter().rect_stroke(
                        drag_rect,
                        2.0,
                        Stroke::new(2.0, Color32::from_rgb(255, 240, 100)),
                    );
                }

                // Manejo de Clic o Arrastre Universal sobre la pantalla 256x240
                if response.drag_started() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        let nx =
                            (((pos.x - rect.min.x) / self.scene_zoom).clamp(0.0, 255.0)) as u16;
                        let ny =
                            (((pos.y - rect.min.y) / self.scene_zoom).clamp(0.0, 239.0)) as u16;
                        self.drag_start_nes = Some((nx, ny));
                        self.drag_current_nes = Some((nx, ny));
                    }
                }
                if response.dragged() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        let nx =
                            (((pos.x - rect.min.x) / self.scene_zoom).clamp(0.0, 255.0)) as u16;
                        let ny =
                            (((pos.y - rect.min.y) / self.scene_zoom).clamp(0.0, 239.0)) as u16;
                        self.drag_current_nes = Some((nx, ny));
                    }
                }
                if response.drag_stopped() {
                    if let (Some((sx, sy)), Some((ex, ey))) =
                        (self.drag_start_nes.take(), self.drag_current_nes.take())
                    {
                        let rx = sx.min(ex);
                        let ry = sy.min(ey);
                        let rw = sx.max(ex).saturating_sub(rx) + 8;
                        let rh = sy.max(ey).saturating_sub(ry) + 8;
                        let sc = &project.scenes[active_sc_idx];
                        if let Some(region_ent) = resolve_screen_region_to_entity(
                            &sc.screen_rgb,
                            &sc.chr_8kb,
                            &sc.palette_table,
                            sc.ppu_ctrl_bits,
                            rx,
                            ry,
                            rw.min(64),
                            rh.min(64),
                        ) {
                            self.editing_single_tile_mode = false;
                            self.status_feedback = format!(
                                "Seleccionada región de pantalla: {} ({} bloques 8x8 listos para pintar a la derecha).",
                                region_ent.name,
                                region_ent.pieces.len()
                            );
                            self.custom_selected_entity = Some(region_ent);
                        }
                    }
                } else if response.clicked() {
                    self.drag_start_nes = None;
                    self.drag_current_nes = None;
                    if let Some(pos) = response.interact_pointer_pos() {
                        let nes_x =
                            (((pos.x - rect.min.x) / self.scene_zoom).clamp(0.0, 255.0)) as u16;
                        let nes_y =
                            (((pos.y - rect.min.y) / self.scene_zoom).clamp(0.0, 239.0)) as u16;

                        // Si no estamos en modo región libre, primero verificar si hizo clic sobre un sprite OAM
                        let hit_oam = if self.selection_mode
                            != ScreenSelectionMode::FreeScreenRegion
                        {
                            entities
                                .iter()
                                .enumerate()
                                .find(|(_, e)| e.contains_screen_point(nes_x, nes_y))
                        } else {
                            None
                        };

                        if let Some((hit_idx, hit_ent)) = hit_oam {
                            self.selected_entity_idx = hit_idx;
                            self.custom_selected_entity = None;
                            self.editing_single_tile_mode = false;
                            self.anim_current_frame = 0;
                            self.status_feedback = format!(
                                "Seleccionado en pantalla: '{}' ({}x{} px en X={}, Y={})",
                                hit_ent.name,
                                hit_ent.width,
                                hit_ent.height,
                                hit_ent.bbox_x,
                                hit_ent.bbox_y
                            );
                        } else {
                            // Si hizo clic en CUALQUIER otra parte de la pantalla (montañas, árboles, rocas, suelo, HUD),
                            // resolver automáticamente ese bloque 16x16 del escenario y abrirlo a la derecha!
                            let sc = &project.scenes[active_sc_idx];
                            if let Some(bg_ent) = resolve_screen_region_to_entity(
                                &sc.screen_rgb,
                                &sc.chr_8kb,
                                &sc.palette_table,
                                sc.ppu_ctrl_bits,
                                nes_x,
                                nes_y,
                                16,
                                16,
                            ) {
                                self.editing_single_tile_mode = false;
                                self.status_feedback = format!(
                                    "Seleccionado elemento de pantalla en (X={}, Y={}) — puedes pintarlo a la derecha o arrastrar un recuadro más grande.",
                                    bg_ent.bbox_x, bg_ent.bbox_y
                                );
                                self.custom_selected_entity = Some(bg_ent);
                            }
                        }
                    }
                }
            });
    }

    fn render_right_inspector_panel(
        &mut self,
        ui: &mut egui::Ui,
        project: &mut NgsProject,
        active_entity: Option<&MetaspriteEntity>,
        active_sc_idx: usize,
    ) {
        ui.label(
            RichText::new("PASO 3 — Pinta o Reemplaza tu Selección")
                .strong()
                .size(15.0)
                .color(Color32::from_rgb(130, 220, 255)),
        );

        ui.horizontal(|ui| {
            ui.selectable_value(
                &mut self.inspector_tab,
                InspectorTab::PaintCanvas,
                "[1. Pintar Dibujo]",
            );
            ui.selectable_value(
                &mut self.inspector_tab,
                InspectorTab::Properties,
                "[2. Propiedades / PNG]",
            );
            ui.selectable_value(
                &mut self.inspector_tab,
                InspectorTab::ReplaceHd,
                "[3. Reemplazar HD]",
            );
        });
        ui.separator();

        if self.editing_single_tile_mode {
            if let Some(g_idx) = self.selected_global_tile_idx {
                self.render_single_tile_editor(ui, project, g_idx);
                return;
            }
        }

        let Some(entity) = active_entity else {
            ui.label("Haz clic sobre cualquier personaje o elemento en la pantalla central para editarlo aquí.");
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

        ui.horizontal(|ui| {
            if ui
                .add_enabled(!self.undo_stack.is_empty(), egui::Button::new("Deshacer"))
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
                .add_enabled(!self.redo_stack.is_empty(), egui::Button::new("Rehacer"))
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
            if ui.button("Restaurar Original").clicked() {
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

        // Selector de Color de la Paleta Activa
        let palette_table = project.scenes[active_sc_idx].palette_table;
        let is_bg_pal = entity
            .pieces
            .first()
            .map(|p| (p.palette_idx & 0x80) != 0)
            .unwrap_or(false);
        let pal_base = if is_bg_pal {
            ((entity.primary_palette as usize) & 3) * 4
        } else {
            16 + ((entity.primary_palette as usize) & 3) * 4
        };
        ui.separator();
        ui.label("1. Elige un color y luego haz clic o arrastra sobre el dibujo grande:");
        ui.horizontal(|ui| {
            for c_idx in 0..4u8 {
                let nes_col = if c_idx == 0 {
                    palette_table[0]
                } else {
                    palette_table[(pal_base + (c_idx as usize)) & 31]
                };
                let [r, g, b] = nes_color_rgb_u8(nes_col);
                let col32 = if c_idx == 0 && !is_bg_pal {
                    Color32::from_rgb(35, 35, 45)
                } else {
                    Color32::from_rgb(r, g, b)
                };
                let is_active = self.active_color_idx == c_idx;
                let (swatch_rect, resp) =
                    ui.allocate_exact_size(Vec2::new(68.0, 26.0), Sense::click());
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
                let txt = if c_idx == 0 && !is_bg_pal {
                    "0: Transp".to_string()
                } else {
                    format!("C{} (${:02X})", c_idx, nes_col)
                };
                ui.painter().text(
                    swatch_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    txt,
                    egui::FontId::proportional(11.0),
                    if (r as u16 + g as u16 + b as u16) > 380 && (c_idx != 0 || is_bg_pal) {
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
            "Mostrar cuadrícula de píxeles y bloques 8x8",
        );

        let canvas_rgba = entity.render_rgba_canvas(
            &project.scenes[active_sc_idx].chr_8kb,
            &project.scenes[active_sc_idx].palette_table,
        );
        let w = (entity.width as usize).max(8);
        let h = (entity.height as usize).max(8);
        let max_dim = w.max(h) as f32;
        let cell_px = (264.0 / max_dim).clamp(4.0, 16.0);
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
            "Mapeo Automático Activo: Al pintar aquí, la herramienta actualiza automáticamente CHR-ROM / CHR-RAM y sincroniza PRG-ROM.",
        );
    }

    fn render_entity_properties_tab(
        &mut self,
        ui: &mut egui::Ui,
        project: &mut NgsProject,
        entity: &MetaspriteEntity,
        active_sc_idx: usize,
    ) {
        ui.label(RichText::new("Propiedades de la Selección").strong());
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
        ui.label(format!("• Clave de Identificación: {}", entity.signature_key));
        ui.label(format!(
            "• Posición en Pantalla: X = {}, Y = {}",
            entity.bbox_x, entity.bbox_y
        ));
        ui.label(format!(
            "• Dimensiones Compuestas: {} x {} píxeles",
            entity.width, entity.height
        ));
        ui.label(format!(
            "• Bloques 8x8 ensamblados: {} piezas",
            entity.pieces.len()
        ));

        let vram_list: Vec<String> = entity
            .unique_vram_tiles()
            .iter()
            .map(|t| format!("${:03X}", t))
            .collect();
        ui.label(format!("• Tiles VRAM constitutivos: {}", vram_list.join(", ")));

        ui.separator();
        if ui
            .button("Exportar Selección como Archivo .PNG Transparente...")
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
                            "Selección exportada como PNG en '{}'.",
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
            RichText::new("Reemplazar o Expandir con .PNG / .BMP (Memoria Ilimitada)")
                .strong()
                .color(Color32::from_rgb(255, 215, 115)),
        );
        ui.label("Importa cualquier imagen .PNG o .BMP externa de dos maneras automáticas:");
        ui.add_space(6.0);

        if ui
            .button(
                RichText::new("1. Vincular .PNG / .BMP en Memoria Extendida HD (Sin límite de color/tamaño)")
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
                            "Imagen '{}' ({}x{} RGBA) vinculada en Memoria Extendida HD (#{}) para '{}'.",
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
            .button("2. Adaptar y Pintar .PNG / .BMP Automáticamente en los Tiles NES")
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
                            "Imagen '{}' adaptada automáticamente a los tiles y memoria de '{}'.",
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
                    "Activo HD vinculado actualmente: #{} '{}' ({}x{} RGBA)",
                    bound.asset_id, bound.name, bound.width, bound.height
                ),
            );
            if ui.button("Quitar reemplazo HD y volver al gráfico NES").clicked() {
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
            if ui.button("<- Volver a la Selección").clicked() {
                self.editing_single_tile_mode = false;
            }
            ui.label(RichText::new(&tile_info.label).strong());
        });
        ui.weak(format!("ID Único: {}", tile_info.id));

        if ui.button("Restaurar Bloque Original").clicked() {
            project.restore_tile_original(&tile_info.id);
            project.auto_sync_workspace();
            self.texture_dirty = true;
        }

        let pixels = project.get_tile_pixels(global_idx);
        let cell_px = 24.0;
        let (rect, resp) =
            ui.allocate_exact_size(Vec2::splat(8.0 * cell_px), Sense::click_and_drag());
        let pal = project.palettes[project.selected_palette.min(7)];

        for r in 0..8 {
            for c in 0..8 {
                let c_idx = pixels[r][c] as usize;
                let [red, green, blue] = nes_color_rgb_u8(pal[c_idx & 3]);
                let p_min = Pos2::new(
                    rect.min.x + (c as f32) * cell_px,
                    rect.min.y + (r as f32) * cell_px,
                );
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
        active_entity: Option<&MetaspriteEntity>,
        active_sc_idx: usize,
    ) {
        let Some(selected_entity) = active_entity else {
            ui.label(RichText::new("Timeline de Animación y Fotogramas").strong());
            ui.weak("Selecciona un personaje en la escena para ver sus poses de animación.");
            return;
        };

        let anim_seq = build_entity_animation_sequence(
            selected_entity,
            active_sc_idx,
            &project.scenes,
            &project.entity_custom_names,
        );

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
                    "Timeline de '{}' ({} poses detectadas en las escenas)",
                    selected_entity.name,
                    anim_seq.frames.len()
                ))
                .strong(),
            );
            let play_txt = if self.anim_playing {
                "[Pausar Anim]"
            } else {
                "[Play Anim]"
            };
            if ui.button(play_txt).clicked() {
                self.anim_playing = !self.anim_playing;
            }
            ui.checkbox(&mut self.anim_loop, "Loop");
            ui.add(egui::Slider::new(&mut self.anim_fps, 1..=20).text("FPS"));
        });

        ui.separator();

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
                                42.0,
                            );
                            if ui.selectable_label(is_curr, &frame.label).clicked() {
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
                let p_min = Pos2::new(
                    offset_x + (px as f32) * scale,
                    offset_y + (py as f32) * scale,
                );
                let p_rect = Rect::from_min_size(p_min, Vec2::splat(scale.max(1.0)));
                ui.painter()
                    .rect_filled(p_rect, 0.0, Color32::from_rgb(r, g, b));
            }
        }
    }

    fn compose_scene_preview_rgb(
        &self,
        project: &NgsProject,
        entities: &[MetaspriteEntity],
        active_entity: Option<&MetaspriteEntity>,
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

        let mut all_to_draw: Vec<&MetaspriteEntity> = entities.iter().collect();
        if let Some(act) = active_entity {
            if act.signature_key.starts_with("SCR_REG_") {
                all_to_draw.push(act);
            }
        }

        for ent in all_to_draw {
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
