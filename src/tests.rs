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
                    if a % 8 == 0 { false } else { true }
                } else {
                    if a % 15 == 0 { false } else { true }
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
    use core::{convert::TryInto, ffi::c_void};
    use std::{
        collections::HashMap,
        sync::{LazyLock, Mutex},
    };

    use crate::{Card, do_get_cards};
    static MUTEX: LazyLock<Mutex<bool>> = LazyLock::new(|| Mutex::new(true));

    #[test]
    fn b() {
        let a = MUTEX.lock();
        if let Result::Err(_) = a {
            return;
        }
        let ptr = unsafe { crate::alloc(21) } as *mut u8;
        crate::init();
        crate::add_card(100);
        crate::add_card(101);
        crate::add_card(102);
        let orig_time = 24 * 60 * 60 * 30;
        crate::grade_card(100, 1 + orig_time, 0);
        crate::grade_card(101, 2 + orig_time, 0);
        crate::grade_card(102, 3 + orig_time, 0);
        unsafe { crate::get_cards(ptr as i64, (4 + orig_time).try_into().unwrap()) };
        let len_due_cards = unsafe { u32::from_le_bytes(read_bytes(ptr)) };
        let len_non_due_cards = unsafe { u32::from_le_bytes(read_bytes(ptr.add(4))) };
        let len_new_cards = unsafe { u32::from_le_bytes(read_bytes(ptr.add(8))) };
        let type_next_card = unsafe { u8::from_le_bytes(read_bytes(ptr.add(12))) };
        let next_card = unsafe { u64::from_le_bytes(read_bytes(ptr.add(14))) };

        assert_eq!(len_non_due_cards, 3);
        assert_eq!(len_due_cards, 0);
        assert_eq!(len_new_cards, 0);
        assert_eq!(type_next_card, crate::NON_DUE_CARD);
        assert_eq!(next_card, 100);
        unsafe {
            crate::dealloc(ptr as *mut c_void);
        }
    }

    #[test]
    fn b_1() {
        let a = MUTEX.lock();
        if let Result::Err(_) = a {
            return;
        }
        let ptr = unsafe { crate::alloc(21) } as *mut u8;
        crate::init();
        crate::add_card(100);
        crate::add_card(101);
        crate::add_card(102);
        let orig_time = 24 * 60 * 60 * 30;
        crate::grade_card(100, 1 + orig_time, 0);
        crate::grade_card(101, 2 + orig_time, 0);
        crate::grade_card(102, 3 + orig_time, 0);
        unsafe { crate::get_cards(ptr as i64, (30 + orig_time).try_into().unwrap()) };
        let len_due_cards = unsafe { u32::from_le_bytes(read_bytes(ptr)) };
        let len_non_due_cards = unsafe { u32::from_le_bytes(read_bytes(ptr.add(4))) };
        let len_new_cards = unsafe { u32::from_le_bytes(read_bytes(ptr.add(8))) };
        let type_next_card = unsafe { u8::from_le_bytes(read_bytes(ptr.add(12))) };
        let next_card = unsafe { u64::from_le_bytes(read_bytes(ptr.add(14))) };

        assert_eq!(len_non_due_cards, 0);
        assert_eq!(len_due_cards, 3);
        assert_eq!(len_new_cards, 0);
        assert_eq!(type_next_card, crate::DUE_CARD);
        assert_eq!(next_card, 100);
        unsafe {
            crate::dealloc(ptr as *mut c_void);
        }
    }

    #[test]
    fn c() {
        let a = MUTEX.lock();
        if let Result::Err(_) = a {
            return;
        }
        crate::init();
        crate::add_card(110);
        crate::add_card(111);
        crate::add_card(112);
        crate::add_card(113);
        crate::add_card(114);
        crate::add_card(115);
        crate::add_card(116);
        crate::add_card(117);
        crate::add_card(118);
        crate::add_card(119);
        crate::add_card(120);
        crate::add_card(121);
        crate::add_card(122);

        let mut orig_time = 24 * 60 * 60 * 30;
        {
            let set: &_ = &*crate::CARDS_SET;
            let cards_map: HashMap<u64, Card> = set.read().unwrap().clone();
            orig_time += 2;
            assert_eq!(
                do_get_cards(orig_time, cards_map),
                crate::DoGetCardsReturn {
                    is_short_term: false,
                    len_due_cards: 0,
                    len_new_cards: 10,
                    next_card: 122,
                    len_non_due_cards: 0,
                    type_next_card: crate::NEW_CARD,
                }
            );
        }
        orig_time += 1;
        crate::grade_card(122, orig_time, 0);
        let orig_time_121 = orig_time + 1;

        {
            let set: &_ = &*crate::CARDS_SET;
            let cards_map: HashMap<u64, Card> = set.read().unwrap().clone();
            orig_time += 1;

            assert_eq!(
                do_get_cards(orig_time, cards_map),
                crate::DoGetCardsReturn {
                    is_short_term: false,
                    len_due_cards: 0,
                    len_new_cards: 9,
                    next_card: 121,
                    len_non_due_cards: 1,
                    type_next_card: crate::NEW_CARD,
                }
            );
        }

        orig_time += 15 + 1 + 5;
        {
            let set: &_ = &*crate::CARDS_SET;
            let cards_map: HashMap<u64, Card> = set.read().unwrap().clone();
            orig_time += 1;
            assert_eq!(
                do_get_cards(orig_time, cards_map),
                crate::DoGetCardsReturn {
                    is_short_term: true,
                    len_due_cards: 1,
                    len_new_cards: 9,
                    next_card: 122,
                    len_non_due_cards: 0,
                    type_next_card: crate::DUE_CARD,
                }
            );
        }
        orig_time += 1;
        crate::grade_card(121, orig_time, 1);

        {
            let set: &_ = &*crate::CARDS_SET;
            let cards_map: HashMap<u64, Card> = set.read().unwrap().clone();
            orig_time += 1;
            assert_eq!(
                do_get_cards(orig_time, cards_map),
                crate::DoGetCardsReturn {
                    is_short_term: true,
                    len_due_cards: 1,
                    len_new_cards: 8,
                    next_card: 122,
                    len_non_due_cards: 1,
                    type_next_card: crate::DUE_CARD,
                }
            );
        }
        orig_time += 1;
        crate::grade_card(122, orig_time, 1);
        {
            let set: &_ = &*crate::CARDS_SET;
            let cards_map: HashMap<u64, Card> = set.read().unwrap().clone();
            orig_time += 1;
            assert_eq!(
                do_get_cards(orig_time, cards_map),
                crate::DoGetCardsReturn {
                    is_short_term: false,
                    len_due_cards: 0,
                    len_new_cards: 8,
                    next_card: 120,
                    len_non_due_cards: 2,
                    type_next_card: crate::NEW_CARD,
                }
            );
        }
        orig_time += 1;
        crate::grade_card(120, orig_time, 1);
        {
            let set: &_ = &*crate::CARDS_SET;
            let cards_map: HashMap<u64, Card> = set.read().unwrap().clone();
            orig_time += 1;
            assert_eq!(
                do_get_cards(orig_time, cards_map),
                crate::DoGetCardsReturn {
                    is_short_term: false,
                    len_due_cards: 0,
                    len_new_cards: 7,
                    next_card: 119,
                    len_non_due_cards: 3,
                    type_next_card: crate::NEW_CARD,
                }
            );
        }
        orig_time += 1;
        crate::grade_card(119, orig_time, 1);
        {
            let set: &_ = &*crate::CARDS_SET;
            let cards_map: HashMap<u64, Card> = set.read().unwrap().clone();
            orig_time += 1;
            assert_eq!(
                do_get_cards(orig_time, cards_map),
                crate::DoGetCardsReturn {
                    is_short_term: false,
                    len_due_cards: 0,
                    len_new_cards: 6,
                    next_card: 118,
                    len_non_due_cards: 4,
                    type_next_card: crate::NEW_CARD,
                }
            );
        }
        orig_time += 1;
        crate::grade_card(118, orig_time, 1);
        {
            let set: &_ = &*crate::CARDS_SET;
            let cards_map: HashMap<u64, Card> = set.read().unwrap().clone();
            orig_time += 1;
            assert_eq!(
                do_get_cards(orig_time, cards_map),
                crate::DoGetCardsReturn {
                    is_short_term: false,
                    len_due_cards: 0,
                    len_new_cards: 5,
                    next_card: 117,
                    len_non_due_cards: 5,
                    type_next_card: crate::NEW_CARD,
                }
            );
        }
        orig_time += 1;
        crate::grade_card(117, orig_time, 1);
        {
            let set: &_ = &*crate::CARDS_SET;
            let cards_map: HashMap<u64, Card> = set.read().unwrap().clone();
            orig_time += 1;
            assert_eq!(
                do_get_cards(orig_time, cards_map),
                crate::DoGetCardsReturn {
                    is_short_term: false,
                    len_due_cards: 0,
                    len_new_cards: 4,
                    next_card: 116,
                    len_non_due_cards: 6,
                    type_next_card: crate::NEW_CARD,
                }
            );
        }
        orig_time += 1;
        crate::grade_card(116, orig_time, 1);
        {
            let set: &_ = &*crate::CARDS_SET;
            let cards_map: HashMap<u64, Card> = set.read().unwrap().clone();
            orig_time += 1;
            assert_eq!(
                do_get_cards(orig_time, cards_map),
                crate::DoGetCardsReturn {
                    is_short_term: false,
                    len_due_cards: 0,
                    len_new_cards: 3,
                    next_card: 115,
                    len_non_due_cards: 7,
                    type_next_card: crate::NEW_CARD,
                }
            );
        }

        orig_time += 1;
        crate::grade_card(115, orig_time, 1);
        {
            let set: &_ = &*crate::CARDS_SET;
            let cards_map: HashMap<u64, Card> = set.read().unwrap().clone();
            orig_time += 1;
            assert_eq!(
                do_get_cards(orig_time, cards_map),
                crate::DoGetCardsReturn {
                    is_short_term: false,
                    len_due_cards: 0,
                    len_new_cards: 2,
                    next_card: 114,
                    len_non_due_cards: 8,
                    type_next_card: crate::NEW_CARD,
                }
            );
        }

        orig_time += 1;
        crate::grade_card(114, dbg!(orig_time), 1);
        {
            let set: &_ = &*crate::CARDS_SET;
            let cards_map: HashMap<u64, Card> = set.read().unwrap().clone();
            orig_time += 1;
            eprintln!("{orig_time_121}");
            assert_eq!(
                do_get_cards(orig_time, cards_map),
                crate::DoGetCardsReturn {
                    is_short_term: false,
                    len_due_cards: 0,
                    len_new_cards: 1,
                    next_card: 113,
                    len_non_due_cards: 9,
                    type_next_card: crate::NEW_CARD,
                }
            );
        }

        orig_time += 1;
        println!("{orig_time_121}");
        crate::grade_card(113, orig_time, 1);
        {
            let set: &_ = &*crate::CARDS_SET;
            let cards_map: HashMap<u64, Card> = set.read().unwrap().clone();
            orig_time += 1;
            assert_eq!(
                do_get_cards(orig_time, cards_map),
                crate::DoGetCardsReturn {
                    is_short_term: false,
                    len_due_cards: 0,
                    len_new_cards: 1,
                    next_card: 112,
                    len_non_due_cards: 9,
                    type_next_card: crate::NEW_CARD,
                }
            );
        }

        orig_time += 1;
        crate::grade_card(112, orig_time, 1);
        {
            let set: &_ = &*crate::CARDS_SET;
            let cards_map: HashMap<u64, Card> = set.read().unwrap().clone();
            orig_time += 1;
            assert_eq!(
                do_get_cards(dbg!(orig_time), cards_map),
                crate::DoGetCardsReturn {
                    is_short_term: false,
                    len_due_cards: 0,
                    len_new_cards: 0,
                    next_card: 122,
                    len_non_due_cards: 10,
                    type_next_card: crate::NON_DUE_CARD,
                }
            );
        }
    }

    unsafe fn read_bytes<const N: usize>(p: *mut u8) -> [u8; N] {
        let mut m = [0; N];
        for i in 0..N {
            m[i] = unsafe { *p.add(i) };
        }
        m
    }

    #[test]
    fn get_interval_a() {
        assert_eq!(crate::get_interval_secs_short_term(0), 15 * 60);
    }
}
