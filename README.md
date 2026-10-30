# LibreToon 0.2 — dibujo y animación 2D en Rust

Editor nativo de Windows con eframe/egui, iconos vectoriales originales y una mascota propia. Esta versión incorpora rellenos, contornos y trazados Bézier editables.

## Abrir y desarrollar

- `Abrir-LibreToon.cmd`: abre la versión optimizada; si no existe, la compila. `Abrir-Trazo.cmd` es un alias histórico.
- `Desarrollar.cmd`: compila en desarrollo y reinicia al guardar cambios en `src`. El cierre normal permite terminar la recuperación pendiente.
- Requisitos: Rust estable, MSVC x64/x86 y Windows SDK. `.vsconfig` permite importar los componentes necesarios en Visual Studio Installer.

## Herramientas

| Tecla | Herramienta | Uso |
| --- | --- | --- |
| B | Pincel Estándar | Tinta de relleno con el color de relleno y el grosor elegido. |
| Y | Pincel Vectorial | Dibujo a mano alzada convertido en curvas Bézier con contorno. |
| P | Pluma | Clic para esquinas; arrastrar para crear tiradores y curvas. |
| A | Editar nodos | Seleccionar un trazado y mover anclas o tiradores. |
| V | Selección | Seleccionar tinta, rellenos o fragmentos mediante un marco. |
| Q | Transformación libre | Mover, escalar y rotar lo seleccionado. |
| R | Rectángulo | Relleno y contorno, solo relleno o solo contorno. |
| K | Bote de pintura | Rellenar regiones cerradas de la capa y fotograma activos. |
| W / Mayús+W | Anchura | Ensanchar o afinar un contorno directamente sobre el trazado. |

Se puede empezar y continuar un dibujo en la zona gris, fuera del escenario blanco. El tamaño del escenario se cambia en Archivo > Ajustes de documento; la vista y los archivos conservan el dibujo exterior para el futuro trabajo de cámara. Los límites del panel de trabajo siguen siendo los límites visibles de interacción.

El suavizado recoge las muestras del ratón recibidas en cada cuadro. Mientras se dibuja aplica el filtro incremental; al soltar ajusta el trazo a curvas suaves con tolerancia regulada por el control de suavizado. El pincel estándar convierte ese recorrido en una silueta rellena con extremos redondeados. El vectorial conserva las anclas de su línea central para editarlas con A. Los trazos antiguos no se modifican automáticamente.

## Pluma y edición de nodos

- P: clic añade una esquina; clic y arrastrar crea una tangente simétrica.
- Mayús al crear una tangente: intervalos de 45°. Alt permite romper la tangente.
- Clic en el primer nodo: cerrar el trazado. Enter o doble clic: terminar un trazado abierto. Con solo relleno activado se cierra al terminar.
- Esc cancela el trazado provisional. Ctrl+Z durante su creación retira el último nodo.
- Guardar, cambiar de herramienta o de fotograma confirma el trazado provisional si tiene al menos dos nodos. Cerrar normalmente el editor también lo confirma.
- A: selecciona el trazado y arrastra un ancla o un tirador. Mover un ancla desplaza sus tiradores. Alt mueve un tirador sin cambiar el contrario.
- Supr elimina el nodo activo; sin nodo activo elimina la selección. Esc cancela el arrastre actual.

La edición de nodos actúa sobre el contorno exterior. Los huecos de los rellenos se conservan al transformar, guardar y recortar, pero todavía no tienen un editor de nodos propio.

## Relleno y contorno

La barra lateral muestra dos selectores superpuestos: contorno detrás y relleno delante. X intercambia ambos colores y sus estados; D vuelve a contorno negro y relleno blanco. La casilla con una diagonal roja activa o desactiva el componente elegido. Las casillas Relleno y Contorno activan cada componente para rectángulos y trazados cerrados de la pluma. El estándar usa relleno; el vectorial usa contorno. Si se desactiva el componente que necesita un pincel, ese pincel no pinta.

Los controles preparan el dibujo siguiente. Para cambiar una selección existente, elige los colores y pulsa **Aplicar estilo a la selección**. Para aplicar solo relleno a un trazado abierto hay que cerrarlo primero. El grosor del contorno se mantiene al escalar.

## Propiedades y ajustes de documento

El panel derecho contiene solo controles de la herramienta activa. El nombre de una capa se edita con doble clic en la línea de tiempo; sus botones permiten crearla, subirla, bajarla y borrarla. Los FPS se editan directamente en la cabecera de la línea de tiempo. El campo en segundos permite cambiar la duración total; al reducirla se recortan los fotogramas finales, recuperables con Ctrl+Z.

Archivo > Ajustes de documento permite elegir HD, Full HD, 4K UHD, 4K DCI, vertical o cuadrado, y modificar ancho y alto libremente. Desde la misma ventana se pueden cambiar los FPS. Cambiar la resolución modifica el escenario sin escalar el dibujo. El porcentaje de zoom de la cabecera expresa la escala real en pantalla.

## Bote de pintura

K rellena una región cerrada usando el color de relleno. Trabaja sobre los contornos y límites de formas de la capa y fotograma activos. Las propiedades permiten no cerrar huecos, o cerrar huecos pequeños, medianos y grandes: hasta 6, 16 y 36 unidades del documento, respectivamente.

Los puentes de cierre son temporales: no se añaden ni modifican trazados. Una zona abierta hacia el exterior se rechaza para evitar rellenos sin límite. Volver a pintar una región sin cambios sustituye su color en vez de acumular caras idénticas. El resultado es un relleno vectorial editable y se puede deshacer.

## Perfiles de contorno y herramienta de anchura

El pincel vectorial y la pluma ofrecen Uniforme, Puntas finas, De fino a grueso, De grueso a fino, Centro ancho y Ondulado, además de un perfil personalizado. El perfil se distribuye por la longitud del trazado. La gráfica permite arrastrar puntos horizontalmente para situarlos y verticalmente para ajustar el grosor; el valor del punto seleccionado se puede escribir en porcentaje. Hay botones para añadir y quitar puntos interiores. El grosor relativo admite de 0 a 400 % del grosor base.

Para modificar un contorno existente, selecciona W (también Mayús+W), pulsa sobre su línea y arrastra perpendicularmente. Arrastrar hacia fuera ensancha; acercarse al eje afina. Los tiradores muestran la anchura en cada punto del perfil. El panel permite editar el perfil del contorno seleccionado. Con las otras herramientas, **Aplicar perfil a la selección** aplica el perfil preparado.

La anchura es simétrica a ambos lados y conserva la línea central Bézier. Los perfiles, colores y tiradores se mantienen en los datos nativos del FLA. Para SVG y el XFL destinado a otras aplicaciones, el contorno variable se representa mediante su silueta rellena; si otra aplicación elimina los datos nativos, esa silueta conserva su apariencia, pero ya no su perfil de anchura editable. La herramienta de anchura actúa sobre contornos, no sobre la tinta de relleno del pincel estándar.

## Selección parcial y transformación

- V + arrastrar desde una zona vacía: seleccionar la parte de la geometría que intersecta el marco. Ya no es necesario encerrar el dibujo entero.
- Ctrl+arrastrar: iniciar el marco incluso desde el interior de la tinta. Mayús añade a la selección.
- Clic en el interior selecciona el relleno; clic en el borde selecciona el contorno cuando ambos están presentes.
- Arrastrar la tinta seleccionada mueve solo ese fragmento. Un relleno recortado deja un hueco real al apartarlo; el resto del dibujo permanece.
- Q muestra el marco de transformación: ocho tiradores para escalar, tirador circular para rotar y cruz para recolocar el pivote.
- Mayús mantiene proporciones al escalar y ajusta la rotación a intervalos de 15°.
- Ctrl+A selecciona el dibujo visible del fotograma; Ctrl+D duplica; Supr borra; Ctrl+Z/Ctrl+Y deshacen y rehacen.

Los recortes son ediciones que se pueden deshacer. Los contornos Bézier se subdividen conservando las curvas; los recortes de relleno usan polígonos con tolerancia de 0,04 unidades del documento. No hay todavía lazo, fusión automática de dibujos superpuestos ni un motor completo de topología equivalente al de Animate.

## Navegación y animación

Rueda: zoom centrado en el cursor. Botón central o Espacio+arrastrar: desplazar temporalmente. La cabecera permite rotar la vista y escribir el ángulo; Encajar restablece posición, zoom y rotación. La rotación de la vista no transforma el documento.

Se mantienen capas, fotogramas, reproducción, papel cebolla, deshacer/rehacer y exportación SVG del fotograma actual. Aún no hay presión de tableta, símbolos, interpolaciones, audio, cámara ni exportación de vídeo.

## Archivos y compatibilidad

Guardar produce FLA modernos: ZIP con XFL, rellenos sólidos, huecos, contornos y curvas cuadráticas aproximadas desde los Bézier cúbicos. Una entrada adicional `LIBRETOON/document.json` conserva las anclas cúbicas originales y los estilos exactos al reabrir en LibreToon. Esa entrada solo se usa si el XML coincide con el documento; si otra aplicación cambia el XFL, se lee su contenido actualizado. Las curvas cuadráticas importadas se representan mediante Bézier cúbicos editables.

Se admiten una escena blanca con resolución de 1 a 16.384 píxeles por eje, capas normales, 1–240 fps enteros, fotogramas y exposiciones. Se rechazan FLA binarios antiguos, símbolos, imágenes, textos, grupos, audio, vídeo, interpolaciones, filtros, scripts, gradientes y topologías de relleno no admitidas. No se descartan silenciosamente esos elementos.

La prueba histórica de `ejemplo-interoperabilidad.fla` en Animate 2024 cubría un trazo, una capa, 24 fotogramas y 12 fps. Las curvas y rellenos nuevos tienen pruebas automatizadas de formato; su interoperabilidad completa con Animate requiere validación manual allí. Otra aplicación puede eliminar los datos nativos adicionales al guardar.

Los `.trazo` antiguos siguen siendo importables. La recuperación automática usa `%LOCALAPPDATA%\Trazo\recovery.json`, conservando la ubicación histórica. Guarda tus proyectos importantes en archivos propios. El SVG conserva curvas cúbicas y rellenos con huecos; su ventana de exportación utiliza el tamaño actual del escenario.

## Comprobaciones

```powershell
cargo fmt --all -- --check
cargo test --locked
cargo test --release --locked
cargo test --release --locked benchmark_ -- --ignored --nocapture --test-threads=1
cargo build --release --locked
```

Arquitectura y mediciones: [PERFORMANCE.md](PERFORMANCE.md). `document.rs` define el modelo; `vector.rs` implementa curvas, ajuste y recortes; `paint.rs` conserva geometría preparada; `canvas.rs` gestiona gestos; `workspace.rs` contiene la línea de tiempo; `inspector.rs` contiene propiedades y ajustes; `width.rs` define perfiles y siluetas; `bucket.rs` calcula regiones de relleno; `fla.rs` y `fla_zip.rs` gestionan FLA/XFL.
