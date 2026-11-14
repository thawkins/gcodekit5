; 5-Axis Simultaneous Machining Test
; Tests full 5-axis simultaneous motion (XYZ + AB rotary)
;
; GCodeKit5 Test File - 5-Axis
; Machine: XYZAB (5-Axis with Trunnion Table)

G21              ; Metric
G90              ; Absolute

; ============ SETUP ============
G0 X0 Y0 Z100 A0 B0
S4000 M3

; ============ 5-AXIS POSITIONING ============

; Test: Move to position with tilted orientation
G1 X50 Y50 Z20 A45 B0 F1000    ; Tilt A axis 45 degrees
G1 X100 Y50 Z20 A45 B0 F1000   ; Move X (tilted)
G1 X100 Y100 Z20 A45 B0 F1000  ; Move Y (tilted)

; Test: Different orientations
G0 X150 Y50 Z50 A0 B0
G1 X150 Y50 Z20 A0 B45 F1000   ; Tilt B axis 45 degrees
G1 X200 Y50 Z20 A0 B45 F1000   ; Move X
G1 X200 Y100 Z20 A0 B45 F1000  ; Move Y

; Test: Combined AB tilt (5-axis simultaneous)
G0 X300 Y50 Z50 A0 B0
G1 X300 Y50 Z20 A30 B30 F800   ; Tilt both A and B
G1 X350 Y50 Z20 A30 B30 F800   ; Move while maintaining tilt
G1 X350 Y100 Z20 A30 B30 F800  ; Continue with simultaneous motion

; ============ 5-AXIS SWARF CUTTING ============

; Side-wall machining with constant tilt
G0 X400 Y0 Z50 A0 B0
G1 F600

; Pass 1: Cut with B=30 degrees
G1 X400 Y0 Z10 A0 B30
G1 X400 Y100 Z10 A0 B30

; Pass 2: Change tilt angle
G0 X420 Y0 Z50 A0 B45
G1 X420 Y0 Z10 A0 B45 F600
G1 X420 Y100 Z10 A0 B45

; Pass 3: Combined AB tilt
G0 X440 Y0 Z50 A0 B0
G1 X440 Y0 Z10 A15 B30 F600
G1 X440 Y100 Z10 A15 B30

; ============ 5-AXIS CONTOURING ============

; Contour following with varying tilt
G0 X500 Y0 Z50 A0 B0

; Start position
G1 X500 Y0 Z10 A0 B0 F800

; Follow contour with tilt adjustment
G1 X520 Y20 Z10 A5 B5 F800
G1 X540 Y40 Z10 A10 B10 F800
G1 X560 Y60 Z10 A15 B15 F800
G1 X580 Y80 Z10 A20 B20 F800
G1 X600 Y100 Z10 A25 B25 F800

; ============ 5-AXIS TURBINE BLADE SIMULATION ============

; Simulate turbine blade machining
G0 X700 Y0 Z50 A0 B0

; Root section
G1 X700 Y0 Z20 A0 B45 F1000
G1 X700 Y20 Z20 A5 B45 F1000

; Mid section
G1 X700 Y40 Z25 A10 B35 F1000
G1 X700 Y60 Z30 A15 B25 F1000

; Tip section
G1 X700 Y80 Z35 A20 B15 F1000
G1 X700 Y100 Z35 A25 B10 F1000

; ============ CLEANUP ============
G0 Z100
G0 A0 B0
G0 X0 Y0
M5
M30
