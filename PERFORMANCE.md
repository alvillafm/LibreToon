# Base de rendimiento

Esta iteración reduce trabajo durante la edición sin cambiar el formato de los proyectos. No constituye una comparación con Adobe Animate.

## Datos e historial

`SharedVec<T>` utiliza `Arc<Vec<T>>` con copia al modificar. El documento comparte capas, las capas comparten la lista de fotogramas, cada fotograma comparte sus trazos y cada trazo comparte sus puntos. Crear un estado del historial es O(1) y no duplica geometría. Al editar se copian los contenedores compartidos de la rama afectada; cambiar los puntos de un trazo copia esos puntos si otra instantánea los conserva. Duplicar un fotograma comparte inicialmente el dibujo y mantiene independientes las ediciones posteriores.

Esto reduce memoria, pero no establece un límite de bytes para el historial: siguen existiendo hasta 60 estados y los trazos realmente modificados necesitan memoria propia. El coste de modificar un contenedor compartido depende del número de elementos de ese contenedor.

## Pincel

`StrokeSmoother` conserva el muestreo por distancia y actualiza solo la cola afectada por el filtro de ventana y la interpolación. Los puntos anteriores ya estables se conservan. Una muestra que salta una distancia grande sigue requiriendo trabajo proporcional a los nuevos puntos generados. El filtro original se conserva únicamente en las pruebas como referencia; se compara el resultado completo de cada prefijo del trazo, incluidos extremos, saltos, muestras repetidas y diferentes intensidades.

El renderizado y la teselación del trazo completo siguen teniendo coste proporcional a su longitud. La mejora del filtro no elimina ese coste.

## Línea de tiempo y reproducción

Solo se generan dibujos, interacciones y textos de ayuda para las filas y columnas visibles. La extensión completa del contenido se conserva para desplazar la vista. La prueba de interfaz compara el número de formas generadas con un documento pequeño y otro de 100 capas × 10.000 fotogramas. La reproducción solicita repintados según el siguiente fotograma y conserva el tiempo sobrante para reducir deriva.

## Recuperación

Un único trabajador serializa y escribe fuera del hilo de interfaz. Mantiene como máximo una instantánea pendiente, reemplazada por la edición más reciente, además de la que esté escribiendo. Agrupa cambios durante 250 ms sin posponerlos indefinidamente. Al cerrar normalmente, escribe la última instantánea y espera al trabajador. Una interrupción abrupta puede perder los cambios aún pendientes.

El JSON se escribe con un búfer de tamaño acotado a un archivo temporal, sin construir otro documento JSON completo en memoria, y después se renombra. Los proyectos JSON anteriores siguen siendo legibles y el formato FLA conserva sus limitaciones existentes. Guardar FLA manualmente todavía se ejecuta en el hilo principal.

## Validación reproducible

Desde una terminal con Rust estable y MSVC/Windows SDK disponibles:

```powershell
cargo fmt --all -- --check
cargo test --locked
cargo test --release --locked benchmark_ -- --ignored --nocapture --test-threads=1
```

Las mediciones manuales no tienen umbrales temporales en las pruebas automáticas: cambian según CPU, carga, antivirus y modo de compilación. El benchmark del pincel compara 5.000 muestras con el filtro anterior en cada prefijo; el del historial informa tiempo por instantánea frente a las copias de geometría del modelo anterior para 2.400.000 puntos lógicos. Son mediciones de componentes, no FPS globales ni resultados de Animate.

Antes de añadir funciones grandes, medir latencia de entrada y tiempo de cuadro (mediana, p95 y p99), memoria residente y picos al guardar con documentos representativos. Quedan pendientes caché de geometría/teselación, descarte espacial de trazos fuera de vista, límite del historial por memoria, formato de recuperación incremental y pruebas comparativas con Animate usando los mismos documentos y equipo.


## Medición local del 30 de octubre de 2026

Windows x64, AMD Ryzen 5 3550H (4 núcleos / 8 hilos), Rust 1.99.1. Perfil `release` predeterminado, sin opciones adicionales de CPU ni LTO. Una ejecución por benchmark, secuencial; el tiempo por instantánea es un promedio de las iteraciones indicadas. Son resultados sintéticos locales, sin comparación con Animate.

| Escenario | Referencia anterior | Implementación actual |
| --- | ---: | ---: |
| Suavizar cada prefijo de un trazo sintético de 5.000 muestras | 698,0637 ms en total | 2,1456 ms en total |
| Crear instantánea con 2.400.000 puntos lógicos | 7,027081 ms por copia de geometría, 100 iteraciones | 12 ns por instantánea compartida, 100.000 iteraciones |
| Formas de la interfaz con 10 capas × 100 fotogramas frente a 100 × 10.000 | — | 846 frente a 846 en la misma ventana, prueba sin optimizar |

El primer benchmark conserva el cálculo original para comparar y verifica igualdad de todos los puntos finales. La reducción de tiempo del filtro es de aproximadamente 325,3 veces en este escenario; no mide el renderizado, la teselación, el guardado ni la latencia global del editor. La referencia del historial reproduce las copias profundas de geometría del modelo anterior, no una ejecución del programa antiguo completo; además, la geometría fuente está compartida en la preparación del benchmark. Los 12 ns no incluyen editar, liberar geometría antigua ni serializar una recuperación.

Validación realizada: 27 regresiones superadas tanto en desarrollo como en `release`, y los 2 benchmarks manuales superados en `release`; comprobación de formato Rust y sintaxis de PowerShell. El ejecutable de desarrollo permaneció abierto ocho segundos sin errores de arranque con una recuperación temporal aislada. Esta comprobación no sustituye una sesión de dibujo manual.

Los lanzadores normales abren `target/release/libretoon.exe`; `Desarrollar.cmd` conserva el ciclo rápido de compilación de desarrollo. Al guardar código, el reinicio del editor permite el cierre normal para que termine la recuperación pendiente.


## Iteración de dibujo vectorial

El filtro incremental anterior se mantiene durante el dibujo. Ahora se procesan todas las muestras PointerMoved de cada cuadro, no solo la posición final. Al terminar se ejecuta un ajuste de mínimos cuadrados con subdivisión acotada y tangentes continuas. El pincel vectorial conserva esos nodos; el estándar genera el área de tinta una sola vez al finalizar. El trabajo de cierre del estándar depende de la longitud y de las intersecciones del recorrido.

La geometría preparada se conserva en coordenadas del documento. La caché distingue identidad de los datos compartidos, topología y nivel de detalle; cambiar solo el color no vuelve a teselar. Conserva aproximadamente hasta 32 MiB de polilíneas y mallas y descarta geometría que deja de usarse. No es un límite global de RAM. Los borradores evitan la caché para no mantener una referencia que obligue a copiar el trazo completo en cada muestra.

Los recortes de rellenos usan operaciones booleanas de polígonos; las curvas del borde se aproximan con tolerancia de 0,04 unidades. Los contornos se cortan por sus intersecciones con el marco y subdivisión de De Casteljau: mantienen sus tiradores. Estas operaciones se ejecutan al terminar el marco de selección, no en cada movimiento del cursor.

Medición local adicional, mismo Ryzen 5 3550H y Rust 1.99.1, perfil release predeterminado, ejecución secuencial única:

| Componente, trazo sintético de 5.000 muestras | Resultado |
| --- | ---: |
| Ajuste a 24 segmentos Bézier | 2,4306 ms |
| Convertir ese recorrido en tinta de relleno de 8 unidades de grosor | 19,3265 ms |
| Recortar el relleno resultante con un marco | 0,9341 ms |
| Filtro incremental durante todos los prefijos | 2,0469 ms |
| Filtro anterior durante todos los prefijos | 649,1774 ms |

Los tiempos de ajuste y relleno corresponden al cierre del trazo, no a cada muestra. No incluyen renderizado ni latencia de entrada global. Las diferencias respecto a la medición anterior dependen de la carga del equipo y no sustituyen una comparación estadística. La prueba de línea de tiempo sigue mostrando el mismo trabajo con 1.000 y 1.000.000 de celdas: 872 formas de interfaz en ambos casos tras añadir los controles nuevos.

Se añadieron regresiones de dibujo fuera del escenario, conservación de varias muestras en un cuadro, pluma, modos de rectángulo, selección parcial, huecos y conservación de área, subdivisión de contornos cúbicos, continuidad del ajuste, caché, curvas y rellenos en FLA, y rechazo de metadatos nativos desactualizados. La comprobación visual con un perfil aislado confirmó dibujo fuera del blanco, separación y desplazamiento de un fragmento y edición de un tirador Bézier.

Siguen pendientes perfiles globales de escenas representativas, selección espacial para documentos grandes, fusión automática de rellenos superpuestos y comparación equivalente con Animate. La caché no elimina el coste de transformar y enviar los vértices visibles a la interfaz gráfica.


## Iteración de interfaz, relleno y anchura

El modelo conserva las dimensiones del escenario y sus FPS con valores predeterminados compatibles con los proyectos anteriores. Cambiar resolución no escala geometría; el encaje y las conversiones entre pantalla y documento usan el centro y tamaño actuales.

Los perfiles de anchura guardan hasta 128 puntos normalizados por longitud, con multiplicadores de 0 a 4 e interpolación suave. Los puntos intermedios necesarios para representar el perfil se muestrean incluso sobre segmentos rectos. El contorno conserva su eje; su silueta se calcula al invalidarse y se guarda en la caché de geometría. La identidad del perfil y el grosor base forman parte de la clave de la caché. Colores distintos reutilizan la geometría. Los recortes remapean el perfil al subtrazado seleccionado.

El bote trabaja solo sobre el fotograma de la capa activa. Construye las regiones delimitadas por la tinta y selecciona la que contiene el clic. Los puentes para cerrar huecos son temporales. Las caras generadas por el bote no se reutilizan como barreras, evitando que el área se contraiga al volver a colorearla. Esta implementación recorre los contornos activos al hacer clic; no usa un índice espacial y escenas con muchos contornos necesitan perfiles adicionales.

Medición release en el Ryzen 5 3550H y Rust 1.99.1 usados en las pruebas anteriores: promedio de 100 ejecuciones por operación, secuenciales.

| Operación | Tiempo medio |
| --- | ---: |
| Silueta de contorno ondulado procedente de 2.000 muestras, con eje Bézier ajustado | 0,158595 ms |
| Bote dentro de un rectángulo de 400 × 300 unidades | 0,078634 ms |

Son escenarios sintéticos pequeños y no miden latencia global, trabajo gráfico ni un documento de producción. Las regresiones comprueban resolución y FPS en FLA/SVG, perfiles no válidos, conservación de área hueca en contornos cerrados, recortes con perfil, cierre de huecos por nivel, recoloración, duración reversible y ensanchado local con deshacer. Una regresión adicional verifica Mayús+W con pulsación y liberación en el mismo cuadro.

La revisión visual confirmó la limpieza del panel derecho, los controles de FPS y duración en la línea de tiempo, los presets de resolución y el aspecto del trazo de puntas finas. La revisión de ventanas se detuvo cuando se recibió Escape. El zoom de cabecera se corrigió para mostrar la escala real y los ajustes de documento se centraron después de esa revisión.


## Corrección de líneas en puntas de anchura cero

La silueta variable era correcta, pero el suavizado la rodeaba con una línea cerrada de egui. Sus uniones en picos casi colineales generaban vértices muy alejados de la tinta. La regresión de renderizado reprodujo una salida de unos 50 puntos de pantalla más allá de la punta a escala 0,2.

Los rellenos y contornos variables ahora usan una franja de antialiasing explícita, con extrusión limitada a 1,5 píxeles físicos y manejo de huecos y orientación del contorno. No se modifica el eje, el perfil ni la geometría guardada. Las pruebas comprueban la malla final que entrega egui, no solo el polígono del documento: puntas finas, extremos casi nulos, anchura cero interior, curvas, siluetas exportadas, rotaciones, zoom y varias densidades de píxeles.


Medición del borde sobre una silueta Bézier obtenida de 2.000 muestras y perfil de puntas finas, con geometría ya preparada: promedio de 200 cuadros en release. Incluye preparar las formas de borde y su teselación, sin dibujar el cuerpo ni enviar datos a la GPU. El borde anterior tardó 62,613 µs por cuadro y el limitado 77,342 µs: unos 14,729 µs adicionales por contorno en este escenario. La prioridad de esta corrección es eliminar las uniones sin límite; no se afirma una mejora de rendimiento.

Las tres regresiones nuevas pasan junto a la batería anterior. Cubren la malla final, distintas escalas y rotación, puntas casi nulas, ceros interiores, rellenos exportados, huecos y varias densidades de píxeles. Los trazos guardados no se reescriben: la corrección se aplica al volver a renderizarlos.

## Herramientas de estudio y exportación

La vista de solo escenario añade cuatro semiplanos de máscara, incluidos los giros de vista. No recorta los datos del documento. Las reglas generan marcas solo para el intervalo visible, con un límite adicional de 2.001 marcas por regla. Las guías son estado de vista de la sesión.

La simetría duplica únicamente la geometría de la pincelada, forma o trazado afectado. Con ambos ejes activos puede generar cuatro versiones. Los pinceles tonales ejecutan las intersecciones al soltar el gesto, sobre los rellenos del fotograma de la capa activa. Conservan los contornos, pero su coste depende de los polígonos y sus intersecciones; no tienen índice espacial ni un límite de latencia demostrado para dibujos complejos.

La exportación toma una instantánea compartida y renderiza en un trabajador. Usa la teselación del editor y rasterización de triángulos en CPU; mantiene un búfer RGB del fotograma actual y envía fotogramas a FFmpeg por una tubería. No materializa la película completa en memoria. El GIF usa paleta por fotograma. La resolución máxima es 4.096 × 4.096; ese búfer RGB ocupa aproximadamente 48 MiB, además de la geometría, el contexto y la memoria del proceso FFmpeg. Esto no constituye un límite global de RAM. La caché geométrica conserva el límite previo.

Las regresiones verifican máscara con rotación, guías arrastrables, simetría con anclas y perfil, portapapeles/espejo con deshacer, edición tonal acotada al relleno con conservación de área/contornos, atajos con modificadores y renderizado sin capas ocultas. La prueba manual `export_all_containers_with_ffmpeg` codifica MP4/SVG/MKV/AVI/MOV/GIF y decodifica los formatos raster para comprobar el número, orden y color de los fotogramas. No mide rendimiento global ni compara con Animate.

La revisión visual no se completó: el control de aplicaciones se detuvo al recibirse Escape. Las comprobaciones automáticas no sustituyen esa revisión.

### Ajustes posteriores de interfaz

La cabecera unificada envía los comandos nativos de mover, maximizar/restaurar, minimizar y cerrar. Se ha añadido una regresión de los tres botones de ventana. Las reglas ocupan los bordes del área central; su franja se reserva fuera del área de dibujo, con etiquetas enteras cuando procede y texto vertical girado. El trabajo de las marcas sigue acotado al intervalo visible.

Se revisaron renderizados sin ventana de la interfaz principal, las categorías de preferencias, el desplegable de pinceles y la exportación. Estos permiten comprobar colocación, textos y superposiciones, pero no sustituyen una prueba manual de movimiento y redimensionado en Windows. La batería actual supera 83 pruebas automáticas; las seis pruebas manuales permanecen excluidas por defecto.

## Rellenos toon, composición y recursos

Fill Brush recorta la pincelada a los rellenos existentes del fotograma de la capa activa. Los pinceles tonales guardan contadores independientes de sombra y luz en cada fragmento vectorial. La vista previa excluye zonas que ya han alcanzado el nivel permitido. El recorte de la vista previa hace operaciones booleanas durante el gesto: esta ruta necesita perfiles con dibujos complejos y no se afirma una latencia constante. El cierre modifica únicamente los rellenos afectados y mantiene sus contornos.

Las exposiciones conservan los fotogramas físicos compatibles con el formato anterior, con una lista adicional de claves. Al confirmar una edición se comparten las referencias de la exposición afectada; el coste depende de su duración. Los proyectos anteriores siguen cargando como fotogramas independientes cuando no contienen metadatos de claves. Las filas/columnas de la interfaz siguen virtualizadas; la regresión con 100 capas y 10.000 fotogramas mantiene el trabajo gráfico acotado a la ventana.

La composición resuelve únicamente el fotograma solicitado: visibilidad, opacidad, máscaras de silueta, instancias de símbolos y cámara activa. Las máscaras y las instancias transformadas pueden crear geometría temporal y reducir la reutilización de caché. La caché de teselación mantiene su límite previo; no se introduce un límite global de memoria para recursos o historial. Los símbolos y sus capas usan los contenedores compartidos existentes, con ciclos rechazados y profundidad máxima de ocho símbolos. La recuperación durante la edición de símbolos genera una instantánea del proyecto completo con las definiciones actualizadas.

Los degradados lineales subdividen las mallas en los límites de los colores; los radiales usan hasta 32 subdivisiones por lado de cada triángulo. SVG conserva definiciones de degradado. El XFL de interoperabilidad proyecta la escena y aproxima cada relleno degradado con una cuadrícula de 24 por 24 recortes. La información nativa conserva los recursos editables; el límite de 64 MiB del contenedor sigue vigente. El guardado manual FLA continúa en el hilo de interfaz y puede resultar costoso con escenas grandes.

Medición sintética local en perfil release predeterminado, una ejecución secuencial: resolver 200 rectángulos planos tardó de media 95,923 microsegundos por fotograma (200 iteraciones). Aplicar una pincelada toon sobre 200 rellenos rectangulares superpuestos tardó 1,30868 ms por operación (20 iteraciones, restaurando la instantánea entre ellas). No incluye la previsualización, teselación, envío a GPU, latencia global ni una comparación con Animate.

La validación añade regresiones del gesto real de Fill Brush y del bloqueo de capa, límites toon y recortes posteriores, degradados radiales rasterizados, claves/exposiciones, máscaras/opacidad, persistencia de recursos, cortes de cámara contiguos/reordenables y modos de reproducción de símbolos. Se revisaron renderizados sin ventana del timeline, las propiedades, la biblioteca, perspectiva y cámaras; no sustituyen una sesión manual de edición completa en Windows.


## Escenas, interpolación y miniaturas (30/10/2026)

Los contenidos de las escenas y sus capas siguen usando SharedVec. Cambiar de escena intercambia referencias; la recuperación integra la escena activa en la instantánea COW y serializa fuera del hilo de interfaz. Los símbolos son recursos compartidos del proyecto. Reproducción, SVG y raster de vídeo/GIF usan el mismo resolvedor de escenas ordenadas, interpolaciones y cámaras.

Solo se interpolan las claves vecinas del fotograma actual. Los identificadores de objeto se indexan una vez por capa/fotograma para evitar búsquedas cuadráticas. Se conservan anclas, tiradores, huecos, degradados y matrices de símbolos; la interpolación genera geometría temporal, no reescribe todos los fotogramas del documento. Las transformaciones directas materializan una clave al iniciar la modificación. Las miniaturas solo resuelven los elementos visibles y el selector de fotogramas virtualiza sus filas; la caché de teselación tiene el mismo límite de 32 MiB y caducidad por fotograma que el lienzo.

Las pruebas cubren posición/giro/escala, fotograma de símbolo, aceleración, trayectoria, claves de cámara, arrastre real, selección y cuentagotas de degradados, persistencia FLA, orden de escenas y píxeles del raster exportado. La revisión visual usa renderizados sin ventana de ocho estados de interfaz; no equivale a una sesión de edición completa en Windows.

Benchmark reproducible: `cargo test --release --locked benchmark_motion_resolution -- --ignored --nocapture`. Resuelve 200 rectángulos animados durante 200 iteraciones, sin teselación, GPU ni interfaz. El guardado FLA conserva las escenas editables en metadatos y concatena una proyección XFL de sus fotogramas; continúa siendo síncrono y puede ser costoso con películas grandes.

Resultado local de una ejecución secuencial en perfil release predeterminado: **281,083 microsegundos por fotograma resuelto**. Este dato no mide FPS de la aplicación ni es comparable por sí solo con otros editores.


## Inicio, documentos y referencias (noviembre de 2026)

Cambiar de pestaña mueve el contexto completo de edición entre sesiones; el documento y el historial mantienen los contenedores SharedVec. Cada documento tiene recuperación independiente. El listado de proyectos contiene rutas explícitas y carpetas elegidas, sin recorrer el disco completo. Los fotogramas de las tarjetas guardadas se rasterizan fuera del hilo de interfaz con una petición pendiente como máximo. La caché conserva hasta 96 tarjetas de 8 imágenes de 192 × 108: aproximadamente 61 MiB de texeles como máximo, más la sobrecarga de texturas y la memoria temporal del proyecto que procesa el trabajador. Las tarjetas idénticas se reducen a un fotograma. No es un límite global de memoria de la aplicación.

El maniquí tiene 16 articulaciones y una jerarquía de tamaño fijo. No recorre los fotogramas del proyecto para calcular la pose. Sus formas se generan solo en el lienzo y no forman parte del resolvedor de exportación. Las proporciones cambian longitudes, y las rotaciones conservan las longitudes de los huesos.

Los glifos del texto se convierten en contornos al confirmar la edición. El renderizado posterior utiliza la caché geométrica existente, sin volver a cargar la fuente. Las operaciones booleanas sobre texto son atómicas o generan una proyección geométrica al aplicar máscaras. La composición y selección no destruyen el texto editable por fragmentos.

El perfil de presión se limita a 128 puntos y el muestreo de la entrada a 2.048 muestras mediante reducción progresiva. El aerosol genera un máximo de 2.049 sellos por gesto; el grafito hasta cinco fragmentos por sello. La vista previa del aerosol depende de la longitud del gesto, y los documentos con muchas pinceladas translúcidas necesitan perfiles adicionales. No se afirma latencia constante ni una mejora global de FPS. La presión necesita un evento real del dispositivo o un controlador Wintab; no puede validarse físicamente sin tableta.

Validación automatizada añadida: aislamiento de documentos/historial/vista/símbolos, pestañas, curvas redondeadas, persistencia de texto y referencias, exclusión de exportación de bocetos anidados, conservación de longitudes del maniquí, presión y renderizados de propiedades. `cargo test --locked render_studio_review_images -- --ignored --nocapture` genera imágenes PPM para revisión visual; `LIBRETOON_PREVIEWS` elige la carpeta de salida. Estos renderizados no sustituyen una sesión manual con Windows y tableta.
