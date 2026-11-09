//! MHL ignore patterns.

/// Simple `.gitignore`-style matching for MHL ignore patterns: `*` and `?` wildcards,
/// a trailing `/` matches folders only, and a pattern without `/` matches a name at
/// any depth.
pub fn ignored(rel: &str, is_dir: bool, patterns: &[String]) -> bool {
    patterns.iter().any(|p| {
        let (p, dir_only) = match p.strip_suffix('/') {
            Some(s) => (s, true),
            None => (p.as_str(), false),
        };
        if dir_only && !is_dir {
            return false;
        }
        if p.contains('/') {
            wildcard(p.trim_start_matches('/'), rel)
        } else {
            rel.rsplit('/').next().is_some_and(|name| wildcard(p, name))
        }
    })
}

fn wildcard(p: &str, s: &str) -> bool {
    let (p, s): (Vec<char>, Vec<char>) = (p.chars().collect(), s.chars().collect());
    let (mut pi, mut si, mut star, mut mark) = (0, 0, None, 0);
    while si < s.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == s[si]) {
            pi += 1;
            si += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = si;
            pi += 1;
        } else if let Some(st) = star {
            pi = st + 1;
            mark += 1;
            si = mark;
        } else {
            return false;
        }
    }
    p[pi..].iter().all(|&c| c == '*')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignore_patterns() {
        let p: Vec<String> =
            [".DS_Store", "ascmhl", "ascmhl/", "._*", "logs/*.txt"].iter().map(|s| s.to_string()).collect();
        assert!(ignored("DCIM/.DS_Store", false, &p));
        assert!(ignored("ascmhl", true, &p));
        assert!(ignored("DCIM/._DSC_0001.NEF", false, &p));
        assert!(ignored("logs/a.txt", false, &p));
        assert!(!ignored("DCIM/DSC_0001.NEF", false, &p));
        assert!(!ignored("other/logs/a.txt", false, &p));
    }
}
