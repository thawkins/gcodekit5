# Glossary

## A

**Arc (G2/G3)**: Circular motion command. G2 is clockwise, G3 is counter-clockwise.

**Axis**: A direction of motion. Standard CNC machines have X (left/right), Y (front/back), and Z (up/down) axes. Rotary axes are A, B, and C.

## C

**CAM**: Computer-Aided Manufacturing. The process of generating toolpaths (G-code) from a design.

**CNC**: Computer Numerical Control. Automated machine control using programmed commands.

**Coordinate System**: Reference frame for positioning. G54–G59 are work coordinate systems in G-code.

## D

**DRO**: Digital Readout. Display showing the current machine position for each axis.

**DXF**: Drawing Exchange Format. A CAD file format commonly used for 2D drawings and CNC import.

## E

**E-Stop**: Emergency Stop. An immediate halt of all machine motion.

## F

**Feed Hold**: A pause command that stops motion but retains the current position for resuming.

**Feed Rate**: Speed of cutting motion, typically in mm/min or in/min.

**FluidNC**: A modern CNC controller firmware with WiFi and modular configuration support.

## G

**G-code**: Programming language for CNC machines. Commands like G0, G1, G2, G3 control motion.

**Gerber**: A file format used to describe PCB (printed circuit board) copper layers and drill patterns.

**GRBL**: Open-source CNC motion control firmware, commonly used on Arduino-based controllers.

**grblHAL**: An enhanced GRBL implementation with extended features and network support.

## H

**Hatching**: Filling a closed shape with parallel lines for engraving or area removal.

**Homing**: Process of moving the machine to its reference position using limit switches.

## I

**Isolation Routing**: A PCB milling technique that removes copper around traces to create electrical isolation.

## J

**Jog**: Manual movement of machine axes using buttons or keyboard shortcuts.

## M

**M-code**: Miscellaneous function commands in G-code (e.g., M3 spindle on, M5 spindle off, M8 coolant on).

**MPos**: Machine Position. The absolute position of the machine relative to the home/reference point.

## N

**NavCube**: A 3D navigation control in the visualizer for quickly switching between standard camera views.

## O

**Override**: A real-time adjustment to feed rate, spindle speed, or rapid speed, expressed as a percentage of the programmed value.

## P

**Peck Drilling**: A drilling technique that retracts periodically to clear chips, preventing tool breakage.

**Probing**: Using a touch probe to automatically measure workpiece position or tool length.

## R

**Rapid (G0)**: A fast positioning move that does not cut material.

## S

**Soft Limit**: A firmware-enforced boundary that prevents the machine from moving beyond its configured travel range.

**Spindle**: The rotating tool holder on a CNC machine, or a laser module on a laser engraver.

**Spoilboard**: A sacrificial surface on a CNC machine bed that protects the machine table from cuts.

**Step Size**: The distance the machine moves per jog button press.

**STL**: Stereolithography file format for 3D models, used for importing 3D geometry.

**SVG**: Scalable Vector Graphics. An XML-based vector image format used for importing 2D designs.

## T

**Toolpath**: The sequence of machine moves that form a cutting or engraving operation.

## V

**V-Carve**: An engraving technique using a V-shaped bit that varies cut depth to produce variable-width lines.

## W

**WCS**: Work Coordinate System. A user-defined zero point for machining, independent of the machine's home position.

**WPos**: Work Position. The position of the machine relative to the active Work Coordinate System offset.

**Work Offset**: The distance from machine zero (home) to the work zero point.
