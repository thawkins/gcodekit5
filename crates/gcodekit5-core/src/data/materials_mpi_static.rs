//! Static (hard-coded) material records derived from the upstream
//! "material-properties-interchange" project.
//!
//! We vendor the *data* into Rust structs (rather than shipping the upstream YAML).
//! Upstream: https://github.com/mvernacc/material-properties-interchange

use crate::data::materials::{
    CoolantType, CuttingParameters, Material, MaterialCategory, MaterialId,
};

pub fn load_mpi_derived_materials() -> Vec<Material> {
    let mut out = Vec::new();

    // AISI 304
    {
        let mut m = Material::new(
            MaterialId("mpi_aisi_304".to_string()),
            "AISI 304 (stainless steel)".to_string(),
            MaterialCategory::FerrousMetal,
            "stainless steel".to_string(),
        );
        m.description = "Imported (static) from MPI dataset".to_string();
        m.density = 7930.0;
        m.machinability_rating = 6;
        m.tensile_strength = Some(505.0);
        m.melting_point = Some(1400.0);
        m.notes = "AISI 304: density ~7930 kg/m³; melting range ~1400–1450 °C; UTS (annealed) ~505–620 MPa. Sources: https://asm.matweb.com/search/SpecificMaterial.asp?bassnum=mq304a ; https://www.azom.com/properties.aspx?ArticleID=965".to_string();

        // 12k spindle baseline (assumes ~6mm, 2-flute carbide endmill)
        let mut p = CuttingParameters::default();
        p.rpm_range = (3000, 12000);
        p.feed_rate_range = (250.0, 600.0);
        p.plunge_rate_percent = 30.0;
        p.max_doc = 1.5;
        p.stepover_percent = (15.0, 30.0);
        p.surface_speed_m_min = Some(60.0);
        p.chip_load_mm = Some(0.015);
        p.coolant_type = CoolantType::WaterSoluble;
        p.notes = "304 stainless starter values; adjust by tool diameter using surface speed + chip load. Sources: https://www.easyspeedsandfeeds.com/304-ss ; https://www.harveytool.com/resources/general-machining-guidelines".to_string();
        m.set_cutting_params("endmill_flat".to_string(), p);

        out.push(m);
    }

    // AISI 316L
    {
        let mut m = Material::new(
            MaterialId("mpi_aisi_316l".to_string()),
            "AISI 316L (stainless steel)".to_string(),
            MaterialCategory::FerrousMetal,
            "stainless steel".to_string(),
        );
        m.description = "Imported (static) from MPI dataset".to_string();
        m.density = 8000.0;
        m.machinability_rating = 6;
        m.tensile_strength = Some(579.0);
        m.melting_point = Some(1375.0);
        m.notes = "AISI 316L: density ~7980–8000 kg/m³; melting range ~1375–1400 °C; UTS (annealed) typically ~485–620 MPa. Source: https://atlassteels.com.au/wp-content/uploads/2021/06/Stainless-Steel-316-316L-316H-Grade-Data-Sheet-27-04-21.pdf".to_string();

        // 12k spindle baseline (assumes ~6mm, 2-flute carbide endmill)
        let mut p = CuttingParameters::default();
        p.rpm_range = (3000, 12000);
        p.feed_rate_range = (250.0, 600.0);
        p.plunge_rate_percent = 30.0;
        p.max_doc = 1.5;
        p.stepover_percent = (15.0, 30.0);
        p.surface_speed_m_min = Some(60.0);
        p.chip_load_mm = Some(0.015);
        p.coolant_type = CoolantType::WaterSoluble;
        p.notes = "316L stainless starter values; adjust by tool diameter using surface speed + chip load. Sources: https://www.easyspeedsandfeeds.com/304-ss ; https://www.harveytool.com/resources/general-machining-guidelines".to_string();
        m.set_cutting_params("endmill_flat".to_string(), p);

        out.push(m);
    }

    // AISI 4130
    {
        let mut m = Material::new(
            MaterialId("mpi_aisi_4130".to_string()),
            "AISI 4130 (steel)".to_string(),
            MaterialCategory::FerrousMetal,
            "steel".to_string(),
        );
        m.description = "Imported (static) from MPI dataset".to_string();
        m.density = 7850.0;
        m.machinability_rating = 6;
        m.tensile_strength = Some(670.0);
        m.melting_point = Some(1420.0);
        m.notes = "AISI 4130 (normalized): density ~7850 kg/m³; UTS ~670 MPa; melting range ~1420–1510 °C. Sources: https://asm.matweb.com/search/SpecificMaterial.asp?bassnum=m4130r ; https://www.azom.com/article.aspx?ArticleID=6742".to_string();

        // 12k spindle baseline (assumes ~6mm, 2-flute carbide endmill)
        let mut p = CuttingParameters::default();
        p.rpm_range = (3000, 12000);
        p.feed_rate_range = (400.0, 1000.0);
        p.plunge_rate_percent = 30.0;
        p.max_doc = 2.0;
        p.stepover_percent = (15.0, 35.0);
        p.surface_speed_m_min = Some(180.0);
        p.chip_load_mm = Some(0.03);
        p.coolant_type = CoolantType::WaterSoluble;
        p.notes = "4130 steel starter values; adjust by tool diameter using surface speed + chip load. Sources: https://www.machiningdoctor.com/mds/?matId=400 ; https://www.harveytool.com/resources/general-machining-guidelines".to_string();
        m.set_cutting_params("endmill_flat".to_string(), p);

        out.push(m);
    }

    // AlSi10Mg
    {
        let mut m = Material::new(
            MaterialId("mpi_alsi10mg".to_string()),
            "AlSi10Mg (Aluminum alloy)".to_string(),
            MaterialCategory::NonFerrousMetal,
            "Aluminum alloy".to_string(),
        );
        m.description = "Imported (static) from MPI dataset".to_string();
        m.density = 2670.0;
        m.machinability_rating = 8;
        m.tensile_strength = Some(379.0);
        m.melting_point = Some(570.0);
        m.notes = "AlSi10Mg: density ~2.67 g/cm³; melting range ~570–595 °C; UTS (stress relieved AM) ~379 MPa. Sources: https://info.stratasysdirect.com/rs/626-SBR-192/images/DMLM_Aluminum_AlSi10Mg_Material_Datasheet_202002.pdf ; https://www.sunrise-metal.com/aluminum-alloy-alsi10mg/".to_string();

        // 12k spindle baseline (assumes ~6mm, 2-flute carbide endmill)
        let mut p = CuttingParameters::default();
        p.rpm_range = (8000, 12000);
        p.feed_rate_range = (800.0, 1800.0);
        p.plunge_rate_percent = 40.0;
        p.max_doc = 3.0;
        p.stepover_percent = (30.0, 60.0);
        p.surface_speed_m_min = Some(250.0);
        p.chip_load_mm = Some(0.04);
        p.coolant_type = CoolantType::AirOnly;
        p.notes = "AlSi10Mg (aluminum alloy) starter values; prioritize chip evacuation (air blast/MQL). Sources: https://www.machiningdoctor.com/calculators/chip-load-calculator/ ; https://www.harveytool.com/resources/general-machining-guidelines".to_string();
        m.set_cutting_params("endmill_flat".to_string(), p);

        out.push(m);
    }

    // Al 6061
    {
        let mut m = Material::new(
            MaterialId("mpi_al_6061".to_string()),
            "Al 6061 (Aluminum alloy)".to_string(),
            MaterialCategory::NonFerrousMetal,
            "Aluminum alloy".to_string(),
        );
        m.description = "Imported (static) from MPI dataset".to_string();
        // MPI file does not specify density; use a standard engineering value.
        m.density = 2700.0;
        m.machinability_rating = 8;
        m.tensile_strength = Some(310.0);
        m.melting_point = Some(582.0);
        m.notes = "Al 6061-T6: UTS ~310 MPa; melting range ~582–652 °C (solidus–liquidus). Source: https://asm.matweb.com/search/specificmaterial.asp?bassnum=ma6061t6".to_string();

        // 12k spindle baseline (assumes ~6mm, 2-flute carbide endmill)
        let mut p = CuttingParameters::default();
        p.rpm_range = (8000, 12000);
        p.feed_rate_range = (900.0, 2200.0);
        p.plunge_rate_percent = 40.0;
        p.max_doc = 3.0;
        p.stepover_percent = (35.0, 65.0);
        p.surface_speed_m_min = Some(300.0);
        p.chip_load_mm = Some(0.05);
        p.coolant_type = CoolantType::AirOnly;
        p.notes = "6061 aluminum starter values for carbide endmills; adjust by tool diameter using surface speed + chip load. Sources: https://www.machiningdoctor.com/mds/?matId=3850 ; https://www.harveytool.com/resources/general-machining-guidelines".to_string();
        m.set_cutting_params("endmill_flat".to_string(), p);

        out.push(m);
    }

    // D6AC
    {
        let mut m = Material::new(
            MaterialId("mpi_d6ac".to_string()),
            "D6AC (steel)".to_string(),
            MaterialCategory::FerrousMetal,
            "steel".to_string(),
        );
        m.description = "Imported (static) from MPI dataset".to_string();
        m.density = 7870.0;
        m.machinability_rating = 6;
        m.tensile_strength = Some(1517.0);
        m.melting_point = Some(1427.0);
        m.notes = "D6AC: density ~0.284 lb/in³ (~7870 kg/m³); UTS ~220 ksi (1517 MPa) depending on temper; melting point ~1427 °C. Source: https://steelprogroup.com/alloy-steel/d6ac-steel-overview/".to_string();

        // 12k spindle baseline (assumes ~6mm, 2-flute carbide endmill)
        let mut p = CuttingParameters::default();
        p.rpm_range = (2500, 9000);
        p.feed_rate_range = (250.0, 650.0);
        p.plunge_rate_percent = 25.0;
        p.max_doc = 1.0;
        p.stepover_percent = (10.0, 25.0);
        p.surface_speed_m_min = Some(120.0);
        p.chip_load_mm = Some(0.02);
        p.coolant_type = CoolantType::WaterSoluble;
        p.notes = "High-strength alloy steel starter values; start conservative and increase only if rigidity allows. Sources: https://www.harveytool.com/resources/general-machining-guidelines ; https://www.lakeshorecarbide.com/lakeshorecarbidecomspeedandfeedcharts.aspx".to_string();
        m.set_cutting_params("endmill_flat".to_string(), p);

        out.push(m);
    }

    // In718
    {
        let mut m = Material::new(
            MaterialId("mpi_in718".to_string()),
            "In718 (nickel alloy)".to_string(),
            MaterialCategory::NonFerrousMetal,
            "nickel alloy".to_string(),
        );
        m.description = "Imported (static) from MPI dataset".to_string();
        m.density = 8220.0;
        m.machinability_rating = 6;
        m.tensile_strength = Some(1241.0);
        m.melting_point = Some(1260.0);
        m.notes = "Inconel 718: density ~8190–8220 kg/m³; melting range ~1260–1336 °C; UTS (STA) often ~1240–1375 MPa. Sources: https://asm.matweb.com/search/specificmaterial.asp?bassnum=ninc34 ; https://en.wikipedia.org/wiki/Inconel_718".to_string();

        // 12k spindle baseline (assumes ~6mm, 2-flute carbide endmill)
        let mut p = CuttingParameters::default();
        p.rpm_range = (1200, 3000);
        p.feed_rate_range = (120.0, 300.0);
        p.plunge_rate_percent = 20.0;
        p.max_doc = 0.8;
        p.stepover_percent = (8.0, 20.0);
        p.surface_speed_m_min = Some(30.0);
        p.chip_load_mm = Some(0.01);
        p.coolant_type = CoolantType::Synthetic;
        p.notes = "Inconel 718 starter values; very heat-sensitive/work-hardening—prefer coolant and avoid dwelling. Sources: https://www.machiningdoctor.com/mds/?matId=5700 ; https://fmcarbide.com/pages/material-inconel-718".to_string();
        m.set_cutting_params("endmill_flat".to_string(), p);

        out.push(m);
    }

    // Ti-6Al-4V ELI
    {
        let mut m = Material::new(
            MaterialId("mpi_ti_6al_4v_eli".to_string()),
            "Ti-6Al-4V ELI (titanium alloy)".to_string(),
            MaterialCategory::NonFerrousMetal,
            "titanium alloy".to_string(),
        );
        m.description = "Imported (static) from MPI dataset".to_string();
        m.density = 4430.0;
        m.machinability_rating = 6;
        m.tensile_strength = Some(931.0);
        m.melting_point = Some(1649.0);
        m.notes = "Ti-6Al-4V ELI: density ~4430 kg/m³; melting range ~1649–1660 °C; UTS (annealed) typically ~828–950 MPa. Source: https://www.upmet.com/sites/default/files/datasheets/ti-6al-4v-eli.pdf".to_string();

        // 12k spindle baseline (assumes ~6mm, 2-flute carbide endmill)
        let mut p = CuttingParameters::default();
        p.rpm_range = (1500, 4000);
        p.feed_rate_range = (150.0, 500.0);
        p.plunge_rate_percent = 20.0;
        p.max_doc = 1.0;
        p.stepover_percent = (10.0, 25.0);
        p.surface_speed_m_min = Some(45.0);
        p.chip_load_mm = Some(0.03);
        p.coolant_type = CoolantType::WaterSoluble;
        p.notes = "Ti-6Al-4V starter values; keep tool engaged (avoid rubbing) and use coolant. Sources: https://www.machiningdoctor.com/mds/?matId=6670 ; https://www.tru-edge.com/wp-content/uploads/2019/09/Feeds-and-Speeds-Endmills.pdf".to_string();
        m.set_cutting_params("endmill_flat".to_string(), p);

        out.push(m);
    }

    // copper
    {
        let mut m = Material::new(
            MaterialId("mpi_copper".to_string()),
            "copper (copper and copper alloys)".to_string(),
            MaterialCategory::NonFerrousMetal,
            "copper and copper alloys".to_string(),
        );
        m.description = "Imported (static) from MPI dataset".to_string();
        m.density = 8960.0;
        m.machinability_rating = 7;
        m.tensile_strength = Some(210.0);
        m.melting_point = Some(1083.0);
        m.notes = "Copper (annealed): density 8960 kg/m³; melting point 1083 °C; UTS ~210 MPa. Sources: https://amesweb.info/Materials/Density_of_Copper.aspx ; https://kupfer.de/kupferwerkstoffe/kupfer/eigenschaften/?lang=en ; https://quickparts.com/wp-content/uploads/2024/05/Copper.pdf".to_string();

        // 12k spindle baseline (assumes ~6mm, 2-flute carbide endmill)
        let mut p = CuttingParameters::default();
        p.rpm_range = (6000, 12000);
        p.feed_rate_range = (600.0, 1800.0);
        p.plunge_rate_percent = 30.0;
        p.max_doc = 2.0;
        p.stepover_percent = (25.0, 50.0);
        p.surface_speed_m_min = Some(180.0);
        p.chip_load_mm = Some(0.04);
        p.coolant_type = CoolantType::WaterSoluble;
        p.notes = "Copper starter values; use lubricant/coolant to reduce built-up edge. Sources: https://internaltool.com/docs/reference/speeds-and-feeds.pdf ; https://www.harveytool.com/resources/general-machining-guidelines".to_string();
        m.set_cutting_params("endmill_flat".to_string(), p);

        out.push(m);
    }

    out
}
