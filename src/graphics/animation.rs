use crate::graphics::metasprite::{reconstruct_scene_metasprites, MetaspriteEntity};
use crate::project::CapturedScene;
use std::collections::HashMap;

/// Fotograma individual dentro del Timeline de Animación de un personaje/entidad (Fase G4).
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct AnimationFrame {
    pub frame_index: usize,
    pub scene_idx: usize,
    pub label: String,
    pub entity: MetaspriteEntity,
    pub tile_signature: Vec<u16>,
}

/// Secuencia de animación reconstruida para la entidad seleccionada.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct AnimationSequence {
    pub entity_name: String,
    pub frames: Vec<AnimationFrame>,
    pub fps: u8,
    pub loop_enabled: bool,
}

impl Default for AnimationSequence {
    fn default() -> Self {
        Self {
            entity_name: String::new(),
            frames: Vec::new(),
            fps: 6,
            loop_enabled: true,
        }
    }
}

/// Construye la tira de fotogramas de animación para la entidad seleccionada buscando sus poses
/// a través de las escenas capturadas y fotogramas registrados.
pub fn build_entity_animation_sequence(
    selected_entity: &MetaspriteEntity,
    active_scene_idx: usize,
    scenes: &[CapturedScene],
    custom_names: &HashMap<String, String>,
) -> AnimationSequence {
    let mut frames = Vec::new();
    let mut seen_signatures: Vec<Vec<u16>> = Vec::new();

    // 1. Incluir siempre el fotograma de la escena activa primero
    let first_sig = selected_entity.unique_vram_tiles();
    seen_signatures.push(first_sig.clone());
    frames.push(AnimationFrame {
        frame_index: 1,
        scene_idx: active_scene_idx,
        label: "Pose 1 (Actual)".to_string(),
        entity: selected_entity.clone(),
        tile_signature: first_sig,
    });

    // 2. Buscar poses equivalentes de esta entidad en las demás escenas capturadas
    for (sc_idx, sc) in scenes.iter().enumerate() {
        if sc_idx == active_scene_idx {
            continue;
        }
        let candidates =
            reconstruct_scene_metasprites(&sc.oam, sc.ppu_ctrl_bits, custom_names);
        for cand in candidates {
            let same_dimensions =
                cand.width == selected_entity.width && cand.height == selected_entity.height;
            let same_palette = cand.primary_palette == selected_entity.primary_palette;
            let close_pos = (cand.bbox_x as i32 - selected_entity.bbox_x as i32).abs() <= 48
                && (cand.bbox_y as i32 - selected_entity.bbox_y as i32).abs() <= 48;

            if same_dimensions && (same_palette || close_pos) {
                let sig = cand.unique_vram_tiles();
                if !seen_signatures.contains(&sig) {
                    seen_signatures.push(sig.clone());
                    let next_num = frames.len() + 1;
                    frames.push(AnimationFrame {
                        frame_index: next_num,
                        scene_idx: sc_idx,
                        label: format!("Pose {} (Esc #{})", next_num, sc_idx + 1),
                        entity: cand,
                        tile_signature: sig,
                    });
                }
            }
        }
    }

    AnimationSequence {
        entity_name: selected_entity.name.clone(),
        frames,
        fps: 6,
        loop_enabled: true,
    }
}
