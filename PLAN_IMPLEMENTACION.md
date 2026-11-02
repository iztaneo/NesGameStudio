# `NesGameStudio` — Diseño Arquitectónico y Plan de Implementación

**`NesGameStudio`** es un entorno de desarrollo visual (IDE de Modding y Remasterización) independiente diseñado para cargar los archivos **`.c`** y **`.asm`** generados por [`NesEmuRust`](https://github.com/iztaneo/NesEmuRust), permitiendo transformar, expandir y modernizar cualquier juego clásico de NES en un juego nativo de PC (**x86/x64**) de alta calidad **de forma 100% visual, sin tener que entender ni editar manualmente código C ni ensamblador 6502**.

---

## 1. Filosofía y Arquitectura de Lenguaje Base (Híbrida: Rust + C)

### 1.1. Decisión de Lenguaje Base: **Arquitectura Híbrida (Editor en Rust + Juegos Generados en C)**
Para obtener la máxima productividad en la interfaz gráfica y la máxima velocidad de compilación en los juegos finales, **`NesGameStudio`** utiliza un modelo híbrido de dos capas:

| Capa del Sistema | Lenguaje y Tecnologías Base | Justificación Técnica |
| :--- | :--- | :--- |
| **1. El Editor / IDE Visual (`NesGameStudio`)** | **Rust (`Edition 2021`)** + `eframe`/`egui` (GUI), `cpal` (Audio en vivo), `image` (Carga `.PNG`/`.BMP`), `serde_json` (Proyectos `.ngs.json`) | Permite reutilizar directamente los núcleos de CPU, PPU y APU de `NesEmuRust` para la Vista Previa y la Rockola Aislada, manipular archivos de código `.c`/`.asm` de decenas de miles de líneas con seguridad de memoria absoluta y construir paneles visuales complejos rápidamente. |
| **2. El Motor de los Juegos Exportados (`Runtime Target`)** | **C Estándar Puro (`C99/C11`)** + `Win32 API` (`user32.lib`, `gdi32.lib`, `winmm.lib`) compilado vía `cl.exe /O2`, `gcc` o `clang` | El código que `NesGameStudio` inyecta o genera desde cero es **100% C puro sin dependencias externas**. Compila en ~1 segundo a un ejecutable `.exe` nativo ultraligero (`< 2 MB`) que corre en cualquier PC Windows sin instalar runtimes. |

### 1.2. Edición 100% Visual e Inyección Automática en C
- **El usuario nunca necesita editar código a mano**: Todas las mejoras (gráficos HD, capas Parallax, niveles, música `.MP3`/`.WAV` y reglas del juego) se realizan mediante herramientas visuales (pinceles, selectores de archivos, reproductor de música y lienzos de mapas), y **`NesGameStudio` modifica y recompila el archivo `.c` automáticamente por detrás**.
- **Exploración Total de Recursos**: Al abrir un juego (`.c` / `.asm`), `NesGameStudio` permite **visualizar todos los gráficos del juego** en una galería interactiva y **reproducir aisladamente cada canción y efecto de sonido** del juego en una rockola integrada (*Sound Test / Jukebox*).

---

## 2. Diagrama de Arquitectura de `NesGameStudio`

```mermaid
flowchart TD
    subgraph Entrada["1. Archivos de Entrada (Generados por NesEmuRust)"]
        CSRC["Código Fuente Base (Juego.c)"]
        ASMSRC["Desensamblado Comentado (Juego.asm)"]
    end

    subgraph NucleoStudio["2. Núcleo de NesGameStudio (Aplicación GUI en Rust + egui)"]
        PARSER["Analizador Estructural de Juego.c y Juego.asm (PRG_ROM / CHR_MEM / Rutinas)"]
        PREVIEW["Motor de Ejecución y Sondeo Aislado (Video 60 FPS + Motor APU Independiente)"]
        PROJ["Proyecto de Estudio (Juego.ngs.json + Carpeta assets/)"]
    end

    subgraph Modulos["3. Los 4 Módulos Visuales de Edición"]
        M_GFX["Módulo 1: Galería Total de Gráficos y Estudio HD (CHR + Sprites 32-bit RGBA)"]
        M_LVL["Módulo 2: Editor de Pantallas, Mapas y Nuevos Niveles en C"]
        M_AUD["Módulo 3: Rockola Aislada de Música/SFX + Inyector .MP3 / .WAV"]
        M_RAM["Módulo 4: Buscador de Variables RAM + Visor Sincronizado .ASM <-> .C"]
    end

    subgraph Salida["4. Inyector Automático C y Compilador Nativo"]
        INJECT["Cirujano de Código C (Inyecta Hooks HD, Audio, Niveles y Reglas sin intervención manual)"]
        COMPILER["Compilador Nativo (MSVC cl.exe / GCC / Clang)"]
        FINAL_EXE["Juego Remasterizado Independiente (Juego_Remaster_x64.exe + assets/)"]
    end

    CSRC --> PARSER
    ASMSRC --> PARSER
    PARSER <--> PREVIEW
    PARSER <--> PROJ

    PREVIEW <--> M_GFX
    PREVIEW <--> M_LVL
    PREVIEW <--> M_AUD
    PREVIEW <--> M_RAM

    M_GFX --> INJECT
    M_LVL --> INJECT
    M_AUD --> INJECT
    M_RAM --> INJECT

    INJECT -->|"Reescribe Juego_Mod.c automáticamente"| COMPILER
    COMPILER --> FINAL_EXE
```

---

## 3. Diseño Detallado de los 4 Módulos Principales

### Módulo 1: Galería Completa de Gráficos y Estudio HD (`🎨 Todos los Gráficos y Sprites HD`)
Permite **visualizar y editar todos los gráficos contenidos en el juego** sin ver un solo número hexadecimal:

1. **Explorador Completo de Bancos Gráficos (*Full Game Graphic Browser*)**:
   - **Juegos con `CHR-ROM`** (*Super Mario Bros.*, *Mario 3*, *Castlevania*, etc.): Al abrir el archivo `.c`, `NesGameStudio` decodifica inmediatamente **todos los bancos de `CHR_MEM[]`** y muestra una galería completa con todos los personajes, enemigos de todos los niveles, jefes, objetos y bloques del juego sin necesidad de jugar hasta ese nivel.
   - **Juegos con `CHR-RAM`** (*Contra*, *Zelda*, *Mega Man*): Combina un escáner de patrones gráficos dentro de `PRG_ROM[]` con un **Catálogo de Capturas por Nivel**, permitiendo explorar y guardar en la galería todas las hojas de sprites descomprimidas de cada fase.
   - **Selector de Paletas y Armador de Metasprites**: Permite alternar entre todas las paletas de colores detectadas en el juego y ver los personajes completos (`16×32`, `32×32`) armados a partir de sus bloques de `8×8` u `8×16`.
2. **Edición Visual y Reemplazo por Gráficos HD (32-bit RGBA)**:
   - **Modo Pixel Art (8-bit)**: Pincel visual y botón **"Exportar / Importar `.BMP`"** para editar en Paint, GIMP o Aseprite.
   - **Modo Remaster HD (16×16 / 32×32 TrueColor)**: Seleccionas cualquier sprite o bloque en la galería y eliges una imagen `.PNG` / `.BMP` de **32 bits (16.7 millones de colores + transparencia alfa)**.
3. **Inyección Automática en el `.c`**:
   - `NesGameStudio` modifica automáticamente `ppu_render_scanline()` en el archivo `.c`, amplía el `FRAMEBUFFER` a `512×480` (`2x`) o `1024×960` (`4x`) e inserta el código C que dibuja tus nuevos gráficos HD.

---

### Módulo 2: Editor de Pantallas y Creador de Niveles (`🗺️ Pantallas y Niveles C`)
Permite editar los escenarios existentes o agregar niveles completamente nuevos con el mouse:

```mermaid
flowchart LR
    A["Lienzo Visual del Escenario"] --> P0["Capas Traseras Parallax (0.1x, 0.3x, 0.6x - Cielo, Montañas, Selva Lejana)"]
    A --> B["Capa Jugable: NameTable Original + Sprites (1.0x)"]
    A --> C["Capa Overlay en C: Nuevos Enemigos, Plataformas y Jefes HD (1.0x)"]
    A --> PF["Capas Delanteras / Foreground (1.3x + Transparencia Alfa: Niebla, Lluvia, Árboles en 1er plano)"]
    A --> D["Nivel Nuevo 100% en C (Mapa Libre Multicapa)"]
```

1. **Sistema de Profundidad Multicapa (*Unlimited Parallax & Foreground Layers*)**:
   - La NES original solo soportaba **1 capa de fondo** sin transparencia real. En `NesGameStudio` puedes agregar **múltiples capas de profundidad superpuestas** (tanto detrás del escenario como delante del jugador):
     - **Capas de Fondo (*Back Parallax*, velocidad `0.0x` a `0.9x`)**: Se dibujan automáticamente detrás de los píxeles transparentes (`pixel == 0`) del escenario original, desplazándose de forma suave según la cámara acumulada del juego (`camera_world_x * factor_parallax`).
     - **Capas de Primer Plano (*Foreground + Alpha Blending*, velocidad `1.1x` a `2.0x`)**: Se dibujan **por encima de los sprites y del escenario** usando canal Alfa de 32 bits (`RGBA`), permitiendo efectos modernos como neblina translúcida, lluvia, haces de luz (*god rays*), columnas o follaje que pasa frente a la cámara.
2. **Editor Visual de Pantallas (`NameTables` + Atributos)**:
   - Muestra el escenario como un lienzo interactivo donde tomas bloques de la Galería Gráfica y los pintas directamente sobre el mapa.
3. **Capa de Expansión en C (*Entity Overlay*)**:
   - Arrastras y sueltas **nuevos objetos, plataformas móviles, torretas o jefes HD** sobre cualquier nivel existente.
   - `NesGameStudio` escribe automáticamente las estructuras en C (`struct CustomEntity`), su física en punto flotante (`float`) y sus colisiones dentro del `.c`.
4. **Creador de Niveles Nuevos (*Custom Stages*)**:
   - Diseñador de niveles desde cero sin el límite de 2 KB de RAM ni de 4 colores de la NES. `NesGameStudio` genera automáticamente la función `run_custom_stage_c()` y la conecta al flujo del juego en el archivo `.c`.

---

### Módulo 3: Rockola Aislada de Música/Efectos e Inyector `.MP3` / `.WAV` (`🎵 Sound Test & Audio Studio`)
Permite **escuchar aisladamente cada canción y efecto de sonido del juego** y reemplazarlos con un clic:

```mermaid
flowchart LR
    A["Detector de Rutina de Sonido ([AUDIO / APU])"] --> B["Rockola Aislada (Play / Stop Track #00..#FF)"]
    B --> C["Control por Canal (Solo / Mute: Pulse 1, Pulse 2, Triangle, Noise, DMC)"]
    B --> D["Exportar Canción Original a .WAV"]
    B --> E["Botón: Reemplazar este Track por archivo .MP3 o .WAV HD"]
```

1. **Reproductor Aislado de Música y Efectos (*Standalone Sound Test / Jukebox*)**:
   - `NesGameStudio` localiza las subrutinas clasificadas como `[AUDIO / APU]` en el `.asm`/`.c` y ejecuta el motor de sonido del juego **en un entorno aislado (sin correr el resto del juego)**.
   - Muestra una lista de pistas (`Track #00`, `Track #01`, `Track #02`...) con botones **▶️ Play**, **⏸️ Pausa** y **⏹️ Stop** para que puedas escuchar aisladamente cada canción de nivel, música de jefe o efecto de disparo/explosión.
   - Incluye botones **Solo / Mute por cada uno de los 5 canales** (`Pulse 1`, `Pulse 2`, `Triangle`, `Noise`, `DMC`), visualizador de ondas/notas en vivo y opción para **exportar cualquier canción o efecto original del juego a un archivo `.WAV`**.
2. **Reemplazo Visual por `.MP3` o `.WAV` de Alta Calidad**:
   - Al escuchar una pista en la rockola (por ejemplo, descubres que el `Track #01` es la música de la Jungla en *Contra* y el `Track #12` es el sonido del disparo), simplemente haces clic en **"🔁 Reemplazar esta pista por `.MP3` / `.WAV`"**.
   - Eliges tu archivo `.mp3` o `.wav` del disco y `NesGameStudio` inyecta automáticamente en el código `.c` el interceptor que silencia esa pista de 8 bits y reproduce tu archivo `.mp3` (vía `mciSendStringA`) o mezcla tu `.wav` polifónico a `44.1 kHz` cuando el juego solicite ese sonido.

---

### Módulo 4: Buscador de Variables RAM y Modificador Visual de Reglas (`🔍 Variables y Reglas`)
Permite cambiar vidas, armas, velocidades, gravedad o trucos mediante controles visuales:

1. **Escáner de Variables en Tiempo Real (*Variable Finder*)**:
   - Filtras valores (`3` vidas -> `2` vidas) en la vista previa y descubres al instante qué casilla de `RAM[]` controla cada elemento del juego.
2. **Panel de Reglas y Trucos sin Código**:
   - Le asignas un nombre (`VIDAS_JUGADOR`, `ARMA_ACTUAL`) y defines reglas con menús desplegables (*"Valor inicial = 30"*, *"Congelar siempre en Spread Gun"*, *"Al presionar tecla C dar invencibilidad"*).
   - `NesGameStudio` inyecta automáticamente ese código C en `nes_step_one_frame_60hz()`.
3. **Visor Sincronizado `.ASM` <-> `.C` (Radiografía Opcional)**:
   - Muestra lado a lado el ensamblador y el código C generado con casillas para activar/desactivar instrucciones específicas con un clic.

### Módulo 5: Creador de Juegos Nuevos desde Cero (`🚀 Motor Gráfico 2D/2.5D, Colisiones, VFX, IA y Scripts`)
Convierte a `NesGameStudio` en un **"Mini-Unity" especializado en juegos 2D / 2.5D Retro y HD**, separando estrictamente el **programa principal (`main.c`, que nunca se toca)** de los **sistemas del motor (`engine/`)** y de los **scripts de cada personaje o nivel (`scripts/`)** que se traducen a C y se conectan vía `#include`:

```mermaid
flowchart TD
    MAIN["main.c (Intacto: Ventana Win32 + Game Loop 60 FPS + Audio waveOut)"] --> ENG["engine/ (Librerías Base en C del Motor)"]
    MAIN --> SCR["scripts/ (Scripts generados/traducidos a C e incluidos con #include)"]

    subgraph Motores["Sistemas del Motor (engine/)"]
        COL["ngs_collision.h (Hitboxes AABB, Tilemap Sólido, Plataformas One-Way, Eventos on_collision)"]
        VFX["ngs_vfx_alpha.h (Alpha Blending 0..255, Luz Aditiva, Emisor de Partículas, Screen Shake)"]
        AI["ngs_ai_behaviors.h (Patrulla, Persecución, Disparo en 8 dir, Máquinas de Estado de Jefes)"]
    end

    subgraph ScriptsJuego["Scripts Modulares del Juego (scripts/)"]
        S_PL["player_script.h (on_spawn, on_update, on_collision)"]
        S_EN["enemies_script.h (IA de enemigos y jefes traducida a C)"]
        S_ST["stage_01_script.h (Mapa, Capas Parallax y Triggers del nivel)"]
    end

    ENG --> COL
    ENG --> VFX
    ENG --> AI
    SCR --> S_PL
    SCR --> S_EN
    SCR --> S_ST
```

1. **Arquitectura de Scripts Traducidos e Incluidos (Sin tocar `main.c`)**:
   - Cada personaje, enemigo, jefe o nivel tiene su propio script modular (con eventos estilo Unity: `on_spawn`, `on_update`, `on_collision`, `on_destroy`).
   - `NesGameStudio` traduce las reglas visuales o scripts de alto nivel a archivos `.h` independientes dentro de `scripts/` y los enlaza automáticamente en el juego sin alterar el núcleo de `main.c`.

2. **Sistema de Detección de Colisiones (`engine/ngs_collision.h`)**:
   - **Colisiones de Escenario (*Tilemap Colliders*)**: En el editor pintas qué cuadros del mapa son **Sólidos (`SOLID`)**, **Plataformas Atravesables desde abajo (`ONE_WAY`, estilo *Contra*)**, **Escaleras (`LADDER`)**, **Agua (`WATER`)** o **Trampas de Daño (`HAZARD`)**. El motor resuelve automáticamente la gravedad, pendientes y topes de pared.
   - **Cajas de Colisión por Entidad (*AABB Hitboxes & Hurtboxes*)**: Cada sprite tiene un rectángulo visual editable (`hitbox` de ataque y `hurtbox` de daño). Cuando dos cajas se tocan, el motor invoca automáticamente el evento `on_collision(self, other)` del script correspondiente.

3. **Efectos Visuales, Transparencias y Partículas (`engine/ngs_vfx_alpha.h`)**:
   - **Transparencia Real (`Alpha Blending 0..255`)**: Cualquier sprite, capa Parallax o neblina tiene una propiedad `alpha` (`0` = invisible, `128` = 50% translúcido, `255` = opaco) mezclada por canal `RGBA` de 32 bits.
   - **Mezcla Aditiva de Luz (`Additive Blending / Glow`)**: Permite que disparos láser, fuego, explosiones y proyectiles sumen luz real sobre los píxeles del fondo.
   - **Emisor de Partículas y Efectos de Cámara**: Funciones listas para usar desde cualquier script (`vfx_explosion(x, y)`, `vfx_sparks(x, y)`, `vfx_rain()`, `vfx_screen_shake(intensidad)`, `vfx_fade_in_out()`).

4. **Inteligencia Artificial (IA) de Personajes y Jefes (`engine/ngs_ai_behaviors.h`)**:
   - **Comportamientos Preconstruidos Seleccionables**: Desde el editor puedes asignarle a cualquier enemigo una IA lista para usar sin escribir código:
     - `AI_PATROL_SMART`: Camina por una plataforma y da vuelta al detectar un borde o pared.
     - `AI_TURRET_SNIPER`: Calcula el ángulo hacia `(player.x, player.y)` y dispara proyectiles dirigidos cada `N` frames.
     - `AI_CHASER / JUMPER`: Persigue al jugador y salta obstáculos.
     - `AI_FLYING_WAVE`: Movimiento aéreo senoidal o en picada.
   - **Máquinas de Estado para Jefes (*Boss Scripting*)**: Permite encadenar fases visuales o por script (*"Fase 1: Dispara abanico -> Si `hp < 50%` -> Fase 2: Se vuelve semitransparente, tiembla la pantalla y embiste"*).

5. **Plantillas de Género y Biblioteca Global de "Mashup"**:
   - Incluye plantillas base (*Run & Gun*, *Plataformas*, *Top-Down Aventura*, *Shoot 'em Up*) y permite combinar recursos extraídos de cualquier juego NES con gráficos HD y sonidos propios.

---

## 4. Estructura del Repositorio (`NesGameStudio`)

```text
NesGameStudio/
├── Cargo.toml                  # Aplicación GUI en Rust (eframe/egui, rfd, serde, image, cpal)
├── README.md                   # Descripción general y arquitectura en Mermaid
├── PLAN_IMPLEMENTACION.md      # Este documento de diseño e implementación
└── src/
    ├── main.rs                 # Ventana principal de NesGameStudio
    ├── project.rs              # Proyecto (.ngs.json), catálogo de pistas y carpeta assets/
    ├── parser/                 # Analizador y re-escritor automático de archivos .c y .asm
    ├── runtime/                # Motor de vista previa a 60 FPS y Rockola APU aislada
    ├── editors/
    │   ├── graphics.rs         # Módulo 1: Galería total de gráficos CHR y Sprites HD 32-bit
    │   ├── levels.rs           # Módulo 2: Editor de Pantallas NameTable y Niveles en C
    │   ├── audio.rs            # Módulo 3: Rockola aislada (Sound Test) e Inyector .MP3 / .WAV
    │   └── ram_inspector.rs    # Módulo 4: Buscador de Variables RAM, Reglas y Visor .ASM <-> .C
    └── codegen/                # Cirujano de código C y compilador automático a .exe nativo
```

---

## 5. Fases de Implementación Propuestas

1. **Fase 1 — Base del Estudio, Carga de `.c`/`.asm` y Compilación Automática**:
   - Interfaz principal con pestañas para los 4 módulos, analizador de arreglos `PRG_ROM[]`/`CHR_MEM[]` desde el `.c`, y botón de compilación directa a `.exe` nativo.
2. **Fase 2 — Galería Completa de Gráficos y Estudio HD (8-bit `.BMP` + 32-bit RGBA)**:
   - Visualizador de todos los bancos gráficos del juego, capturador de niveles `CHR-RAM`, editor de píxeles, importador de Sprites HD `.PNG`/`.BMP` e inyección automática del renderizador HD en el `.c`.
3. **Fase 3 — Rockola Aislada de Música/Efectos (*Sound Test*) e Inyector `.MP3` / `.WAV`**:
   - Reproductor aislado de pistas y efectos del juego con control Solo/Mute por canal, exportador a `.WAV` y reemplazo directo de pistas por archivos `.MP3`/`.WAV` inyectados en el `.c`.
4. **Fase 4 — Vista Previa en Vivo, Buscador de Variables RAM y Modificador de Reglas**:
   - Escáner interactivo de variables en `RAM`, etiquetador automático de `#define` y generador visual de reglas/trucos en C.
5. **Fase 5 — Editor Visual de Pantallas y Creador de Niveles en C**:
   - Lienzo de edición de escenarios, colocación de nuevas entidades C (*Overlay*) y creador de niveles 100% en C.
