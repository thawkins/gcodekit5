#[cfg(test)]
mod tests {
    use crate::gerber::{GerberConverter, GerberLayerType, GerberParameters};

    #[test]
    fn test_hatch_generation_events() {
        // ... (existing test)
    }

    #[test]
    fn test_gerber_rubout_generation() {
        let params = GerberParameters {
            layer_type: GerberLayerType::TopCopper,
            rubout: true,
            board_width: 20.0,
            board_height: 20.0,
            tool_diameter: 1.0,
            isolation_width: 0.0,
            ..Default::default()
        };

        // Simple Gerber: A 10x10 square at 5,5
        // D10* (Aperture 10)
        // %ADD10C,5*% (Circle 5mm)
        // D02 X50000 Y50000* (Move to 5,5)
        // D01 X150000 Y50000* (Draw to 15,5)
        // D01 X150000 Y150000* (Draw to 15,15)
        // D01 X50000 Y150000* (Draw to 5,15)
        // D01 X50000 Y50000* (Draw to 5,5)

        // Actually let's just use a simple flash to create a shape
        // %ADD10C,10*% (Circle 10mm)
        // D10*
        // X100000Y100000D03* (Flash at 10,10)

        let gerber_content = r#"
%MOIN*%
%FSLAX24Y24*%
%ADD10C,0.3937*%
D10*
X100000Y100000D03*
M02*
"#;

        let result = GerberConverter::generate(&params, gerber_content);
        match result {
            Ok(gcode) => {
                assert!(gcode.contains("; Rubout"));
                assert!(gcode.contains("G1 Z-0.1")); // Cutting move
            }
            Err(e) => {
                panic!("Failed to generate G-code: {:?}", e);
            }
        }
    }
}
