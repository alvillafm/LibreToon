//! Embedded UI catalogues. Locale is per UI thread; no network is used at runtime.
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    #[default]
    #[serde(rename = "en")]
    English,
    #[serde(rename = "es")]
    Spanish,
    #[serde(rename = "de")]
    German,
    #[serde(rename = "fr")]
    French,
    #[serde(rename = "it")]
    Italian,
    #[serde(rename = "pt-BR")]
    PortugueseBrazil,
    #[serde(rename = "pt-PT")]
    PortuguesePortugal,
    #[serde(rename = "ru")]
    Russian,
    #[serde(rename = "zh-CN")]
    ChineseSimplified,
    #[serde(rename = "ja")]
    Japanese,
    #[serde(rename = "ko")]
    Korean,
}
impl Language {
    pub const ALL: [Self; 11] = [
        Self::English,
        Self::Spanish,
        Self::German,
        Self::French,
        Self::Italian,
        Self::PortugueseBrazil,
        Self::PortuguesePortugal,
        Self::Russian,
        Self::ChineseSimplified,
        Self::Japanese,
        Self::Korean,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::Spanish => "Español",
            Self::German => "Deutsch",
            Self::French => "Français",
            Self::Italian => "Italiano",
            Self::PortugueseBrazil => "Português (Brasil)",
            Self::PortuguesePortugal => "Português (Portugal)",
            Self::Russian => "Русский",
            Self::ChineseSimplified => "简体中文",
            Self::Japanese => "日本語",
            Self::Korean => "한국어",
        }
    }
    pub fn id(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::Spanish => "es",
            Self::German => "de",
            Self::French => "fr",
            Self::Italian => "it",
            Self::PortugueseBrazil => "pt-BR",
            Self::PortuguesePortugal => "pt-PT",
            Self::Russian => "ru",
            Self::ChineseSimplified => "zh-CN",
            Self::Japanese => "ja",
            Self::Korean => "ko",
        }
    }
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|l| l.id() == id)
    }
    fn font_index(self) -> u32 {
        match self {
            Self::Korean => 1,
            Self::ChineseSimplified => 2,
            _ => 0,
        }
    }
}
thread_local! {static LOCALE:std::cell::Cell<Language>=const{std::cell::Cell::new(Language::English)};}
pub fn set_locale(locale: Language) {
    LOCALE.with(|v| v.set(locale));
}
#[cfg(test)]
pub fn set_language(spanish: bool) {
    set_locale(if spanish {
        Language::Spanish
    } else {
        Language::English
    });
}
pub fn locale() -> Language {
    LOCALE.with(|l| l.get())
}
static CATALOGUES: std::sync::LazyLock<Vec<std::collections::HashMap<String, String>>> =
    std::sync::LazyLock::new(|| {
        [
            include_str!("../assets/locales/en.json"),
            include_str!("../assets/locales/es.json"),
            include_str!("../assets/locales/de.json"),
            include_str!("../assets/locales/fr.json"),
            include_str!("../assets/locales/it.json"),
            include_str!("../assets/locales/pt-BR.json"),
            include_str!("../assets/locales/pt-PT.json"),
            include_str!("../assets/locales/ru.json"),
            include_str!("../assets/locales/zh-CN.json"),
            include_str!("../assets/locales/ja.json"),
            include_str!("../assets/locales/ko.json"),
        ]
        .into_iter()
        .map(|s| serde_json::from_str(s).expect("validated embedded UI catalogue"))
        .collect()
    });
pub fn catalogue(language: Language) -> &'static std::collections::HashMap<String, String> {
    &CATALOGUES[language as usize]
}
pub fn tr(text: &str) -> &str {
    let language = locale();
    catalogue(language)
        .get(text)
        .map(String::as_str)
        .unwrap_or_else(|| {
            if language == Language::Spanish {
                text
            } else {
                english(text)
            }
        })
}
pub fn deserialize_language<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Option<Language>, D::Error> {
    let code = Option::<String>::deserialize(d)?;
    Ok(code.and_then(|s| Language::from_id(&s)))
}
/// Preserve placeholders in dynamic application messages without translating user data.
pub fn message(template: &str, args: &[(&str, String)]) -> String {
    let mut result = tr(template).to_owned();
    for (name, value) in args {
        result = result.replace(&format!("{{{name}}}"), value);
    }
    result
}
pub fn ensure_fonts(ctx: &eframe::egui::Context, language: Language) {
    use eframe::egui::{FontData, FontDefinitions, FontFamily};
    let index = language.font_index();
    let key = eframe::egui::Id::unique("libretoon-cjk-font-index");
    if ctx.data(|d| d.get_temp::<u32>(key)) == Some(index) {
        return;
    }
    let mut fonts = FontDefinitions::default();
    let mut font = FontData::from_static(include_bytes!("../assets/fonts/NotoSansCJK-Regular.ttc"));
    font.index = index;
    fonts
        .font_data
        .insert("libretoon-cjk".into(), std::sync::Arc::new(font));
    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .push("libretoon-cjk".into());
    }
    ctx.set_fonts(fonts);
    ctx.data_mut(|d| d.insert_temp(key, index));
}

pub fn english(text: &str) -> &str {
    match text {
        "Orden en la capa" => "Order in layer",
        "Enviar al fondo" => "Send to back",
        "Enviar atrás" => "Send backward",
        "Traer adelante" => "Bring forward",
        "Traer al frente" => "Bring to front",
        "0 = fondo · números mayores = delante" => "0 = back · higher numbers = front",
        "Archivo" => "File",
        "Edición" => "Edit",
        "Guardar" => "Save",
        "Guardar como…" => "Save as…",
        "Abrir…" => "Open…",
        "Nuevo" => "New",
        "Ajustes de documento…" => "Document settings…",
        "Ajustes de documento" => "Document settings",
        "Preferencias del programa…" => "Program preferences…",
        "Exportar vídeo…" => "Export video…",
        "Exportar GIF…" => "Export GIF…",
        "Copiar" => "Copy",
        "Cortar" => "Cut",
        "Pegar" => "Paste",
        "Solo escenario" => "Stage only",
        "Nuevo documento" => "New document",
        "Abrir proyecto" => "Open project",
        "Guardar una copia" => "Save a copy",
        "Deshacer" => "Undo",
        "Rehacer" => "Redo",
        "Guardar · Ctrl+S" => "Save · Ctrl+S",
        "Deshacer · Ctrl+Z" => "Undo · Ctrl+Z",
        "Rehacer · Ctrl+Y" => "Redo · Ctrl+Y",
        "SELECCIÓN" => "SELECTION",
        "EDICIÓN" => "EDITING",
        "PINTURA" => "PAINT",
        "VECTORIAL" => "VECTOR",
        "FORMAS" => "SHAPES",
        "LIENZO" => "CANVAS",
        "COLOR" => "COLOR",
        "Propiedades" => "Properties",
        "Pincel Estándar" => "Standard brush",
        "Pincel Vectorial" => "Vector brush",
        "Pincel de relleno" => "Fill brush",
        "Pincel de sombreado" => "Shading brush",
        "Pincel de iluminación" => "Lighting brush",
        "Regla" => "Ruler",
        "Simetría" => "Symmetry",
        "Espejo" => "Mirror",
        "Pluma" => "Pen",
        "Editar nodos" => "Edit nodes",
        "Bote de pintura" => "Paint bucket",
        "Anchura del contorno" => "Stroke width",
        "Rectángulo" => "Rectangle",
        "Selección" => "Selection",
        "Transformación libre" => "Free transform",
        "Rotar vista" => "Rotate view",
        "Lazo" => "Lasso",
        "Selección poligonal" => "Polygon selection",
        "Varita mágica" => "Magic wand",
        "Selección de objeto" => "Object selection",
        "Goma de borrar" => "Eraser",
        "Cuentagotas" => "Eyedropper",
        "Línea" => "Line",
        "Círculo / elipse" => "Circle / ellipse",
        "Polígono / estrella" => "Polygon / star",
        "Mano" => "Hand",
        "Lupa" => "Zoom",
        "Pinceles" => "Brushes",
        "Relleno" => "Fill",
        "Contorno" => "Stroke",
        "Intercambiar colores · X" => "Swap colors · X",
        "Blanco y negro · D" => "Black and white · D",
        "Grosor" => "Width",
        "Suavizado" => "Smoothing",
        "Perfil del contorno" => "Stroke profile",
        "Cierre de huecos" => "Close gaps",
        "Aplicar estilo a la selección" => "Apply style to selection",
        "Lados / puntas" => "Sides / points",
        "Estrella" => "Star",
        "Radio interior" => "Inner radius",
        "Tolerancia" => "Tolerance",
        "Selecciona los rellenos o contornos de color parecido en la capa." => {
            "Select fills or strokes with similar colors in the layer."
        }
        "Acercar (+)" => "Zoom in (+)",
        "Alejar (−)" => "Zoom out (−)",
        "Uniforme" => "Uniform",
        "Puntas finas" => "Tapered",
        "De fino a grueso" => "Thin to thick",
        "De grueso a fino" => "Thick to thin",
        "Centro ancho" => "Wide center",
        "Ondulado" => "Wavy",
        "Personalizado" => "Custom",
        "Personalizada" => "Custom",
        "Ninguno" => "None",
        "Pequeño" => "Small",
        "Mediano" => "Medium",
        "Grande" => "Large",
        "Mostrar reglas" => "Show rulers",
        "Arrastra desde las reglas para crear guías. Arrastra una guía fuera para eliminarla." => {
            "Drag from a ruler to create a guide. Drag a guide outside to remove it."
        }
        "Borrar guías" => "Clear guides",
        "Horizontal" => "Horizontal",
        "Vertical" => "Vertical",
        "Ejes en el centro del escenario. Se aplica a pinceles y formas." => {
            "Axes use the stage center. Applies to brushes and shapes."
        }
        "Voltear horizontalmente" => "Flip horizontally",
        "Voltear verticalmente" => "Flip vertically",
        "Intensidad" => "Strength",
        "Mezcla" => "Blend",
        "Exposición" => "Exposure",
        "Contraste" => "Contrast",
        "Interfaz" => "Interface",
        "Tema: Negro" => "Theme: Black",
        "Idioma" => "Language",
        "Atajos de teclado" => "Keyboard shortcuts",
        "Pulsa un atajo y después la combinación nueva. Escape cancela; Supr lo elimina." => {
            "Click a shortcut, then press the new combination. Escape cancels; Delete clears it."
        }
        "Restablecer atajos" => "Reset shortcuts",
        "Atajo reservado para una acción del programa" => "Shortcut reserved for a program action",
        "Papel cebolla" => "Onion skin",
        "Antes: " => "Before: ",
        "Después: " => "After: ",
        "Fotograma anterior" => "Previous frame",
        "Fotograma siguiente" => "Next frame",
        "Primer fotograma" => "First frame",
        "Reproducir / pausar · Espacio" => "Play / pause · Space",
        "Velocidad del documento" => "Document frame rate",
        "Añadir capa" => "Add layer",
        "Insertar fotograma vacío" => "Insert blank frame",
        "Duplicar fotograma" => "Duplicate frame",
        "Eliminar fotograma" => "Delete frame",
        "Eliminar capa" => "Delete layer",
        "Subir capa" => "Move layer up",
        "Bajar capa" => "Move layer down",
        "Doble clic para cambiar el nombre" => "Double-click to rename",
        "Encajar escenario y centrar vista" => "Fit stage and center view",
        "Rotar lienzo: arrastra sobre el escenario" => "Rotate canvas: drag on the stage",
        "Ángulo de la vista. Doble clic para escribir 0 y restablecer." => {
            "View angle. Double-click and enter 0 to reset."
        }
        "Escena 1" => "Scene 1",
        " /  Escenario" => " /  Stage",
        "Sin título.fla" => "Untitled.fla",
        "Ancho" => "Width",
        "Alto" => "Height",
        "Velocidad" => "Frame rate",
        "Aplicar" => "Apply",
        "Cancelar" => "Cancel",
        "Exportar" => "Export",
        "Exportando…" => "Exporting…",
        "Exportado:" => "Exported:",
        "Exportación cancelada" => "Export cancelled",
        "La exportación se interrumpió" => "Export interrupted",
        "Nombre" => "Name",
        "Carpeta" => "Folder",
        "Formato" => "Format",
        "Resolución" => "Resolution",
        "Conservar proporción" => "Keep aspect ratio",
        "Fotogramas" => "Frames",
        "Calidad (CRF)" => "Quality (CRF)",
        "Códec" => "Codec",
        "Colores" => "Colors",
        "Repeticiones (0 = infinito)" => "Loops (0 = forever)",
        "Tramado" => "Dithering",
        "Secuencia de fotogramas" => "Frame sequence",
        "Exporta el fotograma actual como SVG vectorial." => {
            "Exports the current frame as vector SVG."
        }
        "GIF usa intervalos de centésimas de segundo; los FPS pueden redondearse." => {
            "GIF uses hundredths of a second; frame rates may be rounded."
        }
        "Buscar FFmpeg…" => "Browse for FFmpeg…",
        "Reemplazar archivo" => "Replace file",
        "Ya existe un archivo de destino. ¿Reemplazarlo?" => {
            "An output file already exists. Replace it?"
        }
        "Nombre de archivo no válido" => "Invalid file name",
        "La carpeta no existe" => "Folder does not exist",
        "No se pudo iniciar FFmpeg. Selecciona ffmpeg.exe en las opciones de exportación" => {
            "Could not start FFmpeg. Select ffmpeg.exe in export options"
        }
        "Duración total. Al acortar se recortan los fotogramas finales; Ctrl+Z permite recuperarlos." => {
            "Total duration. Shortening removes the final frames; Ctrl+Z restores them."
        }
        "Sesión recuperada" => "Session recovered",
        "Dibuja tu primer fotograma" => "Draw your first frame",
        "Proyecto guardado" => "Project saved",
        "Proyecto abierto" => "Project opened",
        "Tema" => "Theme",
        "Negro" => "Black",
        "Línea de tiempo" => "Timeline",
        "CAPAS" => "LAYERS",
        "Mostrar / ocultar capa" => "Show / hide layer",
        "Inicio" => "Start",
        "Final" => "End",
        "+ Punto" => "+ Point",
        "− Punto" => "− Point",
        "Aplicar perfil a la selección" => "Apply profile to selection",
        "No cerrar huecos" => "Do not close gaps",
        "Cerrar huecos pequeños" => "Close small gaps",
        "Cerrar huecos medianos" => "Close medium gaps",
        "Cerrar huecos grandes" => "Close large gaps",
        "Pausar" => "Pause",
        "Reproducir" => "Play",
        "Fotograma" => "Frame",
        "Capa" => "Layer",
        "Posición" => "Position",
        "Anchura" => "Width",
        "Eliminar punto" => "Delete point",
        "Suavizar" => "Smooth",
        "Esquina" => "Corner",
        "Cerrar" => "Close",
        "Minimizar" => "Minimize",
        "Maximizar / restaurar" => "Maximize / restore",
        "FFmpeg incluido: no requiere descarga." => "FFmpeg is included; no download required.",
        "Configurar FFmpeg incluido" => "Configure bundled FFmpeg",
        "Perspectiva" => "Perspective",
        "Cámara" => "Camera",
        "Cámaras" => "Cameras",
        "Biblioteca" => "Library",
        "Símbolos" => "Symbols",
        "Imágenes" => "Images",
        "Vídeos" => "Videos",
        "Sonidos" => "Sounds",
        "Sin recursos" => "No assets",
        "Degradado" => "Gradient",
        "Lineal" => "Linear",
        "Radial" => "Radial",
        "Añadir color" => "Add color",
        "Posición del color" => "Color position",
        "Opacidad" => "Opacity",
        "Punta de entrada" => "Start cap",
        "Punta final" => "End cap",
        "Circular" => "Round",
        "Plana" => "Butt",
        "Cuadrada" => "Square",
        "Triangular" => "Triangle",
        "Símbolo gráfico" => "Graphic symbol",
        "Bucle" => "Loop",
        "Reproducir una vez" => "Play once",
        "Fotograma único" => "Single frame",
        "Fotograma: " => "Frame: ",
        "Volver a escena" => "Back to scene",
        "Convertir a símbolo" => "Convert to symbol",
        "Convertir a símbolo · F8" => "Convert to symbol · F8",
        "Editar símbolo" => "Edit symbol",
        "Insertar símbolo" => "Insert symbol",
        "Eliminar símbolo" => "Delete symbol",
        "El símbolo está en uso" => "Symbol is in use",
        "Niveles de Cartoon: " => "Toon levels: ",
        "Añadir cámara" => "Add camera",
        "Eliminar cámara" => "Delete camera",
        "Duración: " => "Duration: ",
        "Mostrar perspectiva" => "Show perspective",
        "Puntos de fuga" => "Vanishing points",
        "Horizonte: " => "Horizon: ",
        "Líneas de fuga" => "Vanishing lines",
        "Último fotograma" => "Last frame",
        "Máscara de recorte" => "Clipping mask",
        "CARPETAS" => "FOLDERS",
        "FOTOGRAMAS" => "FRAMES",
        "Crear carpeta" => "Create folder",
        "Eliminar carpeta" => "Delete folder",
        "Fotograma clave" => "Keyframe",
        "Fotograma clave en blanco" => "Blank keyframe",
        "Ampliar duración de fotograma" => "Extend frame exposure",
        "Reducir duración de fotograma" => "Shorten frame exposure",
        "Eliminar fotograma clave" => "Delete keyframe",
        "Nombre de capa" => "Layer name",
        "Gestor de escenas" => "Scene manager",
        "Reproducir todas las escenas" => "Play all scenes",
        "Eliminar escena" => "Delete scene",
        "Se eliminará todo el contenido de esta escena. ¿Continuar?" => {
            "All contents of this scene will be deleted. Continue?"
        }
        "Selector de fotogramas" => "Frame picker",
        "Eliminar" => "Delete",
        "Galería de proyectos" => "Project gallery",
        "Texto" => "Text",
        "Pincel difuminador" => "Airbrush",
        "Pincel borrador" => "Sketch brush",
        "Maniquí" => "Mannequin",
        "HERRAMIENTA" => "TOOL",
        "PROYECTOS" => "PROJECTS",
        "EN CURSO" => "IN PROGRESS",
        "ARCHIVOS" => "FILES",
        "GEOMETRÍA" => "GEOMETRY",
        "TABLETA" => "TABLET",
        "TINTA" => "INK",
        "TRAZO" => "STROKE",
        "SUAVIZADO" => "SMOOTHING",
        "PERFIL DEL CONTORNO" => "STROKE PROFILE",
        "CONTENIDO" => "CONTENT",
        "TIPOGRAFÍA" => "TYPOGRAPHY",
        "RELLENO" => "FILL",
        "CONTORNO" => "OUTLINE",
        "PROPORCIONES" => "PROPORTIONS",
        "POSE" => "POSE",
        "CAPA DE REFERENCIA" => "REFERENCE LAYER",
        "TRANSFORMACIÓN Y ANIMACIÓN" => "TRANSFORM & ANIMATION",
        "RELLENO Y DEGRADADO" => "FILL & GRADIENT",
        "CÁMARA" => "CAMERA",
        "PERSPECTIVA" => "PERSPECTIVE",
        "Presión de lápiz activada" => "Pen pressure enabled",
        "Presión de lápiz desactivada" => "Pen pressure disabled",
        "Lápiz detectado · presión → grosor" => "Pen detected · pressure → width",
        "Ratón: grosor constante · esperando lápiz" => "Mouse: constant width · waiting for pen",
        "Radio de esquina" => "Corner radius",
        "Dureza" => "Hardness",
        "Complexión" => "Build",
        "Escala" => "Scale",
        "Guardar los cambios" => "Save changes",
        "Descartar" => "Discard",
        "Guardar y cerrar" => "Save and close",
        "Tamaño" => "Size",
        "Interlineado" => "Line height",
        "Caracteres" => "Letter spacing",
        "Palabras" => "Word spacing",
        "Izquierda" => "Left",
        "Centro" => "Center",
        "Derecha" => "Right",
        "Outline" => "Outline",
        "Tu próximo movimiento" => "Your next movement",
        "LIBRETOON  /  ESTUDIO DE ANIMACIÓN" => "LIBRETOON  /  ANIMATION STUDIO",
        "Nuevo proyecto" => "New project",
        "Crear animación" => "Create animation",
        "Añadir carpeta de proyectos" => "Add project folder",
        "Cambiar de documento" => "Switch document",
        "Cerrar documento" => "Close document",
        "Buscar por nombre de archivo…" => "Search by file name…",
        "Tu galería empieza aquí. Crea un proyecto, abre un archivo o añade una carpeta." => {
            "Your gallery starts here. Create a project, open a file or add a folder."
        }
        "Haz clic en el lienzo para colocar un maniquí articulado." => {
            "Click the canvas to place an articulated mannequin."
        }
        "Colocar maniquí" => "Place mannequin",
        "Una referencia por escena" => "One reference per scene",
        "Referencia de dibujo · no se exporta" => "Drawing reference · not exported",
        "Arrastra las articulaciones para posar. El centro de la cadera mueve todo el maniquí." => {
            "Drag joints to pose. The pelvis moves the entire mannequin."
        }
        "Restablecer pose" => "Reset pose",
        "Conservar posición y proporciones" => "Keep position and proportions",
        "Retirar maniquí" => "Remove mannequin",
        "Eliminar la referencia" => "Remove reference",
        "Aplicar cambios" => "Apply changes",
        "Insertar texto" => "Insert text",
        "Texto vectorial editable" => "Editable vector text",
        "Haz clic en el lienzo para colocar texto o sobre un texto existente para editarlo." => {
            "Click the canvas to position text, or click existing text to edit it."
        }
        "＋ Cargar fuente TTF / OTF" => "＋ Load TTF / OTF font",
        "Boceto gris · no se exporta" => "Gray sketch · not exported",
        "Activar / desactivar presión de lápiz" => "Enable / disable pen pressure",
        _ => text,
    }
}
