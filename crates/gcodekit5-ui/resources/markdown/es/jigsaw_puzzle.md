# Generador de puzle

Crea un patrón de corte tipo puzle y genera G-code.

Esta herramienta está diseñada principalmente para **cortadoras láser** (o herramientas CNC muy pequeñas) y produce una trayectoria de herramienta 2D que define las piezas del puzle que encajan entre sí.

## Qué genera
- Un rectángulo exterior (el límite general del puzle)
- Líneas de corte internas que forman las piezas, utilizando un patrón de conectores de puzle pseudoaleatorio
- Múltiples pasadas opcionales para cortar materiales más gruesos

## Parámetros clave
### Dimensiones del puzle
- **Ancho / Alto**: Tamaño total del puzle terminado.
- **Radio de esquina**: Redondea las esquinas exteriores.

### Configuración de la cuadrícula
- **Piezas horizontales / Piezas verticales**: Cantidad de piezas en cada dirección. Más piezas implican más cortes internos.

### Parámetros del patrón
- **Ancho de corte**: Compensa el ancho de corte (ancho de corte láser / diámetro de la herramienta). Aumente este valor si las piezas están demasiado juntas.
- **Semilla aleatoria**: Cambia la forma de las piezas manteniendo la misma cuadrícula. Usa **Aleatorizar** para probar rápidamente nuevos patrones.
- **Tamaño de pestaña / Variación**: Controla el tamaño y la variación del conector.

### Configuración del láser
- **Pasadas**: Número de pasadas de corte repetidas.
- **Potencia (S)**: Valor de potencia del husillo/láser (el láser GRBL típico usa `S`).
- **Velocidad de avance**: Velocidad de corte.

### Desplazamientos de trabajo
- **Desplazamiento X / Y**: Mueve todo el puzle lejos del origen de la máquina.
- **Inicio antes de comenzar**: Si está activado, se inserta `$H` al principio.

## Flujo de trabajo
1. Elige el tamaño del puzle y la cantidad de piezas.
2. Configura el ancho de corte y el tamaño del conector.
3. Selecciona la potencia/velocidad de avance del láser.
4. Genera, previsualiza en el Visualizador y ejecuta.

## Notas
- Para el corte CNC: asegúrese de que el diámetro de la herramienta y la configuración del ancho de corte sean adecuados, y considere reducir la cantidad de piezas.
- Siempre previsualice el resultado antes de cortar.

## Relacionado
- [Herramientas CAM](help:cam_tools)
- [Visualizador](help:visualizer)
- [Índice](help:index)
