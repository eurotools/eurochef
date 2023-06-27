use nohash_hasher::IntMap;

pub fn parse_hashcodes(string: &str) -> IntMap<u32, String> {
    let res: IntMap<u32, String> = string.lines().filter_map(|l| parse_hashcode(l)).collect();

    let base_count = res
        .values()
        .filter(|k| k.ends_with("_HASHCODE_BASE"))
        .count();

    info!(
        "Loaded {} hashcodes ({} base hashcodes)",
        res.len(),
        base_count
    );

    res
}

fn parse_hashcode(line: &str) -> Option<(u32, String)> {
    if !line.starts_with("#define") {
        return None;
    }

    let parts: Vec<&str> = line.split_whitespace().skip(1).collect();
    if parts.len() != 2 {
        return None;
    }

    Some((parse_int::parse(parts[1]).ok()?, parts[0].to_string()))
}