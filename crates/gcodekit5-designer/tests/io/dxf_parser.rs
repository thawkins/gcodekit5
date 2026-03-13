use gcodekit5_designer::dxf_parser::{
    DxfArc, DxfCircle, DxfEntity, DxfEntityType, DxfFile, DxfHeader, DxfLine, DxfParser,
    DxfPolyline, DxfText, DxfUnit,
};
use gcodekit5_designer::model::Point;

#[test]
fn test_dxf_unit_conversion_inches_to_mm() {
    let factor = DxfUnit::Inches.to_mm_factor();
    assert!((factor - 25.4).abs() < 0.01);
}

#[test]
fn test_dxf_unit_conversion_feet_to_mm() {
    let factor = DxfUnit::Feet.to_mm_factor();
    assert!((factor - 304.8).abs() < 0.01);
}

#[test]
fn test_dxf_line_creation() {
    let line = DxfLine {
        start: Point::new(0.0, 0.0),
        end: Point::new(10.0, 10.0),
        layer: "Lines".to_string(),
        color: 1,
    };

    assert_eq!(line.start, Point::new(0.0, 0.0));
    assert_eq!(line.end, Point::new(10.0, 10.0));
}

#[test]
fn test_dxf_circle_creation() {
    let circle = DxfCircle {
        center: Point::new(5.0, 5.0),
        radius: 3.0,
        layer: "Circles".to_string(),
        color: 1,
    };

    assert_eq!(circle.center, Point::new(5.0, 5.0));
    assert_eq!(circle.radius, 3.0);
}

#[test]
fn test_dxf_arc_creation() {
    let arc = DxfArc {
        center: Point::new(0.0, 0.0),
        radius: 5.0,
        start_angle: 0.0,
        end_angle: 90.0,
        layer: "Arcs".to_string(),
        color: 1,
    };

    assert_eq!(arc.radius, 5.0);
    assert_eq!(arc.start_angle, 0.0);
}

#[test]
fn test_dxf_polyline_creation() {
    let polyline = DxfPolyline {
        vertices: vec![
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(10.0, 10.0),
        ],
        bulges: Vec::new(), // For Github mutants error
        closed: false,
        layer: "Polylines".to_string(),
        color: 1,
    };

    assert_eq!(polyline.vertices.len(), 3);
    assert!(!polyline.closed);
}

#[test]
fn test_dxf_text_creation() {
    let text = DxfText {
        content: "Hello".to_string(),
        position: Point::new(0.0, 0.0),
        height: 2.5,
        rotation: 0.0,
        layer: "Text".to_string(),
        color: 1,
    };

    assert_eq!(text.content, "Hello");
    assert_eq!(text.height, 2.5);
}

#[test]
fn test_dxf_entity_type() {
    let line = DxfEntity::Line(DxfLine {
        start: Point::new(0.0, 0.0),
        end: Point::new(1.0, 1.0),
        layer: "0".to_string(),
        color: 256,
    });

    assert_eq!(line.entity_type(), DxfEntityType::Line);
}

#[test]
fn test_dxf_file_creation() {
    let mut file = DxfFile::new();
    assert_eq!(file.entity_count(), 0);

    file.add_entity(DxfEntity::Line(DxfLine {
        start: Point::new(0.0, 0.0),
        end: Point::new(1.0, 1.0),
        layer: "Lines".to_string(),
        color: 1,
    }));

    assert_eq!(file.entity_count(), 1);
}

#[test]
fn test_dxf_file_layers() {
    let mut file = DxfFile::new();

    file.add_entity(DxfEntity::Line(DxfLine {
        start: Point::new(0.0, 0.0),
        end: Point::new(1.0, 1.0),
        layer: "Layer1".to_string(),
        color: 1,
    }));

    file.add_entity(DxfEntity::Circle(DxfCircle {
        center: Point::new(0.0, 0.0),
        radius: 1.0,
        layer: "Layer2".to_string(),
        color: 1,
    }));

    let layers = file.layer_names();
    assert_eq!(layers.len(), 2);
}

#[test]
fn test_dxf_file_scale() {
    let mut file = DxfFile::new();

    file.add_entity(DxfEntity::Line(DxfLine {
        start: Point::new(0.0, 0.0),
        end: Point::new(10.0, 10.0),
        layer: "0".to_string(),
        color: 256,
    }));

    file.scale(2.0);

    if let DxfEntity::Line(line) = &file.entities[0] {
        assert_eq!(line.end, Point::new(20.0, 20.0));
    } else {
        panic!("Expected line entity");
    }
}

#[test]
fn test_dxf_file_unit_conversion() {
    let mut file = DxfFile::new();

    file.add_entity(DxfEntity::Circle(DxfCircle {
        center: Point::new(0.0, 0.0),
        radius: 1.0,
        layer: "0".to_string(),
        color: 256,
    }));

    file.convert_units(DxfUnit::Inches, DxfUnit::Millimeters);

    if let DxfEntity::Circle(circle) = &file.entities[0] {
        assert!((circle.radius - 25.4).abs() < 0.1);
    }
}

#[test]
fn test_dxf_header_default() {
    let header = DxfHeader::default();
    assert_eq!(header.version, "AC1021");
    assert_eq!(header.unit, DxfUnit::Millimeters);
}

#[test]
fn test_dxf_parser_validate() {
    let valid_dxf = "SECTION\nENDSEC";
    let result = DxfParser::validate_header(valid_dxf);
    assert!(result.is_ok());

    let invalid_dxf = "INVALID";
    let result = DxfParser::validate_header(invalid_dxf);
    assert!(result.is_err());
}
