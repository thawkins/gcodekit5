//! Tests for GRBL response and status parsers.

#[cfg(test)]
mod response_parser_tests {
    use super::super::response_parser::{GrblResponse, GrblResponseParser};
    use gcodekit5_core::{CNCPoint, Units};

    #[test]
    fn parse_probe_result_with_flag() {
        let parser = GrblResponseParser::new();
        let line = "[PRB:10.123,20.456,-5.789:1]";
        let result = parser.parse(line).expect("should parse");
        match result {
            GrblResponse::ProbeResult { position, success } => {
                assert!((position.x - 10.123).abs() < 0.001);
                assert!((position.y - 20.456).abs() < 0.001);
                assert!((position.z + 5.789).abs() < 0.001);
                assert!(success);
            }
            other => panic!("Expected ProbeResult, got {:?}", other),
        }
    }

    #[test]
    fn parse_probe_result_without_flag() {
        let parser = GrblResponseParser::new();
        let line = "[PRB:0.000,0.000,0.000]";
        let result = parser.parse(line).expect("should parse");
        match result {
            GrblResponse::ProbeResult { position, success } => {
                assert_eq!(position.x, 0.0);
                assert_eq!(position.y, 0.0);
                assert_eq!(position.z, 0.0);
                assert!(success); // defaults to true when flag missing
            }
            other => panic!("Expected ProbeResult, got {:?}", other),
        }
    }

    #[test]
    fn parse_probe_result_failure_flag() {
        let parser = GrblResponseParser::new();
        let line = "[PRB:0.000,0.000,-10.000:0]";
        let result = parser.parse(line).expect("should parse");
        match result {
            GrblResponse::ProbeResult { success, .. } => {
                assert!(!success);
            }
            other => panic!("Expected ProbeResult, got {:?}", other),
        }
    }

    #[test]
    fn parse_probe_result_malformed() {
        let parser = GrblResponseParser::new();
        assert!(parser.parse("[PRB:xyz:1]").is_none());
    }
}

#[cfg(test)]
mod six_axis_tests {
    use super::super::capabilities::{GrblCapabilities, GrblVersion};
    use super::super::status_parser::{MachinePosition, WorkCoordinateOffset, WorkPosition};

    #[test]
    fn test_grbl_machine_position_6axis() {
        let pos = MachinePosition::parse("100.000,50.000,25.000,90.000,45.000,0.000");
        assert!(pos.is_some());
        let pos = pos.unwrap();
        assert_eq!(pos.x, 100.0);
        assert_eq!(pos.y, 50.0);
        assert_eq!(pos.z, 25.0);
        assert_eq!(pos.a, Some(90.0));
        assert_eq!(pos.b, Some(45.0));
        assert_eq!(pos.c, Some(0.0));
    }

    #[test]
    fn test_grbl_machine_position_3axis() {
        let pos = MachinePosition::parse("100.000,50.000,25.000");
        assert!(pos.is_some());
        let pos = pos.unwrap();
        assert_eq!(pos.x, 100.0);
        assert_eq!(pos.y, 50.0);
        assert_eq!(pos.z, 25.0);
        assert_eq!(pos.a, None);
        assert_eq!(pos.b, None);
        assert_eq!(pos.c, None);
    }

    #[test]
    fn test_grbl_machine_position_4axis() {
        let pos = MachinePosition::parse("100.000,50.000,25.000,45.000");
        assert!(pos.is_some());
        let pos = pos.unwrap();
        assert_eq!(pos.x, 100.0);
        assert_eq!(pos.y, 50.0);
        assert_eq!(pos.z, 25.0);
        assert_eq!(pos.a, Some(45.0));
        assert_eq!(pos.b, None);
        assert_eq!(pos.c, None);
    }

    #[test]
    fn test_grbl_work_position_6axis() {
        let pos = WorkPosition::parse("10.000,20.000,30.000,45.000,22.500,15.000");
        assert!(pos.is_some());
        let pos = pos.unwrap();
        assert_eq!(pos.x, 10.0);
        assert_eq!(pos.y, 20.0);
        assert_eq!(pos.z, 30.0);
        assert_eq!(pos.a, Some(45.0));
        assert_eq!(pos.b, Some(22.5));
        assert_eq!(pos.c, Some(15.0));
    }

    #[test]
    fn test_grbl_work_coordinate_offset_6axis() {
        let offset = WorkCoordinateOffset::parse("90.000,30.000,-5.000,45.000,22.500,0.000");
        assert!(offset.is_some());
        let offset = offset.unwrap();
        assert_eq!(offset.x, 90.0);
        assert_eq!(offset.y, 30.0);
        assert_eq!(offset.z, -5.0);
        assert_eq!(offset.a, Some(45.0));
        assert_eq!(offset.b, Some(22.5));
        assert_eq!(offset.c, Some(0.0));
    }

    #[test]
    fn test_grbl_capabilities_max_axes() {
        let version = GrblVersion::new(1, 1, 0);
        let caps = GrblCapabilities::for_version(version);
        assert_eq!(caps.max_axes, 6);
    }
}

#[cfg(test)]
mod status_parser_tests {
    use super::super::status_parser::StatusParser;

    #[test]
    fn parse_probe_pin_active() {
        let line = "<Idle|MPos:0.000,0.000,0.000|FS:0,0|Pn:P>";
        let full = StatusParser::parse_full(line);
        assert_eq!(full.probe_pin, Some(true));
    }

    #[test]
    fn parse_probe_pin_inactive() {
        let line = "<Idle|MPos:0.000,0.000,0.000|FS:0,0|Pn:XYZ>";
        let full = StatusParser::parse_full(line);
        assert_eq!(full.probe_pin, Some(false));
    }

    #[test]
    fn parse_no_probe_pin_field() {
        let line = "<Idle|MPos:0.000,0.000,0.000|FS:0,0>";
        let full = StatusParser::parse_full(line);
        assert_eq!(full.probe_pin, None);
    }

    #[test]
    fn parse_probe_pin_with_other_pins() {
        let line = "<Idle|MPos:10.000,20.000,-5.000|FS:100,1000|Pn:XYZPD>";
        let full = StatusParser::parse_full(line);
        assert_eq!(full.probe_pin, Some(true));
        assert_eq!(full.mpos.map(|p| p.x), Some(10.0));
    }
}
