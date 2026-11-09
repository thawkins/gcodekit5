# Gerber a G-code

La herramienta Gerber a G-code permite convertir archivos Gerber estándar (comúnmente utilizados en la fabricación de PCB) a G-code para el fresado de aislamiento CNC.

## Uso

1. **Seleccionar directorio**: Haga clic en "Examinar..." para seleccionar la carpeta que contiene sus archivos Gerber. La herramienta detectará y asignará automáticamente los archivos de capas comunes (p. ej., Cobre superior, Perforaciones).

2. **Tipo de capa**: Seleccione la capa que desea procesar en el menú desplegable. El archivo detectado para esa capa se mostrará a continuación.

3. **Parámetros**:

* **Ancho/Alto de la placa**: Dimensiones de su PCB.

* **Desplazamiento X/Y**: Desplaza el origen del G-code.

* **Velocidad de avance**: Velocidad de corte en mm/min.

* **Velocidad del husillo**: RPM del husillo.

* **Profundidad de corte**: Profundidad Z del corte (negativa para cortar en el material).
* **Z seguro**: Altura de retracción para movimientos rápidos.

* **Diámetro de la herramienta**: Diámetro de la broca de grabado (broca en V o fresa).

* **Ancho de aislamiento**: Ancho adicional para dejar espacio alrededor de las pistas.

* **Eliminar exceso de cobre**: (Desgaste) Si está marcada, genera trayectorias para eliminar todo el cobre que no forma parte de las pistas.

4. **Agujeros de alineación**:

* **Generar agujeros de alineación**: Añade operaciones de perforación para los pines de alineación.

* **Diámetro del agujero**: Diámetro de los agujeros de alineación.

* **Margen**: Distancia desde el borde de la placa hasta los agujeros de alineación.

## Salida

Haga clic en **Generar G-code** para crear la trayectoria. El resultado se cargará en el editor/visualizador de G-code.
