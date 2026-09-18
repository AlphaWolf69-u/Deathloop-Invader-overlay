//! Game peer counters, not adapter-wide traffic or an Internet speed test.
use deathloop_invader_tool::GameProcess;
use std::{collections::VecDeque, time::Instant};

#[derive(Debug, Clone)]
pub struct Sample {
    key: (u32, u64, u64, u32),
    tx: u32,
    rx: u32,
    packets: u32,
    ping: Option<i32>,
    reply: u64,
    loss: Option<f32>,
}

// The client host peer remains in gameplay state 3 during a live invasion.
// Native +C67F90 accepts the indexed host by connection state alone.
fn select_peer(host: bool, host_index: i32, states: &[(u32, u32)]) -> Result<usize, String> {
    if !host {
        let index = usize::try_from(host_index).map_err(|_| "Waiting for game peer")?;
        return match states.get(index) {
            Some((2, _)) => Ok(index),
            _ => Err("Waiting for game peer".into()),
        };
    }
    let mut active = states
        .iter()
        .enumerate()
        .filter(|(_, (connection, gameplay))| *connection == 2 && *gameplay >= 6);
    let index = active
        .next()
        .map(|(i, _)| i)
        .ok_or("Waiting for game peer")?;
    if active.next().is_some() {
        return Err("Multiple active peers".into());
    }
    Ok(index)
}

pub fn read(g: &GameProcess) -> Result<Sample, String> {
    let root = g.read_memory::<u64>(g.base_address + 0x5BD1010)?;
    if g.read_memory::<u64>(root)? != g.base_address + 0x2613250 {
        return Err("Unsupported game".into());
    }
    let lobby = g.base_address + 0x3333088;
    let role = g.read_memory::<u8>(root + 0x6A730)?;
    if role > 1 {
        return Err("Invalid host/client flag".into());
    }
    let host_index_address = g.base_address + 0x332F2D0 + 0x3E98;
    let host_index = g.read_memory::<i32>(host_index_address)?;
    let list = g.read_memory::<u64>(lobby + 0x25C8)?;
    let count = g.read_memory::<u32>(lobby + 0x25D4)?;
    if count > 16 || (count > 0 && list < 0x10000) {
        return Err("Peer list unavailable".into());
    }
    let mut states = Vec::with_capacity(count as usize);
    for i in 0..count {
        let p = list + u64::from(i) * 0x3E0;
        states.push((g.read_memory::<u32>(p + 4)?, g.read_memory::<u32>(p + 8)?));
    }
    let index = select_peer(role == 1, host_index, &states)?;
    let p = list + index as u64 * 0x3E0;
    let processor = g.read_memory::<u64>(p + 0x278)?;
    if processor < 0x10000 {
        return Err("Peer counters unavailable".into());
    }
    let ping = g.read_memory::<i32>(p + 0x328)?;
    let loss = g.read_memory::<f32>(processor + 0x101870)?;
    let sample = Sample {
        key: (g.pid, p, processor, g.read_memory::<u32>(p)?),
        tx: g.read_memory::<u32>(processor + 0x1017F8)?,
        rx: g.read_memory::<u32>(processor + 0x1017FC)?,
        packets: g.read_memory::<u32>(processor + 0x10181C)?,
        ping: (0..=60000).contains(&ping).then_some(ping),
        reply: g.read_memory::<u64>(p + 0x2E8)?,
        loss: (g.read_memory::<u32>(processor + 0x101868)? > 0
            && loss.is_finite()
            && (0.0..=1.0).contains(&loss))
        .then_some(loss * 100.0),
    };
    if g.read_memory::<u64>(lobby + 0x25C8)? != list
        || g.read_memory::<u32>(lobby + 0x25D4)? != count
        || g.read_memory::<u64>(p + 0x278)? != processor
        || g.read_memory::<u32>(p + 4)? != 2
        || g.read_memory::<u32>(p + 8)? != states[index].1
        || g.read_memory::<u32>(p)? != sample.key.3
        || g.read_memory::<u8>(root + 0x6A730)? != role
        || (role == 0 && g.read_memory::<i32>(host_index_address)? != host_index)
        || g.read_memory::<u64>(g.base_address + 0x5BD1010)? != root
    {
        return Err("Peer changing".into());
    }
    Ok(sample)
}

#[derive(Default)]
pub struct Monitor {
    previous: Option<(Sample, Instant)>,
    last_rx: Option<Instant>,
    ping: Option<(i32, Instant)>,
    variation: VecDeque<f64>,
    rates: Option<(f64, f64, f64)>,
    observed: Option<Sample>,
}
fn delta(new: u32, old: u32, seconds: f64, limit: f64) -> Option<f64> {
    let rate = f64::from(new.wrapping_sub(old)) / seconds;
    (rate.is_finite() && rate <= limit).then_some(rate)
}
impl Monitor {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
    pub fn display(&mut self, g: &GameProcess) -> String {
        match read(g) {
            Ok(s) => self.accept(s, Instant::now()),
            Err(e) => {
                self.reset();
                format!("Network: {e}")
            }
        }
    }
    fn accept(&mut self, s: Sample, now: Instant) -> String {
        if self.previous.as_ref().is_some_and(|(p, _)| p.key != s.key) {
            self.reset();
        }
        if let Some((old, time)) = &self.previous {
            let dt = now.duration_since(*time).as_secs_f64();
            if dt >= 1.0 {
                self.rates = delta(s.tx, old.tx, dt, 100_000_000.0)
                    .zip(delta(s.rx, old.rx, dt, 100_000_000.0))
                    .zip(delta(s.packets, old.packets, dt, 100_000.0))
                    .map(|((tx, rx), packets)| (tx / 1024., rx / 1024., packets));
            }
        }
        if let Some(old) = &self.observed {
            if s.rx != old.rx {
                self.last_rx = Some(now);
            }
            if let Some(ping) = s.ping
                && (s.reply != old.reply || s.ping != old.ping)
            {
                if let Some((last, _)) = self.ping {
                    self.variation.push_back(f64::from((ping - last).abs()));
                    if self.variation.len() > 20 {
                        self.variation.pop_front();
                    }
                }
                self.ping = Some((ping, now));
            }
        } else {
            self.last_rx = Some(now);
            self.ping = s.ping.map(|p| (p, now));
        }
        let ping = self
            .ping
            .filter(|(_, t)| now.duration_since(*t).as_secs() < 10)
            .map(|(p, _)| format!("{p} ms"))
            .unwrap_or("N/A".into());
        let jitter = if self.variation.is_empty() || ping == "N/A" {
            "N/A".into()
        } else {
            format!(
                "{:.1} ms",
                self.variation.iter().sum::<f64>() / self.variation.len() as f64
            )
        };
        let loss = s.loss.map(|v| format!("{v:.1}%")).unwrap_or("N/A".into());
        let traffic = self
            .rates
            .map(|(tx, rx, pps)| format!("RX {rx:.1} / TX {tx:.1} KiB/s | TX {pps:.0} packets/s"))
            .unwrap_or("Traffic: sampling...".into());
        let gap = self
            .last_rx
            .map(|t| now.duration_since(t).as_secs_f64())
            .unwrap_or(0.);
        self.observed = Some(s.clone());
        if self
            .previous
            .as_ref()
            .is_none_or(|(_, t)| now.duration_since(*t).as_secs_f64() >= 1.)
        {
            self.previous = Some((s, now));
        }
        format!(
            "Ping: {ping} | RTT variation: {jitter} | RX loss: {loss}\n{traffic} | RX idle: {gap:.1}s"
        )
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invader_uses_connected_indexed_host_in_state_three() {
        assert_eq!(select_peer(false, 0, &[(2, 3)]), Ok(0));
        assert_eq!(select_peer(false, 1, &[(2, 6), (2, 3)]), Ok(1));
    }
    #[test]
    fn invalid_or_disconnected_host_is_not_replaced_with_another_peer() {
        for index in [-1, 1, i32::MAX] {
            assert!(select_peer(false, index, &[(2, 3)]).is_err());
        }
        assert!(select_peer(false, 0, &[]).is_err());
        assert!(select_peer(false, 0, &[(0, 3), (2, 6)]).is_err());
    }
    #[test]
    fn host_keeps_gameplay_filter_and_rejects_ambiguity() {
        assert!(select_peer(true, 0, &[(2, 3)]).is_err());
        assert_eq!(select_peer(true, -1, &[(2, 3), (2, 6)]), Ok(1));
        assert!(select_peer(true, -1, &[(2, 6), (2, 6)]).is_err());
        assert!(select_peer(true, -1, &[(0, 6)]).is_err());
    }
    #[test]
    fn wrap_and_reset() {
        assert_eq!(delta(4, u32::MAX - 5, 1., 100.), Some(10.));
        assert_eq!(delta(2, 100, 1., 100.), None);
    }
    #[test]
    fn peer_change_clears_rates() {
        let mut m = Monitor::default();
        let t = Instant::now();
        let mut s = Sample {
            key: (1, 2, 3, 4),
            tx: 0,
            rx: 0,
            packets: 0,
            ping: Some(40),
            reply: 0,
            loss: Some(0.),
        };
        m.accept(s.clone(), t);
        s.tx = 1024;
        s.rx = 2048;
        s.packets = 10;
        assert!(
            m.accept(s.clone(), t + std::time::Duration::from_secs(1))
                .contains("RX 2.0 / TX 1.0")
        );
        s.key.0 = 2;
        assert!(
            m.accept(s, t + std::time::Duration::from_secs(2))
                .contains("sampling")
        );
    }
    #[test]
    fn unchanged_poll_is_not_a_new_ping_sample() {
        let mut m = Monitor::default();
        let t = Instant::now();
        let mut s = Sample {
            key: (1, 2, 3, 4),
            tx: 0,
            rx: 0,
            packets: 0,
            ping: Some(40),
            reply: 0,
            loss: None,
        };
        m.accept(s.clone(), t);
        s.ping = Some(60);
        s.reply = 1;
        m.accept(s.clone(), t + std::time::Duration::from_millis(250));
        m.accept(s.clone(), t + std::time::Duration::from_millis(500));
        assert_eq!(m.variation.len(), 1);
        assert!(
            m.accept(s, t + std::time::Duration::from_secs(12))
                .contains("Ping: N/A")
        );
    }
}
