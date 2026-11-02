# LibreToon

Antes de modificar código, leer `README.md` y `PERFORMANCE.md` y comprobar el estado de Git. El objetivo es un editor de animación 2D nativo en Rust con baja latencia y uso de memoria controlado.

- Mantener los datos compartidos del documento y su aislamiento al editar. No introducir copias profundas del proyecto en rutas de interacción.
- Evitar trabajo proporcional a todo el proyecto al dibujar, mover el cursor o recorrer la línea de tiempo. Calcular solo lo visible o lo invalidado cuando sea posible.
- Mantener la serialización de recuperación fuera del hilo de interfaz, con una cola acotada y escritura de la última edición al cerrar normalmente.
- Conservar compatibilidad con los documentos existentes. La compatibilidad FLA es limitada: importar lo compatible y registrar en el informe los elementos omitidos o aproximados, sin ejecutar scripts. Esta política de importación tolerante fue solicitada explícitamente por el usuario.
- Para cambios en historial, pincel, línea de tiempo o recuperación, ejecutar las pruebas de regresión correspondientes y `cargo test --locked`. Comprobar también `cargo fmt --all -- --check`.
- Si cambia una ruta de rendimiento, usar las mediciones reproducibles descritas en `PERFORMANCE.md` cuando sean relevantes. Identificar perfil de compilación, escenario y límites de cada resultado. No afirmar superioridad sobre Adobe Animate sin una comparación equivalente.
- Las pruebas de rendimiento automáticas verifican crecimiento del trabajo y aislamiento de datos, sin umbrales de tiempo dependientes de la máquina. Los benchmarks temporales son manuales.
- No publicar, hacer push ni descartar cambios ajenos sin una instrucción del usuario.

- Para cambios de dibujo, conservar anclas y tiradores en transformaciones, SVG y guardado nativo. Los rellenos y contornos deben poder editarse por separado. No limitar el dibujo al escenario blanco.
- Leer las limitaciones de selección y FLA descritas en README antes de ampliar geometría. Los metadatos nativos no pueden ocultar cambios o contenido no admitido del XFL.
- Antes de pedir al usuario que pruebe una modificación, compilar la versión release que abren los lanzadores normales.

- El panel derecho es exclusivo de propiedades de herramientas: no reintroducir documento, FPS, capa activa ni paneles de ayuda general allí. FPS, duración y capas se gestionan desde la línea de tiempo; resolución desde Archivo > Ajustes de documento.
- Conservar el perfil de anchura, el eje y la resolución al guardar, transformar y recortar. Los puentes del bote no deben alterar los contornos originales y los rellenos abiertos al exterior se deben rechazar.

- No rodear las siluetas de grosor variable con `Shape::closed_line` para suavizarlas: sus uniones pueden producir líneas largas en puntas de anchura cero. Mantener acotada la franja de antialiasing y ejecutar las regresiones que comprueban los vértices finales de renderizado.
