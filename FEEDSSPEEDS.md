# Feeds and Speeds Calculator Integration Plan

## Overview

This document outlines the plan to integrate the CAMTools Feeds and Speeds Calculator with the existing Tools, Materials, and Devices databases. The calculator currently exists but uses hardcoded/placeholder values and needs to be fully integrated with the application's data layer.

## Current State Analysis

### Existing Calculator (`crates/gcodekit5-camtools/src/speeds_feeds.rs`)

**Core Logic:**
- Uses standard machining formulas:
  - **RPM** = (Surface Speed × 1000) / (π × Tool Diameter)
  - **Chip Load** = Feed Rate / (RPM × Flutes)
  - **Feed Rate** = RPM × Chip Load × Flutes
- Includes clamping logic for safety (max RPM: 24,000, min RPM: 1,000)
- Returns `CalculationResult` with warnings when clamping occurs

**Current Data Sources:**
| Source | Fields Used |
|--------|-------------|
| Material | `cutting_params[tool_type]`: `surface_speed_m_min`, `chip_load_mm`, `rpm_range`, `feed_rate_range` |
| Tool | `diameter`, `flutes`, `tool_type`, `params.rpm`, `params.feed_rate`, `params.rpm_range` |
| Device | `max_feed_rate` (for clamping) |

**Current UI Issues:**
- UI is a placeholder (`crates/gcodekit5-ui/src/ui/gtk/cam_tools/speeds_feeds.rs`)
- Hardcoded material and tool dropdowns
- Calculate button sets static values (12,000 RPM, 1,500 mm/min)
- Does NOT call `SpeedsFeedsCalculator::calculate()`
- No integration with actual Tools Manager or Materials Database

---

## Database Gaps Analysis

### Tools Database (`gcodekit5-core/src/data/tools.rs`)

**Current Fields:**
- Basic: `id`, `number`, `name`, `description`, `tool_type`
- Geometry: `diameter`, `shaft_diameter`, `length`, `flute_length`, `flutes`, `corner_radius`, `tip_angle`
- Material: `material` (HSS, Carbide, etc.), `coating` (TiN, TiAlN, etc.), `shank`
- Cutting Params: `rpm`, `rpm_range`, `feed_rate`, `plunge_rate`, `stepover_percent`, `depth_per_pass`

**Missing Fields for Feeds/Speeds:**

| Field | Type | Purpose |
|-------|------|---------|
| `helix_angle` | `Option<f32>` | Chip evacuation, cutting forces, deflection calc |
| `max_rpm_rating` | `Option<u32>` | Tool's safe maximum RPM from manufacturer |
| `stick_out_max` | `Option<f32>` | Max recommended stick-out for rigidity |
| `core_diameter` | `Option<f32>` | Tool core for deflection calculations |
| `rake_angle` | `Option<f32>` | Cutting edge geometry for chip formation |
| `relief_angle` | `Option<f32>` | Clearance angle for cutting efficiency |
| `max_chip_load` | `Option<f32>` | Per-tooth max chip load rating |
| `min_doc` | `Option<f32>` | Minimum depth of cut for engagement |
| `max_doc_percent` | `Option<f32>` | Max depth as % of diameter |
| `chip_breaker` | `bool` | Whether tool has chip breaker geometry |
| `through_coolant` | `bool` | Tool supports through-spindle coolant |

### Materials Database (`gcodekit5-core/src/data/materials.rs`)

**Current Fields:**
- Physical: `density`, `machinability_rating`, `tensile_strength`, `melting_point`
- Machining: `chip_type`, `heat_sensitivity`, `abrasiveness`, `surface_finish`
- Safety: `dust_hazard`, `fume_hazard`, `required_ppe`, `coolant_required`
- Cutting Params: `cutting_params: HashMap<String, CuttingParameters>` per tool type

**Missing Fields for Feeds/Speeds:**

| Field | Type | Purpose |
|-------|------|---------|
| `brinell_hardness` | `Option<f32>` | Hardness for SFM adjustment |
| `rockwell_hardness` | `Option<f32>` | Alternative hardness scale |
| `thermal_conductivity` | `Option<f32>` | Heat dissipation for feed adjustments |
| `specific_cutting_force` | `Option<f32>` | Kc value for power calculations (N/mm²) |
| `tool_material_speed_factors` | `HashMap<ToolMaterial, f32>` | SFM multipliers per tool material (HSS vs Carbide) |
| `hardness_speed_adjustment` | `Option<f32>` | SFM adjustment factor per hardness unit |
| `recommended_coolant_pressure` | `Option<f32>` | Pressure for through-coolant (bar) |

**CuttingParameters Struct Additions:**

| Field | Type | Purpose |
|-------|------|---------|
| `sfm_range` | `(f32, f32)` | Surface feet per minute range |
| `chipload_range` | `(f32, f32)` | Min/max chip load per tooth |
| `power_factor` | `Option<f32>` | Power requirement multiplier |
| `ramp_angle_max` | `Option<f32>` | Maximum ramp/plunge angle |
| `adaptive_stepover_max` | `Option<f32>` | Max stepover for adaptive/high-speed machining |

### Devices Database (`gcodekit5-devicedb/src/model.rs`)

**Current Fields:**
- Workspace: `x_axis`, `y_axis`, `z_axis`, `a_axis` limits
- Capabilities: `num_axes`, `has_spindle`, `has_laser`, `has_coolant`
- Limits: `max_feed_rate`, `max_s_value`, `max_spindle_speed_rpm`
- Power: `cnc_spindle_watts`, `laser_watts`

**Missing Fields for Feeds/Speeds:**

| Field | Type | Purpose |
|-------|------|---------|
| `spindle_power_curve` | `Option<Vec<(u32, f32)>>` | Power vs RPM curve for calculations |
| `max_torque` | `Option<f32>` | Max spindle torque (Nm) |
| `spindle_type` | `SpindleType` | Belt, direct, gear, etc. |
| `feed_acceleration` | `Option<f32>` | Max acceleration (mm/s²) |
| `rigidity_class` | `RigidityClass` | Light/Medium/Heavy duty |
| `recommended_kt_factor` | `Option<f32>` | Machine-specific constant |
| `coolant_capacity` | `Option<f32>` | Coolant flow rate (L/min) |

---

## Milestones and Tasks

### Milestone 1: Database Schema Updates (Week 1)

#### Task 1.1: Extend Tools Schema
- [ ] Add `ToolGeometry` struct with helix_angle, core_diameter, rake_angle, relief_angle
- [ ] Add `ToolLimits` struct with max_rpm_rating, max_chip_load, stick_out_max
- [ ] Add `ToolFeatures` struct with chip_breaker, through_coolant flags
- [ ] Update `Tool` struct to include new sub-structs
- [ ] Update `ToolCuttingParams` with min_doc, max_doc_percent
- [ ] Update Default implementation for Tool
- [ ] Update serialization/deserialization tests

**Files:**
- `crates/gcodekit5-core/src/data/tools.rs`
- `crates/gcodekit5-core/src/data/tools_test.rs`

**Estimated:** 4-6 hours

#### Task 1.2: Extend Materials Schema
- [ ] Add physical properties: brinell_hardness, thermal_conductivity, specific_cutting_force
- [ ] Add `tool_material_speed_factors: HashMap<ToolMaterial, f32>` to Material
- [ ] Extend `CuttingParameters` with sfm_range, chipload_range, power_factor, ramp_angle_max
- [ ] Update `Material::default()` implementations
- [ ] Update tests

**Files:**
- `crates/gcodekit5-core/src/data/materials.rs`
- `crates/gcodekit5-core/src/data/materials_test.rs`

**Estimated:** 4-6 hours

#### Task 1.3: Extend Devices Schema
- [ ] Add `SpindleType` enum: BeltDrive, DirectDrive, GearDrive, HighSpeed
- [ ] Add `RigidityClass` enum: LightDuty, MediumDuty, HeavyDuty, Industrial
- [ ] Add spindle_power_curve: Vec<(rpm, power)>
- [ ] Add max_torque, feed_acceleration, rigidity_class
- [ ] Update `DeviceProfile` struct
- [ ] Update default implementations

**Files:**
- `crates/gcodekit5-devicedb/src/model.rs`
- `crates/gcodekit5-devicedb/src/device_defaults.rs`

**Estimated:** 3-4 hours

---

### Milestone 2: Enhanced Calculation Engine (Week 2)

#### Task 2.1: Advanced Calculation Methods
- [ ] Implement `calculate_rpm_with_adjustments()` - considers tool coating, material hardness
- [ ] Implement `calculate_chip_load_with_adjustments()` - considers helix angle, rake angle
- [ ] Implement `calculate_power_requirement()` - verifies spindle can handle cut
- [ ] Implement `calculate_deflection_estimate()` - warns on excessive stick-out
- [ ] Implement `calculate_adaptive_params()` - for high-speed machining
- [ ] Add machine rigidity factor to feed rate calculations

**Files:**
- `crates/gcodekit5-camtools/src/speeds_feeds.rs`

**Estimated:** 6-8 hours

#### Task 2.2: Material-Specific Formulas
- [ ] Create `MaterialCalculator` trait for different material families
- [ ] Implement `AluminumCalculator` - high SFM, moderate chip load
- [ ] Implement `SteelCalculator` - conservative SFM, variable chip load
- [ ] Implement `WoodCalculator` - very high SFM, high chip load
- [ ] Implement `PlasticCalculator` - high SFM, low chip load to prevent melting
- [ ] Implement `CompositeCalculator` - special considerations for fibers

**Files:**
- `crates/gcodekit5-camtools/src/speeds_feeds/materials/` (new module)

**Estimated:** 6-8 hours

#### Task 2.3: Tool Type Specific Logic
- [ ] Implement `EndMillCalculator` - standard formulas
- [ ] Implement `VBitCalculator` - tip diameter compensation
- [ ] Implement `DrillCalculator` - point angle, web thickness considerations
- [ ] Implement `BallMillCalculator` - scallop height calculations
- [ ] Implement `ChamferCalculator` - edge preparation logic

**Files:**
- `crates/gcodekit5-camtools/src/speeds_feeds/tools/` (new module)

**Estimated:** 6-8 hours

---

### Milestone 3: Database Integration Layer (Week 3)

#### Task 3.1: Tools Manager Integration
- [ ] Create `SpeedsFeedsToolService` to query Tools Manager
- [ ] Implement tool selection by diameter and type
- [ ] Implement tool filtering by material compatibility
- [ ] Add tool parameter validation
- [ ] Cache frequently used tools

**Files:**
- `crates/gcodekit5-camtools/src/speeds_feeds/tool_service.rs` (new)

**Estimated:** 4-5 hours

#### Task 3.2: Materials Database Integration
- [ ] Create `SpeedsFeedsMaterialService` to query Materials Database
- [ ] Implement material lookup by name/category
- [ ] Implement cutting parameter retrieval for tool type
- [ ] Add material hardness-based SFM adjustment
- [ ] Cache material data

**Files:**
- `crates/gcodekit5-camtools/src/speeds_feeds/material_service.rs` (new)

**Estimated:** 4-5 hours

#### Task 3.3: Device Profile Integration
- [ ] Create `SpeedsFeedsDeviceService` to query Device Manager
- [ ] Get active device profile
- [ ] Validate calculated parameters against machine limits
- [ ] Apply machine rigidity factor
- [ ] Check spindle power adequacy

**Files:**
- `crates/gcodekit5-camtools/src/speeds_feeds/device_service.rs` (new)

**Estimated:** 3-4 hours

#### Task 3.4: Unified Data Access
- [ ] Create `SpeedsFeedsDataContext` struct aggregating all services
- [ ] Implement data loading and caching
- [ ] Add error handling for missing data
- [ ] Provide fallback to default values

**Files:**
- `crates/gcodekit5-camtools/src/speeds_feeds/data_context.rs` (new)

**Estimated:** 3-4 hours

---

### Milestone 4: UI Implementation (Week 4)

#### Task 4.1: Tool Selection Dialog
- [ ] Create `SpeedsFeedsToolSelector` dialog
- [ ] Display tools in table with diameter, flutes, material, coating
- [ ] Add filtering by tool type (end mill, drill, v-bit)
- [ ] Add filtering by diameter range
- [ ] Show tool details on selection
- [ ] Connect to Tools Manager backend

**Files:**
- `crates/gcodekit5-ui/src/ui/gtk/cam_tools/speeds_feeds_tool_selector.rs` (new)

**Estimated:** 6-8 hours

#### Task 4.2: Material Selection Dialog
- [ ] Create `SpeedsFeedsMaterialSelector` dialog
- [ ] Display materials grouped by category (Metal, Wood, Plastic, etc.)
- [ ] Show material properties and hardness
- [ ] Add search functionality
- [ ] Connect to Materials Database

**Files:**
- `crates/gcodekit5-ui/src/ui/gtk/cam_tools/speeds_feeds_material_selector.rs` (new)

**Estimated:** 5-6 hours

#### Task 4.3: Main Calculator UI Redesign
- [ ] Replace hardcoded dropdowns with selection buttons
- [ ] Add "Select Tool" button opening tool selector
- [ ] Add "Select Material" button opening material selector
- [ ] Display selected tool/material info
- [ ] Add input fields for operation parameters:
  - Depth of Cut (mm)
  - Width of Cut / Stepover (%)
  - Tool Stick-Out (mm)
  - Coolant On/Off
- [ ] Add "Calculate" button calling actual calculation engine
- [ ] Display results with color-coded warnings

**Files:**
- `crates/gcodekit5-ui/src/ui/gtk/cam_tools/speeds_feeds.rs` (rewrite)

**Estimated:** 8-10 hours

#### Task 4.4: Results Display Panel
- [ ] Create detailed results panel with:
  - Calculated RPM
  - Calculated Feed Rate
  - Chip Load per Tooth
  - Surface Speed (SFM and m/min)
  - Material Removal Rate (MRR)
  - Estimated Power Required
  - Estimated Deflection
- [ ] Add visual indicators for:
  - Within safe limits (green)
  - Near limits (yellow)
  - Exceeds limits (red)
- [ ] Show warnings list with explanations
- [ ] Add "Copy to Clipboard" for G-code or M-codes

**Files:**
- `crates/gcodekit5-ui/src/ui/gtk/cam_tools/speeds_feeds_results.rs` (new)

**Estimated:** 6-8 hours

---

### Milestone 5: Advanced Features (Week 5-6)

#### Task 5.1: Recipe Save/Load
- [ ] Create `SpeedsFeedsRecipe` struct for saved calculations
- [ ] Add save dialog with name and description
- [ ] Add recipe list view
- [ ] Implement recipe loading
- [ ] Store recipes in settings file

**Files:**
- `crates/gcodekit5-settings/src/recipes.rs` (new)

**Estimated:** 4-5 hours

#### Task 5.2: Operation Type Support
- [ ] Add operation type selector: Slotting, Pocketing, Profiling, Adaptive, Drilling, Plunging
- [ ] Implement radial engagement calculations
- [ ] Implement axial engagement calculations
- [ ] Adjust feed rate for engagement type
- [ ] Add ramp/plunge feed calculations

**Files:**
- `crates/gcodekit5-camtools/src/speeds_feeds/operations.rs` (new)

**Estimated:** 6-8 hours

#### Task 5.3: Visualization
- [ ] Add chip thinning visualization for small stepovers
- [ ] Show tool engagement diagram
- [ ] Display recommended vs actual chipload comparison
- [ ] Add spindle load gauge
- [ ] Visual deflection warning indicator

**Files:**
- `crates/gcodekit5-ui/src/ui/gtk/cam_tools/speeds_feeds_viz.rs` (new)

**Estimated:** 6-8 hours

#### Task 5.4: G-Code Generation
- [ ] Generate G-code for:
  - Spindle start with calculated RPM
  - Feed rate setting
  - Coolant on/off
- [ ] Add "Send to Editor" button
- [ ] Add "Copy G-Code" button
- [ ] Format with comments explaining parameters

**Files:**
- `crates/gcodekit5-camtools/src/speeds_feeds/gcode_gen.rs` (new)

**Estimated:** 4-5 hours

---

## Implementation Details

### Calculation Priority Order

When calculating feeds and speeds, data should be prioritized as follows:

1. **Material-specific cutting parameters** (highest priority)
   - From Material.cutting_params[tool_type]
   - Use if available for specific tool type

2. **Material general properties** (if no specific params)
   - Calculate from hardness, machinability rating
   - Apply tool material speed factors

3. **Tool-specific defaults** (fallback)
   - From Tool.params
   - Use tool's recommended values

4. **Global defaults** (last resort)
   - Conservative values based on material family

### Formula Reference

#### RPM Calculation
```rust
// Base RPM from surface speed
let rpm = (surface_speed_m_min * 1000.0) / (PI * diameter_mm);

// Adjust for tool coating
coating_factor = match tool.coating {
    Some(TiAlN) => 1.15,  // +15% for heat resistance
    Some(DLC) => 1.20,     // +20% for low friction
    _ => 1.0,
};

// Adjust for material hardness (if brinell > 200)
hardness_factor = if brinell > 200.0 {
    1.0 - ((brinell - 200.0) / 1000.0)
} else {
    1.0
};

final_rpm = rpm * coating_factor * hardness_factor;
```

#### Chip Load Calculation
```rust
// Base chip load from material/tool combo
let chip_load_mm = cutting_params.chip_load_mm.unwrap_or_else(|| {
    // Calculate from feed rate range
    let avg_feed = (feed_range.0 + feed_range.1) / 2.0;
    let avg_rpm = (rpm_range.0 + rpm_range.1) / 2;
    avg_feed / (avg_rpm as f32 * flutes as f32)
});

// Adjust for helix angle (higher helix = lower chip load)
helix_factor = if helix_angle > 45.0 { 0.9 } else { 1.0 };

// Adjust for radial engagement
engagement_factor = match engagement_percent {
    0.0..=25.0 => 1.3,  // Low engagement allows higher chip load
    25.0..=50.0 => 1.0, // Standard
    50.0..=75.0 => 0.85, // Higher engagement needs lower chip load
    _ => 0.7,            // Full slotting is most demanding
};

final_chip_load = chip_load_mm * helix_factor * engagement_factor;
```

#### Feed Rate Calculation
```rust
feed_rate = rpm * chip_load * flutes;

// For high-speed machining (adaptive)
if operation_type == Adaptive {
    // Increase feed rate based on lower engagement
    feed_rate *= 1.5;
}

// For plunging/ramping
if operation_type == Plunging {
    // Reduce feed rate
    feed_rate *= 0.5;
}
```

#### Power Requirement Estimation
```rust
// Specific cutting force (Kc) from material
kc = material.specific_cutting_force.unwrap_or(1000.0); // N/mm²

// Chip cross-section
chip_area = depth_of_cut * (chip_load * diameter); // mm²

// Cutting force
force = kc * chip_area; // Newtons

// Cutting power (at tool tip)
power_kw = (force * cutting_speed_m_min) / 60000.0;

// Spindle power required (with efficiency factor)
spindle_power = power_kw / 0.85; // 85% efficiency
```

### Clamping Strategy

```rust
// Apply machine limits
if calculated_rpm > device.max_spindle_speed_rpm {
    warnings.push(format!(
        "RPM clamped from {} to {} (machine limit)",
        calculated_rpm, device.max_spindle_speed_rpm
    ));
    unclamped_rpm = Some(calculated_rpm);
    calculated_rpm = device.max_spindle_speed_rpm;
}

if calculated_feed > device.max_feed_rate {
    warnings.push(format!(
        "Feed rate clamped from {:.1} to {:.1} (machine limit)",
        calculated_feed, device.max_feed_rate
    ));
    unclamped_feed = Some(calculated_feed);
    calculated_feed = device.max_feed_rate;
}

// Tool limit checks
if let Some(max_rpm) = tool.max_rpm_rating {
    if calculated_rpm > max_rpm {
        warnings.push(format!(
            "RPM {} exceeds tool rating of {}",
            calculated_rpm, max_rpm
        ));
    }
}
```

---

## Testing Strategy

### Unit Tests
- [ ] Test calculation formulas with known values
- [ ] Test clamping logic
- [ ] Test data priority/fallback chain
- [ ] Test edge cases (zero values, very small tools, etc.)

### Integration Tests
- [ ] Test tool selection from database
- [ ] Test material lookup and parameter retrieval
- [ ] Test device limit validation
- [ ] Test full calculation workflow

### UI Tests
- [ ] Test tool selector dialog
- [ ] Test material selector dialog
- [ ] Test calculation button
- [ ] Test result display
- [ ] Test recipe save/load

### Validation Tests
- [ ] Compare calculated values against industry reference tables
- [ ] Validate against manufacturer recommendations
- [ ] Test with real machining scenarios

---

## Success Criteria

1. **Functionality:**
   - [ ] Calculator uses real data from Tools, Materials, and Devices databases
   - [ ] All standard tool types supported
   - [ ] All material categories supported
   - [ ] Machine limits are respected and warned

2. **Accuracy:**
   - [ ] Calculated RPM within 5% of manufacturer recommendations
   - [ ] Calculated feed rates produce acceptable chip loads
   - [ ] Power estimates within 20% of actual

3. **Usability:**
   - [ ] Tool selection takes < 3 clicks
   - [ ] Material selection takes < 3 clicks
   - [ ] Results displayed within 100ms of calculation
   - [ ] Warnings are clear and actionable

4. **Reliability:**
   - [ ] No crashes with missing/invalid data
   - [ ] Graceful fallback to defaults
   - [ ] Settings persist across sessions

---

## Timeline Summary

| Milestone | Duration | Cumulative |
|-----------|----------|------------|
| 1. Database Schema Updates | 1 week | Week 1 |
| 2. Enhanced Calculation Engine | 1 week | Week 2 |
| 3. Database Integration Layer | 1 week | Week 3 |
| 4. UI Implementation | 1 week | Week 4 |
| 5. Advanced Features | 2 weeks | Week 6 |
| **Total** | **6 weeks** | |

---

## Future Enhancements (Post-MVP)

- [ ] Cloud database of manufacturer cutting parameters
- [ ] Machine learning for parameter optimization based on user feedback
- [ ] Vibration prediction and avoidance
- [ ] Tool wear estimation
- [ ] Cost per part calculator
- [ ] Integration with CAM toolpath generation
- [ ] Real-time adjustment based on spindle load feedback
