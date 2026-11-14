; 4-Axis Rotary Table Test (XYZA)
; Tests 4-axis simultaneous motion
;
; GCodeKit5 Test File - 4-Axis
; Machine: XYZA (4-Axis with A-axis rotary)

G21              ; Metric
G90              ; Absolute
G17              ; XY plane

; ============ SETUP ============
G0 X0 Y0 Z50 A0
S2000 M3         ; Spindle on

; ============ A-AXIS ROTATION TESTS ============

; Face milling with A rotation
G0 X25 Y0 Z10 A0
G1 X25 Y50 Z-5 A0 F1000       ; Linear cut at A=0
G1 X25 Y50 Z-5 A45 F1000       ; Rotate A to 45 degrees
G1 X25 Y0 Z-5 A45 F1000       ; Return pass
G1 X25 Y0 Z-5 A90 F1000       ; Rotate to 90
G1 X25 Y50 Z-5 A90 F1000      ; Another pass

; Indexing moves
G0 X50 Y0 Z10 A0
G1 X50 Y50 Z-3 A0 F1200       ; Cut 1
G0 Z10                         ; Retract
A45                            ; Index to 45
G1 X50 Y50 Z-3 A45 F1200       ; Cut 2
G0 Z10
A90
G1 X50 Y50 Z-3 A90 F1200       ; Cut 3
G0 Z10
A135
G1 X50 Y50 Z-3 A135 F1200      ; Cut 4
G0 Z10
A180
G1 X50 Y50 Z-3 A180 F1200      ; Cut 5

; Continuous 4-axis machining
; Simultaneous X, Y, Z, A motion
G0 X75 Y0 Z10 A0
G1 X75 Y50 Z-5 A90 F800       ; Move Y while rotating A
G1 X100 Y50 Z-5 A180 F800     ; Move X and continue A rotation
G1 X100 Y0 Z-5 A270 F800      ; Move Y and A
G1 X75 Y0 Z-5 A360 F800       ; Return home with full A rotation

; ============ DRILLING PATTERN WITH A ROTATION ============

; Drill pattern on a rotated surface
G0 X125 Y25 Z20 A0

; Drill holes at various A angles
G81 R5 Z-10 F500
A0
X125 Y25
A45
X125 Y25
A90
X125 Y25
A135
X125 Y25
A180
X125 Y25

G80              ; Cancel drill cycle

; ============ CLEANUP ============
G0 Z50
G0 A0            ; Reset A axis
G0 X0 Y0
M5
M30
