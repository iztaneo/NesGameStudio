mod codegen;
mod editors;
mod parser;
mod project;
mod runtime;
#[cfg(target_os = "windows")]
mod win32_show;

use eframe::egui::{self, Color32};
use editors::audio::AudioEditorState;
use editors::graphics::GraphicsEditorState;
use project::NgsProject;
use runtime::jukebox_apu::JukeboxEngine;

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
    status_bar_msg: String,
}

impl Default for NesGameStudioApp {
    fn default() -> Self {
        // Inicia limpio, sin cargar ningún archivo automáticamente
        Self {
            project: NgsProject::default(),
            active_tab: ActiveTab::GraphicsGallery,
            graphics_editor: GraphicsEditorState::default(),
            audio_editor: AudioEditorState::default(),
            jukebox: JukeboxEngine::new(),
            subroutine_filter: "TODAS".to_string(),
            show_about_modal: false,
            show_guide_window: false,
            status_bar_msg: "Listo. Usa el menú Archivo -> Abrir Código .C / .ASM... para comenzar.".to_string(),
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
                // 1. MENÚ ARCHIVO
                ui.menu_button("Archivo", |ui| {
                    if ui.button("📄 Nuevo Proyecto en Blanco").clicked() {
                        self.jukebox.playing = false;
                        self.project.clear_to_new_project();
                        self.graphics_editor.invalidate_texture();
                        self.status_bar_msg = "Nuevo proyecto en blanco inicializado.".to_string();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("📂 Abrir Código .C / .ASM (de NesEmuRust)...").clicked() {
                        ui.close_menu();
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Código C de NesEmuRust", &["c"])
                            .pick_file()
                        {
                            self.jukebox.playing = false;
                            self.audio_editor = editors::audio::AudioEditorState::default();
                            match parser::load_c_and_asm_into_project(&path, &mut self.project) {
                                Ok(msg) => {
                                    self.graphics_editor.invalidate_texture();
                                    self.status_bar_msg = msg;
                                }
                                Err(e) => self.status_bar_msg = e,
                            }
                        }
                    }
                    if ui.button("🎮 Abrir ROM .NES Directamente...").clicked() {
                        ui.close_menu();
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("ROM de NES", &["nes"])
                            .pick_file()
                        {
                            self.jukebox.playing = false;
                            self.audio_editor = editors::audio::AudioEditorState::default();
                            match parser::load_nes_rom_into_project(&path, &mut self.project) {
                                Ok(msg) => {
                                    self.graphics_editor.invalidate_texture();
                                    self.status_bar_msg = msg;
                                }
                                Err(e) => self.status_bar_msg = e,
                            }
                        }
                    }
                    ui.separator();
                    if ui.button("💾 Guardar Proyecto (.ngs.json)...").clicked() {
                        ui.close_menu();
                        if let Some(path) = rfd::FileDialog::new()
                            .set_file_name(&format!("{}.ngs.json", self.project.game_title))
                            .add_filter("Proyecto NesGameStudio", &["json"])
                            .save_file()
                        {
                            match self.project.save_project_manifest(&path) {
                                Ok(_) => self.status_bar_msg = format!("Proyecto guardado en {}", path.display()),
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
                                self.project.nes_live.bus.apu.audio_samples.clear();
                            }
                            let n = self.project.scenes.len() + 1;
                            self.project
                                .capture_current_live_scene(format!("Escena {}: Captura en Vivo", n));
                            self.graphics_editor.invalidate_texture();
                            self.status_bar_msg = "Nueva escena de mapas de bits capturada.".to_string();
                        }
                    }
                    ui.separator();
                    if ui.button("🧹 Limpiar Tiles Editados").clicked() {
                        self.project.modified_tiles.clear();
                        self.status_bar_msg = "Se limpiaron las ediciones de tiles.".to_string();
                        ui.close_menu();
                    }
                    if ui.button("🧹 Limpiar Reemplazos de Audio (.MP3/.WAV)").clicked() {
                        self.project.audio_replacements.clear();
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

                // Indicador (no clicable) de la vista actual
                ui.separator();
                let current = match self.active_tab {
                    ActiveTab::GraphicsGallery => "Vista actual: 🎨 Gráficos",
                    ActiveTab::AudioJukebox => "Vista actual: 🎵 Rockola",
                    ActiveTab::SubroutinesInspector => "Vista actual: 🔍 Código",
                };
                ui.weak(current);
            });
        });

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
