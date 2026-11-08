mod codegen;
mod editors;
mod graphics;
mod parser;
mod project;
mod runtime;
#[cfg(target_os = "windows")]
mod win32_show;
mod workspace;

use eframe::egui::{self, Color32, RichText};
use editors::audio::AudioEditorState;
use editors::graphics::GraphicsEditorState;
use project::NgsProject;
use runtime::jukebox_apu::JukeboxEngine;
use std::path::PathBuf;
use workspace::WorkspaceTemplate;

const WINDOW_TITLE: &str = "NesGameStudio — IDE Visual de Remasterización HD, Audio & Creación 2D/2.5D";

#[derive(PartialEq, Eq, Clone, Copy)]
enum ActiveTab {
    GraphicsGallery,
    AudioJukebox,
    SubroutinesInspector,
}

struct NesGameStudioApp {
    project: NgsProject,
    active_tab: ActiveTab,
    graphics_editor: GraphicsEditorState,
    audio_editor: AudioEditorState,
    jukebox: JukeboxEngine,
    subroutine_filter: String,
    show_about_modal: bool,
    show_guide_window: bool,
    show_workspace_wizard: bool,
    wizard_is_existing_game: bool,
    wizard_game_title: String,
    wizard_template: WorkspaceTemplate,
    wizard_existing_source: Option<PathBuf>,
    wizard_target_parent_dir: PathBuf,
    status_bar_msg: String,
}

impl Default for NesGameStudioApp {
    fn default() -> Self {
        let default_ws_parent = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("Workspaces");
        Self {
            project: NgsProject::default(),
            active_tab: ActiveTab::GraphicsGallery,
            graphics_editor: GraphicsEditorState::default(),
            audio_editor: AudioEditorState::default(),
            jukebox: JukeboxEngine::new(),
            subroutine_filter: "TODAS".to_string(),
            show_about_modal: false,
            show_guide_window: false,
            show_workspace_wizard: false,
            wizard_is_existing_game: false,
            wizard_game_title: "Mi_Juego_NGS".to_string(),
            wizard_template: WorkspaceTemplate::PlatformerAction,
            wizard_existing_source: None,
            wizard_target_parent_dir: default_ws_parent,
            status_bar_msg: "Listo. Crea un Workspace (Juego Nuevo o Existente) o abre un proyecto desde el menú Archivo.".to_string(),
        }
    }
}

impl eframe::App for NesGameStudioApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Si la rockola está activa, alimentar el flujo de audio real desde la CPU 6502 + APU del juego cargado
        if self.jukebox.playing && self.project.loaded {
            self.jukebox.step_real_audio_frame(&mut self.project);
            ctx.request_repaint();
        }

        // BARRA DE MENÚ ESTÁNDAR DE WINDOWS (Archivo | Edición | Vista | Compilar | Ayuda)
        egui::TopBottomPanel::top("main_menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                // 1. MENÚ ARCHIVO (CON GESTIÓN COMPLETA DE WORKSPACE FASE G0)
                ui.menu_button("Archivo", |ui| {
                    if ui.button("✨ Crear Workspace para Juego Nuevo...").clicked() {
                        self.wizard_is_existing_game = false;
                        self.show_workspace_wizard = true;
                        ui.close_menu();
                    }
                    if ui
                        .button("🚀 Crear Workspace desde Juego Existente (.NES / .C)...")
                        .clicked()
                    {
                        self.wizard_is_existing_game = true;
                        self.show_workspace_wizard = true;
                        ui.close_menu();
                    }
                    if ui
                        .button("📂 Abrir Workspace Existente (project.ngs.json)...")
                        .clicked()
                    {
                        ui.close_menu();
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Workspace NesGameStudio", &["json"])
                            .pick_file()
                        {
                            self.jukebox.playing = false;
                            self.audio_editor = editors::audio::AudioEditorState::default();
                            match workspace::load_workspace(&path, &mut self.project) {
                                Ok(msg) => {
                                    self.graphics_editor.invalidate_texture();
                                    self.status_bar_msg = msg;
                                }
                                Err(e) => self.status_bar_msg = e,
                            }
                        }
                    }
                    ui.separator();
                    if ui.button("📂 Abrir Código .C / .ASM (Auto-Workspace)...").clicked() {
                        ui.close_menu();
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Código C de NesEmuRust", &["c"])
                            .pick_file()
                        {
                            self.jukebox.playing = false;
                            self.audio_editor = editors::audio::AudioEditorState::default();
                            let stem = path
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("Juego_NES");
                            let ws_root = self
                                .wizard_target_parent_dir
                                .join(format!("{}_Workspace", workspace::sanitize_slug(stem)));
                            match workspace::create_workspace_from_existing_game(
                                &path,
                                &ws_root,
                                &mut self.project,
                            ) {
                                Ok(msg) => {
                                    self.graphics_editor.invalidate_texture();
                                    self.status_bar_msg = msg;
                                }
                                Err(e) => self.status_bar_msg = e,
                            }
                        }
                    }
                    if ui.button("🎮 Abrir ROM .NES Directamente (Auto-Workspace)...").clicked() {
                        ui.close_menu();
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("ROM de NES", &["nes"])
                            .pick_file()
                        {
                            self.jukebox.playing = false;
                            self.audio_editor = editors::audio::AudioEditorState::default();
                            let stem = path
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("Juego_NES");
                            let ws_root = self
                                .wizard_target_parent_dir
                                .join(format!("{}_Workspace", workspace::sanitize_slug(stem)));
                            match workspace::create_workspace_from_existing_game(
                                &path,
                                &ws_root,
                                &mut self.project,
                            ) {
                                Ok(msg) => {
                                    self.graphics_editor.invalidate_texture();
                                    self.status_bar_msg = msg;
                                }
                                Err(e) => self.status_bar_msg = e,
                            }
                        }
                    }
                    ui.separator();
                    if ui.button("💾 Guardar / Sincronizar Workspace Actual").clicked() {
                        ui.close_menu();
                        if let Some(ws_dir) = self.project.workspace_dir.clone() {
                            match workspace::save_workspace(&self.project, &ws_dir) {
                                Ok(msg) => self.status_bar_msg = msg,
                                Err(e) => self.status_bar_msg = e,
                            }
                        } else if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                            let ws_dir = folder.join(format!(
                                "{}_Workspace",
                                workspace::sanitize_slug(&self.project.game_title)
                            ));
                            match workspace::save_workspace(&self.project, &ws_dir) {
                                Ok(msg) => {
                                    self.project.workspace_dir = Some(ws_dir);
                                    self.status_bar_msg = msg;
                                }
                                Err(e) => self.status_bar_msg = e,
                            }
                        }
                    }
                    ui.separator();
                    if ui.button("🚪 Salir").clicked() {
                        std::process::exit(0);
                    }
                });

                // 2. MENÚ EDICIÓN
                ui.menu_button("Edición", |ui| {
                    if ui.button("⏭ Avanzar Juego +60 Frames y Capturar Mapas de Bits").clicked() {
                        ui.close_menu();
                        if self.project.loaded {
                            for _ in 0..60 {
                                self.project.nes_live.step_frame();
                                self.project.sync_live_chr_ram_signatures();
                                self.project.nes_live.bus.apu.audio_samples.clear();
                            }
                            let n = self.project.scenes.len() + 1;
                            self.project
                                .capture_current_live_scene(format!("Escena {}: Captura en Vivo", n));
                            self.project.auto_sync_workspace();
                            self.graphics_editor.invalidate_texture();
                            self.status_bar_msg = "Nueva escena capturada y sincronizada en el Workspace.".to_string();
                        }
                    }
                    ui.separator();
                    if ui.button("🔄 Restaurar Todos los Gráficos al Original").clicked() {
                        let all_ids: Vec<_> = self.project.tile_patches.keys().cloned().collect();
                        for id in all_ids {
                            self.project.restore_tile_original(&id);
                        }
                        self.project.modified_tiles.clear();
                        self.project.auto_sync_workspace();
                        self.graphics_editor.invalidate_texture();
                        self.status_bar_msg = "Todos los gráficos fueron restaurados a su estado original.".to_string();
                        ui.close_menu();
                    }
                    if ui.button("🧹 Limpiar Reemplazos de Audio (.MP3/.WAV)").clicked() {
                        self.project.audio_replacements.clear();
                        self.project.audio_event_replacements.clear();
                        self.project.auto_sync_workspace();
                        self.status_bar_msg = "Se limpiaron los reemplazos de audio.".to_string();
                        ui.close_menu();
                    }
                });

                // 3. MENÚ VISTA / MÓDULOS
                ui.menu_button("Vista", |ui| {
                    if ui
                        .radio_value(
                            &mut self.active_tab,
                            ActiveTab::GraphicsGallery,
                            "🎨 Gráficos (personajes, escenarios y colores)",
                        )
                        .clicked()
                    {
                        ui.close_menu();
                    }
                    if ui
                        .radio_value(
                            &mut self.active_tab,
                            ActiveTab::AudioJukebox,
                            "🎵 Rockola (música y efectos del juego)",
                        )
                        .clicked()
                    {
                        ui.close_menu();
                    }
                    if ui
                        .radio_value(
                            &mut self.active_tab,
                            ActiveTab::SubroutinesInspector,
                            "🔍 Código (rutinas .ASM / .C)",
                        )
                        .clicked()
                    {
                        ui.close_menu();
                    }
                });

                // 4. MENÚ COMPILAR
                ui.menu_button("Compilar", |ui| {
                    ui.checkbox(
                        &mut self.project.widescreen_16_9,
                        "🖥 Activar Widescreen 16:9 Real (426x240)",
                    );
                    ui.separator();
                    if ui.button("🔨 Generar Archivo .C Modificado").clicked() {
                        ui.close_menu();
                        match codegen::generate_modified_c_file(&self.project) {
                            Ok(p) => self.status_bar_msg = format!("Código C generado: {}", p.display()),
                            Err(e) => self.status_bar_msg = e,
                        }
                    }
                    if ui.button("🚀 Compilar y Ejecutar .EXE Nativo (x64)").clicked() {
                        ui.close_menu();
                        match codegen::compile_and_run_modified_c(&self.project, true) {
                            Ok(msg) => self.status_bar_msg = msg,
                            Err(e) => self.status_bar_msg = e,
                        }
                    }
                });

                // 5. MENÚ AYUDA
                ui.menu_button("Ayuda", |ui| {
                    if ui.button("📖 Guía Rápida: ¿Cómo se usa?").clicked() {
                        self.show_guide_window = true;
                        ui.close_menu();
                    }
                    if ui.button("ℹ Acerca de NesGameStudio").clicked() {
                        self.show_about_modal = true;
                        ui.close_menu();
                    }
                });

                // Indicador de la vista actual y botones rápidos de Workspace
                ui.separator();
                let current = match self.active_tab {
                    ActiveTab::GraphicsGallery => "Vista actual: 🎨 Gráficos",
                    ActiveTab::AudioJukebox => "Vista actual: 🎵 Rockola",
                    ActiveTab::SubroutinesInspector => "Vista actual: 🔍 Código",
                };
                ui.weak(current);
                ui.separator();
                if ui.small_button("✨ Nuevo Juego (Workspace)").clicked() {
                    self.wizard_is_existing_game = false;
                    self.show_workspace_wizard = true;
                }
                if ui.small_button("🚀 Importar Juego a Workspace").clicked() {
                    self.wizard_is_existing_game = true;
                    self.show_workspace_wizard = true;
                }
                if let Some(ws_dir) = &self.project.workspace_dir {
                    ui.colored_label(
                        Color32::from_rgb(100, 230, 150),
                        format!("📁 Workspace: {}", ws_dir.display()),
                    );
                }
            });
        });

        self.render_workspace_wizard_window(ctx);

        if self.show_guide_window {
            let mut open = true;
            egui::Window::new("Guía Rápida de NesGameStudio")
                .open(&mut open)
                .default_width(620.0)
                .show(ctx, |ui| editors::guide::welcome_ui(ui));
            self.show_guide_window = open;
        }

        // BARRA DE ESTADO INFERIOR
        egui::TopBottomPanel::bottom("bottom_status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!(
                    "Proyecto: {} | Mapper: {} | PRG: {} KB | Escenas CHR: {} | Subrutinas: {}",
                    self.project.game_title,
                    self.project.mapper_id,
                    self.project.prg_rom.len() / 1024,
                    self.project.scenes.len(),
                    self.project.subroutines.len()
                ));
                ui.separator();
                ui.colored_label(Color32::from_rgb(130, 220, 255), &self.status_bar_msg);
            });
        });

        if self.show_about_modal {
            egui::Window::new("Acerca de NesGameStudio")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.heading("NesGameStudio — Fase 1");
                    ui.label("IDE Visual de Modding, Remasterización HD/2.5D y Creación de Juegos en C.");
                    ui.label("Integra el núcleo 6502 + PPU + APU de NesEmuRust para descomprimir gráficos CHR-RAM y reproducir música real.");
                    if ui.button("Cerrar").clicked() {
                        self.show_about_modal = false;
                    }
                });
        }

        // PANEL CENTRAL
        egui::CentralPanel::default().show(ctx, |ui| match self.active_tab {
            ActiveTab::GraphicsGallery => {
                self.graphics_editor.ui(ctx, ui, &mut self.project);
            }
            ActiveTab::AudioJukebox => {
                self.audio_editor.ui(ui, &mut self.project, &mut self.jukebox);
            }
            ActiveTab::SubroutinesInspector => {
                self.render_subroutines_tab(ui);
            }
        });
    }
}

impl NesGameStudioApp {
    fn render_workspace_wizard_window(&mut self, ctx: &egui::Context) {
        if !self.show_workspace_wizard {
            return;
        }
        let mut open = self.show_workspace_wizard;
        let mut should_close = false;
        egui::Window::new("🗂️ Generador de Workspace de Proyecto (Juego Nuevo o Existente)")
            .open(&mut open)
            .resizable(false)
            .default_width(580.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(
                        &mut self.wizard_is_existing_game,
                        false,
                        "✨ Crear Juego Nuevo desde Cero",
                    );
                    ui.selectable_value(
                        &mut self.wizard_is_existing_game,
                        true,
                        "🚀 Crear Workspace desde Juego Existente (.NES / .C)",
                    );
                });
                ui.separator();

                if !self.wizard_is_existing_game {
                    ui.label(
                        RichText::new("Configura tu nuevo videojuego 2D / HD-2D:")
                            .strong()
                            .color(Color32::from_rgb(130, 220, 255)),
                    );
                    ui.horizontal(|ui| {
                        ui.label("Nombre del Juego:");
                        ui.text_edit_singleline(&mut self.wizard_game_title);
                    });
                    ui.add_space(4.0);
                    ui.label("Selecciona una Plantilla Inicial (funcional a 60 FPS):");
                    for &tmpl in WorkspaceTemplate::all() {
                        ui.radio_value(&mut self.wizard_template, tmpl, tmpl.label_es());
                        ui.indent(tmpl.code(), |ui| {
                            ui.weak(tmpl.description_es());
                        });
                    }
                } else {
                    ui.label(
                        RichText::new("Importa una ROM .NES o código .C/.ASM a un Workspace aislado:")
                            .strong()
                            .color(Color32::from_rgb(130, 220, 255)),
                    );
                    ui.horizontal(|ui| {
                        let current_src = self
                            .wizard_existing_source
                            .as_ref()
                            .map(|p| p.display().to_string())
                            .unwrap_or_else(|| "(Ningún archivo seleccionado)".to_string());
                        ui.label(format!("Archivo base: {}", current_src));
                        if ui.button("📂 Elegir .NES o .C...").clicked() {
                            if let Some(p) = rfd::FileDialog::new()
                                .add_filter("Juego NES o Código C", &["nes", "c"])
                                .pick_file()
                            {
                                if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                                    self.wizard_game_title = stem.to_string();
                                }
                                self.wizard_existing_source = Some(p);
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Nombre del Workspace:");
                        ui.text_edit_singleline(&mut self.wizard_game_title);
                    });
                }

                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(format!(
                        "Carpeta raíz de Workspaces: {}",
                        self.wizard_target_parent_dir.display()
                    ));
                    if ui.button("📁 Cambiar carpeta...").clicked() {
                        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                            self.wizard_target_parent_dir = folder;
                        }
                    }
                });

                let preview_ws_dir = self.wizard_target_parent_dir.join(format!(
                    "{}_Workspace",
                    workspace::sanitize_slug(&self.wizard_game_title)
                ));
                ui.weak(format!(
                    "Se generará en: {} (con carpetas base/, src/, assets/, scenes/, scripts/, build/ y project.ngs.json)",
                    preview_ws_dir.display()
                ));

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui
                        .button(
                            RichText::new("🚀 Generar Workspace y Abrir en el Estudio")
                                .strong()
                                .color(Color32::from_rgb(100, 240, 150)),
                        )
                        .clicked()
                    {
                        self.jukebox.playing = false;
                        self.audio_editor = editors::audio::AudioEditorState::default();
                        let res = if !self.wizard_is_existing_game {
                            workspace::create_workspace_for_new_game(
                                &self.wizard_game_title,
                                self.wizard_template,
                                &preview_ws_dir,
                                &mut self.project,
                            )
                        } else if let Some(src) = self.wizard_existing_source.clone() {
                            workspace::create_workspace_from_existing_game(
                                &src,
                                &preview_ws_dir,
                                &mut self.project,
                            )
                        } else {
                            Err("Primero selecciona un archivo .NES o .C base.".to_string())
                        };

                        match res {
                            Ok(msg) => {
                                self.graphics_editor.invalidate_texture();
                                self.status_bar_msg = msg;
                                should_close = true;
                            }
                            Err(e) => {
                                self.status_bar_msg = e;
                            }
                        }
                    }
                    if ui.button("Cancelar").clicked() {
                        should_close = true;
                    }
                });
            });
        if should_close {
            open = false;
        }
        self.show_workspace_wizard = open;
    }

    fn render_subroutines_tab(&mut self, ui: &mut egui::Ui) {
        if !self.project.loaded {
            editors::guide::welcome_ui(ui);
            return;
        }
        editors::guide::tab_hint(
            ui,
            "Lista de rutinas del código del juego agrupadas por tipo. Despliega una para ver sus instrucciones.",
        );
        ui.heading("🔍 Explorador de Subrutinas y Vectores (.ASM <-> .C)");
        ui.horizontal(|ui| {
            ui.label(format!(
                "Vectores del Juego: NMI=${:04X} | RESET=${:04X} | IRQ=${:04X}",
                self.project.vec_nmi, self.project.vec_reset, self.project.vec_irq
            ));
            ui.separator();
            ui.label("Filtrar:");
            for cat in ["TODAS", "[AUDIO / APU]", "[VIDEO / PPU]", "[JOYPAD / INPUT]", "[GAME LOGIC]"] {
                if ui.selectable_label(self.subroutine_filter == cat, cat).clicked() {
                    self.subroutine_filter = cat.to_string();
                }
            }
        });
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            for sub in &self.project.subroutines {
                if self.subroutine_filter != "TODAS" && sub.category != self.subroutine_filter {
                    continue;
                }
                let header = format!(
                    "{} — {} (${:04X}) [Línea {}]",
                    sub.category, sub.name, sub.addr, sub.line_in_asm
                );
                ui.collapsing(header, |ui| {
                    for line in &sub.preview_asm {
                        ui.monospace(line);
                    }
                });
            }
        });
    }
}

fn main() -> eframe::Result<()> {
    #[cfg(target_os = "windows")]
    win32_show::ensure_on_default_desktop();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(WINDOW_TITLE)
            .with_inner_size([1220.0, 760.0])
            .with_visible(true)
            .with_active(true),
        ..Default::default()
    };
    eframe::run_native(
        WINDOW_TITLE,
        options,
        Box::new(|cc| {
            cc.egui_ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            cc.egui_ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            #[cfg(target_os = "windows")]
            win32_show::force_show_window_after_creation(WINDOW_TITLE);
            Ok(Box::new(NesGameStudioApp::default()))
        }),
    )
}
