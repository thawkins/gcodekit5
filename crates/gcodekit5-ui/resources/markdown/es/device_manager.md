# Administrador de dispositivos

En el Administrador de dispositivos, usted crea y mantiene los **perfiles de dispositivo** (las definiciones de su máquina), incluyendo información de conexión, límites de viaje y capacidades.

Un *perfil de dispositivo* es independiente de la conexión activa de la máquina: es un registro guardado que se puede reutilizar y compartir.

## Panel izquierdo: lista de dispositivos
- **Buscar** filtra los perfiles por nombre/descripción.
- El perfil que coincide con la conexión actual muestra un distintivo de **Conectado**.
- **Agregar dispositivo** crea un nuevo perfil.

## Panel derecho: editar pestañas
Los perfiles de dispositivo se editan mediante pestañas (p. ej., Conexión, Área de trabajo, Capacidades).

### Botones de acción
- **💾 Guardar** - Guarda los cambios en el perfil del dispositivo.
- **❌ Cancelar** - Descarta los cambios y cierra el formulario de edición.
- **🗑️ Eliminar** - Elimina el perfil del dispositivo (con confirmación).
- **🔄 Sincronizar desde el dispositivo** - Actualiza la información del dispositivo conectado (solo se habilita cuando está conectado). Esta información muestra:
- Dimensiones máximas de recorrido desde $130, $131, $132 (recorrido máximo en X/Y/Z).
- Velocidad máxima del husillo desde $30.
- Capacidad del modo láser desde $32.
- Información de la versión del firmware.
- **✓ Establecer como activo** - Establece este perfil como el dispositivo predeterminado.

### Conexión
Configurar cómo acceder al controlador:
- Puerto serie y velocidad de transmisión.
- Host/puerto TCP (si es compatible).
- Tiempo de espera de conexión / reconexión automática.

### Área de trabajo / límites de recorrido
Establecer los límites de recorrido de la máquina (X/Y/Z mín./máx.). Estas funciones se utilizan en toda la aplicación para:
- Superposiciones de límites del visualizador
- Comportamientos de ajuste al dispositivo del diseñador
- Comprobaciones de integridad de CAM

### Capacidades
Habilite las funciones que afectan a la interfaz de usuario y las trayectorias de la herramienta:
- **Tiene husillo** → habilita los campos de potencia del husillo y velocidad máxima del husillo
- **Tiene láser** → habilita los campos de potencia del láser
- Compatibilidad con refrigerante (si su controlador/máquina lo utiliza)

## Consejos para el flujo de trabajo
- Mantenga un perfil marcado como **Activo** como su máquina predeterminada.
- Utilice nombres descriptivos (p. ej., «Shapeoko 4 XL (GRBL)» o «Plataforma de diodo láser»).
- Si cambia el recorrido físico, actualice los límites inmediatamente para que las previsualizaciones sigan siendo precisas.
- **Configuración rápida:** Conéctese a su dispositivo, cree un nuevo perfil y, a continuación, utilice **Sincronizar desde el dispositivo** para completar automáticamente las dimensiones y capacidades desde la configuración del controlador.

## Relacionado
- [Configuración del dispositivo](help:device_config)
- [Control de la máquina](help:machine_control)
- [Visualizador](help:visualizer)
- [Índice](help:index)


