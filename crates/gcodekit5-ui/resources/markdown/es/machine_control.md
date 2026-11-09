# Control de la Máquina

<p align="center"><img src="../../help_images/machine_control.png" alt="Control de Máquina" width="700"></p>

El Control de la máquina es el lugar principal para conectarse al controlador y ejecutar tareas.
---
## Conexión
1. Editor de G-code **Edición y revisión del G-code** para lanzar a la máquina o guardar en archivo mediante el menu "Archivo". También puede mediante el Menú "Archivo" recuperar un archivo guardado anteriormente.
2. Seleccione el puerto serie. Si no aparece pulse el botón de Actualizar despues de haber encendido la máquina.
3. Haga clic en **Conectar**. Aparecerá como Conectado.
4. **Consola del dispositivo** para ver los mensajes de inicio y enviar comandos a la máquina.

Si tiene problemas de conexión:
- Verifique los permisos del dispositivo serie (p. ej., `/dev/ttyACM0`).
- Confirme la velocidad de transmisión correcta para su firmware.

---
## Advertencia de Fuera de Límites
  <img src="../../help_images/warning_limits.png" alt="Fuera de límites" width="700">
- En el caso de haber realizado un diseño que se sale de los límites del área de trabajo de la máquina, al generar el G-code, aparece una **Advertencia por Fuera de Límites** para que el usuario actue en consecuencia.

---
## Movimientos en manual
- Utilice el panel de control en pantalla.
- Utilice el control manual con teclado (si está habilitado) para un posicionamiento rápido.
- Configure **Paso (mm)** y **Avance manual** para controlar el movimiento.

---
## Inicio / Desbloqueo / Reinicio
- **Inicio** ejecuta el ciclo de inicio del firmware (requiere `$22=1` en GRBL).
- **Desbloqueo** borra las alarmas si el controlador está en estado de alarma.
- **Reinicio** realiza un reinicio suave.

---
## Transmisión de un trabajo
1. Una vez Cargado o generado el G-code.
2. Haga clic en **Enviar** para iniciar la transmisión.
3. Use **Pausa/Reanudar** según sea necesario.
4. **Detener** cancela la transmisión.

## En caso de emergencia pulsar E-STOP y se enviará a la máquina una parada por emergencia
---
## Relacionado
[Editor de G-code](help:gcode_editor)
[Consola de Dispositivo](help:device-console)
[Visualizer](help:visualizer)
[Index](help:index)
