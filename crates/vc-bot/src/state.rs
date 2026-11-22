//! Shared runtime state between the HTTP ingest and the Discord worker.

use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::sync::{mpsc, oneshot};
use vc_core::{Engine, Layout, RoomPlanner, Snapshot, Update};

use crate::config::Config;

#[derive(Debug, Clone)]
pub struct CheckReply {
    pub status: &'static str,
    pub channel: Option<String>,
}

pub enum BotCommand {
    Apply(Layout),
    Lookup {
        player: String,
        ack: oneshot::Sender<CheckReply>,
    },
    Unmute {
        player: String,
        ack: oneshot::Sender<bool>,
    },
}

pub struct AppState {
    pub cfg: Arc<Config>,
    pub engine: Mutex<Engine>,
    pub planner: Mutex<RoomPlanner>,
    pub tx: mpsc::UnboundedSender<BotCommand>,
    pub started: Instant,
    pub seq: AtomicU64,
    pub bot_online: AtomicBool,
    pub last_update: Mutex<Option<Update>>,
    /// รหัส OTP สำหรับจับคู่ระหว่าง Discord และ เกม (code, expires_at)
    pub pair_code: Mutex<Option<(String, Instant)>>,
    /// Layouts handed to the worker but not committed yet.
    in_flight: AtomicUsize,
}

impl AppState {
    pub fn new(cfg: Arc<Config>, tx: mpsc::UnboundedSender<BotCommand>) -> Arc<Self> {
        let engine = Engine::new(vc_core::EngineConfig {
            enter_streak: cfg.enter_streak,
            exit_streak: cfg.exit_streak,
            radius_mode: cfg.radius_mode,
            require_verified: cfg.require_verified,
            hard_cap_distance: 48.0,
        });
        let planner = RoomPlanner::new(vc_core::PlannerConfig {
            zone_prefix: cfg.zone_prefix.clone(),
            max_zones: cfg.zone_limit,
            solo_grace_plans: cfg.solo_grace_plans,
        });

        Arc::new(Self {
            cfg,
            engine: Mutex::new(engine),
            planner: Mutex::new(planner),
            tx,
            started: Instant::now(),
            seq: AtomicU64::new(0),
            bot_online: AtomicBool::new(false),
            last_update: Mutex::new(None),
            pair_code: Mutex::new(None),
            in_flight: AtomicUsize::new(0),
        })
    }

    /// สุ่มรหัส OTP 6 หลักสำหรับจับคู่ (อายุ 5 นาที)
    pub fn generate_pair_code(&self) -> String {
        use std::time::{Duration, SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let code = format!("{:06}", (nanos % 900000 + 100000) as u32);
        let expires = Instant::now() + Duration::from_secs(300);

        *self.pair_code.lock().unwrap_or_else(|e| e.into_inner()) = Some((code.clone(), expires));
        tracing::info!(code = %code, "สร้างรหัสจับคู่ Pairing Code สำเร็จ (อายุ 5 นาที)");
        code
    }

    /// Fold a positional snapshot in and queue the resulting room plan.
    pub fn ingest(&self, snapshot: &Snapshot) -> (Update, Layout) {
        let update = {
            let mut engine = self.engine.lock().unwrap();
            engine.apply_snapshot(snapshot)
        };
        let layout = self.plan(&update.groups);
        (update, layout)
    }

    /// Work out the next layout and hand it to the Discord worker.
    ///
    /// While a previous layout is still being applied the planner has not been
    /// committed, so planning again would hand out a second `New` slot for the
    /// same group and Discord ends up with duplicate rooms.
    pub fn plan(&self, groups: &[vc_core::Group]) -> Layout {
        if self.in_flight.load(Ordering::SeqCst) > 0 {
            return Layout::default();
        }
        let layout = {
            let mut planner = self.planner.lock().unwrap();
            planner.plan(groups)
        };
        self.dispatch(layout.clone());
        layout
    }

    fn dispatch(&self, layout: Layout) {
        self.in_flight.fetch_add(1, Ordering::SeqCst);
        if self.tx.send(BotCommand::Apply(layout)).is_err() {
            self.in_flight.fetch_sub(1, Ordering::SeqCst);
        }
    }    /// Called by the worker once a layout has been applied (success or not).
    pub fn finish_job(&self) {
        let mut current = self.in_flight.load(Ordering::SeqCst);
        while current > 0 {
            match self.in_flight.compare_exchange(
                current,
                current - 1,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => break,
                Err(actual) => current = actual,
            }
        }
    }

    #[cfg(test)]
    pub fn in_flight(&self) -> usize {
        self.in_flight.load(Ordering::SeqCst)
    }

    pub fn explicit_connect(&self, a: &str, b: &str) -> vc_core::Update {
        let update = {
            let mut engine = self.engine.lock().unwrap();
            engine.explicit_connect(a, b);
            engine.groups()
        };
        self.plan(&update.groups);
        update
    }

    pub fn explicit_disconnect(&self, a: &str, b: &str) -> vc_core::Update {
        let update = {
            let mut engine = self.engine.lock().unwrap();
            engine.explicit_disconnect(a, b);
            engine.groups()
        };
        self.plan(&update.groups);
        update
    }

    pub fn force_lobby(&self, player: &str) {
        let room = {
            let mut planner = self.planner.lock().unwrap();
            planner.detach_player(&vc_core::normalize(player))
        };
        if room.is_none() {
            return;
        }
        let layout = Layout {
            rooms: Vec::new(),
            close: Vec::new(),
            to_lobby: vec![vc_core::normalize(player)],
        };
        self.dispatch(layout);
    }

    /// Clear the whole graph and ask the worker to evacuate every zone channel.
    pub fn hard_reset(&self) -> usize {
        {
            let mut engine = self.engine.lock().unwrap();
            engine.clear();
        }
        let (close, players) = {
            let mut planner = self.planner.lock().unwrap();
            planner.drain()
        };
        let rooms = close.len();
        let layout = Layout {
            rooms: Vec::new(),
            close,
            to_lobby: players,
        };
        self.dispatch(layout);
        rooms
    }

    pub fn reset_player(&self, player: &str) -> bool {
        let key = vc_core::normalize(player);
        {
            let mut engine = self.engine.lock().unwrap();
            engine.explicit_drop_player(&key);
        }
        let tracked = {
            let mut planner = self.planner.lock().unwrap();
            planner.detach_player(&key).is_some()
        };
        let layout = Layout {
            rooms: Vec::new(),
            close: Vec::new(),
            to_lobby: vec![key],
        };
        self.dispatch(layout);
        tracked
    }

    pub async fn lookup(&self, player: &str) -> CheckReply {
        let missing = CheckReply {
            status: "not_found",
            channel: None,
        };
        let (tx, rx) = oneshot::channel();
        if self
            .tx
            .send(BotCommand::Lookup {
                player: player.to_string(),
                ack: tx,
            })
            .is_err()
        {
            return missing;
        }
        match tokio::time::timeout(Duration::from_millis(1500), rx).await {
            Ok(Ok(reply)) => reply,
            _ => missing,
        }
    }

    pub async fn unmute(&self, player: &str) -> bool {
        let (tx, rx) = oneshot::channel();
        if self
            .tx
            .send(BotCommand::Unmute {
                player: player.to_string(),
                ack: tx,
            })
            .is_err()
        {
            return false;
        }
        match tokio::time::timeout(Duration::from_millis(3000), rx).await {
            Ok(Ok(success)) => success,
            _ => false,
        }
    }

    pub fn set_update(&self, update: Update) -> u64 {
        *self.last_update.lock().unwrap() = Some(update);
        self.seq.fetch_add(1, Ordering::Relaxed) + 1
    }

    pub fn uptime_secs(&self) -> u64 {
        self.started.elapsed().as_secs()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_cfg() -> Arc<Config> {
        Arc::new(Config {
            token: String::new(),
            guild_id: 1,
            lobby_channel_id: 2,
            category_id: None,
            admin_channel_id: None,
            api_secret: "test".into(),
            bind: "127.0.0.1:0".parse().unwrap(),
            zone_prefix: "room ".into(),
            zone_limit: 10,
            solo_grace_plans: 2,
            enter_streak: 1,
            exit_streak: 1,
            radius_mode: vc_core::RadiusMode::Max,
            require_verified: true,
            move_concurrency: 1,
        })
    }

    #[test]
    fn planner_stops_while_a_layout_is_in_flight() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let state = AppState::new(test_cfg(), tx);
        let group = vc_core::Group::new(vec!["a".into(), "b".into()]);

        let first = state.plan(std::slice::from_ref(&group));
        assert_eq!(first.rooms.len(), 1);
        assert_eq!(state.in_flight(), 1);
        assert!(matches!(rx.try_recv(), Ok(BotCommand::Apply(_))));

        // Worker is still busy: planning again would create a duplicate room.
        assert!(state.plan(std::slice::from_ref(&group)).is_empty());

        state.finish_job();
        assert_eq!(state.in_flight(), 0);
        assert_eq!(state.plan(std::slice::from_ref(&group)).rooms.len(), 1);
    }
}
