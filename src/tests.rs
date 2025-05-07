#[cfg(test)]
#[cfg(all(target_os = "wasi", target_env = "p1"))]
mod integration {
    use crate::scheduler::from_scheduler::Response;
    use crate::scheduler::response_correct::CardToView::NoCard;
    use crate::scheduler::{FromScheduler, ToScheduler};
    use crate::{get_cards, read_bytes, write_bytes};
    use prost::Message;
    use std::ffi::c_void;

    fn split_u64(v: u64) -> (u32, u32) {
        let hi32 = (v >> 32) as u32;
        let lo32 = v as u32;
        (hi32, lo32)
    }

    //
    // split u64 to 2 u32
    #[allow(dead_code)]

    fn merge_u32(high: u32, low: u32) -> u64 {
        (high as u64) << 32 | (low as u64)
    }
    #[test]
    fn basic() {
        let p = unsafe { crate::alloc(100) };
        assert!((p as u64) < u32::MAX as u64);
        unsafe {
            crate::dealloc(p);
        }
        let p = ToScheduler {
            cards: vec![],
            custom_params: "".to_string(),
            tags_to_query: vec![],
        };
        let mut v = Vec::new();
        p.encode(&mut v).unwrap();
        let p = unsafe { crate::alloc(v.len()) };
        unsafe {
            write_bytes(p as *mut u8, &v);
        }
        let res = get_cards(p as u64, v.len() as u32, 0);
        let (p_n, l) = split_u64(res);
        let b = unsafe { read_bytes(p_n as *mut u8, l) };
        let r = FromScheduler::decode(&mut b.as_slice()).unwrap();
        let r = match r.response.unwrap() {
            Response::GoodValue(g) => g,
            Response::BadValue(e) => panic!("bad value: {:?}", e),
        };
        assert_eq!(r.card_to_view.unwrap(), NoCard(Default::default()));
        unsafe {
            crate::dealloc(p);
        }
        unsafe {
            crate::dealloc(p_n as *mut c_void);
        }
    }
}
#[cfg(not(all(target_os = "wasi", target_env = "p1")))]
#[cfg(test)]
mod draw_charts {
    use std::fs::File;

    use crate::Card;
    use std::io::Write;
    #[test]
    fn draw_chart_no_errors() {
        let mut file = File::create("./plots/no-errors.txt").unwrap();
        let mut c = Card::new();
        let f = (0..20)
            .map(|a| {
                c.grade(c.get_due_timestamp().unwrap_or_default(), true);
                (
                    a,
                    c.get_interval().unwrap().as_secs() as f64 / 60.0 / 60.0 / 24.0,
                )
            })
            .fold(String::new(), |old, (i, v)| format!("{old}\n{i}|{v}"));
        file.write_all(f.as_bytes()).unwrap();
    }

    #[test]
    fn draw_chart_with_errors() {
        let mut file = File::create("./plots/frequent-errors.txt").unwrap();
        let mut c = Card::new();
        let f = (0..110)
            .map(|a| {
                let correct = if a > 80 {
                    true
                } else if a > 20 {
                    a % 8 != 0
                } else {
                    a % 15 != 0
                };
                c.grade(c.get_due_timestamp().unwrap_or_default() + 1, correct);
                (
                    a,
                    c.get_interval().unwrap().as_secs() as f64 / 60.0 / 60.0 / 24.0,
                )
            })
            .fold(String::new(), |old, (i, v)| format!("{old}\n{i}|{v}"));
        file.write_all(f.as_bytes()).unwrap();
    }
}
#[cfg(test)]
mod test_unit {
    use crate::scheduler::from_scheduler::Response;
    use crate::scheduler::response_correct::CardToView::NoCard;
    use crate::{Card, proc_cards};
    use std::collections::HashSet;
    use std::time::Duration;

    #[test]
    fn no_cards_no_nothing() {
        let h = HashSet::new();
        let ret = proc_cards(vec![], h, "{}".to_string(), 0);
        let ret = match ret {
            Response::GoodValue(g) => g,
            Response::BadValue(e) => panic!("wrong type: {:?}", e),
        };
        assert_eq!(ret.card_to_view.unwrap(), NoCard(Default::default()));
    }
    #[test]
    fn a() {
        let mut m = Card::new();

        assert_eq!(m.get_due_timestamp(), None);
        assert_eq!(m.get_interval(), None);
        assert_eq!(m.last_seen(), None);

        let orig = 60 * 60 * 24 * 365;
        m.grade(orig, true);

        assert_eq!(m.get_due_timestamp(), Some(orig + 60 * 15));
        assert_eq!(m.get_interval(), Some(Duration::from_secs(60 * 15)));
        assert_eq!(m.last_seen(), Some(orig));

        let orig = m.get_due_timestamp().unwrap() + 1;
        m.grade(orig, true);

        assert_eq!(m.get_due_timestamp(), Some(orig + 60 * 15 * 4));
        assert_eq!(m.get_interval(), Some(Duration::from_secs(60 * 15 * 4)));
        assert_eq!(m.last_seen(), Some(orig));

        let orig = m.get_due_timestamp().unwrap() + 1;
        m.grade(orig, true);

        assert_eq!(m.get_due_timestamp(), Some(orig + 60 * 15 * 4 * 4));
        assert_eq!(m.get_interval(), Some(Duration::from_secs(60 * 15 * 4 * 4)));
        assert_eq!(m.last_seen(), Some(orig));

        let orig = m.get_due_timestamp().unwrap() + 1;
        m.grade(orig, true);

        assert_eq!(m.get_due_timestamp(), Some(orig + 60 * 15 * 4 * 4 * 4));
        assert_eq!(
            m.get_interval(),
            Some(Duration::from_secs(60 * 15 * 4 * 4 * 4))
        );
        assert_eq!(m.last_seen(), Some(orig));

        let orig = m.get_due_timestamp().unwrap() + 1;
        m.grade(orig, true);

        assert_eq!(m.get_due_timestamp(), Some(orig + 60 * 60 * 24));
        assert_eq!(m.get_interval(), Some(Duration::from_secs(60 * 60 * 24)));
        assert_eq!(m.last_seen(), Some(orig));
        let orig = m.get_due_timestamp().unwrap() + 1;
        m.grade(dbg!(orig), true);
        assert_eq!(
            m.get_interval(),
            Some(Duration::from_secs(60 * 60 * 24 * 2))
        );
        assert_eq!(m.last_seen(), Some(orig));
        assert_eq!(m.get_due_timestamp(), Some(orig + 60 * 60 * 24 * 2));
    }
}
#[cfg(test)]
mod test_integration {
    use crate::scheduler::response_correct::CardToView;
    use crate::scheduler::{Card, CardRecord};
    use crate::{CustomArgs, ToScheduler, safe_proc_cards};
    use prost_types::Timestamp;

    #[test]
    fn b() {
        crate::init();

        let to_sched = ToScheduler {
            tags_to_query: vec!["a".to_string(), "b".to_string()],
            custom_params: "".to_string(),
            cards: vec![
                Card {
                    id: 100,
                    tags: vec!["a".into(), "b".into()],
                    card_records: vec![CardRecord {
                        when: Some(Timestamp::date_time(2025, 1, 1, 0, 0, 1).unwrap()),
                        correct: false,
                    }],
                },
                Card {
                    id: 101,
                    tags: vec!["d".into(), "b".into()],
                    card_records: vec![CardRecord {
                        when: Some(Timestamp::date_time(2025, 1, 1, 0, 0, 2).unwrap()),
                        correct: false,
                    }],
                },
                Card {
                    id: 102,
                    tags: vec!["a".into(), "c".into()],
                    card_records: vec![CardRecord {
                        when: Some(Timestamp::date_time(2025, 1, 1, 0, 0, 3).unwrap()),
                        correct: false,
                    }],
                },
            ],
        };
        let res = safe_proc_cards(
            to_sched.cards,
            to_sched.tags_to_query.iter().cloned().collect(),
            Timestamp::date_time(2025, 1, 1, 0, 0, 4).unwrap().seconds as u64,
            CustomArgs {
                max_amount_of_new_cards_per_day: Some(10),
                starting_hour: 0.into(),
            },
        );

        assert_eq!(res.amnt_non_due_cards, 3);
        assert_eq!(res.amnt_due_cards, 0);
        assert_eq!(res.amnt_new_cards, 0);
        assert_eq!(
            res.card_to_view.unwrap(),
            CardToView::NonDueCard(Card {
                id: 100,
                tags: vec!["a".into(), "b".into()],
                card_records: vec![]
            })
        );
        assert!(!res.is_short_term);
        assert_eq!(
            *res.future_card_heatmap.get(&100).unwrap(),
            Timestamp::date_time(2025, 1, 1, 0, 0, 16).unwrap().seconds as u64
        );
        assert_eq!(
            *res.future_card_heatmap.get(&101).unwrap(),
            Timestamp::date_time(2025, 1, 1, 0, 0, 17).unwrap().seconds as u64
        );
        assert_eq!(
            *res.future_card_heatmap.get(&102).unwrap(),
            Timestamp::date_time(2025, 1, 1, 0, 0, 18).unwrap().seconds as u64
        );
    }

    #[test]
    fn b_1() {
        crate::init();
        let to_sched = ToScheduler {
            custom_params: "".to_string(),
            tags_to_query: vec!["a".into(), "b".into()],
            cards: vec![
                Card {
                    id: 100,
                    tags: vec!["a".into(), "b".into()],
                    card_records: vec![CardRecord {
                        when: Some(Timestamp::date_time(2025, 1, 1, 0, 0, 1).unwrap()),
                        correct: true,
                    }],
                },
                Card {
                    id: 101,
                    tags: vec!["d".into(), "b".into()],
                    card_records: vec![CardRecord {
                        when: Some(Timestamp::date_time(2025, 1, 1, 0, 0, 2).unwrap()),
                        correct: true,
                    }],
                },
                Card {
                    id: 102,
                    tags: vec!["a".into(), "c".into()],
                    card_records: vec![CardRecord {
                        when: Some(Timestamp::date_time(2025, 1, 1, 0, 0, 3).unwrap()),
                        correct: true,
                    }],
                },
            ],
        };
        let res = safe_proc_cards(
            to_sched.cards,
            to_sched.tags_to_query.iter().cloned().collect(),
            Timestamp::date_time(2025, 1, 1, 0, 15, 4).unwrap().seconds as u64,
            CustomArgs {
                max_amount_of_new_cards_per_day: Some(10),
                starting_hour: 0.into(),
            },
        );

        assert_eq!(res.amnt_non_due_cards, 0);
        assert_eq!(res.amnt_due_cards, 3);
        assert_eq!(res.amnt_new_cards, 0);
        assert_eq!(
            res.card_to_view.unwrap(),
            CardToView::DueCard(Card {
                id: 100,
                tags: vec!["a".into(), "b".into()],
                card_records: vec![]
            })
        );
        assert!(res.is_short_term);
        assert_eq!(
            *res.future_card_heatmap.get(&100).unwrap(),
            Timestamp::date_time(2025, 1, 1, 0, 15, 1).unwrap().seconds as u64
        );
        assert_eq!(
            *res.future_card_heatmap.get(&101).unwrap(),
            Timestamp::date_time(2025, 1, 1, 0, 15, 2).unwrap().seconds as u64
        );
        assert_eq!(
            *res.future_card_heatmap.get(&102).unwrap(),
            Timestamp::date_time(2025, 1, 1, 0, 15, 3).unwrap().seconds as u64
        );
    }
}

// split u64 to 2 u32
