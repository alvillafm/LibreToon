# Aplicación de la auditoría general

Trabajo autorizado: los 45 puntos del informe, sustituyendo el límite agregado del proyecto por almacenamiento y apertura ZIP64 por bloques. Los estados se completan con sus pruebas; las funciones pendientes no se consideran entregadas.

| ID | Estado | Implementación y validación |
| --- | --- | --- |
| S01 | Verificado en desarrollo | Sin límite agregado nativo: escritor/lector ZIP64 por bloques, recuperación sincronizada a disco. Pruebas de más de 256 MiB de recursos y direcciones superiores a 4 GiB; apertura real desde archivo y conservación de formatos previos. |
| D01 | Verificado en desarrollo | Muestreo del degradado de contorno; modos Color/Copiar estilo separados y transparencia de imágenes conservada. Falta ampliar los casos de opacidad de capa. |
| R01 | Verificado en desarrollo | Puppet conserva degradado y paleta del contorno al generar su silueta; regresión añadida. |
| A01 | Verificado en desarrollo | Varita y selección conectada usan dibujo evaluado; selección de componentes diferida sin mutar dibujo ni historial. |
| U01 | Verificado en desarrollo | Luz/receptores y grosor de contorno usan transacciones de propiedad. Una sola entrada de historial por gesto. |
| P01 | Verificado en el equipo de prueba | Presentación directa del buffer GPU en callbacks de egui; geometría preparada en segundo plano, caché residente acotada y transferencia a RAM reservada a exportación/preparación. 18 comprobaciones de píxeles idénticos (gamma/sRGB), integración real sin readbacks, reutilización, preparación RAM y fallback simulado superados. DBGT a 1080p en release: p50 14,50 ms en cuadros nuevos con pipelines calientes y 0,94 ms en repetición residente. Ejecutable habitual compilado. Falta ampliar la matriz de hardware. |
| P02 | Parcial | Se retienen cuadros con dependencias evaluadas sin cambios, incluidos intervalos FX/luz. La caché residente GPU también retiene cuadros no afectados; las cámaras se comparan por sus valores evaluados en cada fotograma. Cambios en símbolos/rig aún invalidan conservadoramente. FPS y cambio de escena invalidan correctamente. |
| P03 | Verificado en desarrollo | Ediciones de alineación agrupadas por trazado; una conversión y una actualización de tangentes automáticas por fuente. |
| P04 | En verificación ampliada | Índice espacial en conectividad y cierre de huecos. Caché acotada de curvas y longitudes para picking. El bote reutiliza caras y barreras por geometría/nivel de cierre; cambiar color o añadir rellenos no repite la unión de contornos. Cuatro entradas acotadas, sin retener recursos ajenos al cálculo. Prueba de reutilización e invalidación y regresiones geométricas superadas. |
| P05 | Parcial | Presupuesto configurable de historial y recuento de geometría/recursos compartidos en segundo plano. La caché GPU tiene un presupuesto configurable conjunto para cuadros, imágenes de capas y normales, con expulsión de cuadros antiguos y conservación del visible. Contadores sin doble recuento de imágenes compartidas. Sigue sin ser un límite global de RAM/VRAM: faltan recursos temporales y coordinación con otras cachés. |
| P06 | Parcial | Cola común acotada para Simplify, conversiones, almacenamiento, multiframe y SVG. Conversiones y multiframe tienen cancelación/progreso. Los rellenos de un clic con geometría compleja pasan al mismo trabajador, sin ampliar el rango de fotogramas; los resultados se descartan si cambia documento o geometría. Las booleanas internas aún no admiten cancelación durante su ejecución. |
| P07 | Parcial | Benchmarks de composición CPU/GPU y viewport real de prueba, incluidos DBGT/1080p. Añadido benchmark de escritura/sincronización y apertura nativas, ejecutado en release sobre tres ejemplos y DBGT (ocho muestras). Resultados y alcance en PERFORMANCE.md. Falta latencia física de entrada y más proyectos/equipos; no se confunden tiempos sin ventana con FPS del monitor. |
| P08 | Parcial | Se evita propagar repetidamente exposiciones sin cambios. Regresión para sucesivas ediciones en un mismo gesto; queda agrupar mejor cambios de recuperación y registrar exposiciones sucias. |
| T01 | Parcial | Estado mixto de tipos de nodo y textos de ayuda/localización. Falta extender la previsualización a todas las operaciones destructivas. |
| T02 | Parcial | Curva de respuesta de presión con gráfico, puntas raster PNG/JPEG portables en presets y miniatura coherente. Estabilizador vectorial opcional por distancia en píxeles de documento, guía de retraso y conservación del extremo al soltar; desactivado por defecto. Pendiente textura orientada de contorno vectorial. |
| T03 | Parcial | Multiframe con progreso/cancelación e informe persistente por fotograma con motivos de omisión. Vista previa opcional de región y puentes de extremos, sin modificar el documento hasta confirmar. Comprobaciones de geometría y cancelación superadas. |
| T04 | Parcial | Diagnóstico detallado y selección del objeto incompatible en ambos extremos. Faltan puntos de correspondencia opcionales y resaltado de nodos concretos. |
| T05 | Parcial | Curvas de progreso independientes por canal para dibujos, cámaras, huesos y pines, con persistencia COW y marcador del fotograma actual. Falta edición de valores absolutos y canales arbitrarios de FX/controladores. |
| T06 | Parcial | Guía contextual opcional, siguiente vista vacía, prueba sin claves, vínculo/duplicado/renombrado por lotes con una acción de deshacer, máscaras copiadas remapeadas y diagnóstico de pesos. Pendiente revisar todos los flujos y aislamiento temporal. |
| T07 | Verificado en desarrollo | Bloqueo alfa con regresión de transparencia y puntas raster personalizadas con validación, importación en segundo plano y persistencia en presets. |
| T08 | Verificado en desarrollo | Expand Stroke / Offset Path desde propiedades de nodos: candidato sobre lienzo, trabajador, cancelación/progreso, conservar original y una acción de deshacer. Límites aplanados a 0,04 px declarados. |
| T09 | Pendiente |  |
| T10 | En verificación ampliada | El editor de diálogo ya permite rango de frase, waveform, scrub conjunto, edición y regeneración por tramos. Añadidos anterior/siguiente clave y protección explícita frente a regeneración. La combinación conserva claves manuales, datos fuera del tramo y pose de salida, con regresión específica. Acceso al dibujo fuente desde diálogo y propiedades de boca normal: símbolo/fotograma o capa/vista del Character Symbol, con retorno al fotograma de escena. Falta ampliar revisión visual con diálogos y rigs complejos. |
| UX01 | Parcial | Cuentagotas Color/Copiar estilo y canal explícito; falta una pasada completa a todos los accesos de color. |
| UX02 | Pendiente |  |
| UX03 | Parcial | El checklist muestra personaje, pieza, vista, estado y vínculo, además de distinguir edición de dibujo, pose de reposo y prueba sin claves. Falta extender el contexto a todas las operaciones de escena y vínculos anidados. |
| UX04 | Parcial | Layer Objects admite rangos Mayús, alternar Ctrl/Cmd y menú contextual con portapapeles/orden compartidos. Cambiar el orden conserva el dibujo evaluado en tweens. Pendiente unificar arrastre de objetos/carpetas y selección de todas las listas. |
| UX05 | Parcial | Controladores restringidos al lienzo, mínimo de ventana compatible con pantallas escaladas y límites adaptativos de timeline/inspector. Nueva regresión de las nueve combinaciones de 1280×720, 1366×768, 1920×1080 y 100/150/200 %, que detectó y corrigió la pérdida de espacio del lienzo. Queda la revisión visual de todas las ventanas, Color y rigs complejos en esa matriz. |
| UX06 | Parcial | Nuevos controles en catálogos y traducción española; algunos controles de nodos ya usan tr(). Se conserva fallback inglés en otros idiomas: no se consideran traducciones completadas. |
| UX07 | Parcial | Requisitos explícitos en multiframe, límites y selección de puntos del perfil, aplicación de estilo y control conducido por un hueso. Pendiente cubrir los restantes controles de vinculación, tweens e importación. |
| UX08 | Verificado en desarrollo | Indicador Guardando/Guardado/Cambios/Atención junto a pestañas, con rutas del proyecto y recuperación copiables. Conservado el scroll horizontal (regresión). |
| UX09 | Parcial | Botones personalizados exponen nombre accesible y foco visible. Falta revisión global de iconos, contraste y teclado. |
| S02 | Verificado en desarrollo | Bloqueo de archivo estable por destino entre instancias, verificación del estado externo y publicación atómica. Prueba de contención; un editor externo que ignore bloqueos conserva una ventana de carrera residual. |
| S03 | Verificado en desarrollo | Retención incremental optativa (0 conserva todo), nunca elimina recuperaciones ni el archivo recién guardado; prueba de cinco versiones. |
| S04 | Parcial | Mutaciones deterministas de archivos nativos/SVG, ZIP truncado/CRC/duplicados/rutas y referencias recursivas. No equivale a una campaña de fuzzing completa. |
| S05 | Parcial | Escritor con fallo de disco y regresiones de publicación/contención/recuperación. Pendiente matriz completa de disco real, permisos, cierre y GPU bloqueada. |
| S06 | Pendiente |  |
| S07 | Parcial | Separados archivo nativo por bloques, geometría de picking, operaciones y ejecutor. Clippy sin errores; eliminadas comparaciones obsoletas del límite agregado, incluida una suma lineal de toda la biblioteca al añadir recursos. Presupuesto Windows por archivo/lint registrado y comprobado en el Action, sin suprimir avisos existentes; verificador probado contra duplicados y reportes incompletos. Faltan servicios de edición comunes y reducir la deuda de avisos. |
| C01 | Pendiente |  |
| C02 | Verificado en desarrollo | Exportación XFL con LinearGradient/RadialGradient nativos para relleno y contorno; no rejilla de polígonos. Prueba de importación de XFL visible sin metadatos nativos. Pendiente ampliar corpus real. |
| C03 | Parcial | Instancias locales SVG use/defs expandidas con transformaciones y herencia, límites de recursión y sin red. Clipping/máscaras siguen limitados y se informa al importar. |
| C04 | Pendiente |  |
| C05 | Parcial | MOV ProRes 4444/4444 XQ con alfa opcional y presets compatibles. Codificación/decodificación FFmpeg comprobada con transparencia y bordes semitransparentes. Gestión ICC e importación de alfa de vídeo pendientes. |
| C06 | Verificado en desarrollo | Informe en Archivo, tamaños de recursos y metadatos compactos, usos en escenas/símbolos, eliminación sin uso y unificación exacta con remapeo. Trabajo en segundo plano, resultado obsoleto descartado y deshacer. |
| C07 | Parcial | Añadida CAPABILITIES.md, actualizado manual de formatos y aviso histórico de PERFORMANCE. Manifiesto/binario/paquetes/wiki usan 1.0.1; los paquetes leen Cargo.toml. Versión actualizada localmente; no se ha publicado una release. |

## Verificación

Última batería completada (24 de noviembre): 515 pruebas superadas, 0 fallos y 54 ignoradas (manuales). También pasan 18 pruebas de empaquetado, dos del verificador de diagnósticos, formato Rust y wiki estricta (20 páginas sin referencias locales rotas). Clippy termina sin errores; quedan 316 avisos únicos registrados, sin ocultarlos. Ejecutable habitual release recompilado. Presentación directa comprobada en 18 casos con diferencia cero, expulsión GPU conservando el cuadro visible, integración del viewport, GIF transparente y captura de interfaz sin escritorio superados. La paridad previa CPU/GPU conserva una diferencia máxima de 1–2 niveles por canal. Benchmarks de render/viewport y almacenamiento ejecutados en release; resultados y alcance en PERFORMANCE.md. No se han publicado cambios ni modificado proyectos del usuario.

## Dependencias externas

- S06: la autenticación real de distribuciones necesita la identidad/certificado o clave del editor y pruebas con paquetes reales. No se ha inventado una firma ni introducido credenciales.
- C01: el corpus de comparación con Animate necesita originales y renders de referencia con permiso para redistribuirlos. Las pruebas sintéticas existentes no se presentan como ese corpus.


## Solicitud adicional: GIF transparente

Añadida exportación RGBA a GIF con fondo transparente opcional, umbral alfa, paleta con entrada transparente y cuadros completos para evitar estelas. Conversión de alfa premultiplicado compartida con PNG. Prueba de archivo GIF codificado/decodificado con FFmpeg superada: transparencia, umbral y eliminación del cuadro anterior sin estelas. Ejecutable habitual recompilado con esta ampliación. GIF admite transparencia binaria; las capas de fondo visibles siguen siendo opacas.

## Ampliación verificada el 24 de noviembre

- Canales de aceleración independientes, preservación nativa y compatibilidad de curvas anteriores. Captura headless revisada del gráfico completo, sin controles recortados. No equivale todavía a un Graph Editor de valores absolutos.
- Vista previa del bote sin mutación previa: candidato, puentes y confirmación/cancelación. Ventana compacta y restringida al lienzo, captura revisada.
- Guía contextual del rig, operaciones por lotes y diagnóstico de pesos. Pruebas de una sola acción de deshacer y máscaras remapeadas al duplicar ojo/pupila.
- Selección por rangos y portapapeles/orden contextual en Layer Objects; regresión de reordenación de geometría evaluada dentro de un tween y restauración con deshacer.
- Navegación de poses labiales al dibujo fuente: símbolo normal y bloque de vistas del Character Symbol; pruebas de pose/vista correctas y retorno al fotograma de escena.
- MOV con alfa: archivo ProRes 4444 real codificado y decodificado con FFmpeg; píxeles transparentes y semitransparentes comprobados.
- Integración GPU real repetida con RTX 2060/Vulkan: presentación sin readbacks y retención selectiva superadas. No es una nueva medición de FPS ni una validación de todas las GPU.

Los estados parciales y pendientes de la tabla siguen abiertos. Esta ampliación no declara completados los 45 puntos ni publica una release.
