# Matriz de capacidades y compatibilidad

Estado del árbol de trabajo: 24 de noviembre de 2026. La versión del manifiesto sigue siendo **0.9.0**; esta auditoría no constituye una nueva versión publicada. El estado de los trabajos restantes está en [AUDIT_IMPLEMENTATION.md](AUDIT_IMPLEMENTATION.md).

## Archivos y recursos

| Área | Comportamiento actual | Límite o comprobación necesaria |
| --- | --- | --- |
| Proyecto nativo | Apertura de JSON anterior y contenedores nativos 1/2; guardado compacto 1/2 o ZIP64 3 según el contenido | Sin techo agregado de 256 MiB. Los contenedores 3 grandes necesitan esta aplicación actualizada. Capacidad práctica según RAM y disco. |
| Guardado y recuperación | Instantánea compartida, trabajo de E/S en segundo plano, archivo temporal sincronizado antes de publicar, copia de seguridad y detección de conflicto | No prometer persistencia de una edición que aún está pendiente. Comprobar el indicador Guardado/Atención. |
| Concurrencia | Archivo de bloqueo estable para escritores LibreToon y publicación sin reemplazo de nuevas versiones | Un programa externo que ignore el bloqueo todavía puede competir en una ventana residual. |
| Medios | Recursos incrustados compartidos y decodificación limitada por recurso | 64 MiB por recurso multimedia; referencias con límites propios. No confundir estos límites con el tamaño total de un `.ltoon`. |
| Informe de almacenamiento | Tamaño de recursos, tamaño de metadatos compactos antes de compresión, referencias, duplicados exactos y archivos sin uso | Limpieza explícita y reversible. Los recursos de símbolos guardados en la biblioteca se consideran utilizados aunque el símbolo no esté en escena. |
| FLA/XFL | Importación tolerante de contenido compatible y exportación experimental. Degradados lineales/radiales nativos en relleno y contorno | ActionScript no se ejecuta. Consultar informe de omisiones/aproximaciones; el límite de intercambio FLA es independiente del formato nativo. No declarar equivalencia completa con Animate. |
| SVG | Trazados, formas básicas, transformaciones, relleno/contorno, gradientes y clones locales `use`/`defs` | Clipping, máscaras, filtros, CSS, texto e imágenes enlazadas no equivalen a SVG nativo completo. Referencias externas no se descargan; recursión y complejidad acotadas. |
| Secuencia de imágenes | Importación en segundo plano, cancelación y exposiciones | Hasta 10.000 fotogramas; cada imagen debe superar las comprobaciones del decodificador. |
| Exportación | Formatos de imagen/vector y vídeo mediante FFmpeg según capacidades instaladas | GIF con transparencia binaria y MOV ProRes 4444 con alfa parcial opcional. MP4/MKV/AVI permanecen opacos. Gestión ICC completa, FPS racionales y motion blur temporal siguen pendientes. |

## Dibujo, animación y rigging

| Área | Disponible | Pendiente en esta auditoría |
| --- | --- | --- |
| Nodos | Modos Corner/Smooth/Symmetric/Automatic, selección múltiple, tangentes, alineación agrupada, Simplify con candidato | Unificar todas las vistas previas destructivas y explicar todas las aproximaciones |
| Conversiones | Expand Stroke y Offset Path con candidato, conservar original, cancelación y deshacer | El resultado es un límite aplanado a 0,04 px; no conserva una curva Bézier analítica equivalente |
| Presión/pinceles | Curva de respuesta, grosor vectorial en píxeles de documento, presets raster, máscaras de punta importables, bloqueo alfa | Estabilizador opcional con guía de retraso disponible; pendiente textura orientada del contorno vectorial |
| Bote | Regiones vectoriales, cierres temporales y relleno multiframe con informe por dibujo | Vista previa opcional de región y puentes entre extremos disponible, con confirmar/cancelar; las siluetas sin eje no tienen puentes de extremos que representar |
| Shape tween | Interpolación compatible y diagnóstico por objeto/canal/geometría | Hints de correspondencia opcionales y resaltado de nodos incompatibles |
| Graph Editor | Curvas de aceleración compartidas o independientes para X/Y/giro/escala según dibujo, cámara, hueso o pin; indicador del fotograma actual | Valores absolutos y canales arbitrarios de efectos/controladores |
| Rigging | Huesos/pines, Character Symbol por vistas y controles de cara/manos/genéricos existentes | Guía contextual, vinculación/duplicado/renombrado por lotes y diagnóstico de pesos disponibles; pendiente ampliar revisión de todos los flujos |
| Lip sync | Claves y poses editables existentes | Anterior/siguiente clave, protección, regeneración por segmento y acceso al dibujo fuente disponibles; pendiente ampliar validación visual con rigs complejos |

## Rendimiento y verificación

El compositor utiliza GPU cuando está disponible y mantiene recursos de escena. En adaptadores compatibles, el viewport presenta directamente el buffer residente mediante callbacks de egui, sin descargar cada cuadro a RAM. La geometría se prepara en segundo plano. La exportación y la preparación explícita de cuadros en RAM mantienen su transferencia cuando es necesaria; hay una ruta compatible para adaptadores o escenas que no pueden utilizar la presentación directa. El render de referencia y la exportación conservan calidad completa.

Las cachés tienen presupuestos separados. La preferencia de historial limita una **estimación de geometría y recursos adicionales compartidos**, no toda la memoria del proceso. La coordinación global de RAM/VRAM sigue pendiente. Los trabajos geométricos de fondo comparten dos trabajadores y cuatro puestos de espera; esto no vuelve cancelable una booleana individual ya iniciada.

Los benchmarks de [PERFORMANCE.md](../PERFORMANCE.md) indican perfil, escenario y alcance. Los tiempos de evaluación con lectura GPU no equivalen a FPS de reproducción de la interfaz. Las pruebas sintéticas no sustituyen un corpus autorizado de Animate ni sesiones físicas de tableta o validaciones en todos los sistemas operativos.

Las versiones de aplicación y paquete se toman de `Cargo.toml`; la ventana usa `CARGO_PKG_VERSION`. Los informes de auditoría anteriores y notas de versión describen su fecha, no garantizan el estado del árbol actual.
