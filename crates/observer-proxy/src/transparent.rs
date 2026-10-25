//! Inspect only the serialized transparent bundle prefix, never guess on malformed data.
use anyhow::{bail, ensure, Result};
use serde::Serialize;

#[derive(Debug, Serialize, PartialEq)]
pub struct TransparentSummary {
    pub version: u32,
    pub inputs: u64,
    pub outputs: u64,
    pub has_transparent_component: bool,
}

struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}
impl Reader<'_> {
    fn take(&mut self, size: usize) -> Result<&[u8]> {
        let end = self
            .pos
            .checked_add(size)
            .ok_or_else(|| anyhow::anyhow!("size overflow"))?;
        ensure!(end <= self.bytes.len(), "truncated transparent bundle");
        let result = &self.bytes[self.pos..end];
        self.pos = end;
        Ok(result)
    }
    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into()?))
    }
    fn compact_size(&mut self) -> Result<u64> {
        let prefix = self.take(1)?[0];
        let (value, minimum) = match prefix {
            0..=252 => (prefix as u64, 0),
            253 => (u16::from_le_bytes(self.take(2)?.try_into()?) as u64, 253),
            254 => (self.u32()? as u64, 65536),
            255 => (u64::from_le_bytes(self.take(8)?.try_into()?), 4294967296),
        };
        ensure!(value >= minimum, "noncanonical CompactSize");
        Ok(value)
    }
    fn script(&mut self) -> Result<()> {
        let length = usize::try_from(self.compact_size()?)?;
        self.take(length)?;
        Ok(())
    }
}

pub fn inspect(bytes: &[u8]) -> Result<TransparentSummary> {
    let mut reader = Reader { bytes, pos: 0 };
    let header = reader.u32()?;
    let version = header & 0x7fffffff;
    let overwintered = header & 0x80000000 != 0;
    match version {
        1 | 2 => ensure!(!overwintered, "unexpected overwinter flag"),
        3..=5 => {
            ensure!(overwintered, "missing overwinter flag");
            let expected_group = match version {
                3 => 0x03c48270,
                4 => 0x892f2085,
                _ => 0x26a7270a,
            };
            ensure!(
                reader.u32()? == expected_group,
                "unexpected version group ID"
            );
            if version == 5 {
                reader.take(12)?;
            }
        }
        _ => bail!("unsupported transaction version {version}"),
    }
    let inputs = reader.compact_size()?;
    ensure!(inputs <= bytes.len() as u64 / 41, "impossible input count");
    for _ in 0..inputs {
        reader.take(36)?;
        reader.script()?;
        reader.take(4)?;
    }
    let outputs = reader.compact_size()?;
    ensure!(outputs <= bytes.len() as u64 / 9, "impossible output count");
    for _ in 0..outputs {
        reader.take(8)?;
        reader.script()?;
    }
    Ok(TransparentSummary {
        version,
        inputs,
        outputs,
        has_transparent_component: inputs != 0 || outputs != 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn prefix(version: u32) -> Vec<u8> {
        let mut tx = (version | 0x80000000).to_le_bytes().to_vec();
        tx.extend_from_slice(
            &(if version == 4 {
                0x892f2085u32
            } else {
                0x26a7270a
            })
            .to_le_bytes(),
        );
        if version == 5 {
            tx.extend_from_slice(&[0; 12]);
        }
        tx
    }
    #[test]
    fn recognizes_transparent_output_in_v4_and_v5() {
        for version in [4, 5] {
            let mut tx = prefix(version);
            tx.extend_from_slice(&[0, 1]);
            tx.extend_from_slice(&42u64.to_le_bytes());
            tx.extend_from_slice(&[1, 0x51]);
            assert_eq!(
                inspect(&tx).unwrap(),
                TransparentSummary {
                    version,
                    inputs: 0,
                    outputs: 1,
                    has_transparent_component: true
                }
            );
            tx.pop();
            assert!(inspect(&tx).is_err());
        }
    }
    #[test]
    fn malformed_data_never_becomes_a_clean_transaction() {
        for bytes in [vec![], vec![5, 0, 0, 128], vec![1, 0, 0, 0, 253, 0, 0]] {
            assert!(inspect(&bytes).is_err());
        }
        let mut tx = prefix(4);
        tx.extend_from_slice(&[0, 0]);
        assert!(!inspect(&tx).unwrap().has_transparent_component);
    }
}
