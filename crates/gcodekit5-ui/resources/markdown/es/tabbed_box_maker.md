# Creador de cajas con pestañas
Genera un diseño de caja con uniones dentadas y produce G-code para corte láser o fresado CNC.
El generador crea múltiples paneles 2D (laterales, tapa/base, separadores) y los organiza en un diseño plano para el corte.

## Entradas clave
### Dimensiones de la caja
- **X (Ancho), Y (Profundidad), H (Alto)**: Tamaño total de la caja.
- **Dimensiones exteriores**: Si está habilitada, las dimensiones se consideran exteriores; de lo contrario, son interiores.

### Configuración de la caja
- **Tipo de caja**: Permite eliminar caras específicas (sin tapa, sin base, etc.).
- **Separadores X/Y**: Número de separadores internos.
- **Clave de separadores**: Indica qué caras incluyen ranuras/pestañas para los separadores.
- **Optimizar diseño**: Intenta rotar/ajustar los paneles para que se adapten a las dimensiones del dispositivo.

### Ajustes del material
- **Espesor**: Espesor del material.
- **Diámetro de corte/herramienta**: Compensación de corte/herramienta.

### Ajustes de las uniones de dedos
- **Ancho del dedo/Ancho del espacio**: Expresado como múltiplos del espesor.
- **Espacios circundantes**: Espacio adicional cerca de los bordes.
- **Juego**: Tolerancia de ajuste.
- **Longitud adicional**: Añade longitud a las pestañas para un ajuste más preciso.

### Ajustes del láser
- **Pasadas/Potencia (S)/Velocidad de avance**: Parámetros de corte (tipo láser), utilizados al generar el G-code final.

### Desplazamientos del origen de trabajo
Desplaza todo el diseño y, opcionalmente, realiza un posicionamiento inicial antes de comenzar.

## Optimizar el diseño
El optimizador intenta posicionar y rotar las piezas para que se ajusten a los límites actuales del dispositivo.

Si un diseño no cabe:
- Se le preguntará si desea **Cancelar** o **Continuar**.
- **Continuar** ignora la condición de fuera de límites.

## Flujo de trabajo
1. Definir las dimensiones de la caja y el grosor del material.
2. Configurar las uniones y las opciones de divisores.
3. Generar y previsualizar los límites.
4. Cortar los paneles y ensamblarlos.

## Relacionado
- [Herramientas CAM](help:cam_tools)
- [Visualizador](help:visualizer)
- [Índice](help:index)
