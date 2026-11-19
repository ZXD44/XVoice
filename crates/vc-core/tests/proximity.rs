use vc_core::model::PlayerSample;
use vc_core::planner::PlannerConfig;
use vc_core::{Engine, EngineConfig, Group, Layout, RadiusMode, RoomPlanner, Slot, Snapshot};

fn player(name: &str, x: f64, z: f64, radius: f64) -> PlayerSample {
    PlayerSample {
        name: name.to_string(),
        dimension: "minecraft:overworld".into(),
        x,
        y: 64.0,
        z,
        radius,
        enabled: true,
        verified: true,
        discord_id: None,
        radio_channel: 0,
    }
}

fn engine(enter: u8, exit: u8) -> Engine {
    Engine::new(EngineConfig {
        enter_streak: enter,
        exit_streak: exit,
        radius_mode: RadiusMode::Max,
        require_verified: true,
        hard_cap_distance: 48.0,
    })
}

fn tick(engine: &mut Engine, players: Vec<PlayerSample>) -> Vec<Group> {
    engine
        .apply_snapshot(&Snapshot {
            seq: 1,
            players,
        })
        .groups
}

fn groups_of(members: &[&str]) -> Vec<Group> {
    vec![Group::new(members.iter().map(|s| s.to_string()).collect())]
}

#[test]
fn names_are_normalized() {
    assert_eq!(vc_core::normalize("  Steve  "), "steve");
    assert_eq!(vc_core::normalize("Notch\tX"), "notch x");
    assert_eq!(vc_core::normalize("ÄRGER"), "ärger");
}

#[test]
fn neighbours_become_one_group() {
    let mut e = engine(1, 1);
    let groups = tick(
        &mut e,
        vec![player("Steve", 0.0, 0.0, 5.0), player("Alex", 3.0, 0.0, 5.0)],
    );
    assert_eq!(groups, groups_of(&["alex", "steve"]));
}

#[test]
fn distant_players_do_not_group() {
    let mut e = engine(1, 1);
    assert!(tick(
        &mut e,
        vec![player("Steve", 0.0, 0.0, 5.0), player("Alex", 30.0, 0.0, 5.0)],
    )
    .is_empty());
}

#[test]
fn hysteresis_holds_pair_until_exit_streak() {
    let mut e = engine(2, 3);
    let near = || vec![player("A", 0.0, 0.0, 5.0), player("B", 1.0, 0.0, 5.0)];
    let far = || vec![player("A", 0.0, 0.0, 5.0), player("B", 40.0, 0.0, 5.0)];

    assert!(tick(&mut e, near()).is_empty(), "one tick is not enough");
    assert_eq!(tick(&mut e, near()), groups_of(&["a", "b"]), "enter streak 2");

    // Going out of range keeps the pair for `exit_streak - 1` ticks.
    assert_eq!(tick(&mut e, far()), groups_of(&["a", "b"]));
    assert_eq!(tick(&mut e, far()), groups_of(&["a", "b"]));
    assert!(tick(&mut e, far()).is_empty(), "exit streak 3");
}

#[test]
fn flicker_at_the_edge_does_not_flap() {
    let mut e = engine(2, 2);
    let near = || vec![player("A", 0.0, 0.0, 5.0), player("B", 4.0, 0.0, 5.0)];
    let far = || vec![player("A", 0.0, 0.0, 5.0), player("B", 6.0, 0.0, 5.0)];

    assert!(tick(&mut e, near()).is_empty());
    assert!(tick(&mut e, far()).is_empty());
    assert!(tick(&mut e, near()).is_empty());
    assert_eq!(tick(&mut e, near()), groups_of(&["a", "b"]));
}

#[test]
fn chain_is_a_single_room() {
    let mut e = engine(1, 1);
    let groups = tick(
        &mut e,
        vec![
            player("A", 0.0, 0.0, 5.0),
            player("B", 4.0, 0.0, 5.0),
            player("C", 8.0, 0.0, 5.0),
        ],
    );
    assert_eq!(groups, groups_of(&["a", "b", "c"]));
}

#[test]
fn different_dimensions_never_mix() {
    let mut e = engine(1, 1);
    let mut nether = player("B", 0.0, 0.0, 5.0);
    nether.dimension = "minecraft:nether".into();
    assert!(tick(&mut e, vec![player("A", 0.0, 0.0, 5.0), nether]).is_empty());
}

#[test]
fn mute_or_unverified_players_are_excluded() {
    let mut e = engine(1, 1);
    let mut silent = player("B", 0.0, 0.0, 5.0);
    silent.enabled = false;
    assert!(tick(&mut e, vec![player("A", 0.0, 0.0, 5.0), silent]).is_empty());

    let mut unverified = player("C", 0.0, 0.0, 5.0);
    unverified.verified = false;
    assert!(tick(
        &mut e,
        vec![player("A", 0.0, 0.0, 5.0), unverified],
    )
    .is_empty());
}

#[test]
fn radius_mode_min_is_strict() {
    let mut e = Engine::new(EngineConfig {
        enter_streak: 1,
        exit_streak: 1,
        radius_mode: RadiusMode::Min,
        require_verified: false,
        hard_cap_distance: 48.0,
    });
    // 8 blocks apart: the wide radius reaches, the narrow one does not.
    let groups = tick(
        &mut e,
        vec![player("A", 0.0, 0.0, 10.0), player("B", 8.0, 0.0, 5.0)],
    );
    assert!(groups.is_empty());

    let groups = tick(
        &mut e,
        vec![player("A", 0.0, 0.0, 10.0), player("B", 4.0, 0.0, 10.0)],
    );
    assert_eq!(groups, groups_of(&["a", "b"]));
}

#[test]
fn absurd_radius_is_rejected() {
    let mut e = engine(1, 1);
    let groups = tick(
        &mut e,
        vec![player("A", 0.0, 0.0, 500.0), player("B", 1.0, 0.0, 500.0)],
    );
    assert!(groups.is_empty());
}

#[test]
fn leaving_player_drops_out_of_group() {
    let mut e = engine(1, 1);
    assert_eq!(
        tick(
            &mut e,
            vec![player("A", 0.0, 0.0, 5.0), player("B", 1.0, 0.0, 5.0)]
        ),
        groups_of(&["a", "b"])
    );
    assert!(tick(&mut e, vec![player("A", 0.0, 0.0, 5.0)]).is_empty());
}

#[test]
fn legacy_pairs_still_work() {
    let mut e = engine(5, 5);
    e.explicit_connect("Steve", "Alex");
    assert_eq!(e.groups().groups, groups_of(&["alex", "steve"]));
    e.explicit_disconnect("alex", "steve");
    assert!(e.groups().groups.is_empty());
    e.explicit_connect("Steve", "Alex");
    e.explicit_drop_player("ALEX");
    assert!(e.groups().groups.is_empty());
}

// ── planner ───────────────────────────────────────────────────────────────

fn planner() -> RoomPlanner {
    RoomPlanner::new(PlannerConfig {
        zone_prefix: "ห้องที่ ".to_string(),
        max_zones: 100,
        solo_grace_plans: 2,
    })
}

fn resolve(planner: &mut RoomPlanner, layout: &mut Layout, next_id: &mut u64) {
    for room in &mut layout.rooms {
        if let Slot::New { name } = &room.slot {
            let id = *next_id;
            *next_id += 1;
            planner.register_room(id, name);
            room.slot = Slot::Keep(id);
        }
    }
    planner.commit(layout);
}

#[test]
fn new_group_creates_a_room_and_moves_members() {
    let mut p = planner();
    let mut next = 100;
    let mut layout = p.plan(&groups_of(&["a", "b"]));
    match &layout.rooms[0].slot {
        Slot::New { name } => assert_eq!(name, "ห้องที่ 1"),
        other => panic!("expected new room, got {other:?}"),
    }
    resolve(&mut p, &mut layout, &mut next);
    assert_eq!(p.room_of("a"), Some(100));
    assert_eq!(p.room_of("b"), Some(100));
    assert_eq!(p.room_count(), 1);
}

#[test]
fn surviving_group_keeps_its_room() {
    let mut p = planner();
    let mut next = 100;
    let mut layout = p.plan(&groups_of(&["a", "b"]));
    resolve(&mut p, &mut layout, &mut next);

    let mut layout = p.plan(&groups_of(&["a", "b"]));
    assert_eq!(layout.rooms[0].slot, Slot::Keep(100));
    assert!(layout.close.is_empty());
    resolve(&mut p, &mut layout, &mut next);
    assert_eq!(p.room_of("a"), Some(100));
}

#[test]
fn newcomer_joins_the_existing_room_instead_of_creating_one() {
    let mut p = planner();
    let mut next = 100;
    let mut layout = p.plan(&groups_of(&["a", "b"]));
    resolve(&mut p, &mut layout, &mut next);

    let mut layout = p.plan(&groups_of(&["a", "b", "c"]));
    assert_eq!(layout.rooms[0].slot, Slot::Keep(100));
    assert_eq!(layout.rooms.len(), 1);
    resolve(&mut p, &mut layout, &mut next);
    assert_eq!(p.room_of("c"), Some(100));
}

#[test]
fn split_creates_second_room_and_closes_emptied_one() {
    let mut p = planner();
    let mut next = 100;
    let mut layout = p.plan(&groups_of(&["a", "b", "c"]));
    resolve(&mut p, &mut layout, &mut next);

    let mut layout = p.plan(&[Group::new(vec!["a".into(), "b".into()]), Group::new(vec!["c".into(), "d".into()])]);
    assert_eq!(layout.rooms.len(), 2);
    assert_eq!(layout.rooms[0].slot, Slot::Keep(100));
    assert!(matches!(layout.rooms[1].slot, Slot::New { .. }));
    resolve(&mut p, &mut layout, &mut next);
    assert_eq!(p.room_of("d"), Some(101));
    assert_eq!(p.room_count(), 2);
}

#[test]
fn solo_player_gets_grace_then_lobby() {
    let mut p = planner();
    let mut next = 100;
    let mut layout = p.plan(&groups_of(&["a", "b"]));
    resolve(&mut p, &mut layout, &mut next);

    // "b" walks away: first plans keep the room, then the lobby return fires.
    let mut layout = p.plan(&groups_of(&["a"]));
    assert!(layout.to_lobby.is_empty());
    resolve(&mut p, &mut layout, &mut next);

    let mut layout = p.plan(&groups_of(&["a"]));
    assert!(layout.to_lobby.is_empty());
    resolve(&mut p, &mut layout, &mut next);

    let mut layout = p.plan(&groups_of(&["a"]));
    assert_eq!(layout.to_lobby, vec!["b".to_string()]);
    resolve(&mut p, &mut layout, &mut next);
    assert_eq!(p.room_of("b"), None);
}

#[test]
fn empty_room_is_closed() {
    let mut p = planner();
    let mut next = 100;
    let mut layout = p.plan(&groups_of(&["a", "b"]));
    resolve(&mut p, &mut layout, &mut next);

    // Both leave the zone entirely.
    let mut layout = p.plan(&[]);
    resolve(&mut p, &mut layout, &mut next);
    for _ in 0..3 {
        let mut layout = p.plan(&[]);
        if !layout.close.is_empty() {
            resolve(&mut p, &mut layout, &mut next);
        }
    }
    assert_eq!(p.room_count(), 0);
}

#[test]
fn adopted_room_with_members_is_reused() {
    // A restart: the channel already exists and we know who sits in it, so the
    // planner must keep it instead of spawning a twin.
    let mut p = planner();
    p.register_room(777, "ห้องที่ 3");
    p.sync_members(777, vec!["a".to_string(), "b".to_string()]);

    let layout = p.plan(&groups_of(&["a", "b"]));
    assert_eq!(layout.rooms[0].slot, Slot::Keep(777));
    assert!(layout.close.is_empty());
}

#[test]
fn room_names_are_unique() {
    let mut p = planner();
    p.register_room(1, "ห้องที่ 1");
    p.register_room(2, "ห้องที่ 2");
    let layout = p.plan(&groups_of(&["a", "b"]));
    match &layout.rooms[0].slot {
        Slot::New { name } => assert_eq!(name, "ห้องที่ 3"),
        other => panic!("expected new room, got {other:?}"),
    }
}
