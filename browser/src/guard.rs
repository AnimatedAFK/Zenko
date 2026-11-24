// Shared rules the pages follow: the ad/tracker blocker, focus mode (blocked sites / allow-list), link cleaning,
// and guessing which IB subject a page belongs to (for downloads and tab groups).
use std::collections::HashSet;

const LIST: &str = include_str!("blocklist.txt");

#[derive(Default)]
pub struct Guard {
    pub ads: bool,
    pub ad_hosts: HashSet<String>,
    /// sites where the blocker is switched off
    pub ads_off: HashSet<String>,
    pub focus: bool,
    pub focus_block: Vec<String>,
    /// when not empty in focus mode, ONLY these sites open
    pub focus_only: Vec<String>,
    pub focus_allow: Vec<String>,
    pub clean: bool,
}

impl Guard {
    pub fn new() -> Self {
        let ad_hosts = LIST.lines().map(|l| l.trim()).filter(|l| !l.is_empty() && !l.starts_with('#')).map(|l| l.to_lowercase()).collect();
        Guard { ads: true, ad_hosts, clean: true, ..Default::default() }
    }

    pub fn is_ad(&self, host: &str) -> bool {
        let mut h = host;
        loop {
            if self.ad_hosts.contains(h) {
                return true;
            }
            match h.find('.') {
                Some(i) => h = &h[i + 1..],
                None => return false,
            }
        }
    }

    /// Should a top-level visit to this host be stopped by focus mode?
    pub fn focus_blocks(&self, host: &str) -> bool {
        if !self.focus || host.is_empty() {
            return false;
        }
        if self.focus_allow.iter().any(|s| host_matches(host, s)) {
            return false;
        }
        if !self.focus_only.is_empty() {
            return !self.focus_only.iter().any(|s| host_matches(host, s));
        }
        self.focus_block.iter().any(|s| host_matches(host, s))
    }
}

pub fn host_matches(host: &str, site: &str) -> bool {
    let site = site.trim().trim_start_matches("https://").trim_start_matches("http://").trim_start_matches("www.");
    let site = site.split('/').next().unwrap_or("").to_lowercase();
    if site.is_empty() {
        return false;
    }
    let host = host.trim_start_matches("www.");
    host == site || host.ends_with(&format!(".{}", site))
}

pub fn host(url: &str) -> String {
    let rest = url.split("://").nth(1).unwrap_or("");
    let h = rest.split(['/', '?', '#']).next().unwrap_or("");
    let h = h.rsplit('@').next().unwrap_or("");
    h.split(':').next().unwrap_or("").to_lowercase()
}

const TRACK: &[&str] = &[
    "utm_source", "utm_medium", "utm_campaign", "utm_term", "utm_content", "utm_id", "utm_name", "fbclid", "gclid", "dclid", "gbraid",
    "wbraid", "msclkid", "mc_eid", "mc_cid", "igshid", "yclid", "_hsenc", "_hsmi", "mkt_tok", "oly_anon_id", "oly_enc_id", "vero_id",
    "rb_clickid", "s_cid", "ref_src", "spm", "trk", "trkCampaign", "si",
];

/// The same address without tracking bits (utm_*, fbclid, gclid...). None when nothing changes.
pub fn clean_url(url: &str) -> Option<String> {
    let (base, frag) = match url.find('#') {
        Some(i) => (&url[..i], &url[i..]),
        None => (url, ""),
    };
    let (path, query) = base.split_once('?')?;
    let h = host(url);
    let keep: Vec<&str> = query
        .split('&')
        .filter(|kv| {
            let k = kv.split('=').next().unwrap_or("");
            // "si" is only tracking on YouTube/Spotify share links
            if k == "si" {
                return !(h.contains("youtu") || h.contains("spotify"));
            }
            !TRACK.contains(&k) && !k.starts_with("utm_")
        })
        .collect();
    if keep.len() == query.split('&').count() {
        return None;
    }
    Some(if keep.is_empty() { format!("{}{}", path, frag) } else { format!("{}?{}{}", path, keep.join("&"), frag) })
}

/// Guess the IB subject of a page from its title and address.
pub fn subject(title: &str, url: &str) -> &'static str {
    let t = format!("{} {}", title, url).to_lowercase();
    let has = |ws: &[&str]| ws.iter().any(|w| t.contains(w));
    if has(&["biolog", "/bio", "cell ", "enzyme", "photosynth", "genetic", "dna", "ecolog", "evolution"]) {
        "Biology"
    } else if has(&["chemi", "/chem", "organic", "stoichiom", "molar", "periodic table", "redox", "titration"]) {
        "Chemistry"
    } else if has(&["math", "calculus", "algebra", "trigonometr", "statistic", "integral", "derivative", "desmos", "geogebra"]) {
        "Maths"
    } else if has(&["french", "français", "francais", "wordreference", "conjug", "/fr/", "le monde", "tv5monde"]) {
        "French"
    } else if has(&["psycholog", "/psych", "cognitive", "sociocultural", "neuro"]) {
        "Psychology"
    } else if has(&["english", "literature", "poem", "poetry", "novel", "shakespeare", "language and lit", "lang & lit"]) {
        "English"
    } else if has(&["theory of knowledge", " tok ", "/tok", "extended essay", " ee ", "internal assessment", " ia "]) {
        "Core"
    } else {
        ""
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cleans() {
        assert_eq!(clean_url("https://a.com/x?utm_source=b&id=3#t").as_deref(), Some("https://a.com/x?id=3#t"));
        assert_eq!(clean_url("https://a.com/x?id=3"), None);
        assert_eq!(clean_url("https://youtu.be/x?si=abc").as_deref(), Some("https://youtu.be/x"));
        let g = Guard::new();
        assert!(g.is_ad("securepubads.g.doubleclick.net"));
        assert!(!g.is_ad("youtube.com"));
        assert_eq!(host("https://user@www.YouTube.com:443/watch?v=1"), "www.youtube.com");
        assert!(host_matches("m.youtube.com", "youtube.com"));
        assert_eq!(subject("Photosynthesis - InThinking", ""), "Biology");
    }
}
