use crate::project::{NgsProject, SubroutineInfo};
use std::fs;
use std::path::Path;

pub fn load_c_and_asm_into_project(c_path: &Path, project: &mut NgsProject) -> Result<String, String> {
    let raw_c = fs::read_to_string(c_path)
        .map_err(|e| format!("Error leyendo archivo C '{}': {}", c_path.display(), e))?;

    let stem = c_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Juego_NES")
        .to_string();

    project.game_title = stem;
    project.c_file_path = Some(c_path.to_path_buf());
    project.raw_c_source = raw_c.clone();
    project.modified_tiles.clear();

    // Extraer Mapper y Vectores si están definidos en el .c
    project.mapper_id = parse_define_u16(&raw_c, "#define MAPPER_ID").unwrap_or(0) as u8;
    project.vec_nmi = parse_define_u16(&raw_c, "#define VEC_NMI").unwrap_or(0x8000);
    project.vec_reset = parse_define_u16(&raw_c, "#define VEC_RESET").unwrap_or(0x8000);
    project.vec_irq = parse_define_u16(&raw_c, "#define VEC_IRQ").unwrap_or(0x8000);

    // Extraer arreglos PRG_ROM[] y CHR_MEM[]
    project.prg_rom = extract_c_byte_array(&raw_c, "PRG_ROM[");
    project.chr_mem = extract_c_byte_array(&raw_c, "CHR_MEM[");

    // Determinar si es CHR-ROM (datos reales en CHR_MEM) o CHR-RAM (CHR_MEM vacío/ceros como Contra)
    let non_zero_chr = project.chr_mem.iter().filter(|&&b| b != 0).count();
    if non_zero_chr > 64 {
        project.is_chr_ram = false;
        project.decoded_tiles_source = project.chr_mem.clone();
    } else {
        project.is_chr_ram = true;
        // Para juegos CHR-RAM (ej. Contra, Zelda, Mega Man), extraemos los bancos de patrones desde PRG_ROM
        project.decoded_tiles_source = extract_chr_ram_patterns_from_prg(&project.prg_rom);
    }

    // Buscar automáticamente el archivo .asm compañero en la misma carpeta
    let candidate_asm = c_path.with_extension("asm");
    project.subroutines.clear();
    if candidate_asm.exists() {
        project.asm_file_path = Some(candidate_asm.clone());
        let _ = load_asm_subroutines(&candidate_asm, project);
    } else {
        // Si no hay .asm externo, escanear etiquetas sub_XXXX directamente del .c
        extract_subroutines_from_c(&raw_c, project);
    }

    Ok(format!(
        "Cargado '{}': PRG={} KB, {} ({} tiles), {} subrutinas detectadas (Mapper {})",
        project.game_title,
        project.prg_rom.len() / 1024,
        if project.is_chr_ram { "CHR-RAM (Escaneado de PRG)" } else { "CHR-ROM" },
        project.total_tiles(),
        project.subroutines.len(),
        project.mapper_id
    ))
}

pub fn load_asm_subroutines(asm_path: &Path, project: &mut NgsProject) -> Result<usize, String> {
    let raw_asm = fs::read_to_string(asm_path)
        .map_err(|e| format!("No se pudo leer '{}': {}", asm_path.display(), e))?;

    project.asm_file_path = Some(asm_path.to_path_buf());
    project.subroutines.clear();

    let lines: Vec<&str> = raw_asm.lines().collect();
    let mut current_category = "[GAME LOGIC]".to_string();

    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.contains("[AUDIO / APU]") {
            current_category = "[AUDIO / APU]".to_string();
        } else if trimmed.contains("[VIDEO / PPU]") {
            current_category = "[VIDEO / PPU]".to_string();
        } else if trimmed.contains("[JOYPAD / INPUT]") {
            current_category = "[JOYPAD / INPUT]".to_string();
        } else if trimmed.contains("[GAME LOGIC]") || trimmed.contains("[MAPPER") {
            current_category = "[GAME LOGIC]".to_string();
        }

        // Detectar etiquetas de subrutina como "sub_8123:" o "nmi_vector_8050:"
        if (trimmed.starts_with("sub_") || trimmed.starts_with("nmi_") || trimmed.starts_with("reset_"))
            && trimmed.ends_with(':')
        {
            let name = trimmed.trim_end_matches(':').to_string();
            let addr = name
                .split('_')
                .last()
                .and_then(|h| u16::from_str_radix(h, 16).ok())
                .unwrap_or(0x8000);

            let mut preview = Vec::new();
            for next_line in lines.iter().skip(idx + 1).take(14) {
                preview.push((*next_line).to_string());
                if next_line.contains("RTS") || next_line.contains("RTI") {
                    break;
                }
            }

            // Si en el cuerpo de la subrutina se toca $4000..$4015, clasificar como [AUDIO / APU]
            let body_joined = preview.join(" ");
            let cat = if body_joined.contains("$400") || body_joined.contains("$4015") {
                "[AUDIO / APU]".to_string()
            } else if body_joined.contains("$200") || body_joined.contains("$4014") {
                "[VIDEO / PPU]".to_string()
            } else if body_joined.contains("$4016") || body_joined.contains("$4017") {
                "[JOYPAD / INPUT]".to_string()
            } else {
                current_category.clone()
            };

            project.subroutines.push(SubroutineInfo {
                addr,
                name,
                category: cat,
                line_in_asm: idx + 1,
                preview_asm: preview,
            });
        }
    }

    Ok(project.subroutines.len())
}

fn extract_subroutines_from_c(raw_c: &str, project: &mut NgsProject) {
    let lines: Vec<&str> = raw_c.lines().collect();
    for (idx, line) in lines.iter().enumerate() {
        let t = line.trim();
        if t.starts_with("case 0x") && t.contains(':') {
            if let Some(hex) = t.strip_prefix("case 0x").and_then(|s| s.get(0..4)) {
                if let Ok(addr) = u16::from_str_radix(hex, 16) {
                    // Verificar si este bloque escribe a registros APU (0x4000..0x4015)
                    let mut preview = Vec::new();
                    let mut is_audio = false;
                    let mut is_video = false;
                    for next in lines.iter().skip(idx).take(12) {
                        preview.push((*next).trim().to_string());
                        if next.contains("0x400") || next.contains("0x4015") {
                            is_audio = true;
                        }
                        if next.contains("0x200") || next.contains("0x4014") {
                            is_video = true;
                        }
                    }
                    if is_audio || is_video || (addr & 0x00FF) == 0 {
                        let category = if is_audio {
                            "[AUDIO / APU]"
                        } else if is_video {
                            "[VIDEO / PPU]"
                        } else {
                            "[GAME LOGIC]"
                        };
                        project.subroutines.push(SubroutineInfo {
                            addr,
                            name: format!("loc_{:04X}", addr),
                            category: category.to_string(),
                            line_in_asm: idx + 1,
                            preview_asm: preview,
                        });
                    }
                }
            }
        }
    }
}

fn parse_define_u16(src: &str, prefix: &str) -> Option<u16> {
    for line in src.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix(prefix) {
            let val_str = rest.split_whitespace().next()?;
            if let Some(hex) = val_str.strip_prefix("0x").or_else(|| val_str.strip_prefix("0X")) {
                return u16::from_str_radix(hex, 16).ok();
            } else {
                return val_str.parse::<u16>().ok();
            }
        }
    }
    None
}

fn extract_c_byte_array(src: &str, marker: &str) -> Vec<u8> {
    let Some(start_pos) = src.find(marker) else {
        return Vec::new();
    };
    let after_marker = &src[start_pos..];
    let Some(open_brace) = after_marker.find('{') else {
        return Vec::new();
    };
    let after_brace = &after_marker[open_brace + 1..];
    let Some(close_brace) = after_brace.find("};") else {
        return Vec::new();
    };
    let body = &after_brace[..close_brace];

    let mut bytes = Vec::with_capacity(65536);
    for token in body.split(|c: char| c == ',' || c.is_whitespace()) {
        let t = token.trim();
        if let Some(hex) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
            if let Ok(b) = u8::from_str_radix(hex, 16) {
                bytes.push(b);
            }
        }
    }
    bytes
}

/// Para juegos CHR-RAM (como Contra), extrae los bancos de PRG_ROM para visualizarlos como tiles 8x8
fn extract_chr_ram_patterns_from_prg(prg: &[u8]) -> Vec<u8> {
    if prg.is_empty() {
        return vec![0u8; 8192];
    }
    // Exponemos toda la PRG_ROM alineada a múltiplos de 16 bytes para navegar todos los bancos de datos/gráficos
    let aligned_len = (prg.len() / 16) * 16;
    prg[..aligned_len].to_vec()
}
