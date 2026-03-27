//! Address → peripheral.register mapping built from CMSIS-SVD (read-only annotation).

use std::path::Path;
use svd_parser::svd::{self, Device, Peripheral, Register, RegisterProperties};

/// Parsed SVD with fast range lookup for memory access annotation.
#[derive(Debug, Clone)]
pub struct SvdIndex {
    /// Sorted by `start`; half-open `[start, end)`.
    spans: Vec<(u64, u64, String)>,
}

#[derive(Debug, thiserror::Error)]
pub enum SvdLoadError {
    #[error("I/O error reading SVD: {0}")]
    Io(#[from] std::io::Error),

    #[error("SVD parse error: {0}")]
    Parse(String),
}

impl SvdIndex {
    /// Load and index registers from an SVD file (XML).
    pub fn load_from_path(path: &Path) -> Result<Self, SvdLoadError> {
        let xml = std::fs::read_to_string(path)?;
        let device = svd_parser::parse(&xml).map_err(|e| SvdLoadError::Parse(e.to_string()))?;
        Ok(Self::from_device(&device))
    }

    /// Build the index from a parsed [`Device`].
    pub fn from_device(device: &Device) -> Self {
        let mut spans = Vec::new();
        for p in &device.peripherals {
            let instances: Vec<svd::PeripheralInfo> = match p {
                Peripheral::Single(pi) => vec![pi.clone()],
                Peripheral::Array(pi, dim) => svd::peripheral::expand(pi, dim).collect(),
            };
            for pinfo in instances {
                if pinfo.derived_from.is_some() && pinfo.registers.is_none() {
                    continue;
                }
                let pbase = pinfo.base_address;
                let per_name = pinfo.name.as_str();
                let merged = merge_reg_props(
                    &device.default_register_properties,
                    &pinfo.default_register_properties,
                );
                for reg in pinfo.all_registers() {
                    collect_register_spans(pbase, per_name, reg, &merged, &mut spans);
                }
            }
        }
        spans.sort_by_key(|x| x.0);
        Self { spans }
    }

    /// Number of indexed register address ranges (after array expansion).
    pub fn register_count(&self) -> usize {
        self.spans.len()
    }

    /// Resolve `addr` to `Peripheral.REGISTER` if it lies in a register span.
    pub fn lookup(&self, addr: u64) -> Option<&str> {
        for (start, end, label) in &self.spans {
            if addr >= *start && addr < *end {
                return Some(label.as_str());
            }
        }
        None
    }

    /// Human-readable note for an RSP memory access `[addr, addr + len)`.
    pub fn annotate_access(&self, addr: u64, len: u64) -> Option<String> {
        if len == 0 {
            return None;
        }
        let last = addr.saturating_add(len).saturating_sub(1);
        let a = self.lookup(addr)?;
        let b = self.lookup(last)?;
        if a == b {
            return Some(format!("{a} ({len} bytes)"));
        }
        Some(format!("{a} .. {b} ({len} bytes)"))
    }
}

fn merge_reg_props(dev: &RegisterProperties, per: &RegisterProperties) -> RegisterProperties {
    let mut m = *dev;
    if per.size.is_some() {
        m.size = per.size;
    }
    if per.access.is_some() {
        m.access = per.access;
    }
    if per.protection.is_some() {
        m.protection = per.protection;
    }
    if per.reset_value.is_some() {
        m.reset_value = per.reset_value;
    }
    if per.reset_mask.is_some() {
        m.reset_mask = per.reset_mask;
    }
    m
}

fn reg_bits(r: &svd::RegisterInfo, merged: &RegisterProperties) -> u32 {
    r.properties.size.or(merged.size).unwrap_or(32)
}

fn collect_register_spans(
    pbase: u64,
    per_name: &str,
    reg: &Register,
    merged: &RegisterProperties,
    out: &mut Vec<(u64, u64, String)>,
) {
    match reg {
        Register::Single(ri) => {
            let bits = reg_bits(ri, merged);
            let bytes = bits.div_ceil(8).max(1) as u64;
            let start = pbase + u64::from(ri.address_offset);
            let end = start + bytes;
            let label = format!("{per_name}.{}", ri.name);
            out.push((start, end, label));
        }
        Register::Array(ri, dim) => {
            let bits = reg_bits(ri, merged);
            let bytes = bits.div_ceil(8).max(1) as u64;
            for (i, name) in svd::array::names(ri, dim).enumerate() {
                let off = u64::from(ri.address_offset) + (i as u64) * u64::from(dim.dim_increment);
                let start = pbase + off;
                let end = start + bytes;
                let label = format!("{per_name}.{name}");
                out.push((start, end, label));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL_SVD: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<device schemaVersion="1.1" xmlns:xs="http://www.w3.org/2001/XMLSchema-instance" xs:noNamespaceSchemaLocation="CMSIS-SVD_Schema_1_1.xsd">
  <name>TESTMCU</name>
  <version>1.0</version>
  <description>Minimal fixture</description>
  <addressUnitBits>8</addressUnitBits>
  <width>32</width>
  <peripherals>
    <peripheral>
      <name>GPIOA</name>
      <baseAddress>0x40020000</baseAddress>
      <registers>
        <register>
          <name>MODER</name>
          <addressOffset>0x0</addressOffset>
          <size>32</size>
        </register>
        <register>
          <name>BSRR</name>
          <addressOffset>0x18</addressOffset>
          <size>32</size>
        </register>
      </registers>
    </peripheral>
  </peripherals>
</device>
"#;

    #[test]
    fn parses_minimal_svd_and_looks_up_register() {
        let device = svd_parser::parse(MINIMAL_SVD).expect("parse fixture");
        let idx = SvdIndex::from_device(&device);
        assert_eq!(idx.lookup(0x4002_0000), Some("GPIOA.MODER"));
        assert_eq!(idx.lookup(0x4002_0018), Some("GPIOA.BSRR"));
        assert_eq!(idx.lookup(0x4002_0003), Some("GPIOA.MODER"));
        assert_eq!(idx.lookup(0x4001_FFFF), None);
        assert_eq!(
            idx.annotate_access(0x4002_0000, 4).as_deref(),
            Some("GPIOA.MODER (4 bytes)")
        );
    }

    #[test]
    fn load_from_path_matches_parse_inline() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("minimal.svd");
        std::fs::write(&path, MINIMAL_SVD).expect("write svd");
        let from_file = SvdIndex::load_from_path(&path).expect("load_from_path");
        let from_inline = SvdIndex::from_device(&svd_parser::parse(MINIMAL_SVD).expect("parse"));
        assert_eq!(from_file.register_count(), from_inline.register_count());
        assert_eq!(
            from_file.lookup(0x4002_0000),
            from_inline.lookup(0x4002_0000)
        );
    }
}
