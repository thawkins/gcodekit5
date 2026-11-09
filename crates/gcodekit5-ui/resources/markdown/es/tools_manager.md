# Herramientas CNC

La pestaña Herramientas CNC gestiona las definiciones de herramientas de corte utilizadas por los generadores CAM y los cálculos de avance/velocidad.

## Qué es (y qué no es) esta biblioteca
- ✅ Una **biblioteca de herramientas** para CAM y generadores.

- ❌ No es una "tabla de herramientas" del controlador (no incluye compensaciones automáticas de longitud de herramienta, etc.).

## Panel izquierdo: explorar y filtrar
Utilice el panel izquierdo para encontrar herramientas rápidamente:
- La **búsqueda** permite buscar por nombre, ID, tipo y texto común de diámetro.
- El filtro **Tipo** limita los resultados (fresas, brocas, brocas en V, etc.).
- El filtro **Material** limita la búsqueda por material/recubrimiento de la herramienta.
- Los filtros **Diámetro mín./máx.** ayudan a filtrar catálogos extensos.

## Panel derecho: editar detalles de la herramienta
Las herramientas se editan mediante pestañas (Geometría, Materiales, Notas, etc.).

### Geometría (lo más importante)
- **Diámetro**: diámetro de corte
- **Diámetro del vástago**: diámetro del vástago (compatibilidad con el portaherramientas)
- **Longitud de la ranura**: longitud de corte axial
- **Longitud total**: longitud total de la herramienta
- **Ranuras**: número de ranuras de corte
- **Radio de esquina** (si corresponde)
- **Ángulo de la punta** (brocas / brocas de centrado / brocas en V)

### Identificación
- El **ID de la herramienta** debe ser único y estable (recomendado: `vendedor_serie_diámetro_ranuras`, p. ej., `harvey_20008_6p0_2f`).
- Los números de herramienta no son necesarios a menos que un flujo de trabajo específico los requiera.

## Gestión de la biblioteca
Utilice el panel **Biblioteca** para:
- Importar catálogos de proveedores (GTC)
- Importar/exportar herramientas personalizadas para copias de seguridad/uso compartido
- Restablecer herramientas personalizadas/importadas (destructivo)

## Ejemplos de flujo de trabajo
- Crear una herramienta → definir geometría → Guardar.
- Importar un catálogo GTC → filtrar por diámetro/material → revisar/editar una herramienta.
- Exportar herramientas personalizadas a JSON para copias de seguridad/uso compartido.

## Relacionado
- [Herramientas CAM](help:cam_tools)
- [Materiales](help:materials_manager)
- [Índice](help:index)
