//! Bounded matching attributes. No credentials or network descriptors exported.
use deathloop_invader_tool::GameProcess;
use serde_json::{Value, json};

#[derive(Clone, Debug, PartialEq)]
pub struct Participant {
    pub service_id: String,
    pub rating: Option<i64>,
    pub reputation: Option<f64>,
    pub environment: Option<String>,
    pub account: Option<String>,
    pub platforms: Value,
    pub inputs: Value,
    pub regions: Value,
    pub build: Option<String>,
}
impl Participant {
    fn parse(service_id: String, v: &Value, host: bool) -> Self {
        let text = |k: &str| {
            v.get(k)
                .and_then(Value::as_str)
                .filter(|s| s.len() <= 256 && !s.chars().any(char::is_control))
                .map(str::to_owned)
        };
        Self {
            service_id,
            rating: v
                .get("mm_rating")
                .and_then(Value::as_i64)
                .filter(|n| *n != i32::MIN as i64),
            reputation: v
                .get("mm_reputation")
                .and_then(Value::as_f64)
                .filter(|v| v.is_finite()),
            environment: text("mm_environment"),
            account: text("fp_platform_id"),
            platforms: clean_list(v.get(if host {
                "mm_host_platform"
            } else {
                "mm_client_platform"
            })),
            inputs: clean_list(v.get(if host {
                "mm_host_input"
            } else {
                "mm_client_input"
            })),
            regions: clean_regions(v.get("mm_regions")),
            build: text("mm_build_version"),
        }
    }
    pub fn data(&self) -> Value {
        json!({"service_id":self.service_id,"rating":self.rating,"reputation":self.reputation,
            "environment":self.environment,"platform_account_id":self.account,
            "platforms":self.platforms,"inputs":self.inputs,"advertised_regions":self.regions,"build":self.build})
    }
    pub fn region_display(&self) -> String {
        let Some(regions) = self.regions.as_array() else {
            return "Advertised region: unavailable".into();
        };
        if regions.is_empty() {
            return "Advertised region: unavailable".into();
        }
        // Julianna can advertise many regions. Show the first and make that explicit.
        let first = &regions[0];
        let region = first["region"].as_str().unwrap_or("unknown");
        let latency = first["latency"]
            .as_u64()
            .map(|n| format!(" | {n} ms to region"))
            .unwrap_or_default();
        let more = if regions.len() > 1 {
            format!(" (+{} regions)", regions.len() - 1)
        } else {
            String::new()
        };
        format!("Advertised region: {region}{more}{latency}")
    }
    pub fn score_display(&self) -> String {
        format!(
            "Rating: {} | Reputation: {}",
            self.rating.map(|v| v.to_string()).unwrap_or("N/A".into()),
            self.reputation
                .map(|v| format!("{v:.6}"))
                .unwrap_or("N/A".into())
        )
    }
}
fn clean_list(v: Option<&Value>) -> Value {
    Value::Array(
        v.and_then(Value::as_array)
            .into_iter()
            .flatten()
            .take(16)
            .filter_map(|v| v.as_str())
            .filter(|s| s.len() <= 64 && !s.chars().any(char::is_control))
            .map(|s| json!(s))
            .collect(),
    )
}
fn clean_regions(v: Option<&Value>) -> Value {
    Value::Array(
        v.and_then(Value::as_array)
            .into_iter()
            .flatten()
            .take(32)
            .filter_map(|v| {
                let region = v.get("region")?.as_str()?;
                if region.len() > 64 || region.chars().any(char::is_control) {
                    return None;
                }
                Some(json!({"region":region,"latency":v.get("latency").and_then(Value::as_u64)}))
            })
            .collect(),
    )
}
#[derive(Clone, Debug)]
pub struct Pair {
    pub key: String,
    pub local_role: &'static str,
    pub local: Participant,
    pub opponent: Participant,
    pub opponent_name: Option<String>,
    pub host_day: Option<i32>,
    pub state: Value,
    pub correlated: bool,
}
pub fn idstr(g: &GameProcess, a: u64, max: usize) -> Result<String, String> {
    let pointer = g.read_memory::<u64>(a)?;
    let count = (g.read_memory::<u32>(a + 8)? & 0x1FFFFFFF) as usize;
    if count == 0 {
        return Ok(String::new());
    }
    if pointer < 0x10000 || count > max {
        return Err("Invalid matching string".into());
    }
    let bytes = g.read_bytes(pointer, count + 1)?;
    if bytes[count] != 0 {
        return Err("Unterminated matching string".into());
    }
    let text =
        String::from_utf8(bytes[..count].to_vec()).map_err(|_| "Invalid UTF-8 matching string")?;
    if text.len() != count
        || g.read_memory::<u64>(a)? != pointer
        || g.read_memory::<u32>(a + 8)? & 0x1FFFFFFF != count as u32
    {
        return Err("Matching string changed".into());
    }
    Ok(text)
}
pub fn read(g: &GameProcess) -> Result<Pair, String> {
    let session = g.base_address + 0x332F2D0;
    let lobby = session + 0x3DB8;
    let peers = g.read_memory::<u32>(lobby + 0x25D4)?;
    let peer_array = g.read_memory::<u64>(lobby + 0x25C8)?;
    if peers != 1 || peer_array < 0x10000 {
        return Err("No single active opponent".into());
    }
    let connection = g.read_memory::<u32>(peer_array + 4)?;
    if !(1..=2).contains(&connection) {
        return Err("Peer inactive".into());
    }
    let is_host = g.read_memory::<u8>(lobby + 0x330)? != 0;
    let profile_root = g.read_memory::<u64>(g.base_address + 0x333A150)?;
    let profile = g.read_memory::<u64>(profile_root)?;
    if profile < 0x10000 {
        return Err("Profile unavailable".into());
    }
    let mm = profile + 0x4028;
    let array = g.read_memory::<u64>(mm + 0xB0)?;
    let count = g.read_memory::<u32>(mm + 0xBC)?;
    if count != 2 || array < 0x10000 {
        return Err("Matching pair unavailable".into());
    }
    let mut local = None;
    let mut opponent = None;
    for i in 0..count {
        let record = array + u64::from(i) * 0x78;
        let v: Value = serde_json::from_str(&idstr(g, record + 0x50, 16384)?)
            .map_err(|_| "Invalid matching JSON")?;
        let host = v
            .get("mm_is_host")
            .and_then(Value::as_i64)
            .ok_or("Missing matching role")?;
        if !(0..=1).contains(&host) {
            return Err("Invalid matching role".into());
        }
        let participant = Participant::parse(idstr(g, record + 0x28, 128)?, &v, host == 1);
        let target = if (host == 1) == is_host {
            &mut local
        } else {
            &mut opponent
        };
        if target.replace(participant).is_some() {
            return Err("Ambiguous matching pair".into());
        }
    }
    let local = local.ok_or("Missing local attributes")?;
    let opponent = opponent.ok_or("Missing opponent attributes")?;
    let snapshot = g.opponent().ok();
    let (opponent_name, host_day) = snapshot
        .map(|s| (s.name, s.host_day))
        .unwrap_or((None, None));
    // Retained matching attributes must correspond to the current roster once
    // the roster is present. Never display an old candidate beside a new name.
    let users = g.read_memory::<u64>(lobby + 0x348)?;
    let user_count = g.read_memory::<u32>(lobby + 0x354)?;
    if user_count > 4 {
        return Err("Roster changing".into());
    }
    if user_count >= 2 {
        if users < 0x10000 {
            return Err("Roster unavailable".into());
        }
        let mut correlated = false;
        for i in 0..user_count {
            let user = g.read_memory::<u64>(users + u64::from(i) * 8)?;
            if user < 0x10000 {
                continue;
            }
            if idstr(g, user + 0x28, 128)? == opponent.service_id {
                if let Some(name) = &opponent_name {
                    if idstr(g, user + 0x6A8, 128)? != *name {
                        return Err("Opponent name changed".into());
                    }
                }
                correlated = true;
            }
        }
        if !correlated {
            return Err("Matching attributes not correlated".into());
        }
    }
    if g.read_memory::<u64>(mm + 0xB0)? != array
        || g.read_memory::<u32>(mm + 0xBC)? != count
        || g.read_memory::<u64>(profile_root)? != profile
        || g.read_memory::<u64>(lobby + 0x25C8)? != peer_array
        || g.read_memory::<u32>(lobby + 0x25D4)? != peers
        || g.read_memory::<u8>(lobby + 0x330)? != u8::from(is_host)
        || g.read_memory::<u32>(peer_array + 4)? != connection
        || g.read_memory::<u64>(lobby + 0x348)? != users
        || g.read_memory::<u32>(lobby + 0x354)? != user_count
    {
        return Err("Session changing".into());
    }
    let state = json!({"controller":g.read_memory::<u32>(mm+0xC0)?,
        "aux":g.read_memory::<u32>(profile+0x41D8)?,"session":g.read_memory::<u32>(session+0x3CE0)?,
        "connection":connection,"peer_gameplay":g.read_memory::<u32>(peer_array+8)?});
    Ok(Pair {
        key: format!(
            "{}:{}:{}:{}",
            g.pid,
            peer_array,
            g.read_memory::<u32>(lobby + 0x338)?,
            opponent.service_id
        ),
        local_role: if is_host { "Colt" } else { "Julianna" },
        local,
        opponent,
        opponent_name,
        host_day,
        state,
        correlated: user_count >= 2,
    })
}
pub struct Cache {
    pub pair: Option<Pair>,
    next: std::time::Instant,
}
impl Default for Cache {
    fn default() -> Self {
        Self {
            pair: None,
            next: std::time::Instant::now(),
        }
    }
}
impl Cache {
    pub fn update(&mut self, g: Option<&GameProcess>) -> bool {
        if g.is_none() {
            self.pair = None;
            return false;
        }
        if std::time::Instant::now() < self.next {
            return false;
        }
        self.next = std::time::Instant::now() + std::time::Duration::from_secs(1);
        self.pair = g.and_then(|g| read(g).ok());
        true
    }
    pub fn region(&self) -> String {
        self.pair
            .as_ref()
            .filter(|p| p.correlated)
            .map(|p| p.opponent.region_display())
            .unwrap_or_else(|| "Advertised region: unavailable".into())
    }
    pub fn scores(&self) -> String {
        self.pair
            .as_ref()
            .filter(|p| p.correlated)
            .map(|p| p.opponent.score_display())
            .unwrap_or_else(|| "Rating / Reputation: waiting for opponent".into())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scores_regions_and_no_secrets() {
        let p = Participant::parse(
            "service".into(),
            &json!({"mm_rating":1973,"mm_reputation":4.291172981262207,
            "mm_regions":[{"region":"WestEurope","latency":17},{"region":"EastUs","latency":140}],
            "pf_net_desc":"SECRET","EntityToken":"SECRET"}),
            true,
        );
        assert_eq!(p.score_display(), "Rating: 1973 | Reputation: 4.291173");
        assert!(p.region_display().contains("(+1 regions)"));
        assert!(!p.data().to_string().contains("SECRET"));
    }
    #[test]
    fn missing_scores_not_zero() {
        let p = Participant::parse("service".into(), &json!({"mm_rating":-2147483648}), false);
        assert_eq!(p.score_display(), "Rating: N/A | Reputation: N/A");
        assert_eq!(p.region_display(), "Advertised region: unavailable");
    }
}
