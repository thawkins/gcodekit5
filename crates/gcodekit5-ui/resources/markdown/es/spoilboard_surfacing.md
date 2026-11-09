# Mecanizado de la base (sufridera o mesa de trabajo)

Genera una trayectoria de mecanizado planar para aplanar la base.

## Qué genera
- Una trayectoria de mecanizado rasterizada de ida y vuelta sobre un área rectangular.
- Retracciones y entradas seguras en el eje Z, adecuadas para una fresa de superficie/fresa de corte.

## Parámetros clave
### Dimensiones de la base
- **Ancho/Alto**: Área a mecanizar.

### Ajustes de la herramienta
- **Diámetro de la herramienta**: Diámetro de la fresa de superficie.
- **Profundidad de corte**: Profundidad por pasada.
- **Solapamiento lateral (%)**: Solapamiento lateral por pasada (normalmente entre el 40 % y el 70 %).

### Ajustes de la máquina
- **Velocidad de avance**: Avance de corte.
- **Velocidad del husillo**: RPM objetivo del husillo.
- **Z seguro**: Altura libre para avances rápidos.
- **Posición inicial antes del inicio**: Opcionalmente, inserta `$H` al inicio.

## Flujo de trabajo
1. Ajuste el rectángulo de superficie para que coincida con el área de la base que desea aplanar.
2. Elija una profundidad conservadora y un paso lateral adecuado.
3. Genere y previsualice los límites.
4. Ejecute con un sistema de extracción de polvo adecuado.

## Seguridad
- Confirme el cero del eje Z y asegúrese de que las abrazaderas estén por debajo de la altura de superficie.
- Utilice profundidades conservadoras si su máquina es de uso ligero.

## Relacionado
- [Herramientas CAM](help:cam_tools)
- [Visualizador](help:visualizer)
- [Índice](help:index)
