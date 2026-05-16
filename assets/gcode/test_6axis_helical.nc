; 6-Axis Helical Interpolation Test
; Tests helical arcs with simultaneous rotary axis motion
; 
; GCodeKit5 Test File - Helical Arcs
; Machine: 6-Axis CNC with Rotary Table

G21              ; Metric units
G90              ; Absolute positioning
G17              ; XY plane

; ============ SETUP ============
G0 X0 Y0 Z50
A0 B0 C0
S3000 M3         ; Start spindle

; ============ HELICAL ARCS WITH ROTARY AXIS ============

; Helix 1: CW arc with continuous A rotation
; Creates a helical cut while tilting the A axis
G0 X50 Y50 Z20
G1 F1000
G2 X50 Y50 Z0 A360 I25 J0 F800   ; Full circle with A rotation

; Helix 2: CCW arc with continuous B rotation
; Vertical helix with B-axis tilt
G0 X100 Y50 Z20 B0
G3 X100 Y50 Z0 B180 I25 J0 F800   ; Half circle with B rotation

; Helix 3: Multi-turn helix with C rotation
; Spiral down while rotating the C axis (rotary table)
G0 X150 Y50 Z50 C0
G1 F1200
G2 X150 Y50 Z10 C720 I25 J0 F600  ; 2 full C rotations

; ============ SIMULTANEOUS 5-AXIS HELIX ============

; Complex 5-axis helix: XYZ + AB rotary motion
G0 X200 Y100 Z30 A0 B0
G1 F800
G2 X200 Y100 Z-10 A45 B45 I25 J0 F600

; Helical thread milling pattern
G0 X300 Y100 Z20
G1 X300 Y100 Z10 A0 F1000       ; Plunge
G2 X300 Y100 Z-30 A720 I25 J0 F600  ; 2 full A rotations

; ============ TAPERED HELIX ============

; Tapered helix with varying B angle
G0 X400 Y100 Z30 B0
G1 F800
G2 X420 Y100 Z20 B15 I10 J0 F600
G2 X440 Y100 Z10 B30 I10 J0 F600
G2 X460 Y100 Z0 B45 I10 J0 F600

; ============ CLEANUP ============
G0 Z50
G0 A0 B0 C0
G0 X0 Y0
M5              ; Stop spindle
M30             ; End
