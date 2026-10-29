# LibreToon 0.2 — animación 2D en Rust

Editor nativo de Windows con eframe/egui. La interfaz usa iconos vectoriales originales y un ratón propio como mascota. El código se mantiene en la carpeta histórica `Trazo`.

## Abrir y desarrollar

- `Abrir-LibreToon.cmd`: abre el ejecutable, sin instalación.
- `Desarrollar.cmd`: compila, abre y vuelve a compilar/reiniciar cuando guardas cambios en el código. Cerrar la aplicación termina el lanzador.
- Rust estable y Microsoft C++ Build Tools con Windows SDK son los requisitos para compilar.

## Selección y transformación

- V: selección. Clic sobre un trazo del pincel o el contorno de un rectángulo.
- Arrastrar en vacío: seleccionar trazos completamente contenidos en el marco.
- Mayús+clic: añadir o quitar trazos de la selección. Ctrl+A: todos los trazos visibles del fotograma.
- Q: transformación libre. Arrastra dentro del marco para mover; usa los ocho tiradores para escalar.
- Arrastra el tirador circular por encima del marco para rotar.
- Arrastra la cruz para recolocar el pivote. Escala y rotación usan ese origen; se puede poner fuera del dibujo.
- Mayús al escalar: mantener proporciones. Mayús al rotar: intervalos de 15°.
- Esc: cancelar la transformación en curso. Cada transformación se deshace de una sola vez.
- Ctrl+D: duplicar selección. Supr: borrar selección.
- La selección puede abarcar varias capas visibles del fotograma actual.

Se transforman los puntos reales del trazo; los rectángulos transformados pasan a trazados cerrados. El grosor del trazo se mantiene al escalar. No incluye aún edición individual de nodos Bézier. El pivote pertenece a la selección de trabajo y no se guarda como propiedad permanente de un símbolo.

## Navegación del lienzo

- Rueda: zoom centrado en el cursor.
- Botón central + arrastrar: desplazar la vista.
- Espacio + botón izquierdo + arrastrar: desplazar temporalmente desde cualquier herramienta.
- Botón de encaje en la cabecera: centrar el escenario, volver al zoom de encaje y restablecer la rotación a 0°.
- Botón de flecha circular: herramienta para rotar la vista arrastrando en el escenario. El ángulo también se puede escribir en la cabecera; 0° restablece la orientación.
- La rotación de la vista no modifica ni exporta el dibujo girado.

## Proyectos FLA

Guardar y Guardar como producen `.fla` modernos: contenedores ZIP con `DOMDocument.xml` XFL, trazos vectoriales, capas, velocidad y fotogramas. No es JSON con otra extensión. Las curvas cuadráticas importadas se aproximan con segmentos editables.

Compatibilidad inicial deliberadamente limitada:

- Escenario blanco de 960 × 540, una escena, 1–60 fps enteros.
- Capas normales, trazos sólidos, fotogramas clave y exposiciones (convertidas en dibujos por fotograma).
- No admite todavía FLA binarios anteriores a CS5, símbolos, imágenes, textos, grupos, audio, vídeo, interpolaciones, filtros, scripts ni rellenos vectoriales. El pincel de Animate suele crear rellenos: esos dibujos todavía no se pueden importar.
- Un archivo con estos elementos se rechaza; no se abre parcialmente ni se sobrescribe.
- La compatibilidad con FLA generales de Animate sigue siendo trabajo pendiente. La salida debe verificarse con Animate para el flujo concreto que se use.

Los antiguos `.trazo` siguen siendo documentos JSON importables. La recuperación automática conserva el documento en `%LOCALAPPDATA%\Trazo\recovery.json` para mantener las sesiones existentes. Guarda los proyectos importantes en archivos FLA propios.

## Otras funciones

Pincel (B), rectángulo (R), color y grosor, capas, reproducción, papel cebolla, deshacer/rehacer y SVG del fotograma actual. Todavía no hay presión de tableta, símbolos, interpolaciones ni exportación de vídeo.

```powershell
cargo test --locked
cargo run --locked
cargo build --release --locked
```

Código: `canvas.rs` gestos y geometría; `workspace.rs` distribución; `design.rs` tema, iconos y logo; `document.rs` documento y SVG; `fla.rs` XFL; `fla_zip.rs` contenedor FLA.

Validación: el archivo ejemplo-interoperabilidad.fla generado por el editor se abrió en Adobe Animate 2024 y mostró el trazo, la capa, los 24 fotogramas y los 12 fps. Pasan 17 pruebas automatizadas, incluidas geometría, gestos de desplazamiento, FLA y rechazo de contenido no admitido.


## Pincel y rotaciones corregidos

El pincel dispone de suavizado regulable de 0 a 100 %, con 70 % inicial. Se aplica durante el dibujo sobre las muestras originales: filtrado según distancia e interpolación suave, conservando inicio y final. Los nuevos trazos tienen extremos redondos; los dibujos anteriores no se modifican automáticamente. La selección se representa con punteado sobre la tinta, sin mostrar los puntos de muestreo.

La transformación mantiene un marco orientado que gira con el dibujo. Las escalas posteriores trabajan en los ejes de ese marco. La rotación conserva distancias al pivote y grosor del trazo.

La rotación de la vista mantiene la escala y solo está disponible en la cabecera. Encajar restablece desplazamiento, zoom de encaje y ángulo a cero. Pruebas añadidas: reducción de vibración del pincel y conservación de extremos, rotación rígida, escala de vista independiente del ángulo y restablecimiento completo del encaje. El algoritmo del pincel está en `src/brush.rs`.
