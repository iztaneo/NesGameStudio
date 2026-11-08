> **DOCUMENTO HISTÓRICO — COPIA ÍNTEGRA, NO EDITAR.**
> Plan recibido del usuario el 2026-11-08 (local −06:00), guardado tal cual para trazabilidad.
> Su conciliación con el plan maestro vive en [`PLAN_IMPLEMENTACION.md`](../../PLAN_IMPLEMENTACION.md), **§9 "NesGameStudio 4.0"**.
> Si este archivo y la §9 difieren, la §9 documenta las decisiones de conciliación; este archivo conserva el texto original recibido.
> Baseline real del repositorio al recibirlo: `e63523e` (G7), worktree limpio.

---

# NesGameStudio 4.0 — Plan maestro ejecutable para el agente de programación

## WorldGraph + Level Studio + NGScript (ENGLISH ONLY) + New Game Runtime + ROM Reconstruction

**Repositorio único autorizado:** [`iztaneo/NesGameStudio`](https://github.com/iztaneo/NesGameStudio)  
**Baseline de auditoría, NO anclaje para sobrescribir código:** [`e63523ea79002e446c24a20faabdf24ccd82d150`](https://github.com/iztaneo/NesGameStudio/commit/e63523ea79002e446c24a20faabdf24ccd82d150) (G7).  
**Documento maestro que debe conservarse y conciliarse:** [`PLAN_IMPLEMENTACION.md`](https://github.com/iztaneo/NesGameStudio/blob/e63523ea79002e446c24a20faabdf24ccd82d150/PLAN_IMPLEMENTACION.md).  
**Estado:** especificación de implementación; **no implica que las fases ya estén programadas, compiladas o validadas en ejecución**.  
**Prioridad:** primero crear un juego nuevo completo; después importar/reconstruir mapas de ROMs existentes.  
**Política de idioma de código:** **NGScript solamente en inglés**, sin traducciones, alias españoles, modos bilingües ni conversores.

> **Este archivo sustituye los planes World & Level Engine 3.0 y 3.1.** Es la única especificación operativa para la siguiente implementación. Ningún ejemplo o frase del plan histórico que contradiga el inglés exclusivo de NGScript prevalece sobre este documento.

---

# 0. Mandato obligatorio al agente

## 0.1. Alcance y prohibiciones absolutas

1. Trabajar **EXCLUSIVAMENTE en el repositorio `NesGameStudio`**. **No abrir, inspeccionar, editar, recompilar desde fuentes ni proponer modificaciones en `NesEmuRust`**. Utilizar únicamente las APIs que NesGameStudio ya invoca y que pueden identificarse dentro de su propio código. Si falta una capacidad, documentar el bloqueo sin inventar APIs.
2. NO modificar por número de línea. Localizar el código por **archivo orientativo + símbolo exacto + firma actual + referencias + comportamiento observado**. Los números y rutas de este documento son referencias, no instrucciones de aplicación textual.
3. Antes de cada PR, inspeccionar `git status`, HEAD, ramas activas y los cambios existentes; **NO sobrescribir ni revertir modificaciones paralelas** (Rockola, G7, editor gráfico, otros agentes). Integrar con cambios mínimos y tests de regresión.
4. NO eliminar ni reescribir globalmente los editores G7, la Rockola, las opciones de workspace, las plantillas o el exportador existente. Crear módulos nuevos y adaptadores delimitados. Toda migración debe preservar proyectos antiguos.
5. NO introducir `if game_title.contains(...)`, IDs de juegos comerciales, hashes ROM precurados, offsets PRG escritos a mano, archivos externos con perfiles por título ni catálogos inventados. **Constantes del hardware NES son legítimas; direcciones o datos específicos de un juego no lo son.**
6. NO equiparar `CapturedScene` con un nivel, `screen_rgb` con tilemap, 4 tiles contiguos en VRAM con metatile confirmado, ni parecido de píxeles con procedencia comprobada.
7. NO editar bytes de PRG-ROM solo por coincidencia de firmas gráficas. Distinguir **modificar definición de recurso** de **cambiar una instancia/celda**. Modificar datos originales de una ROM solo mediante correspondencia de origen verificada.
8. NO afirmar éxito de creación/exportación si solo hay interfaz, JSON o C generado. La aceptación final exige **compilar y ejecutar un proyecto nuevo**, comprobar colisiones, transiciones y scripts; indicar claramente qué target ha sido probado.
9. NO implementar un segundo lenguaje de lógica, un traductor español↔inglés ni «visual scripting» con runtime paralelo. El **único lenguaje de comportamiento del estudio será NGScript**; el editor visual de bloques produce el mismo AST/IR.
10. No prometer un exportador universal de ROM NES: primero PC nativo C99/C11, si la infraestructura presente lo permite; un verdadero backend ROM NES queda separado, condicionado y no se marca completo sin pruebas en hardware/emulador.

## 0.2. NGScript: resolución definitiva de idioma

- **Nombre:** `NGScript` (ortografía del plan maestro, no `NGSScript`).
- **Extensión:** `.ngscript`.
- **Palabras reservadas, nombres de funciones de biblioteca, parámetros nombrados, eventos y diagnósticos técnicos del lenguaje: INGLÉS**.
- **No existen alias en español**: `entidad`, `estado`, `si`, `cada`, `mover_x`, `disparar`, `al_colisionar_con` y similares **NO forman parte de la gramática final**. El parser debe rechazarlos en contexto de palabras reservadas con un mensaje explícito; no auto-traducirlos.
- **No crear** diccionarios, plugins, traducciones de idioma de script ni funciones `translate_ngscript`.
- **Nombres definidos por el usuario y textos de diálogo:** pueden contener palabras de cualquier idioma **dentro de cadenas** y etiquetas; esto no cambia el inglés exclusivo de la sintaxis.
- **Interfaz del IDE:** podrá mostrar etiquetas en español si ya lo hace; eso es localización de interfaz, NO un traductor de scripts. Para evitar nuevas dependencias, este plan no exige implementar internacionalización.
- **Compatibilidad histórica:** los ejemplos en español de `PLAN_IMPLEMENTACION.md` son documentación conceptual antigua, **no archivos fuente NGScript válidos**. Actualizar el plan maestro y ejemplos activos a inglés cuando se integre W0/W6; no introducir una vía de ejecución legacy bilingüe.
- **Único pipeline:** `.ngscript` (EN) → lexer → parser → AST tipado/IR → validación semántica → C99/C11 + `ngs_scripts_registry.h` → runtime.

## 0.3. Protocolo antialucinación, obligatorio POR CADA PR

El agente debe entregar antes de editar:

```text
BASE/HEAD: <SHA real>
WORKTREE: <limpio o lista de archivos con cambios preexistentes>
SÍMBOLOS ENCONTRADOS: <ruta actual, firma, llamadas y dependencias reales>
LÍMITES: <qué API existe, qué API falta, qué NO se hará>
PRUEBA ROJA: <test que debe fallar antes de implementar>
ARCHIVOS EN ALCANCE: <lista cerrada para este PR>
```

Al finalizar cada PR:

```text
DIFF: <archivos, funciones, breve justificación>
PRUEBAS: <comando exacto, tests verdes/rojos/omitidos>
EVIDENCIA EJECUTABLE: <si aplica, cómo verificar en el juego generado>
COMPATIBILIDAD: <ROM existente, G7, Rockola, workspaces>
LIMITACIONES REALES: <pendientes y bloqueos comprobados>
GATE: PASS / FAIL y por qué
```

**No completar anticipadamente todas las fases.** Ejecutar secuencialmente PRs de tamaño revisable. Si una prueba depende de un compilador/entorno ausente, informar **NOT RUN**, no hacerla pasar artificialmente ni usar un `#[ignore]` para declarar éxito.

---

# 1. Producto objetivo y decisión arquitectónica central

## 1.1. WorldGraph es el modelo central del juego

- Un **juego nuevo comienza diseñando el WorldGraph**, aunque pueda arrancar con un grafo mínimo de un solo nivel y ampliarlo después.
- Una **ROM existente se importa como evidencia** y el sistema intenta inferir un WorldGraph parcial, etiquetando qué conoce y qué desconoce.
- El grafo es **jerárquico y navegable**. La jerarquía agrupa contenido, mientras que las conexiones describen rutas jugables. No confundir ambos tipos de relación.
- Un nodo puede representar mundo, región, nivel, habitación, subsección, minijuego o secuencia. **Una pantalla 256×240 no es obligatoriamente un nodo**; para scrolling, un nivel puede contener muchas pantallas virtuales sin producir cientos de nodos.
- Un mismo proyecto puede combinar **plataformas laterales, vista superior, RPG, shooter y minijuegos**. La perspectiva y mecánicas se configuran por nivel o sección, con herencia desde el mundo cuando convenga; nunca se vinculan al nombre del juego.
- Las conexiones pueden ser **no secuenciales, condicionales, bidireccionales o con retorno a otro punto**. Los ciclos de navegación son legales; los ciclos de contención jerárquica no.

## 1.2. Separar estructura, presentación y lógica

| Subsistema | Dueño de la información | Ejemplos | NO debe hacer |
|---|---|---|---|
| `WorldGraph` | Mundos, nodos, jerarquía, rutas, puntos de entrada/salida | Mundo 1 → Nivel A; puerta B → mazmorra | Contener el bitmap entero del nivel ni la lógica de un jefe |
| `LevelMap` | Escenarios y capas, coordenadas, metatiles, colisiones base | Suelo, paredes, decoración, foreground, cámara | Inventar rutinas 6502 para ROMs importadas |
| `Object/Components` | Instancias de objetos y capacidades | Player, NPC, Enemy, Trigger, Portal | Codificar clases especiales por nombre de juego |
| `NGScript` (EN) | Comportamientos, estados, eventos, condiciones y acciones | `on_update`, `on_collision`, `on_request`, `every(90)` | Guardar un segundo grafo de mundo o reemplazar los datos del nivel |
| `NewGameRuntime` | Física, cámara, mapas, entidades, invocación scripts, navegación | Despacho de eventos y transición A→B | Implementar una excepción C por juego o duplicar el lenguaje |
| `RomReconstruction` | Evidencia, inferencias, relación entre capturas y nodos | Capturas, transiciones observadas, uniones por scrolling | Afirmar cobertura total basándose solo en screenshots |

## 1.3. Los dos flujos de trabajo

```text
JUEGO NUEVO                              ROM NES EXISTENTE
Crear Workspace                         Importar ROM a Workspace
    |                                          |
Diseñar WorldGraph                     Capturar / analizar evidencias
    |                                          |
Definir LevelMaps                      Inferir grafo parcial y secciones
    |                                          |
Colocar objetos/capas                  Mostrar confianza y zonas desconocidas
    |                                          |
Crear comportamiento NGScript          Overlay de modificaciones verificadas
    |                                          |
Compilar runtime C nativo              Exportación condicionada a capacidades
    |                                          |
Ejecutar y probar juego                 Conservar ROM original intacta
    \____________________  ___________________/
                         \/
         World Studio + Level Studio + Asset Studio
         + NGScript Studio + herramientas existentes
```

**Orden de implementación:** crear primero un juego nuevo real y ejecutable, porque su estructura es nuestra fuente de verdad. La inferencia retro se incorpora DESPUÉS usando el mismo formato.

---

# 2. Hallazgos del código actual e instrucciones por símbolo (sin líneas)

**Estado verificado solo en NesGameStudio, commit `e63523e` (G7).** Antes de intervenir, volver a localizar cada símbolo en HEAD: nombres y firmas pueden haber evolucionado.

| Símbolo o archivo a localizar en NesGameStudio | Comportamiento actual observado | Intervención exacta requerida | Prohibición/test relevante |
|---|---|---|---|
| `src/main.rs::ActiveTab`, `NesGameStudioApp` y `update()` | Solo pestañas gráficas, audio y subrutinas | Integrar `WorldStudio`, `LevelStudio`, `NGScriptStudio` como pestañas nuevas o secciones coherentes sin sustituir las existentes | G7 y Rockola siguen accesibles y funcionales |
| `src/editors/mod.rs` | Exporta `audio`, `graphics`, `guide` | Registrar nuevos editores sin duplicar motores o lógica de scripts | Compila sin referencias a módulos inexistentes |
| `src/project.rs::NgsProject` | Estado central, `scenes`, tiles, APU y recursos, sin modelo nativo de WorldGraph | Asociar `WorldDocument` con IDs estables y `ScriptCatalog`; evitar mezclar la jerarquía con `CapturedScene` | Un proyecto con distintos niveles persiste idéntico |
| `src/project.rs::CapturedScene` y `capture_current_live_scene()` | Captura CHR, paleta, OAM, framebuffer y savestate | Conservar como evidencia; si hay metadatos nuevos, versionarlos aparte. No transformarlo automáticamente en `LevelMap` | No llamar «nivel» a una captura |
| `src/project.rs::auto_discover_more_scenes_without_playing()` | Bot de secuencias de botones prefijadas | Mantener como captura exploratoria opcional; nunca usar el número de capturas como número de niveles | Pose/animación no crea un nodo confirmado |
| `src/project.rs::is_live_frame_visually_distinct()` | Detecta novedad visual OAM/RGB | No utilizar como detector definitivo de transiciones; nuevo módulo debe diferenciar scroll/animación/escena | Prueba de falso positivo por enemigo que camina |
| `src/parser.rs::populate_real_scenes_and_audio_from_ines()` | Arranque con START, avance y botones; construye capturas | No convertir estas secuencias en «árbol del juego»; importador genera evidencias | Una ROM que no entra al gameplay no obtiene nivel ficticio |
| `src/editors/graphics.rs::GraphicsEditorState::ui()` | Inspector G7 de captura actual | Conservarlo; añadir navegación contextual desde Level Studio hacia recurso gráfico | No reemplazarlo por World Studio |
| `GraphicsEditorState::render_top_scene_gallery_strip()` | Miniaturas de capturas | Pasar conceptualmente a «Evidencia» en importados, no a lista de niveles verificados | No ordenar «nivel 1/2» por captura sin evidencia |
| `GraphicsEditorState::render_center_interactive_scene()` | Canvas de una pantalla 256×240 | Mantener como vista local; nuevo `LevelStudio` tendrá mapa extenso y scroll propio | El viewport NES no limita tamaño del mapa |
| `GraphicsEditorState::render_entity_paint_canvas_tab()` | Edita patrón gráfico CHR asociado | En Level Studio, separar «Edit Graphic Definition» y «Replace Cell Instance» | Modificar una celda no altera otras apariciones |
| `src/graphics/metasprite.rs::extract_all_loaded_vram_sprite_cards()` | Agrupación heurística 4 tiles consecutivos en tarjetas | No presentar las tarjetas como `MetatileDefinition` confirmado; solo sugerencias gráficas | Bloques no contiguos pueden formar un metatile válido |
| `src/graphics/metasprite.rs::resolve_screen_region_to_entity()` | Reconoce región mediante similitud RGB | No usarlo como fuente de verdad de celdas de mapa; reportar ambigüedad y fuentes | Colores parecidos no permiten reescritura segura |
| `src/project.rs::apply_tile_patch_by_id()` | Parchea CHR y puede sincronizar coincidencias PRG de 16 bytes | Prohibir el uso de este método como sustituto de escribir una celda del mapa; flujo de edición nuevo independiente | Preservar byte-for-byte ROM base no editada |
| `src/workspace.rs::WorkspaceTemplate` | Plantillas PlatformerAction, TopDownAdventure, SpaceShmup, BlankExtended | Conservar como **presets**, no como modo único obligatorio del proyecto | Mezclar modos dentro de un mismo proyecto |
| `workspace::create_workspace_for_new_game()` | Genera ROM base sintética y `main_behavior.ngscript` que hoy solo contiene comentarios | Inicializar grafo mínimo, script EN **validado** cuando exista parser, archivos propios versionados; no fingir gameplay por generar una ROM placeholder | Proyecto nuevo A→B ejecutable en W7 |
| `workspace::{save_workspace,load_workspace}` y `WorkspacePaths` | Guarda escenas/parches/audio y JSON de proyecto | Añadir `world/`, `levels/`, `scripts/`, bindings, checksum/validaciones y migración sin pérdida | Roundtrip y compatibilidad G7 |
| `src/codegen.rs::generate_modified_c_file()` | Modificaciones al C de ROM/remaster | No injertar código de nuevo juego mediante reemplazos de texto en este método; crear pipeline `NewGameCodegen` propio | Dos rutas de exportación claramente separadas |
| `Cargo.toml` | Dependencia por `path` a NesEmuRust, eframe 0.29 y sin crates de serialización declarados en el archivo actual | Si se necesitan `serde`/`serde_json`, añadir explícitamente a NesGameStudio y actualizar lock; inspeccionar dependencias actuales antes | No abrir archivos de la dependencia externa |
| `PLAN_IMPLEMENTACION.md` | Tiene ejemplos conceptuales NGScript con sintaxis española y plan de transpilación a C | Documentar una resolución normativa de diseño: NGScript v1 **English-only** y nueva referencia de gramática. No eliminar otros compromisos del plan maestro | Ningún parser o test acepta keywords en español |

**Enlaces auditables:** [`src/main.rs`](https://github.com/iztaneo/NesGameStudio/blob/e63523ea79002e446c24a20faabdf24ccd82d150/src/main.rs), [`src/project.rs`](https://github.com/iztaneo/NesGameStudio/blob/e63523ea79002e446c24a20faabdf24ccd82d150/src/project.rs), [`src/workspace.rs`](https://github.com/iztaneo/NesGameStudio/blob/e63523ea79002e446c24a20faabdf24ccd82d150/src/workspace.rs), [`src/editors/graphics.rs`](https://github.com/iztaneo/NesGameStudio/blob/e63523ea79002e446c24a20faabdf24ccd82d150/src/editors/graphics.rs), [`src/graphics/metasprite.rs`](https://github.com/iztaneo/NesGameStudio/blob/e63523ea79002e446c24a20faabdf24ccd82d150/src/graphics/metasprite.rs), [`src/parser.rs`](https://github.com/iztaneo/NesGameStudio/blob/e63523ea79002e446c24a20faabdf24ccd82d150/src/parser.rs), [`src/codegen.rs`](https://github.com/iztaneo/NesGameStudio/blob/e63523ea79002e446c24a20faabdf24ccd82d150/src/codegen.rs).

---

# 3. Modelo nativo de datos, independiente del género

## 3.1. Árbol de contención ≠ grafo de navegación

Ejemplo:

```text
Game: My Hybrid Adventure
  World: Green Kingdom [default: SideScrolling]
    Level: Forest Entry [SideScrolling]
      Section: Forest A
      Section: Forest B
    Level: Village [TopDown]
      Room: Inn
      Room: Shop
  World: Outer Space
    Level: Star Run [AutoScrollShooter]
```

Las **conexiones de navegación** se almacenan por separado:

```text
Forest Entry.exit_east    -> Village.spawn_west
Village.door_inn          -> Inn.spawn_front
Inn.door_exit             -> Village.spawn_inn
Village.secret_portal     -> Star Run.spawn_start  [condition: NGScript]
```

El nivel puede contener un mapa largo de scrolling o una cuadrícula de habitaciones; el nodo no es una captura. Un nivel puede referenciar múltiples secciones, y un portal puede conectar nodos que no son hermanos.

## 3.2. Tipos mínimos propuestos (contratos; NO código compilable existente)

```rust
struct WorldDocument {
    schema_version: u32,
    project_id: StableId,
    root_node: NodeId,
    nodes: Map<NodeId, WorldNode>,
    transitions: Map<TransitionId, WorldTransition>,
    levels: Map<LevelId, LevelDefinition>,
    scripts: Map<ScriptId, ScriptDocumentRef>,
    flags: Map<FlagId, FlagDefinition>,
    view_layout: WorldGraphLayout,
}
struct WorldNode {
    id: NodeId,
    parent_id: Option<NodeId>, // únicamente contención; nunca ciclos
    kind: NodeKind,              // World, Region, Level, Room, Section, Minigame, Cutscene
    display_name: String,
    level_id: Option<LevelId>,
    gameplay_override: Option<GameplayProfileId>,
    source: SourceEvidence,
}
struct WorldTransition {
    id: TransitionId,
    from_node: NodeId,
    from_port: PortId,
    to_node: NodeId,
    to_spawn: SpawnId,
    activation: ActivationKind,
    guard: TransitionGuard,   // Always o ScriptHandlerRef verificado
    source: SourceEvidence,
}
struct LevelDefinition {
    id: LevelId,
    topology: LevelTopology,  // ContinuousMap, RoomGrid, OverworldMap, Custom
    gameplay_profile: GameplayProfileId,
    tilesets: Vec<TilesetRef>,
    layers: Vec<LayerDefinition>,
    objects: Vec<PlacedObject>,
    spawns: Vec<SpawnPoint>,
    bounds: LevelBounds,
}
struct PlacedObject {
    id: ObjectId,
    asset: AssetId,
    transform: Transform2D,
    components: Vec<ComponentData>,
    script_bindings: Vec<ScriptBinding>,
}
struct ScriptBinding {
    owner: OwnerRef,          // Game | World | Level | Object | Trigger | Transition
    event: ScriptEvent,       // on_spawn, on_update, on_collision, on_enter, on_exit, on_interact, on_request
    script: ScriptId,
    handler: HandlerId,
}
```

**Ajustar firmas/nombres según convenciones reales**, pero mantener los invariantes. `StableId` debe persistir en disco y sobrevivir renombrados/reordenamientos; no usar `usize` de posición, títulos, rutas de pantalla ni hash de bitmap como identidad de un nodo.

## 3.3. Perfiles de gameplay combinables

`GameplayProfile` define los sistemas que están activos, no un «género mágico» que impide mezclas:

- Cámara: `SideScroll`, `TopDown`, `StaticRoom`, `AutoScroll`, `Custom`.
- Control: ejes, movimiento libre o restringido, acciones ligadas a botones.
- Física: gravedad/rozamiento, colisiones por capa, saltos, movimiento libre, sin física.
- Gameplay: componentes activos (combate, diálogo, inventario, turnos, proyectiles, etc.).
- Herencia: proyecto → mundo → nivel → sección, con overrides claros y resolución determinista.
- El runtime debe reinicializar el perfil al cambiar de nivel **sin perder variables persistentes, inventario o progreso**.
- No generar una gran rama `match game genre`; usar sistemas/composiciones y capacidades por destino.

## 3.4. Modelo de mapas y recursos

- `GraphicResource`: la definición de un tile/imagen compartida.
- `Tileset`: catálogo de recursos gráficos.
- `MetatileDefinition`: composición de tiles y propiedades reutilizables cuando esté definida/confirmada.
- `MapCell`: posición de una referencia a tile/metatile en una capa; NO copia del bitmap.
- `Layer`: Background, Collision, Foreground, Decor, Gameplay; orden y parallax configurables.
- `LevelObject`: instancia de entidad o trigger con posición, componentes y bindings.
- `CapturedScene`: snapshot retro, solo evidencia.
- `SourceEvidence`: `Authored`, `Observed`, `Inferred`, `Unknown`, más referencias a capturas/estado, método y confianza documentada.

Operaciones de edición separadas:

| Operación | Modifica | Qué NO modifica |
|---|---|---|
| Paint map cell | `MapCell` en nivel/capa | Bitmap de todo el tileset |
| Edit tile definition | `GraphicResource` compartido | Posiciones de mapa |
| Duplicate tile then paint | Nuevo recurso + referencia de una celda | Otras apariciones del tile original |
| Place object | Instancia `LevelObject` | Código hardcodeado por título |
| Connect nodes | `WorldTransition` | Datos de bytes ROM sin procedencia |
| Bind event | `ScriptBinding` a `HandlerId` validado | Una segunda implementación de script |

## 3.5. Validaciones obligatorias

1. IDs únicos y referencias existentes.
2. Jerarquía sin ciclos, aunque navegación A→B→A sea válida.
3. `to_spawn` pertenece a destino y los puntos de origen/destino son válidos.
4. Colisiones de IDs, nombres reservados, borrado de nodos con referencias, assets/scripts perdidos muestran diagnóstico.
5. Tamaños de mapas y recursos con límites configurables, sin reservar mapas gigantes completos si una cuadrícula dispersa basta.
6. Guards NGScript deben retornar tipo compatible (booleano) y no causar transiciones recursivas ilimitadas en el mismo frame.
7. Versiones/compatibilidad de target: una feature no soportada bloquea la exportación correspondiente, no se ignora en silencio.

---

# 4. Especificación normativa NGScript 1.0: SOLO INGLÉS

## 4.1. Estado e intención

El `PLAN_IMPLEMENTACION.md` ya especifica el **concepto** de un lenguaje propio `.ngscript → C99/C11`, entidades con estados, temporizadores, colisiones, funciones de IA, efectos y editor visual. Su ejemplo en español NO obliga a implementar una gramática española: **la decisión final de producto sustituye esas palabras por inglés exclusivamente**.

En el baseline verificado, `workspace::create_workspace_for_new_game()` crea `scripts/main_behavior.ngscript` como archivo con comentarios. **No hay evidencia en ese commit de un parser/transpilador funcional**, por lo que las reglas de esta sección son una especificación nueva por construir, no una descripción del estado existente.

## 4.2. Decisiones sintácticas concretas para el MVP

- Versión gramatical: **NGScript 1.0** documentada en `docs/ngscript-language.md`.
- Solo keywords EN: `entity`, `state`, `var`, `if`, `else`, `every`, `on_update`, `on_collision`, `on_spawn`, `on_enter`, `on_exit`, `on_interact`, `on_request`, `transition`, `return`, `true`, `false`.
- Identificadores: ASCII `[A-Za-z_][A-Za-z0-9_]*` como regla inicial; nombres de usuario con Unicode dentro de strings UTF-8. Documentar antes de ampliar el lexer.
- Cuerpo delimitado con `{}`; sentencias terminan en `;`; expresiones binarias y paréntesis explícitos. **No usar mezclas ambiguas de indentación con `cada N frames:`**.
- `every(n)` expresa un temporizador en frames del dueño/estado, con ejecución determinista; `n` entero positivo.
- Tipos mínimos: `int`, `float`, `bool`, `string`, referencias de objetos/entidades validadas. Tipos y conversiones explícitos en documentación.
- Operaciones: asignación, comparación, operadores booleanos, suma/resta, llamadas a acciones permitidas; sin reflexión, memoria arbitraria, punteros, acceso a filesystem/red ni ejecución de C incrustado.
- El `AST` contiene spans origen y `StableId`/`NodeId` para statements persistentes donde se necesita estado de temporizador.
- Todo acceso a recursos del grafo mediante referencias tipadas resueltas en análisis semántico; el IDE puede mostrar un nombre legible, pero el binding interno usa ID estable.
- Efectos avanzados del plan maestro (`shader_2d`, partículas personalizadas, cutscenes, netplay) son **capacidades posteriores** y deben aparecer como `unsupported` hasta existir runtime real.

### Ejemplo canónico que el MVP DEBE poder procesar (sintaxis EN)

```ngscript
entity Guard {
    var health: int = 100;

    state Patrol {
        on_update {
            move_x(speed = 1.5);
            every(90) {
                shoot("Arrow", target = PLAYER, speed = 4.0);
            }
            if (health < 50) {
                change_state(Alert);
            }
        }
    }

    state Alert {
        on_update {
            chase(PLAYER, speed = 2.0);
        }
    }

    on_collision(other) {
        if (other.tag == "PlayerProjectile") {
            health = health - 10;
        }
    }
}

transition DungeonGate {
    on_request {
        return has_item("CopperKey");
    }
}
```

**Contrato del ejemplo:** especificación objetivo; el agente DEBE definir parser/semántica explícita para cada construcción anterior antes de declarar «MVP completado». Si el runtime no soporta `chase`, `shoot` o propiedad `other.tag` en una entrega concreta, se marca error de capacidad y la fase NGScript no pasa su gate hasta implementar las funciones requeridas por el fixture. No resolverlo con no-ops silenciosos.

### Ejemplo de vínculo con WorldGraph

```text
TransitionId: T_DungeonGate
from: Village.GateNorth
  to: Dungeon.SpawnSouth
activation: Interact
condition: ScriptHandlerRef(script=DungeonGate, event=on_request)
return type: bool
```

La condición no está duplicada en un `if` específico en C ni en una tabla paralela de lógica. `on_request` devuelve `true` o `false`; el controlador de WorldGraph resuelve el destino y el spawn. Una solicitud fallida no ejecuta transición ni cambia estado. Un guard que intenta iniciar otra transición durante `on_request` debe ser rechazado o aplazado según una regla explícita y probada.

## 4.3. Biblioteca estándar mínima y semántica de eventos

| Constructo | Parámetros canónicos EN | Semántica requerida |
|---|---|---|
| `on_spawn`, `on_update` | contexto del dueño | Inicialización y actualización una vez por frame |
| `on_collision(other)` | `other` referencia tipada | Se ejecuta una vez por evento de colisión según política documentada |
| `on_enter`, `on_exit`, `on_interact` | entidad/trigger si aplica | Eventos asociados a zona u objeto, bindings validados |
| `on_request` | ningún parámetro MVP | Guard `bool` para transición |
| `every(frames)` | entero > 0 | Dispara al cumplirse cada periodo, no cada frame |
| `move_x(speed=...)` | `float` | Actúa sobre componente de movimiento compatible |
| `chase(target=..., speed=...)` o `chase(PLAYER, speed=...)` | target, speed | Requiere AI/movement; el orden elegido debe fijarse y probarse |
| `shoot(asset, target=..., speed=...)` | recurso, target, speed | Valida asset y componentes antes de generar |
| `change_state(StateId)` | estado del mismo entity | Cambia una sola vez y reinicia timers del estado entrante |
| `has_item(id)` | item existente | Devuelve `bool` desde estado persistente de juego |
| `has_flag(id)`, `set_flag(id, value)` | flag definido | Consulta/modifica estado global verificable |
| `return expression;` | tipo de retorno | Solo válido en handlers compatibles |

**Nota:** completar contratos de funciones en `docs/ngscript-stdlib.md`; resolver inconsistencias (p. ej. `chase` posicional vs nombrado) **antes** de escribir fixtures. Los ejemplos del documento no autorizan inventar nombres alternativos con sobrecargas tácitas.

## 4.4. Editor de bloques y texto: una sola semántica

- Editor textual EN (`.ngscript`), autocompletado EN, diagnósticos EN, referencias al grafo y salto a ubicación fuente.
- Bloques visuales de Event/Condition/Action generan **AST NGScript 1.0**, no un formato ejecutable secundario.
- Para edición visual de scripts ya existentes: conservar comentarios/formato cuando sea posible (CST/ediciones con spans), o informar que una operación reformateará el documento antes de aplicarla; evitar pérdida silenciosa de comentarios.
- Guardar siempre `.ngscript` como fuente principal; C generado debe poder regenerarse. No guardar solo bloques visuales sin script ni generar un script «escondido» que el usuario no puede inspeccionar.
- No añadir selector de idioma ni interruptor EN/ES, traducción automática o modo dual.
- El copiloto futuro recibirá instrucciones naturales en cualquier idioma, pero **la salida que compila es solo NGScript inglés validado**. Esto no requiere un traductor sintáctico.

## 4.5. Pipeline único y límites de seguridad

```text
NGScript EN source (.ngscript)
   -> Lexer (tokens/spans)
   -> Parser (AST)
   -> Semantic Analyzer (types, IDs, capabilities, bound owner/event)
   -> IR versioned (optional)
   -> C Code Generator (scripts/<id>.h/.c)
   -> Registry (scripts/ngs_scripts_registry.h)
   -> NewGameRuntime dispatcher
   -> Binary/EXE real
```

- Tokens/error de sintaxis en español como `entidad`, `estado`, `si`, `cada` producen **error con sugerencia EN**, no conversión automática.
- Scripts deterministas; sin memoria arbitraria, API no autorizada, llamadas host o recursión infinita. Límites de instrucciones, profundidad, cantidad de spawns y transiciones por frame.
- `ScriptBinding` asocia owner + event + handler por IDs estables; renombrar un nivel o entidad no rompe bindings.
- Si un script no compila, fallar exportación con diagnóstico; no ejecutar build parcialmente actualizado ni reutilizar artefactos obsoletos sin advertirlo.
- Rutas de archivos validadas para impedir traversal fuera del Workspace. Generación C escapada/estable y sin permitir inyectar C a partir de strings del usuario.

---

# 5. Diseño de la interfaz: no confundir diagrama y escenario

## 5.1. Navegación principal

```text
NesGameStudio
├── WORLD STUDIO             (WorldGraph jerárquico + rutas navegables)
│   ├── Project / Worlds / Regions / Levels / Rooms / Minigames
│   ├── Graph canvas: nodos, conexiones, zoom, filtros
│   ├── Inspector: metadata, gameplay override, spawn/portals
│   └── Provenance: authored / observed / inferred / unknown
├── LEVEL STUDIO             (tilemap real, no screenshot estático)
│   ├── Canvas 2D desplazable, coordenadas mundiales
│   ├── Layers, tiles, metatiles, colisiones
│   ├── Objetos, triggers, spawns y cámara
│   └── Playtest y preview por viewport
├── NGSCRIPT STUDIO          (ONLY ENGLISH)
│   ├── Editor textual .ngscript + errores/tipos
│   ├── Editor visual de eventos/bloques (mismo AST)
│   ├── Bind event → script handler
│   └── Preview de compilación a C (read-only)
├── ASSET STUDIO             (mantener editor gráfico G7)
├── AUDIO / JUKEBOX          (mantener Rockola)
└── BUILD / PLAYTEST / EXPORT
```

## 5.2. Interacción esperada: nuevo juego híbrido

1. **New Game**: nombre, destino del Workspace, target inicial, perfil de ejemplo opcional.
2. **WorldGraph primero**: proyecto con nodo raíz y un nivel inicial; añadir Mundo A (scrolling), pueblo top-down, mazmorra de habitaciones, minijuego shooter y conexiones no secuenciales. Visualizar minimapa y jerarquía.
3. **Abrir nodo**: Level Studio muestra tilemap correspondiente, sin recrear un mundo a partir de screenshots.
4. **Cambiar GameplayProfile**: el nivel o sección cambia cámara/movimiento/física de modo explícito.
5. **Colocar objetos**: puertas, enemigos, triggers, NPC. Propiedades físicas por componentes.
6. **Bind NGScript**: crear `DungeonGate.ngscript` EN, asociar guard `on_request` al portal; asociar `Guard.on_update` a enemigo.
7. **Save/Reopen**: persistir grafo, mapas, recursos, scripts y vínculos.
8. **Build/Play**: motor C nativo ejecuta nivel inicial, controla personaje, permite transición si se cumple la condición; mantiene progreso al entrar a un modo top-down desde scrolling.
9. **Añadir mundo después**: refactor de grafo no invalida IDs ni scripts existentes.

## 5.3. Interacción esperada: ROM existente

1. Importar ROM sin modificar original.
2. Mostrar capturas observadas con título neutral y procedencia.
3. Identificar posibles transiciones/posiciones de cámara y crear candidatos de nodo con estado **Inferred**, nunca «confirmado» por similitud de imágenes.
4. El usuario puede jugar/capturar nuevos estados, confirmar o rechazar relaciones; sus correcciones se registran como `UserConfirmed` distinguible de observación automática.
5. Mapas de secciones conocidas muestran regiones **Unknown** donde no hay evidencias; no completar con tiles inventados.
6. Edición se guarda como overlay independiente; solo ofrecer exportación de ROM original cuando exista mapeo binario validado por el adaptador.
7. Los NGScript añadidos a una importación son nuevas extensiones/overlays; jamás afirmar que son un desensamblado fiel del 6502 original.

---

# 6. Fases, PRs, símbolos y criterios de aceptación

**Regla:** cada fase se entrega como PR o pequeño conjunto de commits autocontenidos, en orden. No crear un único mega-commit. Los archivos «propuestos» NO se consideran existentes hasta verificarlos en HEAD.

## W0 / PR-00 — Protección de baseline, contrato de idioma y contrato de arquitectura

**Localizar:** `PLAN_IMPLEMENTACION.md`, `Cargo.toml`, `src/main.rs::ActiveTab`, `src/project.rs::NgsProject`, `src/workspace.rs::{WorkspacePaths,create_workspace_for_new_game}`, `src/editors/mod.rs`.

**Implementar:**

1. Registrar HEAD/estado de ramas, tests preexistentes y la lista de archivos que no se deben reescribir.
2. Crear `docs/worldgraph-architecture.md` con jerarquía vs rutas, tipos, tres estados de inferencia y targets.
3. Crear `docs/ngscript-language.md` con **NGScript EN ONLY** y gramática v1 (sección 4), marcando los componentes todavía no implementados como PROPOSED.
4. Añadir en `PLAN_IMPLEMENTACION.md` un apartado de **Decisiones de arquitectura** que refiera a ambos documentos, sin borrar objetivos anteriores. Sustituir los ejemplos didácticos de gramática española por referencia a sintaxis EN; no implementar traductores.
5. Crear pruebas negativas de idioma de NGScript una vez exista parser; por ahora un test estático/documental puede verificar que el plan no reclama doble sintaxis, pero **no sustituye prueba de parser**.

**Gate:** documento coherente y estado del repositorio intacto. Ningún cambio funcional a G7, audio o NesEmuRust.

## W1 / PR-01 — WorldGraph, LevelMap y validadores puros

**Localizar:** `NgsProject`, cualquier modelo nuevo `World*`, `Level*`, o `Script*` que haya aparecido en HEAD.

**Archivos propuestos (solo si no existen equivalentes):** `src/world/{mod,model,ids,validation,commands}.rs`, `src/levels/{mod,model}.rs`.

**Implementar:**

1. `WorldDocument`, `WorldNode`, `WorldTransition`, `LevelDefinition`, `LevelLayer`, `MapCell`, `MetatileDefinition`, `PlacedObject`, `GameplayProfile`, `ScriptBinding`.
2. Tipos de ID persistentes, sin usar índices de `Vec` como referencias cruzadas.
3. Comandos puros: create/rename/move/delete node, attach level, connect/disconnect, add spawn, bind/unbind scripts.
4. Validación de contención acíclica y navegación con ciclos legales; duplicados, vínculos rotos, spawns, perfiles y referencias a scripts inexistentes.
5. Tests sintéticos: scrolling + pueblo top-down + habitación + minijuego shooter bajo el mismo grafo; rutas alternativas; renombrados sin rotura.

**NO hacer:** interfaz, código generado ni inferencia de ROM. **Gate:** T01–T10 verdes.

## W2 / PR-02 — Persistencia versionada y migración de Workspaces

**Localizar:** `WorkspacePaths::{from_root,ensure_directories}`, `workspace::{save_workspace,load_workspace,create_workspace_for_new_game,create_workspace_from_existing_game}`, `NgsProject`.

**Cambios:**

1. Agregar directorios nativos `world/`, `levels/`, reutilizar `scripts/` ya existente; por ejemplo `world/world.json`, `levels/<LevelId>.json`, `scripts/<ScriptId>.ngscript`.
2. Serializar datos estables con versión de schema y validación del manifiesto; preferir `serde`/`serde_json` si se justifican y están configurados exclusivamente en NesGameStudio.
3. Escribir de forma segura (temporales + rename cuando sea viable), no truncar datos válidos ante errores parciales, informar errores de I/O.
4. Cargar Workspaces antiguos sin campos `world`; inicializar estructura vacía o mínima **sin convertir capturas en niveles inventados**.
5. Preservar formatos viejos de `CapturedScene`/parches/audio y archivos `.ngscript` existentes byte-for-byte si no se editan.
6. Script bindings con IDs y rutas relativos seguros. Rechazar `../` o rutas absolutas fuera de Workspace.
7. Crear proyecto nuevo con grafo mínimo; su `main_behavior.ngscript` **solo puede contener sintaxis ejecutable cuando W6 la valide**, de lo contrario mantener comentarios explícitamente etiquetados como placeholder.

**Gate:** guardar→cerrar→abrir produce mismos nodos/celdas/IDs/bindings; G7 y audio siguen cargando. T11–T16.

## W3 / PR-03 — World Studio: grafo jerárquico navegable y editable

**Localizar:** `src/main.rs::{ActiveTab,NesGameStudioApp,update}`, `src/editors/mod.rs`.

**Crear/integrar:** `src/editors/world.rs` o equivalente, sin alterar `GraphicsEditorState::ui` salvo un enlace contextual opcional.

**Implementar:**

1. Panel de jerarquía + canvas de grafos con pan/zoom, nodos, puertos, aristas y selección.
2. Crear, renombrar, copiar, mover y borrar mundos/niveles/habitaciones/minijuegos con validación antes de aplicar.
3. Separar visualmente relación de contención de enlace de juego; permitir rutas alternativas, bucles y retornos.
4. Inspector de perfil jugable y transiciones (origen, destino, spawn, activación, guard NGScript pendiente de validación).
5. Estado `Authored`/`Observed`/`Inferred`/`Unknown`; nunca pintar inferencia como certeza.
6. Ubicaciones visuales del grafo se guardan como layout, NO afectan lógica del juego.
7. Doble clic en nodo que posee `LevelId` abre Level Studio; si todavía no está implementado, mostrar «Level Studio pendiente» sin crear contenido falso.

**Gate:** usuario construye mundo no lineal desde la GUI, salva, abre y conserva layout y relaciones; ninguna regresión de menús previos. T17–T20.

## W4 / PR-04 — Level Studio: canvas real de mapas y capas

**Localizar:** `GraphicsEditorState::render_center_interactive_scene`, `render_entity_paint_canvas_tab`, `NgsProject::apply_tile_patch_by_id`, catálogo de recursos en `graphics`.

**Crear/integrar:** `src/editors/levels.rs`, controles de zoom/pan/grilla/layers y edición de celdas del `LevelDefinition`. Mantener `src/editors/graphics.rs` como editor de activos.

**Implementar:**

1. Canvas de nivel con tamaño mayor al viewport; mapa continuo, grid de habitaciones y mapa superior bajo mismo modelo.
2. Paleta de recursos, tiles/metatiles reutilizables, capas, colisiones por celda y parallax como propiedades configurables.
3. Pintar/borrar/reemplazar celdas y deshacer/rehacer con historial de comandos; selección rectangular, copiar/pegar, navegación en coordenadas mundiales.
4. Herramientas **Edit Asset** (recurso compartido) y **Paint Instance** (celda) explícitamente separadas.
5. Zoom y snapping por 8/16/32 píxeles como herramientas UI, sin asumir que todo formato de ROM original utiliza esa unidad lógica.
6. Mantener referencias de tiles por ID; al cambiar celda no copiar bitmap ni escribir PRG.
7. Si el target actual no puede mostrar un recurso HD, advertirlo sin alterar datos.

**Gate:** editar solo una celda no cambia otros usos; scrolling de mapa de más de 256×240; roundtrip. T21–T26.

## W5 / PR-05 — Objetos, componentes, física declarativa y transiciones

**Localizar:** `LevelDefinition`, `WorldTransition`, cualquier nueva implementación de spawn/collision, `GraphicsEditorState`.

**Implementar:**

1. Componentes `Transform`, `Renderer`, `Collider`, `Movement`, `PlayerController`, `Health`, `TriggerRegion`, `Portal`, `NPC`, `Projectile` como vocabulario inicial extensible.
2. Formulario de propiedades por componente; no crear `struct MarioPipe` o `struct ZeldaDoor`.
3. Entradas y salidas tipadas por `PortId`/`SpawnId`; navegación directa y condicional.
4. `GameplayProfile` heredable por nodo/sección; soportar SideScrolling, TopDown, StaticRoom y AutoScroll como presets de sistemas, no géneros excluyentes.
5. `ScriptBinding` desde inspector enlaza un handler NGScript estable. Si W6 aún no existe, mostrar **NOT EXECUTABLE YET**; no introducir `if key == ...` hardcodeado como reemplazo.
6. Validar borrado de portales, destino de puerta, referencias de objetos y estados globales.

**Gate:** escenas con portal y enemigo configurados íntegramente y guardados; ejecución sigue bloqueada de forma honesta hasta W6/W7. T27–T30.

## W6 / PR-06 — NGScript 1.0 English-only: parser, editor y transpilador

**Fuente funcional:** `PLAN_IMPLEMENTACION.md` (concepto de NGScript) y **sección 4 de este plan** (gramática inglesa final).

**Localizar:** `workspace::create_workspace_for_new_game`, `WorkspacePaths::scripts_dir`, `workspace::{save_workspace,load_workspace}`, `ActiveTab`, `src/editors/mod.rs`; comprobar si se agregaron módulos de scripting en HEAD antes de crear archivos.

**Proponer:** `src/ngscript/{mod,lexer,tokens,parser,ast,semantic,diagnostics,stdlib,codegen_c,registry}.rs`, `src/editors/script.rs`, `docs/ngscript-language.md`, `docs/ngscript-stdlib.md`.

**Implementar EN ORDEN:**

1. Definir gramática EN v1 y fixtures *golden* válidos e inválidos; acordar parseo, tipos, precedencia y errores antes de codegen.
2. Lexer con spans, comentarios, strings, números, tokens EN y parser AST determinista.
3. Rechazo explícito de keywords españolas y de tokens ambiguos. No auto-translation, no alias, no dos dialectos ni selector de idioma.
4. Analizador semántico: tipos, identificadores, estados, funciones permitidas, propietarios/eventos, referencias de assets, flags y transición por ID. Diagnósticos con posición de archivo.
5. Builtins definidos por contrato estable y capability matrix por target. Funciones desconocidas o no soportadas → error, nunca no-op.
6. `ScriptBinding` entre owners `Game|World|Level|Object|Trigger|Transition` y handlers por ID; migraciones ante renombre de etiquetas.
7. Generador C99/C11 determinista: `scripts/<script_id>.h` (y `.c` donde sea necesario) + `scripts/ngs_scripts_registry.h`. No tocar manualmente `main.c`; conectar por interfaz estable en el runtime generado.
8. NGScript Studio textual con validación, errores y referencias; editor de bloques con subconjunto equivalente sobre AST, sin runtime paralelo. Si la edición bidireccional no puede conservar comentarios, mostrar advertencia hasta implementar CST/patching de rangos.
9. Tests: `every(90)` exactamente en frames 90,180,270 bajo semántica documentada; `on_collision`, estados, return bool, error de binding inválido, dos scripts sin colisiones de registry, cadenas UTF-8 y escape seguro en C.
10. Revisar `create_workspace_for_new_game()` para que plantilla inicial `.ngscript` sea **EN válida y compilable**; no permitir reemplazo por español.

**Gate:** `.ngscript` EN de ejemplo produce C compilable; español falla con diagnóstico; el registry contiene los handlers correctos; no se declara runtime completo hasta W7. T31–T44.

## W7 / PR-07 — Runtime de juego nuevo y compilación E2E

**Localizar:** `src/codegen.rs::generate_modified_c_file`, `compile_and_run_modified_c`, `workspace::create_workspace_for_new_game`, código fuente C en proyectos nativos, cualquier runtime nuevo ya existente. No consultar NesEmuRust.

**Crear:** `src/codegen/new_game.rs` o equivalente, más plantillas C propias del proyecto. Evitar colisiones con `src/codegen.rs` actual; refactorizar modulando solo si es seguro frente a cambios paralelos.

**Implementar:**

1. Contrato explícito para target **PC native C99/C11**; generar C con mapas, capas, recursos, entidades, WorldGraph, bindings y scripts.
2. Runtime propio de juegos nuevos: loop de frames, entrada, cámara, física/colliders, tilemap, objetos, instanciación y render inicial.
3. Despacho `on_spawn`, `on_update`, `on_collision`, `on_enter`, `on_exit`, `on_interact` y `on_request`; timers deterministas y guard bool.
4. World Transition Manager que valida guard/target/spawn y realiza exactamente una transición por activación, con protección anti-recursión por frame.
5. Al cambiar SideScrolling → TopDown → Shooter, activar/desactivar sistemas/cámara/controles según `GameplayProfile`; conservar flags/ítems globales.
6. Persistir estado básico de juego y estado global entre cambios de nivel; validar reset de estado local/timers.
7. Generar rutas de scripts sin dependencias de filesystem absoluto, reproducibles y seguras. El compilador externo que se use debe estar documentado y su salida de error visible.
8. Separar `export_native_new_game` del exportador original de ROM modificada. No hacer pasar un `.nes` placeholder por un juego nuevo completo ni inventar compatibilidad NES.

**Prueba de aceptación E2E OBLIGATORIA:** proyecto sintético con **Mundo A: scrolling** + **Pueblo top-down** + **minijuego shooter** + **mazmorra como habitación**, rutas no secuenciales, un portal guardado con `on_request` que requiere llave, un enemigo con estado `Patrol→Alert` y disparo temporizado. Guardar/reabrir/generar/compilar/ejecutar, comprobar:

- Spawn correcto, movimiento y colisiones.
- Bloqueo sin llave; transición exitosa con llave; retorno y ruta alternativa.
- Cambio de cámara/control entre topologías.
- Temporizador y `on_collision` realmente invocados desde código NGScript generado.
- Persistencia de flags/progreso al navegar.
- No hay referencias a títulos de juegos ni fallback de datos ficticios.

**Gate:** la prueba se ejecuta en el target soportado. Si no se puede compilar/ejecutar, fase NO COMPLETADA, aunque pase AST o se genere C. T45–T51.

## W8 / PR-08 — ROM Import Adapter y descubrimiento evidenciado del WorldGraph

**Localizar:** `CapturedScene`, `capture_current_live_scene`, `auto_discover_more_scenes_without_playing`, `is_live_frame_visually_distinct`, `parser::populate_real_scenes_and_audio_from_ines`, `workspace::{save_workspace,load_workspace}`.

**Crear:** `src/rom_world/{mod,evidence,scene_matching,transition_detection,graph_builder}.rs` o equivalentes. **Solo dentro de NesGameStudio.**

**Implementar:**

1. `Observation`: captura/estado, origen, controles aplicados cuando se conozcan, identificadores de origen, tiempo de captura y hashes para deduplicar evidencia (NO perfiles curados por ROM).
2. Alineamiento de fondos entre capturas para proponer desplazamientos, con separación de regiones móviles y HUD; reportar incertidumbre y empates.
3. Detectar **candidatos** de transición por observaciones coherentes antes/después; no inferir un portal solo porque cambió toda la imagen.
4. Construcción incremental del WorldGraph con nodos `Observed`, aristas `Inferred` y regiones `Unknown`, con procedencia auditable. `Confirmed` solo con evidencia específica o confirmación explícita del usuario marcada como tal.
5. Flujo de confirmación/rechazo de conexiones y duplicados, corrección manual, trazabilidad de quién originó una relación (`Auto`, `UserConfirmed`).
6. Si no se conoce coordenada mundial, no asignarla arbitrariamente. Un mapa parcial puede contener fragmentos no posicionados.
7. Usar únicamente APIs ya presentes en NesGameStudio para captura, saves y framebuffer. Si falta exposición de nametable/scroll, NO acceder a fuentes de NesEmuRust ni inventar métodos; emplear evidencia RGB limitada y documentar reducción de precisión.
8. Las scripts NGScript que se agreguen a una importación son capas nuevas opcionales: no vender como traducción fiel de rutinas 6502.

**Gate:** fixture sintético con scroll horizontal une fragmentos; animación enemiga no crea nivel; una pantalla desconocida queda no localizada; ROM sin gameplay detectado NO recibe grafo de mundos inventado. T52–T58.

## W9 / PR-09 — Edición/importación retro no destructiva y exportación condicionada

**Localizar:** `GraphicsEditorState::render_entity_paint_canvas_tab`, `NgsProject::apply_tile_patch_by_id`, `codegen::generate_modified_c_file`, mecanismos existentes de patches y overlays.

**Implementar:**

1. Tres rutas separadas en UI/API: `export_new_game_native`, `export_rom_overlay_preview`, `export_verified_legacy_patch`.
2. Cualquier edición en WorldGraph importado se guarda en overlay. No escribir datos originales sin un origen binario validado y reversible.
3. Editar una celda del mapa no modifica globalmente los bytes de una definición compartida. Si hay ambigüedad, marcarla y exigir resolución explícita.
4. Antes de exportar cambios de geometría a ROM original: validar localización, formato, límites de tamaño, recompresión y checksums donde aplique; si falta alguna condición, denegar esa exportación y ofrecer solo overlay.
5. Estado de capacidad visible: `EditableVisualOnly`, `ExecutableOverlay`, `VerifiedNativePatch`, `Unsupported`.
6. Test de idempotencia: ROM original byte-for-byte conservada tras explorar, editar overlays y revertir.

**Gate:** no aparecen «exportado correctamente» o «compatibilidad total» si solo se guardó un overlay. T59–T64.

## W10 / PR-10 — Calidad, documentación y cierre integral

**Localizar:** tests existentes, CLI/build scripts, workflows CI, `PLAN_IMPLEMENTACION.md`, menú Ayuda y guía.

**Implementar:**

1. Test de regresión G7, Rockola, assets HD, Workspaces antiguos y proyectos nuevos.
2. Pruebas multi-topología, script EN-only, migrations, fallas, corrupción de archivos, rutas peligrosas, ciclos de transición y timeouts.
3. Garantizar que guardar/reabrir sin editar no modifica scripts/ROM. Métricas de carga de mapas grandes y rendering de grafo con numerosos nodos; reporte de benchmarks, sin umbrales inventados.
4. Documentar gramática NGScript, stdlib, archivos de workspace, esquema, limitaciones y compatibilidad de exportación por target.
5. Crear al menos dos ejemplos originales de proyecto: `HybridAdventure` (scrolling+top-down+guard NGScript) y `RoomQuest` (habitaciones RPG+flags+NPC). No copiar assets de juegos comerciales.
6. Validar gates finales: build y ejecución real, no solo mocks. Actualizar roadmap maestro solo con capacidades demostradas.

**Gate:** todas las pruebas necesarias pasan y limitaciones conocidas figuran explícitamente. T65–T70.

---

# 7. Matriz de pruebas de aceptación verificables

> Los números Txx son IDs de prueba estables del **plan**, no implican que ya exista un test compilado. El agente debe darles implementación portable con fixtures de proyecto/ROM sintéticos propios y assertions sobre resultados observables.

| ID | Subsistema | Escenario concreto | Resultado de aceptación |
|---|---|---|---|
| T01 | WorldGraph | Juego con 3 mundos/6 niveles | IDs únicos, jerarquía válida |
| T02 | WorldGraph | A→B→A | Ciclo de navegación permitido |
| T03 | WorldGraph | Mundo A contiene Mundo B que contiene A | Ciclo jerárquico rechazado |
| T04 | WorldGraph | A→B, A→C secreto, C→B | Rutas alternativas modeladas |
| T05 | WorldGraph | Mover nodo de padre y renombrarlo | `NodeId` se mantiene |
| T06 | WorldGraph | Eliminar nodo destino de portal | Diagnóstico de referencia rota o rechazo transaccional |
| T07 | WorldGraph | Entrada a spawn inexistente | Validador falla con ID concreto |
| T08 | Gameplay | Un mundo scrolling y otro top-down | Ambos perfiles aceptados en mismo juego |
| T09 | Gameplay | Sección override de cámara | Resuelve herencia determinísticamente |
| T10 | WorldGraph | Dos niveles con nombre idéntico | No se confunden al usar IDs estables |
| T11 | Workspace | Save→load grafo completo | Roundtrip sin pérdida |
| T12 | Workspace | Archivo legado sin `world/` | Carga sin inventar niveles |
| T13 | Workspace | Guardar sin editar | ROM y `.ngscript` sin cambios de contenido |
| T14 | Workspace | Ruta de script `../../escape` | Rechazo, no escritura externa |
| T15 | Workspace | Manifest truncado | Error explícito, sin reemplazar proyecto válido |
| T16 | Workspace | Cargar G7/Audio desde proyecto legado | Conserva escenas, parches, Rockola |
| T17 | World Studio | Crear y mover nodos por GUI | Jerarquía y aristas correctas |
| T18 | World Studio | Zoom/pan y reordenamiento de canvas | No altera transiciones de juego |
| T19 | World Studio | Crear 2 portales con mismo destino | Ambos permanecen distintos |
| T20 | World Studio | Guardar layout de grafos | Persiste al reabrir |
| T21 | Level Studio | Mapa 1024×240 | Navegación y edición más allá del viewport 256×240 |
| T22 | Level Studio | Pintar una sola celda entre 20 instancias | Solo cambia esa instancia |
| T23 | Level Studio | Editar definición compartida | Cambian instancias solo cuando referencian esa definición |
| T24 | Level Studio | Undo/redo multi-capa | Estado exacto restaurado |
| T25 | Level Studio | Metatile no contiguo en VRAM | Se representa explícitamente, no por suposición |
| T26 | Level Studio | Guardar/reabrir capas, objetos y colisiones | Datos idénticos |
| T27 | Components | Portal con dos `SpawnId` válidos | Destino seleccionado correctamente |
| T28 | Components | Enemigo con `Movement`,`Collider`,`Health` | Propiedades tipadas y persistidas |
| T29 | Components | Nodo top-down desde scrolling | Perfil de movimiento cambia sin destruir estado global |
| T30 | Components | Binding apunta a script inexistente | Error visible, no callback inventado |
| T31 | NGScript EN | Fixture `entity Guard { ... }` | AST tipado esperado |
| T32 | NGScript EN | Fixture `transition DungeonGate { ... }` | Guard `on_request` retorna bool |
| T33 | NGScript EN | `every(90)` en 270 frames | Dispara en 90,180,270 exactamente |
| T34 | NGScript EN | `if (health < 50)` | Transición de estado correcta |
| T35 | NGScript EN | `on_collision(other)` | Handler correcto; no doble dispatch accidental |
| T36 | NGScript EN | `entidad Guard {}` | Parser lo rechaza; no alias automático |
| T37 | NGScript EN | `estado Patrol {}`, `si (...)` | Parser rechaza keywords español |
| T38 | NGScript EN | `mover_x`/`disparar` invocados | No se traducen: símbolo no encontrado |
| T39 | NGScript EN | No hay scripts bilingües/UI selector | Inspección estática y tests de idioma |
| T40 | NGScript EN | Strings `"¡Hola mundo!"` | Cadenas UTF-8 válidas, no modifica sintaxis |
| T41 | NGScript | Referencias a LevelId/ObjectId renombrados | Binding conserva ID y compila |
| T42 | NGScript | Sintaxis rota con línea/columna | Error preciso, sin C parcial ejecutable |
| T43 | NGScript | Dos entidades y 2 scripts de igual handler label | Registry inequívoco por ID |
| T44 | NGScript | AST visual vs AST textual compatible | Igual semántica y C generado |
| T45 | Runtime | Project new: A scrolling→B top-down | Arranca, camina, cambia cámara/controles |
| T46 | Runtime | Porta guard `has_item` sin ítem | Bloquea transición, no mueve player |
| T47 | Runtime | Porta guard con ítem | Transita y usa `SpawnId` correcto |
| T48 | Runtime | Regreso B→A y opción secreta | Ciclos controlados, ruta no lineal |
| T49 | Runtime | Guard enemigo dispara por NGScript | Código ejecutado y entidad aparece |
| T50 | Runtime | Estado e inventario global | Persisten al cambiar nivel |
| T51 | Runtime | Repetir build sin cambios | Artefactos deterministas o diferencias justificadas |
| T52 | ROM recon | Capturas de scrolling solapadas | Alineación y zonas conocidas |
| T53 | ROM recon | Enemigo animado sin cambio del escenario | No crea nodo confirmado |
| T54 | ROM recon | Imagen cambiada sin evidencia de portal | Transición solo candidata, no confirmada |
| T55 | ROM recon | Pantalla no posicionable | Se conserva como fragmento aislado/Unknown |
| T56 | ROM recon | ROM renombrada | Igual grafo inferido, salvo etiquetas inocuas |
| T57 | ROM recon | ROM en menú sin gameplay | No inventar niveles |
| T58 | ROM recon | Usuario confirma conexión | `UserConfirmed` registrado con procedencia |
| T59 | Legacy edit | Edición visual de celda importada | No altera ROM base |
| T60 | Legacy edit | Overlay revertido | Estado original recuperable |
| T61 | Legacy edit | Falta mapping PRG confirmado | Export ROM patch bloqueado |
| T62 | Legacy edit | Varias coincidencias de firma CHR | No parches automáticos destructivos por similitud |
| T63 | Legacy edit | Export overlay | Etiqueta `Preview/Overlay`, no `ROM completa` |
| T64 | Legacy edit | Script nuevo sobre ROM importada | Marcado como extensión, no traducción 6502 |
| T65 | Regresión | Abrir y usar editor G7 | Sigue funcional |
| T66 | Regresión | Abrir Rockola | Sigue funcional; sin cambios en lógica de audio |
| T67 | Regresión | Workspace antiguo con `main_behavior.ngscript` solo comentarios | No presenta comentarios como lógica ejecutable |
| T68 | Calidad | 500 nodos de grafo + mapa de gran tamaño | Sin bloqueo incontrolado; medir tiempos reales |
| T69 | Calidad | Script intenta reentradas A→B→A mismo frame | Límite/diagnóstico, sin loop infinito |
| T70 | Calidad | Build PC runtime con script inválido | Falla con diagnóstico; no ejecuta binario obsoleto |

**Cobertura mínima de entregables:** tests unitarios de modelo, persistencia y parser; tests de integración de UI/comandos cuando sea posible; E2E del runtime nativo en target real. Documentar tests dependientes del entorno que no se puedan ejecutar.

---

# 8. Formato de datos del Workspace y ejemplo mínimo

**Propuesta de directorios, no asumir que existe:**

```text
MyGame_Workspace/
├── project.ngs.json                  # manifest existente: versionar con compatibilidad
├── base/                             # ROM / fuentes originales; NUNCA editar original por inferencia
├── world/
│   ├── world.json                    # WorldDocument, nodos y transiciones
│   └── graph_layout.json             # posiciones UI, opcional
├── levels/
│   ├── LV_FOREST.json                # definiciones de niveles por ID estable
│   ├── LV_VILLAGE.json
│   └── LV_DUNGEON.json
├── assets/
│   ├── graphics/                     # G7/HD existente
│   └── audio/                        # Rockola existente
├── scenes/                           # CapturedScene, evidencia retro
├── scripts/
│   ├── Guard.ngscript                # ONLY ENGLISH
│   ├── DungeonGate.ngscript
│   └── main_behavior.ngscript
├── src/                               # C generado/reutilizado según ruta
└── build/                             # ejecutables y registro de exportación
```

**Persistencia:** `schema_version` explícito, IDs serializados estables, referencias verificadas y migración compatible. `world.json` no debe contener 3 MB de framebuffer de escenas; la evidencia va por separado.

**No almacenar la misma lógica dos veces:** las condiciones visuales del WorldGraph referencian `ScriptBinding` y el handler NGScript; los bloques del editor visual no se guardan como otro código independiente que pueda desincronizarse.

---

# 9. Definición de targets y honestidad de exportación

| Target | Estado que se acepta al finalizar W7 | Qué NO se puede prometer |
|---|---|---|
| **PC Native New Game (C99/C11)** | Proyecto nuevo compilado/ejecutado con mundo/niveles/NGScript funcionales | Que funciona por haber emitido `.h` o mostrado un preview |
| **ROM Original NES Importada** | Visualización e inferencia parcial + edición overlay con procedencia | Recuperación universal de compresión/niveles o recompilación fiel |
| **ROM NES Nueva 6502** | Separado; futuro backend si hay pruebas de encoding/runtime y límites de hardware | No equiparar una ROM base sintética con exportador universal de juegos nuevos |
| **Hybrid/HD/2.5D** | Evolución futura conforme al plan maestro de renderer | No mezclar el gate de W7 con promesas sobre GPU, consolas o shader avanzados |

El plan maestro contempla un runtime exportado C99/C11 y `ngs_engine_step_frame` con input, cámara, scripts, colisiones, VFX, render y audio. **Estos son requisitos por desarrollar, no capacidades que este plan asuma terminadas.** Se progresa con un subconjunto jugable vertical que respeta esa separación.

---

# 10. Riesgos relevantes y mitigaciones

| Riesgo | Mitigación obligatoria |
|---|---|
| Sobrescribir G7 o Rockola por trabajo paralelo | Rama separada, inspección HEAD/diff, integración por módulos nuevos, regresión T65/T66 |
| Inflar interfaz sin runtime | Gates W6/W7: parser + C + binario ejecutado, no solo botones |
| NGScript español reaparece por plan histórico | W0 actualiza decisiones; T36–T39 rechazo real por parser |
| Duplicar lógica WorldGraph vs NGScript | WorldGraph referencias tipadas; ejecución solo desde NGScript + servicios del motor |
| Confundir pantalla con nivel | `CapturedScene` solo evidencia; `LevelMap` nativo separado |
| Inferir de forma falsa un mapa completo | Estados Confirmed/Inferred/Unknown, fuentes auditables y pruebas negativas |
| PRG corrupta por heurísticas de gráficos | Overlay por defecto; export nativo sólo con mapeo verificable |
| Componentes específicos por videojuego | Componentes y perfiles genéricos, tests sintéticos no comerciales |
| Scripts maliciosos o loops infinitos | No eval ni llamadas nativas arbitrarias; límites de ejecución y rutas seguras |
| Scripts viejos de plantilla «solo comentarios» | Mantener texto intacto, no declararlos ejecutables sin parser |
| Perfil híbrido no ejecutable | Declarar capability matrix por target y gate de integración |
| Exportar `.NES` nuevo como placeholder | Mostrar target real; no prometer backend NES sin build y tests reales |
| Dependencia externa no disponible en CI | Reportar limitación; no inspeccionar NesEmuRust ni falsificar test |

---

# 11. Orden de entrega y condiciones de avance

| Orden | PR | Entrega demostrable | Gate |
|---:|---|---|---|
| 0 | W0 | Contratos y baseline sin cambios funcionales | Doc + estado repo |
| 1 | W1 | Grafo y mapas genéricos validados | T01–T10 |
| 2 | W2 | Persistencia y migración | T11–T16 |
| 3 | W3 | WorldGraph visual navegable/editable | T17–T20 |
| 4 | W4 | Level Studio de tiles reales | T21–T26 |
| 5 | W5 | Objetos, componentes, portales, gameplay profiles | T27–T30 |
| 6 | W6 | NGScript EN parser + AST + C + registry | T31–T44 |
| 7 | W7 | Primer juego NUEVO completo y ejecutable | T45–T51 |
| 8 | W8 | Reconstrucción retro con evidencia | T52–T58 |
| 9 | W9 | Overlay retro y export seguro | T59–T64 |
| 10 | W10 | Integración, calidad y guía | T65–T70 |

**Gates críticos:**

- No saltar de W1 a toda la interfaz sin W2: la persistencia es fuente de verdad.
- No anunciar «Behavior Editor» funcional sin parser/semántica y C real.
- No aprobar W7 sin un juego híbrido realmente ejecutado con NGScript y transiciones.
- No anunciar «ROM reconstruida» sin procedencia y estado de incertidumbre.
- No mezclar W8/W9 con la ruta nativa de juegos nuevos ni con API no verificada de NesEmuRust.

---

# 12. Primer mensaje exacto para el agente de programación

Copiar el bloque siguiente como instrucción inicial. **Entregar el documento completo como anexo y pedir SOLO W0 + W1 inicialmente.**

> Vas a desarrollar **NesGameStudio 4.0: WorldGraph + Level Studio + NGScript English-only**, siguiendo íntegramente el plan adjunto.
>
> 1. Trabaja EXCLUSIVAMENTE en `iztaneo/NesGameStudio`. Tienes prohibido abrir, inspeccionar o editar NesEmuRust. No inventes funciones externas.
> 2. Antes de cambiar código, confirma HEAD, branch, git status y los símbolos actuales. La referencia `e63523e` es histórica; NO uses números de línea ni restaures archivos anteriores a cambios de otros agentes.
> 3. Debes preservar el editor gráfico G7, la Rockola, Workspaces, plantillas y código anterior.
> 4. `NGScript` será exclusivamente ENGLISH: keywords, event names, stdlib y named arguments en inglés. Sin traducciones, aliases en español, conversor o modo bilingüe. Extensión `.ngscript`, compilación a C99/C11 y registro de scripts, conforme al plan maestro actualizado.
> 5. El **WorldGraph** es la estructura central: jerarquía de mundos/niveles/habitaciones, conexiones no lineales y gameplay profiles combinables (scrolling, top-down, RPG/shooter). La lógica programable pertenece a NGScript, no a condiciones C por juego.
> 6. Primero implementa **W0 y W1**, estrictamente. Entrega firmas verificadas, modelo, validadores, tests unitarios, diff y límites. **No avances W2–W10 hasta superar el gate correspondiente.**
> 7. No declares implementado nada que no hayas compilado/probado; reporta bloqueos y pruebas no ejecutadas como tales.
>
> **Formato de respuesta obligatorio:** Base HEAD; worktree; símbolos reales encontrados; cambios; tests red/green; archivos tocados; evidencia; riesgos; gate PASS/FAIL.

---

# 13. Criterios finales de aceptación del producto

Para considerar que la **fase de creación de juegos nuevos** está terminada (W0–W7):

1. Se puede empezar un proyecto diseñando el WorldGraph y después construir cada nivel.
2. El grafo soporta jerarquía, rutas alternativas, bucles de navegación, nodos de habitación y minijuegos.
3. Un mismo juego mezcla scrolling lateral, vista superior y un modo adicional, con perfil por nivel/sección.
4. Level Studio permite editar posiciones y bloques, colisiones, capas, spawn y objetos sin tocar otras instancias accidentalmente.
5. NGScript es **SOLO INGLÉS**, con parser/semántica, bind de eventos, editor visual/textual sobre un AST común y transpilación real a C.
6. Guardar/reabrir conserva todos los datos, scripts, IDs, relaciones, cambios y estado declarativo.
7. La ejecución nativa en target soportado demuestra cambios de nivel, condición de puerta, comportamiento temporizado de enemigo, colisiones y persistencia.
8. Ninguna rama por nombre de juego ni modificación a NesEmuRust fue necesaria.
9. El editor G7, Rockola y Workspaces legados continúan funcionando.

Para considerar que la **fase de ROM importada** está terminada (W8–W9):

1. Se muestra un grafo parcial de observaciones/inferencias, no una ficción de juego completo.
2. Se distinguen transiciones observadas, inferidas y confirmadas por el usuario.
3. Las zonas sin información permanecen desconocidas; el usuario puede completarlas con evidencia o confirmaciones explícitas.
4. La ROM original permanece intacta; exportaciones se limitan a capacidades verificadas.
5. NGScript añadido a una importación queda etiquetado como lógica nueva, no conversión automática de 6502.

---

# 14. Fuentes verificadas y limitaciones de esta auditoría

- **Repositorio:** https://github.com/iztaneo/NesGameStudio
- **Último HEAD consultado al redactar:** `e63523ea79002e446c24a20faabdf24ccd82d150`.
- **Plan maestro:** https://github.com/iztaneo/NesGameStudio/blob/e63523ea79002e446c24a20faabdf24ccd82d150/PLAN_IMPLEMENTACION.md
- **G7:** https://github.com/iztaneo/NesGameStudio/commit/e63523ea79002e446c24a20faabdf24ccd82d150
- **Baseline gráfico G0-G6:** https://github.com/iztaneo/NesGameStudio/commit/e968b3b30422ef0ab698e63e14215f7200f46e76
- **En NesGameStudio se verificaron símbolos de:** `src/main.rs`, `src/editors/mod.rs`, `src/editors/graphics.rs`, `src/graphics/metasprite.rs`, `src/project.rs`, `src/parser.rs`, `src/workspace.rs`, `src/codegen.rs`, `Cargo.toml` y `PLAN_IMPLEMENTACION.md`.
- **No se inspeccionó ni modificó NesEmuRust para redactar esta versión.** Las APIs externas son dependencias a tratar como caja negra desde NesGameStudio.
- **No se compiló, ejecutó ni modificó código del repositorio durante la redacción del plan.** Las pruebas T01–T70 son criterios por implementar. Los modelos y ejemplos son contratos propuestos, no afirmaciones de funcionalidad ya existente.

**DECISIÓN FINAL INNEGOCIABLE:** `WorldGraph` diseña/representa el videojuego; `Level Studio` construye escenarios; **`NGScript` English-only** define comportamientos; el runtime nativo los ejecuta; la reconstrucción de ROMs se limita a evidencias reales. **NesEmuRust permanece totalmente fuera del trabajo.**
