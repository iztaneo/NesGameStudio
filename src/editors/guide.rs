use eframe::egui::{self, Color32, RichText};

/// Pantalla de bienvenida con pasos claros cuando no hay proyecto abierto.
pub fn welcome_ui(ui: &mut egui::Ui) {
    ui.vertical_centered(|ui| {
        ui.add_space(30.0);
        ui.heading(RichText::new("Bienvenido a NesGameStudio").size(26.0));
        ui.label("Explora, edita y remasteriza los gráficos y la música de un juego de NES.");
        ui.add_space(20.0);
    });

    egui::Frame::group(ui.style()).inner_margin(16.0).show(ui, |ui| {
        ui.label(RichText::new("¿Cómo empiezo?").strong().size(18.0));
        ui.add_space(6.0);
        step(ui, "1", "Abre cualquier juego de NES", "Menú  Archivo → Abrir ROM .NES...  (cualquier juego: Super Mario, Megaman, Castlevania, Contra, etc.)\n\
            o  Archivo → Abrir Código .C...  (archivo exportado desde NesEmuRust).");
        step(ui, "2", "Espera unos segundos", "El estudio ejecuta el juego por dentro para que él mismo \
            descomprima sus gráficos y arranque su música.");
        step(ui, "3", "Explora y Edita los Gráficos", "Menú  Vista → Gráficos. \
            Encontrarás todos los gráficos del juego y de todos los bancos. Puedes editarlos directamente como \
            'Personaje Completo (16×32)', bloques (16×16) o jefes (32×32) como si dibujaras en un lienzo continuo.");
        step(ui, "4", "Escucha Toda la Música y Efectos", "Menú  Vista → Rockola. \
            Tienes la lista completa de temas musicales y efectos de sonido para reproducirlos con 1 clic \
            o reemplazarlos por tus propios archivos .MP3/.WAV.");
        step(ui, "5", "Genera tu versión", "Menú  Compilar → Compilar y Ejecutar .EXE  para jugar con tus cambios.");
    });
}

fn step(ui: &mut egui::Ui, num: &str, title: &str, body: &str) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!(" {} ", num))
                .strong()
                .size(18.0)
                .color(Color32::BLACK)
                .background_color(Color32::from_rgb(90, 200, 255)),
        );
        ui.vertical(|ui| {
            ui.label(RichText::new(title).strong());
            ui.label(body);
        });
    });
    ui.add_space(8.0);
}

/// Franja de ayuda breve que aparece arriba de cada vista.
pub fn tab_hint(ui: &mut egui::Ui, text: &str) {
    egui::Frame::none()
        .fill(Color32::from_rgb(28, 40, 56))
        .inner_margin(8.0)
        .rounding(4.0)
        .show(ui, |ui| {
            ui.label(RichText::new(format!("💡 {}", text)).color(Color32::from_rgb(190, 225, 255)));
        });
    ui.add_space(6.0);
}
