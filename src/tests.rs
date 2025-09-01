#[cfg(test)]
#[cfg(all(target_os = "wasi", target_env = "p1"))]
mod integration {
    use crate::scheduler::from_scheduler::Response;
    use crate::scheduler::response_correct::CardToView::{DueCard, NewCard, NoCard, NonDueCard};
    use crate::scheduler::{Card, CardRecord, FromScheduler, ResponseCorrect, ToScheduler};
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

    #[test]
    fn has_card_but_no_history() {
        let p = ToScheduler {
            cards: vec![
                Card {
                    id: 361535846,
                    tags: vec!["t3".into(), "t5".into()],
                    card_records: vec![],
                },
                Card {
                    id: 3182784968,
                    tags: vec!["t1".into(), "t2".into()],
                    card_records: vec![],
                },
            ],
            custom_params: "{}".to_string(),
            tags_to_query: vec!["t3".into()],
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
        assert_eq!(
            r,
            ResponseCorrect {
                amnt_new_cards: 1,
                amnt_due_cards: 0,
                amnt_non_due_cards: 0,
                is_short_term: false,
                future_card_heatmap: Default::default(),
                card_to_view: Some(NewCard(Card {
                    id: 361535846,
                    tags: vec!["t3".into(), "t5".into()],
                    card_records: vec![],
                }))
            }
        );
        unsafe {
            crate::dealloc(p);
        }
        unsafe {
            crate::dealloc(p_n as *mut c_void);
        }
    }

    #[test]
    fn has_two_cards_with_history_id_order_one() {
        has_two_cards_with_history(1, 2);
    }
    #[test]
    fn has_two_cards_with_history_id_order_other() {
        has_two_cards_with_history(2, 1);
    }

    #[test]
    fn has_two_cards_with_three_history_id_order_one() {
        has_two_cards_with_three_history(1, 2);
    }
    #[test]
    fn has_two_cards_with_three_history_id_order_other() {
        has_two_cards_with_three_history(2, 1);
    }

    fn has_two_cards_with_history(id1: u32, id2: u32) {
        let p = ToScheduler {
            cards: vec![
                Card {
                    id: id1,
                    tags: vec!["t1".into()],
                    card_records: vec![CardRecord {
                        when: Some(prost_types::Timestamp {
                            seconds: 1000 + 24 * 24 * 60,
                            nanos: 0,
                        }),
                        correct: true,
                    }],
                },
                Card {
                    id: id2,
                    tags: vec!["t1".into()],
                    card_records: vec![CardRecord {
                        when: Some(prost_types::Timestamp {
                            seconds: 1005 + 24 * 24 * 60,
                            nanos: 0,
                        }),
                        correct: true,
                    }],
                },
            ],
            custom_params: "{}".to_string(),
            tags_to_query: vec!["t1".into()],
        };
        let mut v = Vec::new();
        p.encode(&mut v).unwrap();
        let p = unsafe { crate::alloc(v.len()) };
        unsafe {
            write_bytes(p as *mut u8, &v);
        }
        let res = get_cards(p as u64, v.len() as u32, 1005 + 24 * 24 * 60 + 10);
        let (p_n, l) = split_u64(res);
        let b = unsafe { read_bytes(p_n as *mut u8, l) };
        let r = FromScheduler::decode(&mut b.as_slice()).unwrap();
        let r = match r.response.unwrap() {
            Response::GoodValue(g) => g,
            Response::BadValue(e) => panic!("bad value: {:?}", e),
        };
        assert_eq!(r.amnt_due_cards, 0);
        assert_eq!(r.amnt_non_due_cards, 2);
        assert_eq!(r.is_short_term, false);
        assert_eq!(r.amnt_new_cards, 0);
        assert_eq!(
            r.card_to_view.unwrap(),
            NonDueCard(Card {
                id: id1,
                tags: vec!["t1".into()],
                card_records: vec![],
            })
        );
        unsafe {
            crate::dealloc(p);
        }
        unsafe {
            crate::dealloc(p_n as *mut c_void);
        }
    }

    fn has_two_cards_with_three_history(id1: u32, id2: u32) {
        let p = ToScheduler {
            cards: vec![
                Card {
                    id: id1,
                    tags: vec!["t1".into()],
                    card_records: vec![
                        CardRecord {
                            when: Some(prost_types::Timestamp {
                                seconds: 1000 + 24 * 24 * 60,
                                nanos: 0,
                            }),
                            correct: true,
                        },
                        CardRecord {
                            when: Some(prost_types::Timestamp {
                                seconds: 1000 + 24 * 24 * 60 + 6,
                                nanos: 0,
                            }),
                            correct: true,
                        },
                    ],
                },
                Card {
                    id: id2,
                    tags: vec!["t1".into()],
                    card_records: vec![CardRecord {
                        when: Some(prost_types::Timestamp {
                            seconds: 1005 + 24 * 24 * 60,
                            nanos: 0,
                        }),
                        correct: true,
                    }],
                },
            ],
            custom_params: "{}".to_string(),
            tags_to_query: vec!["t1".into()],
        };
        let mut v = Vec::new();
        p.encode(&mut v).unwrap();
        let p = unsafe { crate::alloc(v.len()) };
        unsafe {
            write_bytes(p as *mut u8, &v);
        }
        let res = get_cards(p as u64, v.len() as u32, 1005 + 24 * 24 * 60 + 10);
        let (p_n, l) = split_u64(res);
        let b = unsafe { read_bytes(p_n as *mut u8, l) };
        let r = FromScheduler::decode(&mut b.as_slice()).unwrap();
        let r = match r.response.unwrap() {
            Response::GoodValue(g) => g,
            Response::BadValue(e) => panic!("bad value: {:?}", e),
        };
        assert_eq!(r.amnt_due_cards, 0);
        assert_eq!(r.amnt_non_due_cards, 2);
        assert_eq!(r.is_short_term, false);
        assert_eq!(r.amnt_new_cards, 0);
        assert_eq!(
            r.card_to_view.unwrap(),
            NonDueCard(Card {
                id: id2,
                tags: vec!["t1".into()],
                card_records: vec![],
            })
        );
        unsafe {
            crate::dealloc(p);
        }
        unsafe {
            crate::dealloc(p_n as *mut c_void);
        }
    }

    #[test]
    fn single_card_with_history() {
        let p = ToScheduler {
            cards: vec![Card {
                id: 361535846,
                tags: vec!["t3".into(), "t5".into()],
                card_records: vec![CardRecord {
                    when: Some(prost_types::Timestamp {
                        seconds: 1000 * 100 + 24 * 24 * 60,
                        nanos: 0,
                    }),
                    correct: true,
                }],
            }],
            custom_params: "".to_string(),
            tags_to_query: vec!["t3".into()],
        };
        let mut v = Vec::new();
        p.encode(&mut v).unwrap();
        let p = unsafe { crate::alloc(v.len()) };
        unsafe {
            write_bytes(p as *mut u8, &v);
        }
        let res = get_cards(
            p as u64,
            v.len() as u32,
            1000 * 100 + 24 * 24 * 60 + 15 * 60 + 1,
        );
        let (p_n, l) = split_u64(res);
        let b = unsafe { read_bytes(p_n as *mut u8, l) };
        let r = FromScheduler::decode(&mut b.as_slice()).unwrap();
        let r = match r.response.unwrap() {
            Response::GoodValue(g) => g,
            Response::BadValue(e) => panic!("bad value: {:?}", e),
        };
        assert_eq!(r.amnt_new_cards, 0);
        assert_eq!(r.amnt_non_due_cards, 0);
        assert_eq!(r.is_short_term, true);
        assert_eq!(r.amnt_due_cards, 1);
        assert_eq!(
            r.card_to_view,
            Some(DueCard(Card {
                id: 361535846,
                tags: vec!["t3".into(), "t5".into()],
                card_records: vec![],
            }))
        );

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
    use crate::{proc_cards, Card};
    use std::collections::HashSet;
    use std::time::Duration;

    #[test]
    fn no_cards_no_nothing() {
        let h = HashSet::new();
        let ret = proc_cards(vec![], h, "{}".to_string(), 0, true);
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
        m.grade(orig, true);
        assert_eq!(
            m.get_interval(),
            Some(Duration::from_secs(60 * 60 * 24 * 2))
        );
        assert_eq!(m.last_seen(), Some(orig));
        assert_eq!(m.get_due_timestamp(), Some(orig + 60 * 60 * 24 * 2));
    }

    #[test]
    fn failed_single_card_prod() {
        let mut m = Card::new();

        assert_eq!(m.get_due_timestamp(), None);
        assert_eq!(m.get_interval(), None);
        assert_eq!(m.last_seen(), None);
        /*
        id          res timestamp
        757025923	0	1756181715
        757025923	0	1756181729
        757025923	1	1756181741
        757025923	1	1756181744
        757025923	1	1756181751

         */
        m.grade(0, false);
        m.grade(14, false);
        assert_eq!(29, m.get_due_timestamp().unwrap());
        m.grade(26, true);
        dbg!(&m);
        dbg!("graded right");
        assert_eq!(926, m.get_due_timestamp().unwrap());
        m.grade(29, true);
        assert_eq!(1829, m.get_due_timestamp().unwrap());


    }

}
#[cfg(test)]
mod test_integration {
    use crate::scheduler::response_correct::CardToView;
    use crate::scheduler::{Card, CardRecord};
    use crate::{safe_proc_cards, CustomArgs, ToScheduler};
    use prost_types::Timestamp;

    #[test]
    fn b() {
        crate::init();

        let to_sched = ToScheduler {
            get_new_questions: true,
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
                exam_days: Some(vec![]),
            },
            true,
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
            get_new_questions: true,
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
                exam_days: Some(vec![]),
            },
            true,
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