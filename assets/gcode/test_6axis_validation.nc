; 6-Axis Validation Edge Cases
; Tests for validating 6-axis G-code parser and validator
;
; GCodeKit5 Test File - Validation Edge Cases
; Purpose: Test validation of 6-axis G-code

; ============ VALID CASES ============

; Standard 3-axis moves (should always pass)
G0 X0 Y0 Z10
G1 X50 Y50 Z-5 F1000
G2 X100 Y50 I25 J0
G3 X150 Y50 I25 J0

; 4-axis moves (XYZ + A)
G1 X50 Y50 Z-5 A45 F1000
G1 X100 Y50 Z-5 A90 F1000

; 5-axis moves (XYZ + AB)
G1 X50 Y50 Z-5 A45 B30 F1000

; 6-axis moves (XYZ + ABC)
G1 X50 Y50 Z-5 A45 B30 C90 F1000
G1 X100 Y100 Z-5 A90 B45 C180 F1000

; ============ EDGE CASES FOR VALIDATION ============

; Rotary axis at limits
G1 X50 Y50 Z-5 A0 B0 C0 F1000    ; All at 0
G1 X50 Y50 Z-5 A360 B360 C360    ; All at 360 (wraparound)

; Negative rotary angles (valid for some controllers)
G1 X50 Y50 Z-5 A-90 B-45 C-180 F1000

; Large linear moves with small rotary moves
G1 X200 Y200 Z-10 A1 B1 C1 F2000

; Small linear moves with large rotary moves
G1 X1 Y1 Z-1 A180 B90 C360 F500

; Mixed modal state
G0 X0 Y0 Z50
G1 X50 Y0 Z0 A45 F1000
G1 X100 Y0 Z0 A90 F1000
G0 X150 Y0 Z50    ; Rapid move
G1 X150 Y50 Z-5 B30 F1000

; Multiple coordinate words per line
G1 X50 Y50 Z-5 A45 B30 C90 F1000

; ============ COMMENTS AND WHITESPACE ============

; Comment only line
  ; Indented comment
G1 X50 Y50 Z-5 A45   ; Inline comment

; Empty line

G0 X100 Y100 Z50  ; After empty line

; ============ PROGRAM END ============
M30
