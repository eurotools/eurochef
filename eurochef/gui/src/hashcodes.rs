use nohash_hasher::IntMap;

pub fn parse_hashcodes(string: &str) -> IntMap<u32, String> {
    string.lines().filter_map(|l| parse_hashcode(l)).collect()
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
