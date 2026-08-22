# Grabador láser de imágenes

Convierte un mapa de bits (PNG/JPEG, etc.) en una trayectoria de grabado láser tipo raster (G-code).
Utiliza distínto método de generación de G-code que el utilizado en el Diseñador. Elija el que mejor le convenga.

## Generación
- Trayectoria de grabado de línea de escaneo (horizontal o vertical)
- Escaneo bidireccional opcional para reducir el tiempo de desplazamiento
- Valores de potencia de salida escalados al rango `S` del controlador

## Entradas clave
### Archivo de imagen
- **Trayectoria de imagen**: El mapa de bits de origen.

### Ajustes de salida
- **Ancho**: Ancho de salida deseado en mm (la altura se deriva de la relación de aspecto de la imagen).
- **Velocidad de avance**: Velocidad de avance del grabado.
- **Velocidad de desplazamiento**: Movimientos sin quemar entre segmentos de escaneo.

### Potencia del láser
- **Potencia mínima/máxima (%)**: Asigna el brillo de la imagen a un rango de potencia.
- **Escala de potencia (S)**: Valor máximo `S` del controlador (generalmente 1000 en GRBL).

### Escaneo
- **Dirección de escaneo**: Horizontal/Vertical.
- **Píxeles por mm**: Controla la resolución; a mayor resolución, mayor detalle, pero mayor tiempo de escaneo.
- **Espaciado entre líneas**: Distancia entre las líneas de escaneo.
- **Bidireccional**: Grabado en ambas pasadas (de ida y vuelta).

### Transformaciones
- Invertir, reflejar, rotar para que coincida con la orientación de la pieza en la máquina.

### Tramado
Métodos opcionales para representar la escala de grises mediante puntos/patrones.

### Desplazamientos de trabajo
Desplaza todo el grabado y, opcionalmente, lo posiciona antes de comenzar.

## Flujo de trabajo
1. Seleccione una imagen y confirme la vista previa.
2. Elija el ancho y la resolución de salida.
3. Configure la potencia y la velocidad de avance mínimas y máximas.
4. Genere y previsualice; verifique que los movimientos y límites coincidan con el material.

## Seguridad
- Comience con una potencia conservadora y realice pruebas en una pieza de desecho.
- Verifique que su máquina esté en modo láser (si corresponde) y que la ventilación sea adecuada.

## Relacionado
- [Herramientas CAM](help:cam_tools)
- [Visualizador](help:visualizer)
- [Índice](help:index)
