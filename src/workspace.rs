use crate::codegen;
use crate::graphics::{
    bytes_to_hex, hex_to_bytes_16, ExtendedGraphicAsset, GraphicId, TilePatch,
};
use crate::parser;
use crate::project::{
    AudioEventKey, AudioReplacement, AudioTrigger, CapturedScene, NgsProject,
};
use std::fs;
use std::path::{Path, PathBuf};

/// Plantillas base para la creación de un Workspace de Juego Nuevo desde cero (Fase G0).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkspaceTemplate {
    PlatformerAction,
    TopDownAdventure,
    SpaceShmup,
    BlankExtended,
}

impl WorkspaceTemplate {
    pub fn all() -> &'static [WorkspaceTemplate] {
        &[
            WorkspaceTemplate::PlatformerAction,
            WorkspaceTemplate::TopDownAdventure,
            WorkspaceTemplate::SpaceShmup,
            WorkspaceTemplate::BlankExtended,
        ]
    }

    pub fn code(&self) -> &'static str {
        match self {
            WorkspaceTemplate::PlatformerAction => "platformer_action",
            WorkspaceTemplate::TopDownAdventure => "topdown_adventure",
            WorkspaceTemplate::SpaceShmup => "space_shmup",
            WorkspaceTemplate::BlankExtended => "blank_extended",
        }
    }

    #[allow(dead_code)]
    pub fn from_code(s: &str) -> Self {
        match s.trim() {
            "platformer_action" => WorkspaceTemplate::PlatformerAction,
            "topdown_adventure" => WorkspaceTemplate::TopDownAdventure,
            "space_shmup" => WorkspaceTemplate::SpaceShmup,
            _ => WorkspaceTemplate::BlankExtended,
        }
    }

    pub fn label_es(&self) -> &'static str {
        match self {
            WorkspaceTemplate::PlatformerAction => "🎮 Acción / Plataformas Lateral",
            WorkspaceTemplate::TopDownAdventure => "🗡️ Aventura Top-Down (Estilo RPG / Zelda)",
            WorkspaceTemplate::SpaceShmup => "🚀 Shoot 'em Up Espacial",
            WorkspaceTemplate::BlankExtended => "📄 Lienzo en Blanco (Memoria Extendida Ilimitada)",
        }
    }

    pub fn description_es(&self) -> &'static str {
        match self {
            WorkspaceTemplate::PlatformerAction => {
                "Incluye héroe 16x16, enemigo patrullero, plataforma con suelo y motor de audio activo."
            }
            WorkspaceTemplate::TopDownAdventure => {
                "Incluye aventurero 16x16 en vista superior, cofre/NPC, sala con bordes y música de exploración."
            }
            WorkspaceTemplate::SpaceShmup => {
                "Incluye nave espacial 16x16, nave enemiga, proyectiles, fondo estelar y efectos láser."
            }
            WorkspaceTemplate::BlankExtended => {
                "Escena limpia a 60 FPS con entidad base lista para pintar o reemplazar con gráficos HD."
            }
        }
    }
}

/// Rutas estándar dentro de un Workspace de NesGameStudio (`<Nombre>_Workspace/`).
#[derive(Clone, Debug)]
pub struct WorkspacePaths {
    pub root: PathBuf,
    pub manifest_file: PathBuf,
    pub base_dir: PathBuf,
    pub src_dir: PathBuf,
    pub tile_patches_dir: PathBuf,
    pub sprites_hd_dir: PathBuf,
    pub creations_dir: PathBuf,
    pub audio_bgm_dir: PathBuf,
    pub audio_sfx_dir: PathBuf,
    pub scenes_dir: PathBuf,
    pub scripts_dir: PathBuf,
    pub build_dir: PathBuf,
}

impl WorkspacePaths {
    pub fn from_root(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            manifest_file: root.join("project.ngs.json"),
            base_dir: root.join("base"),
            src_dir: root.join("src"),
            tile_patches_dir: root.join("assets").join("graphics").join("tile_patches"),
            sprites_hd_dir: root.join("assets").join("graphics").join("sprites_hd"),
            creations_dir: root.join("assets").join("graphics").join("creations"),
            audio_bgm_dir: root.join("assets").join("audio").join("bgm"),
            audio_sfx_dir: root.join("assets").join("audio").join("sfx"),
            scenes_dir: root.join("scenes"),
            scripts_dir: root.join("scripts"),
            build_dir: root.join("build"),
        }
    }

    pub fn ensure_directories(&self) -> Result<(), String> {
        let dirs = [
            &self.root,
            &self.base_dir,
            &self.src_dir,
            &self.tile_patches_dir,
            &self.sprites_hd_dir,
            &self.creations_dir,
            &self.audio_bgm_dir,
            &self.audio_sfx_dir,
            &self.scenes_dir,
            &self.scripts_dir,
            &self.build_dir,
        ];
        for d in dirs {
            fs::create_dir_all(d).map_err(|e| {
                format!(
                    "No se pudo crear el directorio del workspace '{}': {}",
                    d.display(),
                    e
                )
            })?;
        }
        Ok(())
    }
}

pub fn sanitize_slug(name: &str) -> String {
    let mut s = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            s.push(ch);
        } else if (ch == ' ' || ch == '_' || ch == '-') && !s.ends_with('_') {
            s.push('_');
        }
    }
    let trimmed = s.trim_matches('_');
    if trimmed.is_empty() {
        "Proyecto_NGS".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Crea un Workspace estructurado en disco a partir de un juego existente (`.nes` o `.c`/`.asm`).
pub fn create_workspace_from_existing_game(
    source_file: &Path,
    workspace_root: &Path,
    project: &mut NgsProject,
) -> Result<String, String> {
    if !source_file.exists() {
        return Err(format!(
            "El archivo fuente '{}' no existe.",
            source_file.display()
        ));
    }

    let paths = WorkspacePaths::from_root(workspace_root);
    paths.ensure_directories()?;

    let ext = source_file
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let file_name = source_file
        .file_name()
        .ok_or_else(|| "Nombre de archivo fuente inválido.".to_string())?;
    let copied_base_path = paths.base_dir.join(file_name);

    if source_file != copied_base_path {
        fs::copy(source_file, &copied_base_path).map_err(|e| {
            format!(
                "Error copiando archivo base a '{}': {}",
                copied_base_path.display(),
                e
            )
        })?;
    }

    // Si es un archivo .c, también copiar su .asm compañero si existe
    if ext == "c" {
        let asm_candidate = source_file.with_extension("asm");
        if asm_candidate.exists() {
            if let Some(asm_name) = asm_candidate.file_name() {
                let copied_asm = paths.base_dir.join(asm_name);
                if asm_candidate != copied_asm {
                    let _ = fs::copy(&asm_candidate, &copied_asm);
                }
            }
        }
    }

    let load_msg = if ext == "nes" {
        parser::load_nes_rom_into_project(&copied_base_path, project)?
    } else if ext == "c" {
        parser::load_c_and_asm_into_project(&copied_base_path, project)?
    } else {
        return Err(format!(
            "Extensión '{}' no soportada. Selecciona una ROM .NES o un archivo .C exportado por NesEmuRust.",
            ext
        ));
    };

    let slug = sanitize_slug(&project.game_title);
    let generated_c_path = paths.src_dir.join(format!("{}.c", slug));
    project.workspace_dir = Some(paths.root.clone());
    project.workspace_template = Some("existing_game".to_string());
    project.base_source_rel_path = Some(format!(
        "base/{}",
        file_name.to_string_lossy()
    ));
    project.c_file_path = Some(generated_c_path.clone());

    // Generar el archivo .c en src/ dentro del workspace para que siempre esté listo para compilar
    if let Ok(written_c) = codegen::generate_modified_c_file(project) {
        project.c_file_path = Some(written_c);
    }

    save_workspace(project, &paths.root)?;

    Ok(format!(
        "Workspace creado en '{}' ({})",
        paths.root.display(),
        load_msg
    ))
}

/// Crea un Workspace completo en disco para un Juego Nuevo desde cero con la plantilla seleccionada.
pub fn create_workspace_for_new_game(
    game_title: &str,
    template: WorkspaceTemplate,
    workspace_root: &Path,
    project: &mut NgsProject,
) -> Result<String, String> {
    let paths = WorkspacePaths::from_root(workspace_root);
    paths.ensure_directories()?;

    let clean_title = if game_title.trim().is_empty() {
        "Nuevo_Juego_NGS".to_string()
    } else {
        game_title.trim().to_string()
    };
    let slug = sanitize_slug(&clean_title);

    // Sintetizar una ROM base ejecutable a 60 FPS para la plantilla elegida
    let rom_bytes = synthesize_template_nes_rom(template);
    let base_rom_path = paths.base_dir.join(format!("{}.nes", slug));
    fs::write(&base_rom_path, &rom_bytes).map_err(|e| {
        format!(
            "No se pudo escribir la base del juego nuevo en '{}': {}",
            base_rom_path.display(),
            e
        )
    })?;

    parser::load_nes_rom_into_project(&base_rom_path, project)?;
    project.game_title = clean_title.clone();
    project.workspace_dir = Some(paths.root.clone());
    project.workspace_template = Some(template.code().to_string());
    project.base_source_rel_path = Some(format!("base/{}.nes", slug));

    let generated_c_path = paths.src_dir.join(format!("{}.c", slug));
    project.c_file_path = Some(generated_c_path);
    if let Ok(written_c) = codegen::generate_modified_c_file(project) {
        project.c_file_path = Some(written_c);
    }

    // Crear un script inicial de ejemplo en scripts/
    let starter_script = format!(
        "// Script inicial de comportamiento para: {}\n// Plantilla: {}\n// Puedes editar los personajes y escenarios directamente desde el Explorador Gráfico Visual.\n",
        clean_title,
        template.label_es()
    );
    let _ = fs::write(paths.scripts_dir.join("main_behavior.ngscript"), starter_script);

    save_workspace(project, &paths.root)?;

    Ok(format!(
        "Workspace de nuevo juego '{}' ({}) creado en '{}'.",
        clean_title,
        template.label_es(),
        paths.root.display()
    ))
}

/// Guarda el estado íntegro del proyecto en su carpeta de Workspace (`project.ngs.json` + parches + escenas).
pub fn save_workspace(project: &NgsProject, workspace_root: &Path) -> Result<String, String> {
    let paths = WorkspacePaths::from_root(workspace_root);
    paths.ensure_directories()?;

    // 1. Guardar parches no destructivos de tiles (`assets/graphics/tile_patches/patches.tsv`)
    let mut patches_lines = Vec::new();
    let mut sorted_keys: Vec<_> = project.tile_patches.keys().collect();
    sorted_keys.sort_by_key(|k| k.short_key());
    for key in sorted_keys {
        if let Some(patch) = project.tile_patches.get(key) {
            let offsets_str: Vec<String> = patch
                .auto_prg_offsets
                .iter()
                .map(|o| o.to_string())
                .collect();
            patches_lines.push(format!(
                "{}\t{}\t{}\t{}",
                patch.id.short_key(),
                bytes_to_hex(&patch.original_bytes),
                bytes_to_hex(&patch.modified_bytes),
                offsets_str.join(",")
            ));
        }
    }
    fs::write(
        paths.tile_patches_dir.join("patches.tsv"),
        patches_lines.join("\n"),
    )
    .map_err(|e| format!("Error guardando parches de tiles: {}", e))?;

    // 2. Guardar activos del Banco de Memoria Extendida Ilimitada (`assets/graphics/creations/extended_assets.tsv`)
    let mut ext_lines = Vec::new();
    for asset in &project.extended_assets {
        let raw_file_name = format!("ext_asset_{}.rgba", asset.asset_id);
        let raw_path = paths.creations_dir.join(&raw_file_name);
        let mut rgba_bytes = Vec::with_capacity(asset.rgba_pixels.len() * 4);
        for &px in &asset.rgba_pixels {
            rgba_bytes.extend_from_slice(&px.to_le_bytes());
        }
        let _ = fs::write(&raw_path, &rgba_bytes);
        let target_str = asset
            .target_graphic_id
            .as_ref()
            .map(|id| id.short_key())
            .unwrap_or_else(|| "-".to_string());
        let src_str = asset.source_path.as_deref().unwrap_or("-");
        ext_lines.push(format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}",
            asset.asset_id,
            asset.name.replace('\t', " "),
            target_str,
            asset.width,
            asset.height,
            raw_file_name,
            src_str.replace('\t', " ")
        ));
    }
    fs::write(
        paths.creations_dir.join("extended_assets.tsv"),
        ext_lines.join("\n"),
    )
    .map_err(|e| format!("Error guardando activos de memoria extendida: {}", e))?;

    // 3. Guardar nombres personalizados de entidades (`assets/graphics/entity_names.tsv`)
    let mut ent_lines = Vec::new();
    for (k, v) in &project.entity_custom_names {
        ent_lines.push(format!("{}\t{}", k.replace('\t', " "), v.replace('\t', " ")));
    }
    let _ = fs::write(
        paths.creations_dir.join("entity_names.tsv"),
        ent_lines.join("\n"),
    );

    // 4. Guardar reemplazos de audio (`assets/audio/audio_replacements.tsv`)
    let mut audio_lines = Vec::new();
    for (key, rep) in &project.audio_event_replacements {
        let trig_str = encode_audio_trigger(&key.trigger);
        let bank_str = key
            .bank_context
            .map(|b| b.to_string())
            .unwrap_or_else(|| "-".to_string());
        audio_lines.push(format!(
            "{}\t{}\t{}\t{}\t{}\t{}",
            trig_str,
            key.command_id,
            bank_str,
            rep.label.replace('\t', " "),
            rep.external_file_path.replace('\t', " "),
            if rep.is_sfx_wav { "1" } else { "0" }
        ));
    }
    let _ = fs::write(
        paths.audio_bgm_dir.join("audio_replacements.tsv"),
        audio_lines.join("\n"),
    );

    // 5. Guardar escenas capturadas (`scenes/scene_XX.bin` + `scenes/index.tsv`)
    let mut scene_index = Vec::new();
    for (idx, sc) in project.scenes.iter().enumerate() {
        let bin_name = format!("scene_{:02}.bin", idx);
        let bin_path = paths.scenes_dir.join(&bin_name);
        if let Ok(bytes) = serialize_captured_scene(sc) {
            let _ = fs::write(&bin_path, bytes);
            scene_index.push(format!("{}\t{}", bin_name, sc.title.replace('\t', " ")));
        }
    }
    let _ = fs::write(paths.scenes_dir.join("index.tsv"), scene_index.join("\n"));

    // 6. Escribir el manifiesto principal `project.ngs.json`
    let base_rel = project
        .base_source_rel_path
        .clone()
        .unwrap_or_else(|| "".to_string());
    let tmpl = project
        .workspace_template
        .clone()
        .unwrap_or_else(|| "existing_game".to_string());

    let mut json_lines = Vec::new();
    json_lines.push("{".to_string());
    json_lines.push("  \"ngs_workspace_version\": \"2.2\",".to_string());
    json_lines.push(format!(
        "  \"game_title\": \"{}\",",
        escape_json(&project.game_title)
    ));
    json_lines.push(format!(
        "  \"workspace_template\": \"{}\",",
        escape_json(&tmpl)
    ));
    json_lines.push(format!(
        "  \"base_source_rel_path\": \"{}\",",
        escape_json(&base_rel)
    ));
    json_lines.push(format!("  \"mapper_id\": {},", project.mapper_id));
    json_lines.push(format!("  \"mirroring_mode\": {},", project.mirroring_mode));
    json_lines.push(format!("  \"is_chr_ram\": {},", project.is_chr_ram));
    json_lines.push(format!(
        "  \"widescreen_16_9\": {},",
        project.widescreen_16_9
    ));
    json_lines.push(format!(
        "  \"selected_palette\": {},",
        project.selected_palette
    ));
    json_lines.push(format!(
        "  \"active_scene_idx\": {},",
        project.active_scene_idx
    ));
    json_lines.push(format!("  \"scenes_captured\": {},", project.scenes.len()));
    json_lines.push(format!(
        "  \"tile_patches_count\": {},",
        project.tile_patches.len()
    ));
    json_lines.push(format!(
        "  \"extended_assets_count\": {},",
        project.extended_assets.len()
    ));
    json_lines.push(format!(
        "  \"audio_triggers_count\": {},",
        project.audio_triggers.len()
    ));
    json_lines.push(format!(
        "  \"audio_replacements_count\": {}",
        project.audio_event_replacements.len()
    ));
    json_lines.push("}".to_string());

    fs::write(&paths.manifest_file, json_lines.join("\n"))
        .map_err(|e| format!("Error guardando '{}': {}", paths.manifest_file.display(), e))?;

    // También actualizar el código C generado en `src/` si existe
    if !project.raw_c_source.is_empty() {
        let _ = codegen::generate_modified_c_file(project);
    }

    Ok(format!(
        "Workspace sincronizado en '{}'",
        paths.manifest_file.display()
    ))
}

/// Abre y restaura un Workspace existente desde su archivo `project.ngs.json` o carpeta raíz.
pub fn load_workspace(manifest_or_dir: &Path, project: &mut NgsProject) -> Result<String, String> {
    let workspace_root = if manifest_or_dir.is_dir() {
        manifest_or_dir.to_path_buf()
    } else {
        manifest_or_dir
            .parent()
            .ok_or_else(|| "Ruta de manifiesto de workspace inválida.".to_string())?
            .to_path_buf()
    };

    let paths = WorkspacePaths::from_root(&workspace_root);
    if !paths.manifest_file.exists() {
        return Err(format!(
            "No se encontró 'project.ngs.json' en '{}'.",
            workspace_root.display()
        ));
    }

    let manifest_text = fs::read_to_string(&paths.manifest_file)
        .map_err(|e| format!("Error leyendo 'project.ngs.json': {}", e))?;

    let saved_title = extract_json_string(&manifest_text, "game_title")
        .unwrap_or_else(|| "Proyecto_NGS".to_string());
    let saved_template = extract_json_string(&manifest_text, "workspace_template")
        .unwrap_or_else(|| "existing_game".to_string());
    let base_rel = extract_json_string(&manifest_text, "base_source_rel_path").unwrap_or_default();
    let widescreen = extract_json_bool(&manifest_text, "widescreen_16_9").unwrap_or(false);
    let selected_palette = extract_json_usize(&manifest_text, "selected_palette").unwrap_or(1);
    let active_scene_idx = extract_json_usize(&manifest_text, "active_scene_idx").unwrap_or(0);

    // 1. Cargar la ROM o código base desde `base/`
    if !base_rel.is_empty() {
        let base_full = workspace_root.join(&base_rel);
        if base_full.exists() {
            let ext = base_full
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if ext == "nes" {
                parser::load_nes_rom_into_project(&base_full, project)?;
            } else if ext == "c" {
                parser::load_c_and_asm_into_project(&base_full, project)?;
            }
        }
    } else {
        project.clear_to_new_project();
    }

    project.game_title = saved_title.clone();
    project.workspace_dir = Some(workspace_root.clone());
    project.workspace_template = Some(saved_template);
    project.base_source_rel_path = if base_rel.is_empty() {
        None
    } else {
        Some(base_rel)
    };
    project.widescreen_16_9 = widescreen;
    project.selected_palette = selected_palette.min(7);

    let slug = sanitize_slug(&project.game_title);
    project.c_file_path = Some(paths.src_dir.join(format!("{}.c", slug)));

    // 2. Restaurar escenas capturadas si existen en `scenes/index.tsv`
    let scene_index_path = paths.scenes_dir.join("index.tsv");
    if scene_index_path.exists() {
        if let Ok(index_str) = fs::read_to_string(&scene_index_path) {
            let mut loaded_scenes = Vec::new();
            for line in index_str.lines() {
                if line.trim().is_empty() {
                    continue;
                }
                let mut parts = line.splitn(2, '\t');
                let bin_name = parts.next().unwrap_or("");
                let title = parts.next().unwrap_or("Escena");
                let bin_path = paths.scenes_dir.join(bin_name);
                if let Ok(bytes) = fs::read(&bin_path) {
                    if let Some(mut sc) = deserialize_captured_scene(&bytes) {
                        sc.title = title.to_string();
                        loaded_scenes.push(sc);
                    }
                }
            }
            if !loaded_scenes.is_empty() {
                project.scenes = loaded_scenes;
            }
        }
    }
    if !project.scenes.is_empty() {
        project.active_scene_idx = active_scene_idx.min(project.scenes.len() - 1);
    }

    // 3. Restaurar parches no destructivos de tiles (`patches.tsv`)
    project.tile_patches.clear();
    project.modified_tiles.clear();
    let patches_path = paths.tile_patches_dir.join("patches.tsv");
    if patches_path.exists() {
        if let Ok(patches_str) = fs::read_to_string(&patches_path) {
            for line in patches_str.lines() {
                if line.trim().is_empty() {
                    continue;
                }
                let cols: Vec<&str> = line.split('\t').collect();
                if cols.len() >= 3 {
                    if let (Some(id), Some(orig), Some(modif)) = (
                        GraphicId::from_short_key(cols[0]),
                        hex_to_bytes_16(cols[1]),
                        hex_to_bytes_16(cols[2]),
                    ) {
                        let mut auto_prg_offsets = Vec::new();
                        if cols.len() >= 4 && !cols[3].trim().is_empty() {
                            for p in cols[3].split(',') {
                                if let Ok(off) = p.trim().parse::<usize>() {
                                    auto_prg_offsets.push(off);
                                }
                            }
                        }
                        project.tile_patches.insert(
                            id.clone(),
                            TilePatch {
                                id: id.clone(),
                                original_bytes: orig,
                                modified_bytes: modif,
                                auto_prg_offsets,
                            },
                        );
                        project.modified_tiles.insert(id.tile_index as usize, modif);
                    }
                }
            }
        }
    }

    // 4. Restaurar activos de Memoria Extendida Ilimitada (`extended_assets.tsv`)
    project.extended_assets.clear();
    let ext_path = paths.creations_dir.join("extended_assets.tsv");
    if ext_path.exists() {
        if let Ok(ext_str) = fs::read_to_string(&ext_path) {
            for line in ext_str.lines() {
                if line.trim().is_empty() {
                    continue;
                }
                let cols: Vec<&str> = line.split('\t').collect();
                if cols.len() >= 6 {
                    let asset_id = cols[0].parse::<usize>().unwrap_or(0);
                    let name = cols[1].to_string();
                    let target_graphic_id = if cols[2] == "-" {
                        None
                    } else {
                        GraphicId::from_short_key(cols[2])
                    };
                    let width = cols[3].parse::<u32>().unwrap_or(16);
                    let height = cols[4].parse::<u32>().unwrap_or(16);
                    let raw_file = paths.creations_dir.join(cols[5]);
                    let source_path = if cols.len() >= 7 && cols[6] != "-" {
                        Some(cols[6].to_string())
                    } else {
                        None
                    };
                    let mut rgba_pixels = Vec::new();
                    if let Ok(bytes) = fs::read(&raw_file) {
                        for chunk in bytes.chunks_exact(4) {
                            rgba_pixels.push(u32::from_le_bytes([
                                chunk[0], chunk[1], chunk[2], chunk[3],
                            ]));
                        }
                    }
                    project.extended_assets.push(ExtendedGraphicAsset {
                        asset_id,
                        name,
                        target_graphic_id,
                        width,
                        height,
                        rgba_pixels,
                        source_path,
                    });
                }
            }
        }
    }

    // 5. Restaurar nombres personalizados de entidades
    project.entity_custom_names.clear();
    let ent_path = paths.creations_dir.join("entity_names.tsv");
    if ent_path.exists() {
        if let Ok(ent_str) = fs::read_to_string(&ent_path) {
            for line in ent_str.lines() {
                if let Some((k, v)) = line.split_once('\t') {
                    project
                        .entity_custom_names
                        .insert(k.to_string(), v.to_string());
                }
            }
        }
    }

    // 6. Restaurar reemplazos de audio
    project.audio_event_replacements.clear();
    project.audio_replacements.clear();
    let audio_path = paths.audio_bgm_dir.join("audio_replacements.tsv");
    if audio_path.exists() {
        if let Ok(aud_str) = fs::read_to_string(&audio_path) {
            for line in aud_str.lines() {
                let cols: Vec<&str> = line.split('\t').collect();
                if cols.len() >= 6 {
                    if let (Some(trigger), Ok(cmd_id)) =
                        (decode_audio_trigger(cols[0]), cols[1].parse::<u8>())
                    {
                        let bank_context = if cols[2] == "-" {
                            None
                        } else {
                            cols[2].parse::<u16>().ok()
                        };
                        let key = AudioEventKey {
                            trigger,
                            command_id: cmd_id,
                            bank_context,
                        };
                        let rep = AudioReplacement {
                            event_key: Some(key.clone()),
                            track_id: cmd_id,
                            label: cols[3].to_string(),
                            external_file_path: cols[4].to_string(),
                            is_sfx_wav: cols[5] == "1",
                        };
                        project.set_event_replacement(key, rep);
                    }
                }
            }
        }
    }

    // Reconstruir el catálogo gráfico aplicando todos los parches cargados
    project.rebuild_global_tile_catalog();
    project.apply_all_patches_to_memory();

    Ok(format!(
        "Workspace '{}' cargado correctamente ({} parches gráficos, {} escenas, {} activos HD).",
        project.game_title,
        project.tile_patches.len(),
        project.scenes.len(),
        project.extended_assets.len()
    ))
}

/// Sintetiza una ROM iNES válida y ejecutable (16 KB PRG-ROM + 8 KB CHR-ROM)
/// con personajes en OAM, escenario en Nametable y motor de sonido en `$8100`.
pub fn synthesize_template_nes_rom(template: WorkspaceTemplate) -> Vec<u8> {
    let mut rom = Vec::with_capacity(16 + 16384 + 8192);
    // Cabecera iNES: 1 banco PRG (16KB), 1 banco CHR (8KB), Mapper 0, Horizontal Mirroring
    rom.extend_from_slice(&[
        b'N', b'E', b'S', 0x1A, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ]);

    let mut prg = vec![0xEAu8; 16384]; // NOP fill
    let mut chr = vec![0x00u8; 8192];

    // Poblar gráficos en CHR ($0000 = Sprites, $1000 = Fondo)
    populate_template_chr(&mut chr, template);

    // Paleta de 32 bytes en PRG $8300
    let palette: [u8; 32] = match template {
        WorkspaceTemplate::PlatformerAction => [
            0x21, 0x16, 0x27, 0x30, 0x21, 0x19, 0x29, 0x39, 0x21, 0x01, 0x11, 0x31, 0x21, 0x06,
            0x16, 0x26, // BG
            0x21, 0x16, 0x27, 0x30, 0x21, 0x14, 0x24, 0x34, 0x21, 0x19, 0x2A, 0x3A, 0x21, 0x02,
            0x12, 0x32, // Sprites
        ],
        WorkspaceTemplate::TopDownAdventure => [
            0x0F, 0x19, 0x29, 0x38, 0x0F, 0x17, 0x27, 0x37, 0x0F, 0x12, 0x22, 0x32, 0x0F, 0x00,
            0x10, 0x30, // BG
            0x0F, 0x19, 0x27, 0x36, 0x0F, 0x16, 0x26, 0x30, 0x0F, 0x12, 0x22, 0x30, 0x0F, 0x14,
            0x24, 0x34, // Sprites
        ],
        WorkspaceTemplate::SpaceShmup => [
            0x0F, 0x01, 0x11, 0x30, 0x0F, 0x02, 0x12, 0x32, 0x0F, 0x06, 0x16, 0x36, 0x0F, 0x00,
            0x10, 0x20, // BG
            0x0F, 0x11, 0x21, 0x30, 0x0F, 0x16, 0x27, 0x38, 0x0F, 0x1A, 0x2A, 0x30, 0x0F, 0x14,
            0x24, 0x34, // Sprites
        ],
        WorkspaceTemplate::BlankExtended => [
            0x0F, 0x00, 0x10, 0x30, 0x0F, 0x12, 0x22, 0x32, 0x0F, 0x16, 0x26, 0x36, 0x0F, 0x19,
            0x29, 0x39, // BG
            0x0F, 0x16, 0x27, 0x30, 0x0F, 0x12, 0x22, 0x30, 0x0F, 0x19, 0x29, 0x30, 0x0F, 0x14,
            0x24, 0x30, // Sprites
        ],
    };
    prg[0x0300..0x0320].copy_from_slice(&palette);

    // Tabla OAM inicial en PRG $8320 (3 Entidades: Héroe 16x16, Enemigo/NPC 16x16, Objeto 8x8)
    // Cada entrada OAM: [Y, Tile, Attr, X]
    let oam_init: [u8; 36] = [
        // Entidad 1: Héroe Principal 16x16 en (X=96, Y=144), Paleta 0
        144, 0x00, 0x00, 96,  // Top-Left
        144, 0x01, 0x00, 104, // Top-Right
        152, 0x02, 0x00, 96,  // Bottom-Left
        152, 0x03, 0x00, 104, // Bottom-Right
        // Entidad 2: Rival / NPC 16x16 en (X=164, Y=144), Paleta 1
        144, 0x04, 0x01, 164, // Top-Left
        144, 0x05, 0x01, 172, // Top-Right
        152, 0x06, 0x01, 164, // Bottom-Left
        152, 0x07, 0x01, 172, // Bottom-Right
        // Entidad 3: Ítem / Cristal 8x8 en (X=132, Y=112), Paleta 2
        112, 0x08, 0x02, 132,
    ];
    prg[0x0320..0x0320 + oam_init.len()].copy_from_slice(&oam_init);

    // Subrutina de sonido en $8100 (offset 0x0100): acepta comando en A o RAM $00F0
    // Activa canales APU ($4015) y toca notas distintas según el ID del comando
    let sound_driver: &[u8] = &[
        0x85, 0xF0, // STA $F0
        0xA9, 0x0F, // LDA #$0F
        0x8D, 0x15, 0x40, // STA $4015
        0xA5, 0xF0, // LDA $F0
        0xF0, 0x1E, // BEQ +30 (RTS)
        0xC9, 0x04, // CMP #$04
        0xB0, 0x0F, // BCS +15 (SFX)
        // BGM Note (Pulse 1 + Triangle)
        0xA9, 0xBF, 0x8D, 0x00, 0x40, // LDA #$BF; STA $4000
        0x0A, 0x0A, 0x0A, // ASL A x3
        0x8D, 0x02, 0x40, // STA $4002
        0xA9, 0xF8, 0x8D, 0x03, 0x40, // LDA #$F8; STA $4003
        0x60, // RTS
        // SFX Note (Pulse 2 sweep)
        0xA9, 0x9F, 0x8D, 0x04, 0x40, // LDA #$9F; STA $4004
        0xA9, 0x89, 0x8D, 0x05, 0x40, // LDA #$89; STA $4005
        0xA5, 0xF0, 0x0A, 0x0A, 0x8D, 0x06, 0x40, // LDA $F0; ASL; ASL; STA $4006
        0xA9, 0x18, 0x8D, 0x07, 0x40, // LDA #$18; STA $4007
        0x60, // RTS
    ];
    prg[0x0100..0x0100 + sound_driver.len()].copy_from_slice(sound_driver);

    // Rutina RESET en $8000 (offset 0x0000)
    let reset_code: &[u8] = &[
        0x78, // SEI
        0xD8, // CLD
        0xA2, 0xFF, 0x9A, // LDX #$FF; TXS
        // Cargar paleta en PPU $3F00..$3F1F
        0xAD, 0x02, 0x20, // LDA $2002
        0xA9, 0x3F, 0x8D, 0x06, 0x20, // LDA #$3F; STA $2006
        0xA9, 0x00, 0x8D, 0x06, 0x20, // LDA #$00; STA $2006
        0xA2, 0x00, // LDX #$00
        // Loop paleta:
        0xBD, 0x00, 0x83, // LDA $8300,X
        0x8D, 0x07, 0x20, // STA $2007
        0xE8, // INX
        0xE0, 0x20, // CPX #$20
        0xD0, 0xF5, // BNE -11
        // Llenar fila de suelo en Nametable $2300 (64 bloques con tile $01)
        0xA9, 0x23, 0x8D, 0x06, 0x20,
        0xA9, 0x00, 0x8D, 0x06, 0x20,
        0xA2, 0x40,
        0xA9, 0x01,
        0x8D, 0x07, 0x20,
        0xCA,
        0xD0, 0xFA,
        // Inicializar OAM RAM $0200..$02FF con $FF (oculto)
        0xA2, 0x00,
        0xA9, 0xFF,
        0x9D, 0x00, 0x02,
        0xE8,
        0xD0, 0xFA,
        // Copiar las 3 entidades iniciales ($8320..$8344) a $0200
        0xA2, 0x00,
        0xBD, 0x20, 0x83,
        0x9D, 0x00, 0x02,
        0xE8,
        0xE0, 0x24,
        0xD0, 0xF5,
        // Transferir OAM DMA ($4014 = $02)
        0xA9, 0x02, 0x8D, 0x14, 0x40,
        // Tocar tema musical inicial (#$01)
        0xA9, 0x01,
        0x20, 0x00, 0x81, // JSR $8100
        // Activar NMI y renderizado (PPUCTRL = $90 -> BG en $1000, Sprites en $0000; PPUMASK = $1E)
        0xA9, 0x90, 0x8D, 0x00, 0x20,
        0xA9, 0x1E, 0x8D, 0x01, 0x20,
        // Bucle infinito principal ($806A)
        0x4C, 0x6A, 0x80,
    ];
    prg[0x0000..reset_code.len()].copy_from_slice(reset_code);

    // Rutina NMI en $8200 (offset 0x0200): mantiene OAM DMA y procesa cola de sonido en $00F0
    let nmi_code: &[u8] = &[
        0x48, 0x8A, 0x48, // PHA; TXA; PHA
        0xA9, 0x02, 0x8D, 0x14, 0x40, // OAM DMA
        0xA5, 0xF0, // LDA $F0
        0xF0, 0x07, // BEQ +7
        0x20, 0x00, 0x81, // JSR $8100
        0xA9, 0x00, 0x85, 0xF0, // LDA #$00; STA $F0
        0x68, 0xAA, 0x68, // PLA; TAX; PLA
        0x40, // RTI
    ];
    prg[0x0200..0x0200 + nmi_code.len()].copy_from_slice(nmi_code);

    // Vectores de interrupción en $FFFA..$FFFF
    prg[0x3FFA] = 0x00;
    prg[0x3FFB] = 0x82; // NMI = $8200
    prg[0x3FFC] = 0x00;
    prg[0x3FFD] = 0x80; // RESET = $8000
    prg[0x3FFE] = 0x00;
    prg[0x3FFF] = 0x82; // IRQ = $8200

    rom.extend_from_slice(&prg);
    rom.extend_from_slice(&chr);
    rom
}

fn populate_template_chr(chr: &mut [u8], _template: WorkspaceTemplate) {
    // Sprites $00..$03: Héroe 16x16 (4 tiles con silueta clara y ojos/armadura)
    let hero_tl: [u8; 16] = [
        0x0F, 0x1F, 0x3F, 0x3F, 0x3F, 0x1F, 0x0F, 0x1F, 0x00, 0x0F, 0x19, 0x19, 0x1F, 0x0F, 0x00,
        0x0F,
    ];
    let hero_tr: [u8; 16] = [
        0xF0, 0xF8, 0xFC, 0xFC, 0xFC, 0xF8, 0xF0, 0xF8, 0x00, 0xF0, 0x98, 0x98, 0xF8, 0xF0, 0x00,
        0xF0,
    ];
    let hero_bl: [u8; 16] = [
        0x3F, 0x3F, 0x3F, 0x1F, 0x1F, 0x1C, 0x3C, 0x3C, 0x1F, 0x1F, 0x0F, 0x0F, 0x00, 0x1C, 0x3C,
        0x00,
    ];
    let hero_br: [u8; 16] = [
        0xFC, 0xFC, 0xFC, 0xF8, 0xF8, 0x38, 0x3C, 0x3C, 0xF8, 0xF8, 0xF0, 0xF0, 0x00, 0x38, 0x3C,
        0x00,
    ];
    chr[0..16].copy_from_slice(&hero_tl);
    chr[16..32].copy_from_slice(&hero_tr);
    chr[32..48].copy_from_slice(&hero_bl);
    chr[48..64].copy_from_slice(&hero_br);

    // Sprites $04..$07: Enemigo / Rival 16x16
    let foe_tl: [u8; 16] = [
        0x07, 0x1F, 0x3F, 0x7F, 0x79, 0x7F, 0x3F, 0x1F, 0x00, 0x07, 0x1F, 0x39, 0x3F, 0x39, 0x1F,
        0x07,
    ];
    let foe_tr: [u8; 16] = [
        0xE0, 0xF8, 0xFC, 0xFE, 0x9E, 0xFE, 0xFC, 0xF8, 0x00, 0xE0, 0xF8, 0x9C, 0xFC, 0x9C, 0xF8,
        0xE0,
    ];
    let foe_bl: [u8; 16] = [
        0x1F, 0x3F, 0x33, 0x33, 0x1E, 0x18, 0x30, 0x30, 0x0F, 0x1F, 0x00, 0x00, 0x1E, 0x18, 0x00,
        0x00,
    ];
    let foe_br: [u8; 16] = [
        0xF8, 0xFC, 0xCC, 0xCC, 0x78, 0x18, 0x0C, 0x0C, 0xF0, 0xF8, 0x00, 0x00, 0x78, 0x18, 0x00,
        0x00,
    ];
    chr[64..80].copy_from_slice(&foe_tl);
    chr[80..96].copy_from_slice(&foe_tr);
    chr[96..112].copy_from_slice(&foe_bl);
    chr[112..128].copy_from_slice(&foe_br);

    // Sprite $08: Cristal / Ítem 8x8
    let item_tile: [u8; 16] = [
        0x18, 0x3C, 0x7E, 0xFF, 0xFF, 0x7E, 0x3C, 0x18, 0x00, 0x18, 0x3C, 0x7E, 0x7E, 0x3C, 0x18,
        0x00,
    ];
    chr[128..144].copy_from_slice(&item_tile);

    // Fondo Pattern Table 1 ($1000 -> byte 4096): Tile #1 = Bloque de suelo/ladrillo
    let ground_tile: [u8; 16] = [
        0xFF, 0x81, 0x81, 0xFF, 0xFF, 0x18, 0x18, 0xFF, 0xFF, 0x7E, 0x7E, 0xFF, 0xFF, 0xE7, 0xE7,
        0xFF,
    ];
    chr[4096 + 16..4096 + 32].copy_from_slice(&ground_tile);
}

fn serialize_captured_scene(sc: &CapturedScene) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    out.extend_from_slice(b"NGSSCN01");
    out.push(sc.ppu_ctrl_bits);
    out.extend_from_slice(&sc.palette_table);
    out.extend_from_slice(&sc.oam);
    out.extend_from_slice(&(sc.chr_8kb.len() as u32).to_le_bytes());
    out.extend_from_slice(&sc.chr_8kb);
    out.extend_from_slice(&(sc.screen_rgb.len() as u32).to_le_bytes());
    out.extend_from_slice(&sc.screen_rgb);
    out.extend_from_slice(&(sc.save_state.len() as u32).to_le_bytes());
    out.extend_from_slice(&sc.save_state);
    Ok(out)
}

fn deserialize_captured_scene(bytes: &[u8]) -> Option<CapturedScene> {
    if bytes.len() < 8 + 1 + 32 + 256 + 12 || &bytes[0..8] != b"NGSSCN01" {
        return None;
    }
    let mut pos = 8;
    let ppu_ctrl_bits = bytes[pos];
    pos += 1;
    let mut palette_table = [0u8; 32];
    palette_table.copy_from_slice(&bytes[pos..pos + 32]);
    pos += 32;
    let mut oam = [0u8; 256];
    oam.copy_from_slice(&bytes[pos..pos + 256]);
    pos += 256;

    let chr_len = u32::from_le_bytes(bytes.get(pos..pos + 4)?.try_into().ok()?) as usize;
    pos += 4;
    let chr_8kb = bytes.get(pos..pos + chr_len)?.to_vec();
    pos += chr_len;

    let rgb_len = u32::from_le_bytes(bytes.get(pos..pos + 4)?.try_into().ok()?) as usize;
    pos += 4;
    let screen_rgb = bytes.get(pos..pos + rgb_len)?.to_vec();
    pos += rgb_len;

    let state_len = u32::from_le_bytes(bytes.get(pos..pos + 4)?.try_into().ok()?) as usize;
    pos += 4;
    let save_state = bytes.get(pos..pos + state_len)?.to_vec();

    Some(CapturedScene {
        title: "Escena".to_string(),
        chr_8kb,
        palette_table,
        oam,
        ppu_ctrl_bits,
        screen_rgb,
        save_state,
    })
}

fn encode_audio_trigger(trigger: &AudioTrigger) -> String {
    match trigger {
        AudioTrigger::Subroutine(addr) => format!("Sub:{}", addr),
        AudioTrigger::RamAddress(addr) => format!("Ram:{}", addr),
        AudioTrigger::DriverCommand {
            entry_point,
            queue_address,
        } => format!("Drv:{}:{}", entry_point, queue_address),
        AudioTrigger::Unknown => "Unk:0".to_string(),
    }
}

fn decode_audio_trigger(s: &str) -> Option<AudioTrigger> {
    let mut parts = s.split(':');
    match parts.next()? {
        "Sub" => Some(AudioTrigger::Subroutine(parts.next()?.parse().ok()?)),
        "Ram" => Some(AudioTrigger::RamAddress(parts.next()?.parse().ok()?)),
        "Drv" => Some(AudioTrigger::DriverCommand {
            entry_point: parts.next()?.parse().ok()?,
            queue_address: parts.next()?.parse().ok()?,
        }),
        _ => Some(AudioTrigger::Unknown),
    }
}

fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn extract_json_string(json: &str, key: &str) -> Option<String> {
    let needle = format!("\"{}\"", key);
    let idx = json.find(&needle)?;
    let rest = &json[idx + needle.len()..];
    let colon = rest.find(':')?;
    let after_colon = rest[colon + 1..].trim_start();
    if !after_colon.starts_with('"') {
        return None;
    }
    let content = &after_colon[1..];
    let mut out = String::new();
    let mut escaped = false;
    for ch in content.chars() {
        if escaped {
            out.push(ch);
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            return Some(out);
        } else {
            out.push(ch);
        }
    }
    None
}

fn extract_json_bool(json: &str, key: &str) -> Option<bool> {
    let needle = format!("\"{}\"", key);
    let idx = json.find(&needle)?;
    let rest = &json[idx + needle.len()..];
    let colon = rest.find(':')?;
    let after_colon = rest[colon + 1..].trim_start();
    if after_colon.starts_with("true") {
        Some(true)
    } else if after_colon.starts_with("false") {
        Some(false)
    } else {
        None
    }
}

fn extract_json_usize(json: &str, key: &str) -> Option<usize> {
    let needle = format!("\"{}\"", key);
    let idx = json.find(&needle)?;
    let rest = &json[idx + needle.len()..];
    let colon = rest.find(':')?;
    let after_colon = rest[colon + 1..].trim_start();
    let digits: String = after_colon
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse::<usize>().ok()
}
