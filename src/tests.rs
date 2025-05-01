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
    use crate::Card;
    use std::time::Duration;
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
    use crate::{safe_proc_cards, CustomArgs, ToScheduler};
    use prost_types::Timestamp;

    #[test]
    fn b() {
        crate::init();

        let to_sched = ToScheduler {
            custom_params: "".to_string(),
            cards: vec![
                Card {
                    id: 100,
                    is_current: true,
                    card_records: vec![CardRecord {
                        when: Some(Timestamp::date_time(2025, 1, 1, 0, 0, 1).unwrap()),
                        correct: false,
                    }],
                },
                Card {
                    id: 101,
                    is_current: true,
                    card_records: vec![CardRecord {
                        when: Some(Timestamp::date_time(2025, 1, 1, 0, 0, 2).unwrap()),
                        correct: false,
                    }],
                },
                Card {
                    id: 102,
                    is_current: true,
                    card_records: vec![CardRecord {
                        when: Some(Timestamp::date_time(2025, 1, 1, 0, 0, 3).unwrap()),
                        correct: false,
                    }],
                },
            ],
        };
        let res = safe_proc_cards(
            to_sched.cards,
            Timestamp::date_time(2025, 1, 1, 0, 0, 4).unwrap().seconds as u64,
            CustomArgs {
                max_amount_of_new_cards_per_day: 10,
                starting_hour: 0,
            },
        );

        assert_eq!(res.amnt_non_due_cards, 3);
        assert_eq!(res.amnt_due_cards, 0);
        assert_eq!(res.amnt_new_cards, 0);
        assert_eq!(
            res.card_to_view.unwrap(),
            CardToView::NonDueCard(Card {
                id: 100,
                is_current: false,
                card_records: vec![]
            })
        );
        assert_eq!(res.is_short_term, false);
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
            cards: vec![
                Card {
                    id: 100,
                    is_current: true,
                    card_records: vec![CardRecord {
                        when: Some(Timestamp::date_time(2025, 1, 1, 0, 0, 1).unwrap()),
                        correct: true,
                    }],
                },
                Card {
                    id: 101,
                    is_current: true,
                    card_records: vec![CardRecord {
                        when: Some(Timestamp::date_time(2025, 1, 1, 0, 0, 2).unwrap()),
                        correct: true,
                    }],
                },
                Card {
                    id: 102,
                    is_current: true,
                    card_records: vec![CardRecord {
                        when: Some(Timestamp::date_time(2025, 1, 1, 0, 0, 3).unwrap()),
                        correct: true,
                    }],
                },
            ],
        };
        let res = safe_proc_cards(
            to_sched.cards,
            Timestamp::date_time(2025, 1, 1, 0, 15, 4).unwrap().seconds as u64,
            CustomArgs {
                max_amount_of_new_cards_per_day: 10,
                starting_hour: 0,
            },
        );

        assert_eq!(res.amnt_non_due_cards, 0);
        assert_eq!(res.amnt_due_cards, 3);
        assert_eq!(res.amnt_new_cards, 0);
        assert_eq!(
            res.card_to_view.unwrap(),
            CardToView::DueCard(Card {
                id: 100,
                is_current: false,
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
    #[test]
    fn c() {
        crate::init();
        
    }
}
