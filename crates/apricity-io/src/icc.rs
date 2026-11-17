const HEADER_LEN: usize = 128;
const TAG_ENTRY_LEN: usize = 12;

fn be_u32(b: &[u8], at: usize) -> Option<usize> {
    let s = b.get(at..at.checked_add(4)?)?;
    Some(u32::from_be_bytes([s[0], s[1], s[2], s[3]]) as usize)
}

/// Read the profile description (`desc` tag) from raw ICC profile bytes.
///
/// Supports the ICC v2 `textDescriptionType` (`desc`, ASCII) and the v4
/// `multiLocalizedUnicodeType` (`mluc`, UTF-16BE; the first record is
/// returned). Returns `None` for truncated or malformed profiles or when no
/// description tag exists; never panics on arbitrary input.
pub fn icc_description(icc: &[u8]) -> Option<String> {
    let count = be_u32(icc, HEADER_LEN)?;
    for i in 0..count.min(256) {
        let e = HEADER_LEN + 4 + i * TAG_ENTRY_LEN;
        if icc.get(e..e + 4)? != b"desc" {
            continue;
        }
        let off = be_u32(icc, e + 4)?;
        let size = be_u32(icc, e + 8)?;
        let data = icc.get(off..off.checked_add(size)?)?;
        return match data.get(..4)? {
            b"desc" => parse_text_description(data),
            b"mluc" => parse_mluc(data),
            _ => None,
        };
    }
    None
}

fn parse_text_description(d: &[u8]) -> Option<String> {
    let n = be_u32(d, 8)?;
    let s = d.get(12..12usize.checked_add(n)?)?;
    let s = s.split(|&b| b == 0).next()?;
    Some(String::from_utf8_lossy(s).into_owned())
}

fn parse_mluc(d: &[u8]) -> Option<String> {
    if be_u32(d, 8)? == 0 {
        return None;
    }
    // First record: language(2) country(2) length(4) offset(4) at byte 16.
    let len = be_u32(d, 20)?;
    let off = be_u32(d, 24)?;
    let s = d.get(off..off.checked_add(len)?)?;
    let units: Vec<u16> = s
        .chunks_exact(2)
        .map(|c| u16::from_be_bytes([c[0], c[1]]))
        .collect();
    Some(
        String::from_utf16_lossy(&units)
            .trim_end_matches('\0')
            .to_owned(),
    )
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    fn wrap(tag_type: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let tag_off = HEADER_LEN + 4 + TAG_ENTRY_LEN;
        let mut tag = tag_type.to_vec();
        tag.extend_from_slice(&[0; 4]);
        tag.extend_from_slice(body);
        let mut p = vec![0u8; HEADER_LEN];
        p.extend_from_slice(&1u32.to_be_bytes());
        p.extend_from_slice(b"desc");
        p.extend_from_slice(&(tag_off as u32).to_be_bytes());
        p.extend_from_slice(&(tag.len() as u32).to_be_bytes());
        p.extend_from_slice(&tag);
        let total = p.len() as u32;
        p[0..4].copy_from_slice(&total.to_be_bytes());
        p
    }

    pub(crate) fn build_v2(desc: &str) -> Vec<u8> {
        let mut body = ((desc.len() + 1) as u32).to_be_bytes().to_vec();
        body.extend_from_slice(desc.as_bytes());
        body.push(0);
        wrap(b"desc", &body)
    }

    pub(crate) fn build_v4(desc: &str) -> Vec<u8> {
        let utf16: Vec<u8> = desc.encode_utf16().flat_map(u16::to_be_bytes).collect();
        let mut body = 1u32.to_be_bytes().to_vec();
        body.extend_from_slice(&12u32.to_be_bytes());
        body.extend_from_slice(b"enUS");
        body.extend_from_slice(&(utf16.len() as u32).to_be_bytes());
        // Offset is from the tag start: type(4) reserved(4) count(4) size(4) + record(12).
        body.extend_from_slice(&28u32.to_be_bytes());
        body.extend_from_slice(&utf16);
        wrap(b"mluc", &body)
    }

    pub(crate) fn build_desc_profile(desc: &str) -> Vec<u8> {
        build_v2(desc)
    }

    #[test]
    fn reads_v2_text_description() {
        assert_eq!(
            icc_description(&build_v2("Display P3")).as_deref(),
            Some("Display P3")
        );
    }

    #[test]
    fn reads_v4_mluc() {
        assert_eq!(
            icc_description(&build_v4("sRGB built-in")).as_deref(),
            Some("sRGB built-in")
        );
    }

    #[test]
    fn malformed_profiles_return_none() {
        assert_eq!(icc_description(&[]), None);
        assert_eq!(icc_description(&[0; 100]), None);
        let good = build_v2("Display P3");
        for cut in [129, 133, 140, good.len() - 3] {
            assert_eq!(icc_description(&good[..cut]), None, "cut {cut}");
        }
        let mut no_desc = good.clone();
        no_desc[132..136].copy_from_slice(b"wtpt");
        assert_eq!(icc_description(&no_desc), None);
        let mut huge = good;
        huge[140..144].copy_from_slice(&u32::MAX.to_be_bytes());
        assert_eq!(icc_description(&huge), None);
    }
}
