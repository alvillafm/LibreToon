# RefTool y recorte vectorial

## RefTool
En **Herramientas > RefTool**, cada pulsación crea una ventana flotante independiente. Usa el icono de carpeta para cargar una imagen PNG, JPEG, BMP o TIFF. Arrastra la barra de título para colocarla, sus bordes para cambiar el tamaño y usa Ajustar o el porcentaje de zoom para inspeccionarla. El cuentagotas de la ventana activa la herramienta existente: pulsa o arrastra sobre la imagen para tomar su color. Las transparencias se muestran sobre blanco y se muestrea ese color visible, con alfa 255.

Las referencias no forman parte del lienzo ni de las exportaciones. Sus imágenes originales, nombres, posiciones, tamaños y zoom se guardan dentro del documento nativo y del FLA nativo de LibreToon. No dependen de que siga existiendo el archivo original. La X elimina la referencia del proyecto; se puede deshacer. Los cambios al editar un símbolo se conservan al volver a la escena.

Límites: 32 ventanas, 16 MiB por imagen comprimida, 32 MiB en total y 16 millones de píxeles decodificados en total. Solo hay un trabajador de carga/decodificación simultáneo. Espera a que termine una importación antes de guardar, cerrar o actualizar. Una carga fallida conserva la imagen anterior.

## Recorte vectorial
Selecciona el icono de tijeras en Vector o **Herramientas > Recorte vectorial**. Pasa el cursor sobre un contorno: el tramo entre cruces se resalta en rojo. Un clic elimina solo ese tramo. Sin cruces se elimina el contorno completo. Los demás contornos y el relleno del objeto se conservan; el resultado mantiene nodos, tiradores Bézier y perfiles de anchura editables. La acción admite deshacer/rehacer.

Los cruces se calculan sobre los ejes vectoriales, también con contornos visibles de otras capas y con transformaciones de carpeta. Las capas bloqueadas no se recortan. Para recortar el interior de un símbolo, entra antes en su edición. Los mapas de bits y el texto no se recortan con esta herramienta. Espacio o botón central mantienen el desplazamiento del lienzo.
