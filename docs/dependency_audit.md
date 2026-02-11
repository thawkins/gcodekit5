# Dependency Audit Report

**Generated**: 2026-01-29  
**Updated**: 2026-01-29 (after full remediation)  
**Project**: gcodekit5 v0.50.2-alpha.0  
**Total Dependencies**: ~529 crates (from Cargo.lock)  
**Debug Build Size**: ~42 GB  

---

## Executive Summary

This audit identifies:
- **2 Security Warnings** (unmaintained crates) - 2 fixed
- **0 Unused Dependencies** (false positives verified)
- **~23 Duplicate Dependency Versions** (reduced from 34)
- Several recommendations for optimization

### Remediation Applied (2026-01-29)

| Issue | Action | Status |
|-------|--------|--------|
| `image 0.22.5` vulnerability | Updated `dxf` from 0.4.0 to 0.6.0 | ✅ Fixed |
| `glib/gio` duplicate versions | Downgraded `glib-build-tools` from 0.21 to 0.20 | ✅ Fixed |
| `thiserror` duplicate versions | Upgraded designer/communication from 1.x to 2.x | ✅ Fixed |
| `stl_io` duplicate versions | Upgraded designer from 0.7 to 0.8 | ✅ Fixed |
| Unused `rfd` | Verified usage - false positive | ✅ Verified |
| Unused `tempfile` | Verified usage - false positive | ✅ Verified |
| `rusttype` unmaintained | Deferred - complex migration, no security risk | ⏳ Pending |

---

## 1. Security Audit Results

Run via `cargo audit`:

### 1.1 Unmaintained Crates (Warnings)

| Crate | Version | Advisory ID | Status | Root Cause |
|-------|---------|-------------|--------|------------|
| ~~`lzw`~~ | ~~0.10.0~~ | ~~RUSTSEC-2020-0144~~ | ~~Unmaintained~~ | ~~Via `image 0.22.5` → `dxf 0.4.0`~~ ✅ **FIXED** |
| `paste` | 1.0.15 | RUSTSEC-2024-0436 | Unmaintained | Via `simba` → `nalgebra` → `rapier3d-f64` (transitive) |
| `rusttype` | 0.9.3 | RUSTSEC-2021-0140 | Unmaintained | Direct dep of `gcodekit5-designer` |

### 1.2 Unsound Crates (Medium Severity)

| Crate | Version | Advisory ID | Severity | Description |
|-------|---------|-------------|----------|-------------|
| ~~`image`~~ | ~~0.22.5~~ | ~~RUSTSEC-2020-0073~~ | ~~5.5 (Medium)~~ | ~~Mutable reference with immutable provenance~~ ✅ **FIXED** |

**Fixed**: Upgraded `dxf` from 0.4.0 to 0.6.0 which no longer depends on the vulnerable `image 0.22.5`.

---

## 2. Unused Dependencies

Run via `cargo +nightly udeps --all-targets`:

| Package | Type | Dependency | Notes |
|---------|------|------------|-------|
| ~~`gcodekit5`~~ | ~~dependencies~~ | ~~`rfd`~~ | ✅ **Verified used** - file dialogs in platform.rs and legacy callbacks |
| ~~`gcodekit5`~~ | ~~dev-dependencies~~ | ~~`tempfile`~~ | ✅ **Verified used** - tests and gcode_editor.rs |

**Note**: These may be false positives. Verify usage before removal.

---

## 3. Duplicate Dependency Versions

The following crates have multiple versions in the dependency tree:

### 3.1 High Impact Duplicates (Large Crates)

| Crate | Versions | Impact |
|-------|----------|--------|
| ~~`image`~~ | ~~0.22.5, 0.25.9~~ | ~~**High** - Large crate with many transitive deps~~ ✅ **Consolidated** |
| ~~`glib`~~ | ~~0.20.12, 0.21.5~~ | ~~**High** - Core GTK binding~~ ✅ **Consolidated to 0.20** |
| ~~`gio`~~ | ~~0.20.12, 0.21.5~~ | ~~**High** - Core GTK binding~~ ✅ **Consolidated to 0.20** |
| ~~`dxf`~~ | ~~0.4.0, 0.6.0~~ | ~~**Medium** - Direct and transitive deps~~ ✅ **Consolidated to 0.6** |
| `png` | 0.17.16, 0.18.0 | **Medium** - 2 versions (transitive) |

### 3.2 Medium Impact Duplicates

| Crate | Versions |
|-------|----------|
| `bitflags` | 1.3.2, 2.10.0 (ecosystem migration in progress) |
| `downcast-rs` | 1.2.1, 2.0.2 (transitive) |
| `itertools` | 0.11.0, 0.13.0, 0.14.0 (transitive) |
| `nom` | 7.1.3, 8.0.0 (transitive) |
| `num-bigint` | 0.3.3, 0.4.6 (transitive) |
| ~~`thiserror`~~ | ~~1.0.69, 2.0.17~~ ✅ **Consolidated to 2.x** |
| `toml` | 0.8.23, 0.9.10 (transitive) |
| `toml_edit` | 0.22.27, 0.23.10 (transitive) |
| `nix` | 0.26.4, 0.30.1 (transitive) |

### 3.3 Low Impact Duplicates

| Crate | Versions |
|-------|----------|
| ~~`glib-macros`~~ | ~~0.20.12, 0.21.5~~ ✅ **Consolidated to 0.20** |
| ~~`glib-sys`~~ | ~~0.20.10, 0.21.5~~ ✅ **Consolidated to 0.20** |
| ~~`gobject-sys`~~ | ~~0.20.10, 0.21.5~~ ✅ **Consolidated to 0.20** |
| ~~`stl_io`~~ | ~~0.7.0, 0.8.6~~ ✅ **Consolidated to 0.8** |
| `hashbrown` | 0.15.5, 0.16.1 |
| `num-traits` | 0.1.43, 0.2.19 |
| `serde_spanned` | 0.6.9, 1.0.4 |
| `toml_datetime` | 0.6.11, 0.7.5 |
| `ttf-parser` | 0.15.2, 0.20.0, 0.25.1 (via rusttype, fontdb, csgrs) |
| `zune-core` | 0.4.12, 0.5.0 |
| `zune-jpeg` | 0.4.21, 0.5.8 |

---

## 4. Large Dependencies Analysis

Notable large dependency chains:

| Dependency | Purpose | Transitive Deps | Notes |
|------------|---------|-----------------|-------|
| `rapier3d-f64` | Physics engine | ~50+ | Via `csgrs` for CSG operations |
| `image` | Image processing | ~40+ | Via multiple paths |
| `gtk4` | GUI framework | ~100+ | Core UI dependency |
| `rav1e` | AV1 encoder | ~30+ | Via `image` for AVIF support |

---

## 5. Recommendations

### 5.1 Priority 1: Security Fixes

1. ~~**Update `dxf` from 0.4.0 to 0.6.0** in `gcodekit5-camtools`~~
   - ~~This removes the vulnerable `image 0.22.5` dependency~~
   - ~~Also removes `lzw 0.10.0` warning~~
   - ✅ **COMPLETED** (2026-01-29)

2. **Replace `rusttype`** in `gcodekit5-designer` (Deferred)
   - Consider migrating to `ttf-parser` (has compatible `OutlineBuilder` trait)
   - `rusttype` is unmaintained since 2021 but has no security issues
   - Complex migration affecting 5 files; lower priority

### 5.2 Priority 2: Consolidate Duplicates

1. ~~**Consolidate GTK bindings** to single version (0.20.x or 0.21.x)~~
   - ~~`glib-build-tools` pulls in 0.21.x~~
   - ~~Other crates use 0.20.x~~
   - ✅ **COMPLETED** - Downgraded `glib-build-tools` to 0.20.0
   
2. **Update `csgrs`** if newer version exists
   - Would help with `dxf` version consolidation

3. **Review `itertools` usage**
   - 4 different versions present
   - Most can likely use 0.14.0

### 5.3 Priority 3: Remove Unused

1. Verify `rfd` usage in main crate
   - If only used in UI crate, remove from main Cargo.toml
   
2. Verify `tempfile` usage in tests
   - May be legitimate dev-dependency

### 5.4 Priority 4: Build Optimization

1. Consider `image` feature flags to reduce compiled features
2. Review if AVIF support (`ravif`/`rav1e`) is needed
3. Consider workspace dependency inheritance for version consistency

---

## 6. Commands for Further Investigation

```bash
# View full dependency tree
cargo tree

# Find duplicates
cargo tree --duplicates

# Find why a crate is included
cargo tree -i <crate-name>

# Security audit
cargo audit

# Find unused dependencies (requires nightly)
cargo +nightly udeps --all-targets

# Analyze build times
cargo build --timings
```

---

## 7. Related Tasks

- **Task 7.1.2**: Remove Unused Dependencies
- **Task 7.1.3**: Consolidate Duplicate Dependency Versions
- **Task 7.2.1**: Setup Dependabot for automated updates
