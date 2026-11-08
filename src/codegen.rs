use crate::project::NgsProject;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Inyecta las modificaciones gráficas (CHR/PRG, Sprites HD) y de audio (.MP3/.WAV) en un archivo C listo para compilar
pub fn generate_modified_c_file(project: &NgsProject) -> Result<PathBuf, String> {
    if project.raw_c_source.is_empty() {
        return Err("Primero abre un archivo .C exportado por NesEmuRust para inyectar cambios.".to_string());
    }

    let base_dir = project
        .c_file_path
        .as_ref()
        .and_then(|p| p.parent())
        .unwrap_or_else(|| Path::new("."));

    let out_c_path = base_dir.join(format!("{}_StudioMod.c", project.game_title));

    // Construir el bloque C de parches automáticos de NesGameStudio
    let mut patch_block = String::new();
    patch_block.push_str("\n/* =====================================================================\n");
    patch_block.push_str(" * BLOQUE INYECTADO AUTOMÁTICAMENTE POR NESGAMESTUDIO (FASE 1)\n");
    patch_block.push_str(" * ===================================================================== */\n");
    patch_block.push_str(&format!(
        "#define NGS_WIDESCREEN_16_9 {}\n",
        if project.widescreen_16_9 { 1 } else { 0 }
    ));

    patch_block.push_str("static int g_ngs_patches_initialized = 0;\n");
    patch_block.push_str("static void ngs_apply_studio_patches_frame(void) {\n");

    // 1. Mantener aplicados los tiles 8x8 editados en CHR_MEM[] (funciona tanto en CHR-ROM como tras descompresión CHR-RAM)
    for (&tile_idx, bytes) in &project.modified_tiles {
        let base_offset = (tile_idx % 512) * 16;
        for (i, &b) in bytes.iter().enumerate() {
            patch_block.push_str(&format!(
                "    CHR_MEM[{} + {}] = 0x{:02X};\n",
                base_offset, i, b
            ));
        }
    }

    patch_block.push_str("    if (g_ngs_patches_initialized) return;\n");
    patch_block.push_str("    g_ngs_patches_initialized = 1;\n");

    // 2. Registrar reemplazos de Sprites HD vinculados (para Enhanced 2D / Hybrid)
    for (&tile_idx, hd) in &project.hd_sprites {
        patch_block.push_str(&format!(
            "    /* HD Sprite Tile #{:04X} ({}x{}, {} px): {} */\n",
            hd.tile_index.max(tile_idx),
            hd.width,
            hd.height,
            hd.rgba_pixels.len(),
            hd.bmp_or_rgba_path.replace('\\', "\\\\")
        ));
    }

    // 3. Si hay pistas .MP3 / .WAV de reemplazo configuradas en la Rockola, iniciar reproducción MCI
    if !project.audio_event_replacements.is_empty() {
        let mut sorted_reps: Vec<_> = project.audio_event_replacements.iter().collect();
        sorted_reps.sort_by_key(|(k, _)| (k.trigger.short_label(), k.command_id));
        for (idx, (event_key, rep)) in sorted_reps.into_iter().enumerate() {
            let escaped_path = rep.external_file_path.replace('\\', "\\\\");
            let alias_id = format!("{}_{:02X}", idx, event_key.command_id);
            if rep.is_sfx_wav {
                patch_block.push_str(&format!(
                    "    /* {} [{}] vinculado a WAV: {} */\n",
                    rep.label,
                    event_key.short_label(),
                    escaped_path
                ));
            } else {
                patch_block.push_str(&format!(
                    "    /* {} [{}] */\n    mciSendStringA(\"open \\\"{}\\\" type mpegvideo alias ngs_bgm_{}\", NULL, 0, NULL);\n",
                    rep.label,
                    event_key.short_label(),
                    escaped_path,
                    alias_id
                ));
                if event_key.command_id == 0 {
                    patch_block.push_str(&format!(
                        "    mciSendStringA(\"play ngs_bgm_{} repeat\", NULL, 0, NULL);\n",
                        alias_id
                    ));
                }
            }
        }
    } else {
        for (&track_id, rep) in &project.audio_replacements {
            let escaped_path = rep.external_file_path.replace('\\', "\\\\");
            if rep.is_sfx_wav {
                patch_block.push_str(&format!(
                    "    /* {} (ID #{:02X}) vinculado a WAV: {} */\n",
                    rep.label, rep.track_id.max(track_id), escaped_path
                ));
            } else {
                patch_block.push_str(&format!(
                    "    /* {} (ID #{:02X}) */\n    mciSendStringA(\"open \\\"{}\\\" type mpegvideo alias ngs_bgm_{:02X}\", NULL, 0, NULL);\n",
                    rep.label, rep.track_id.max(track_id), escaped_path, track_id
                ));
                if track_id == 0 {
                    patch_block.push_str(&format!(
                        "    mciSendStringA(\"play ngs_bgm_{:02X} repeat\", NULL, 0, NULL);\n",
                        track_id
                    ));
                }
            }
        }
    }
    patch_block.push_str("}\n\n");

    // Insertar el bloque justo antes de nes_step_one_frame_60hz y llamar a ngs_apply_studio_patches_once()
    let mut modified_c = project.raw_c_source.clone();
    if let Some(pos) = modified_c.find("void nes_step_one_frame_60hz(void)") {
        modified_c.insert_str(pos, &patch_block);
        modified_c = modified_c.replacen(
            "void nes_step_one_frame_60hz(void) {",
            "void nes_step_one_frame_60hz(void) {\n    ngs_apply_studio_patches_frame();",
            1,
        );
    } else {
        modified_c.push_str(&patch_block);
    }

    fs::write(&out_c_path, modified_c)
        .map_err(|e| format!("Error guardando '{}': {}", out_c_path.display(), e))?;

    Ok(out_c_path)
}

/// Compila el archivo C modificado a un ejecutable .exe nativo x64 y opcionalmente lo ejecuta
pub fn compile_and_run_modified_c(project: &NgsProject, launch_after: bool) -> Result<String, String> {
    let c_path = generate_modified_c_file(project)?;
    let exe_path = c_path.with_extension("exe");

    // Buscar vcvars64.bat de Visual Studio en rutas estándar de Windows
    let vcvars_candidates = [
        r"C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat",
        r"C:\Program Files\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat",
        r"C:\Program Files\Microsoft Visual Studio\2022\Professional\VC\Auxiliary\Build\vcvars64.bat",
        r"C:\Program Files (x86)\Microsoft Visual Studio\2019\Community\VC\Auxiliary\Build\vcvars64.bat",
    ];

    let found_vcvars = vcvars_candidates.iter().find(|p| Path::new(p).exists());

    if let Some(vcvars) = found_vcvars {
        let cmd_str = format!(
            "call \"{}\" >nul && cl.exe /nologo /O2 \"{}\" /Fe:\"{}\" user32.lib gdi32.lib winmm.lib",
            vcvars,
            c_path.display(),
            exe_path.display()
        );
        let output = Command::new("cmd")
            .args(["/C", &cmd_str])
            .output()
            .map_err(|e| format!("Error invocando MSVC cl.exe: {}", e))?;

        if !output.status.success() {
            let err_txt = String::from_utf8_lossy(&output.stdout);
            return Err(format!("Fallo de compilación MSVC:\n{}", err_txt));
        }
    } else {
        // Intentar con GCC si no se encontró Visual Studio
        let output = Command::new("gcc")
            .args([
                "-O2",
                c_path.to_str().unwrap_or_default(),
                "-o",
                exe_path.to_str().unwrap_or_default(),
                "-lgdi32",
                "-lwinmm",
                "-luser32",
            ])
            .output()
            .map_err(|_| {
                format!(
                    "Se generó '{}' correctamente, pero no se encontró cl.exe ni gcc en el PATH.",
                    c_path.display()
                )
            })?;

        if !output.status.success() {
            let err_txt = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Fallo de compilación GCC:\n{}", err_txt));
        }
    }

    if launch_after && exe_path.exists() {
        let _ = Command::new(&exe_path).spawn();
    }

    Ok(format!(
        "✔ Compilado con éxito: {}",
        exe_path.display()
    ))
}
