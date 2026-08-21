# Cuadrícula para mesa de trabajo

Genera un patrón de cuadrícula (líneas) para marcar o grabar ligeramente una mesa de trabajo.

## Qué genera
- Un conjunto de líneas horizontales/verticales con un espaciado fijo.
- Diseñado para marcado láser, grabado con broca en V o grabado superficial.

## Parámetros clave
### Dimensiones de la mesa de trabajo
- **Ancho/Alto**: Tamaño del área de la cuadrícula.
- **Espaciado de la cuadrícula**: Distancia entre líneas.

### Configuración del láser
- **Potencia del láser (S)**: Potencia de salida.
- **Velocidad de avance**: Velocidad de corte/marcado.
- **Modo del láser**: M3 (constante) o M4 (dinámico), según el controlador.

### Posicionamiento inicial
Opcionalmente, inserta `$H` al inicio.

## Flujo de trabajo
1. Establezca el ancho/alto según el área útil de la mesa de trabajo.
2. Seleccione el espaciado (generalmente 10 mm o 25 mm). 3. Seleccione una potencia baja adecuada para el marcado.
4. Genere y previsualice, luego ejecute.

## Relacionado
- [Herramientas CAM](help:cam_tools)
- [Visualizador](help:visualizer)
- [Índice](help:index)
