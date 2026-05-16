; 6-Axis Test G-code
; Test file for 6-axis CNC machines (XYZABC)
; 
; GCodeKit5 Test File
; Machine: 6-Axis CNC
; Material: Aluminum

; ============ INITIALIZATION ============
G21           ; Set units to millimeters
G90           ; Absolute positioning
G17           ; XY plane selection

; ============ STARTUP ============
G0 X0 Y0 Z50   ; Move to safe position
A0 B0 C0       ; Reset rotary axes

; ============ 6-AXIS MOVEMENT TESTS ============

; Test 1: XYZ + A (4th axis rotation around X)
G1 X50 Y50 Z10 A0 F1000
G1 X50 Y50 Z10 A90    ; Rotate A axis 90 degrees
G1 X50 Y50 Z10 A180   ; Continue rotation
G1 X50 Y50 Z10 A270
G1 X50 Y50 Z10 A360

; Test 2: XYZ + B (5th axis rotation around Y)
G0 X100 Y100 Z20
G1 X100 Y100 Z20 B0 F1000
G1 X100 Y100 Z20 B45    ; Rotate B axis 45 degrees
G1 X100 Y100 Z20 B90    ; Continue rotation
G1 X100 Y100 Z20 B180

; Test 3: XYZ + C (6th axis rotation around Z)
G0 X150 Y150 Z30
G1 X150 Y150 Z30 C0 F1000
G1 X150 Y150 Z30 C120   ; Rotate C axis 120 degrees
G1 X150 Y150 Z30 C240   ; Continue rotation
G1 X150 Y150 Z30 C360

; Test 4: All 6 axes moving simultaneously
G0 X0 Y0 Z50 A0 B0 C0
G1 X100 Y100 Z20 A90 B45 C180 F800

; Test 5: Sequential 6-axis positioning
G0 X200 Y0 Z50
G1 X200 Y0 Z20 A45 B30 C90 F1200
G1 X200 Y100 Z20 A90 B45 C180 F1200
G1 X200 Y200 Z20 A135 B60 C270 F1200
G1 X200 Y300 Z20 A180 B90 C360 F1200

; Test 6: Helical interpolation with A axis rotation
G0 X300 Y300 Z50 A0
G2 X350 Y300 Z10 A360 I25 J0 F800  ; CW arc with full A rotation

; Test 7: Helical interpolation with B axis rotation  
G0 X400 Y400 Z50 B0
G3 X450 Y400 Z10 B180 I25 J0 F800  ; CCW arc with B rotation

; ============ RETURN HOME ============
G0 Z50         ; Retract Z
G0 A0 B0 C0    ; Reset rotary axes
G0 X0 Y0       ; Return XY home
G0 Z0          ; Move Z to home

; ============ PROGRAM END ============
M30            ; End of program

; ============ EXPECTED RESULTS ============
; This file tests:
; - Individual axis movements for A, B, C
; - Combined 6-axis movements
; - Helical arcs with rotary axis coordination
; - Modal motion modes (G0, G1, G2, G3)
; - Feed rate changes
