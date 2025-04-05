use core::{cmp::Ordering, ffi::c_void, time::Duration};
use std::{
    collections::HashMap,
    sync::{LazyLock, RwLock, RwLockWriteGuard},
};

static CARDS_SET: LazyLock<RwLock<HashMap<u64, Card>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

static WHEN_WAS_CARD_FIRST_DONE: LazyLock<RwLock<HashMap<u64, u64>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

#[unsafe(export_name = "add-card")]
extern "C" fn add_card(id: u64) {
    let set = &*CARDS_SET;
    set.write().unwrap().insert(id, Card::new());
}

#[unsafe(export_name = "grade-card")]
extern "C" fn grade_card(id: u64, time: u64, correct: u64) {
    let set = &*CARDS_SET;
    let first_done = &*WHEN_WAS_CARD_FIRST_DONE;
    {
        let mut first_done_map: RwLockWriteGuard<HashMap<u64, u64>> = first_done.write().unwrap();
        (*first_done_map).entry(id).or_insert(time);
    }
    let mut cards_map: RwLockWriteGuard<HashMap<u64, Card>> = set.write().unwrap();
    let grade = if correct == 0 { false } else { true };
    (*cards_map)
        .entry(id)
        .and_modify(|card| card.grade(time, grade));
}

#[derive(Debug)]
struct ThickCard {
    id: u64,
    last_seen: Option<u64>,
    is_short_term: bool,
    pos: Option<u32>,
}

impl PartialEq for ThickCard {
    fn eq(&self, other: &ThickCard) -> bool {
        self.id == other.id
    }
}
impl Eq for ThickCard {}

impl Ord for ThickCard {
    fn cmp(&self, other: &ThickCard) -> std::cmp::Ordering {
        match (self.pos, other.pos) {
            (Some(a), Some(b)) => {
                if a > b {
                    Ordering::Greater
                } else if a < b {
                    Ordering::Less
                } else {
                    self.id.cmp(&other.id)
                }
            }
            (Option::None, Some(_)) => Ordering::Less,
            (Some(_), Option::None) => Ordering::Greater,
            (Option::None, Option::None) => self.id.cmp(&other.id),
        }
    }
}
impl PartialOrd for ThickCard {
    fn partial_cmp(&self, other: &ThickCard) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug)]
struct DoGetCardsReturn {
    len_due_cards: u32,
    len_non_due_cards: u32,
    len_new_cards: u32,
    type_next_card: u8,
    next_card: u64,
    is_short_term: bool,
}

fn do_get_cards(n_unix: u64, cards: HashMap<u64, Card>) -> DoGetCardsReturn {
    let mut new_cards: Vec<ThickCard> = vec![];
    let mut due_cards: Vec<ThickCard> = vec![];
    let mut non_due_cards: Vec<ThickCard> = vec![];
    let how_many_new_cards_done_today = WHEN_WAS_CARD_FIRST_DONE
        .read()
        .unwrap()
        .iter()
        .filter(|(_, v)| **v >= n_unix - 24 * 60 * 60)
        .count();
    for (k, v) in cards {
        let t = v.get_due_timestamp();
        let new_thick_card = ThickCard {
            id: k,
            last_seen: v.last_seen(),
            is_short_term: v.is_short_term(),
            pos: v.get_pos(),
        };
        match t {
            Option::None => new_cards.push(new_thick_card),
            Some(t) => {
                if n_unix > t {
                    due_cards.push(new_thick_card);
                } else {
                    non_due_cards.push(new_thick_card);
                }
            }
        };
    }

    due_cards.sort();
    due_cards.reverse();

    non_due_cards.sort_by(|a,b|{a.last_seen.cmp(&b.last_seen)});
    non_due_cards.reverse();

    new_cards.sort();
    new_cards.reverse();
    let len_due_cards = due_cards.len().try_into().unwrap();
    let len_non_due_cards = non_due_cards.len().try_into().unwrap();
    let cards_left = 10_u32.saturating_sub(how_many_new_cards_done_today as u32);
    let len_new_cards = (new_cards.len() as u32).min(cards_left);
    let (type_next_card, next_card) = if len_due_cards > 0 {
        (1, due_cards[0].id)
    } else if len_new_cards > 0 && how_many_new_cards_done_today < 10 {
        (2, new_cards[0].id)
    } else if len_non_due_cards > 0 {
        (3, non_due_cards[0].id)
    } else {
        (4, 0)
    };
    let is_short_term = if let Some(card) = due_cards.get(0) {
        card.is_short_term
    } else {
        false
    };
    DoGetCardsReturn {
        len_due_cards,
        len_non_due_cards,
        len_new_cards,
        type_next_card,
        next_card,
        is_short_term,
    }
}

#[unsafe(export_name = "get-cards")]
unsafe fn get_cards(address: i64, time: i64) {
    let my_ptr = address as *mut u8;
    let set = &*CARDS_SET;
    let cards_map: HashMap<u64, Card> = set.read().unwrap().clone();
    let ret = do_get_cards(time as u64, cards_map);
    unsafe {
        write_bytes(my_ptr, ret.len_due_cards.to_le_bytes().to_vec());
    }
    unsafe {
        write_bytes(my_ptr.add(4), ret.len_non_due_cards.to_le_bytes().to_vec());
    }
    unsafe {
        write_bytes(my_ptr.add(8), ret.len_new_cards.to_le_bytes().to_vec());
    }
    unsafe {
        write_bytes(my_ptr.add(12), ret.type_next_card.to_le_bytes().to_vec());
    }
    unsafe {
        write_bytes(my_ptr.add(13), if ret.is_short_term{1}else{0_u8}.to_le_bytes().to_vec());
    }
    unsafe {
        write_bytes(my_ptr.add(14), ret.next_card.to_le_bytes().to_vec());
    }
}

unsafe fn write_bytes(p: *mut u8, val: Vec<u8>) {
    for (i, v) in val.into_iter().enumerate() {
        unsafe { p.add(i).write_unaligned(v) };
    }
}

#[unsafe(export_name = "init")]
unsafe extern "C" fn init() {}

#[unsafe(export_name = "alloc")]
unsafe extern "C" fn alloc(size: usize) -> *mut c_void {
    unsafe { malloc_rust::rust_malloc(size) }
}

#[unsafe(export_name = "dealloc")]
pub unsafe extern "C" fn dealloc(ptr: *mut c_void) {
    unsafe {
        malloc_rust::rust_free(ptr);
    }
}

#[derive(Debug, Clone)]
enum Card {
    None,
    ShortTerm {
        pos: u8,
        multiplier: u64,
        last_check: u64,
        last_graded_check: u64,
    },
    LongTerm {
        pos: u16,
        multiplier: u64,
        last_check: u64,
        last_graded_check: u64,
    },
}

impl Card {
    fn new() -> Self {
        Card::None
    }
    fn is_short_term(&self) -> bool {
        match self {
            Card::None | Card::LongTerm { .. } => false,
            Card::ShortTerm { .. } => true,
        }
    }
    fn get_pos(&self) -> Option<u32> {
        match self {
            Card::None => None,
            Card::ShortTerm { pos, .. } => Some((*pos).into()),
            Card::LongTerm { pos, .. } => Some(u32::from(*pos) + u32::from(u8::MAX)),
        }
    }
    fn last_seen(&self) -> Option<u64> {
        match self{
            Card::None => None,
            Card::ShortTerm {last_check,..} => Some(*last_check),
            Card::LongTerm { last_check, ..} => Some(*last_check),
        }
    }
    fn grade(&mut self, time: u64, correct: bool) {
        if !correct {
            if let Card::None = self {
                return;
            }
            let multiplyer = match self {
                Card::ShortTerm {
                    multiplier: multiplyer,
                    ..
                } => *multiplyer,
                Card::LongTerm {
                    multiplier: multiplyer,
                    ..
                } => *multiplyer + 1,
                Card::None => 0,
            };
            *self = Card::ShortTerm {
                pos: 0,
                multiplier: multiplyer,
                last_check: time,
                last_graded_check: time,
            };
        } else {
            match self.clone() {
                Card::None => {
                    *self = Card::ShortTerm {
                        pos: 0,
                        multiplier: 0,
                        last_graded_check: time,
                        last_check: time,
                    };
                }
                Card::ShortTerm {
                    pos,
                    multiplier: multiplyer,
                    last_graded_check,
                    ..
                } => {
                    // if we're not due, adjust last check and bail out
                    let due_time = get_interval_secs_short_term(pos) + last_graded_check;
                    if due_time > time {
                        *self = Card::ShortTerm {
                            pos,
                            multiplier: multiplyer,
                            last_check: time,
                            last_graded_check,
                        };
                        return;
                    }
                    // if the interval will be more than 24 hours, move to long term
                    if get_interval_secs_short_term(pos + 1) > 24 * 60 * 60 {
                        *self = Card::LongTerm {
                            pos: 0,
                            multiplier: multiplyer,
                            last_check: time,
                            last_graded_check: time,
                        }
                    } else {
                        *self = Card::ShortTerm {
                            pos: pos + 1,
                            multiplier: multiplyer,
                            last_check: time,
                            last_graded_check: time,
                        }
                    }
                }
                Card::LongTerm {
                    pos,
                    multiplier,
                    last_graded_check,
                    ..
                } => {
                    let (new_pos, last_graded_check) = {
                        let due_time = get_interval_secs_long_term(pos, multiplier);
                        if due_time.round() as u64 > time {
                            (pos + 1, time)
                        } else {
                            (pos + 1, last_graded_check)
                        }
                    };
                    *self = Card::LongTerm {
                        pos: new_pos,
                        multiplier,
                        last_graded_check,
                        last_check: time,
                    }
                }
            }
        }
    }

    fn get_interval(&self) -> Option<Duration> {
        match self {
            Card::None => None,
            Card::ShortTerm { pos, .. } => {
                Some(Duration::from_secs(get_interval_secs_short_term(*pos)))
            }
            Card::LongTerm {
                pos,
                multiplier: multiplyer,
                ..
            } => Some(Duration::from_secs_f64(
                get_interval_secs_long_term(*pos, *multiplyer) * 24.0 * 60.0 * 60.0,
            )),
        }
    }
    fn get_due_timestamp(&self) -> Option<u64> {
        let interval = self.get_interval();
        let interval: u64 = if let Some(interval) = interval {
            interval.as_secs()
        } else {
            return None;
        };
        match self {
            Card::None => None,
            Card::ShortTerm { last_check, .. } => Some(last_check + interval),
            Card::LongTerm { last_check, .. } => Some(last_check + interval),
        }
    }
}

fn get_interval_secs_short_term(pos: u8) -> u64 {
    u64::pow(4, u32::from(pos)) * 15 * 60
}

fn get_interval_secs_long_term(pos: u16, multiplier: u64) -> f64 {
    let power = (2.0_f64 * 0.9_f64.powf((multiplier as f64).powf(0.5))).powf(pos as f64);
    let days = 365_f64.min(power);
    days
}

#[cfg(test)]
mod test {
    use core::ffi::c_void;
    use std::collections::HashMap;

    use crate::{Card, add_card, grade_card};

    #[test]
    fn a() {
        let mut a = Card::new();
        a.grade(1, true);
        eprintln!("{:?} minutes", a.get_interval().unwrap().as_secs() / 60);
        a.grade(2, true);
        eprintln!("{:?} minutes", a.get_interval().unwrap().as_secs() / 60);
        a.grade(3, true);
        eprintln!("{:?} minutes", a.get_interval().unwrap().as_secs() / 60);
    }

    const SECONDS_IN_HOURS: u64 = 60 * 60;
    const SECONDS_IN_DAYS: u64 = 60 * 60 * 24;

    #[test]
    fn b() {
        let mut a = Card::new();
        let mut cur_time = 0;
        for i in 1..100 {
            a.grade(cur_time, i % 20 != 19);
            let interval = a.get_interval().map(|t| t.as_secs()).unwrap_or_default() + 1;
            cur_time += interval;

            let (display_time, units) = match interval {
                0..60 => (interval, "seconds"),
                60..SECONDS_IN_HOURS => (interval / 60, "minutes"),
                SECONDS_IN_HOURS..SECONDS_IN_DAYS => (interval / 60 / 60, "hours"),
                SECONDS_IN_DAYS.. => (interval / 60 / 60 / 24, "days"),
            };
            eprintln!("{a:?}, {display_time} {units}");
        }
    }
    #[test]
    fn c() {
        let ptr = unsafe { super::alloc(21) } as *mut u8;
        add_card(100);
        add_card(101);
        add_card(102);
        let start_time = 30 * 24 * 60 * 60;
        grade_card(100, 1 + start_time, 1);
        let ret = {
            let set = &*super::CARDS_SET;
            let cards_map: HashMap<u64, Card> = set.read().unwrap().clone();
            super::do_get_cards(2 + start_time, cards_map)
        };

        unsafe { super::get_cards(ptr as i64, 2 + start_time as i64) };
        let len_due_cards = unsafe { u32::from_le_bytes(read_bytes(ptr)) };
        let len_non_due_cards = unsafe { u32::from_le_bytes(read_bytes(ptr.add(4))) };
        let len_new_cards = unsafe { u32::from_le_bytes(read_bytes(ptr.add(8))) };
        let type_next_card = unsafe { u8::from_le_bytes(read_bytes(ptr.add(12))) };
        let next_card = unsafe { u64::from_le_bytes(read_bytes(ptr.add(13))) };
        assert_eq!(ret.len_non_due_cards, 1);
        assert_eq!(ret.len_due_cards, 0);
        assert_eq!(ret.len_new_cards, 2);
        assert_eq!(ret.type_next_card, 2);
        assert_eq!(ret.next_card, 102);
        assert_eq!(len_non_due_cards, ret.len_non_due_cards);
        assert_eq!(len_due_cards, ret.len_due_cards);
        assert_eq!(len_new_cards, ret.len_new_cards);
        assert_eq!(type_next_card, ret.type_next_card.into());
        assert_eq!(next_card, ret.next_card);
        unsafe {
            super::dealloc(ptr as *mut c_void);
        }
    }

    #[test]
    fn d() {
        let ptr = unsafe { super::alloc(21) } as *mut u8;
        add_card(100);
        add_card(101);
        add_card(102);
        let start_time = 30 * 24 * 60 * 60;
        grade_card(100, 1 + start_time, 1);
        let new_time = 15 * 60 * 60 + 1 + start_time;
        let ret = {
            let set = &*super::CARDS_SET;
            let cards_map: HashMap<u64, Card> = set.read().unwrap().clone();
            super::do_get_cards(new_time, cards_map)
        };

        unsafe { super::get_cards(ptr as i64, new_time as i64) };
        let len_due_cards = unsafe { u32::from_le_bytes(read_bytes(ptr)) };
        let len_non_due_cards = unsafe { u32::from_le_bytes(read_bytes(ptr.add(4))) };
        let len_new_cards = unsafe { u32::from_le_bytes(read_bytes(ptr.add(8))) };
        let type_next_card = unsafe { u8::from_le_bytes(read_bytes(ptr.add(12))) };
        let next_card = unsafe { u64::from_le_bytes(read_bytes(ptr.add(13))) };
        assert_eq!(ret.len_non_due_cards, 0);
        assert_eq!(ret.len_due_cards, 1);
        assert_eq!(ret.len_new_cards, 2);
        assert_eq!(ret.type_next_card, 1);
        assert_eq!(ret.next_card, 100);
        assert_eq!(len_non_due_cards, ret.len_non_due_cards);
        assert_eq!(len_due_cards, ret.len_due_cards);
        assert_eq!(len_new_cards, ret.len_new_cards);
        assert_eq!(type_next_card, ret.type_next_card.into());
        assert_eq!(next_card, ret.next_card);
        unsafe {
            super::dealloc(ptr as *mut c_void);
        }
    }

    #[test]
    fn e() {
        let ptr = unsafe { super::alloc(21) } as *mut u8;
        for i in 1..100 {
            add_card(100 + i);
        }
        let start_time = 30 * 24 * 60 * 60;

        for i in 0..11 {
            grade_card(100 + i, 1 + start_time + i, 1);
        }
        let new_time = 24 * 60 * 60 - 1 + start_time;
        let ret = {
            let set = &*super::CARDS_SET;
            let cards_map: HashMap<u64, Card> = set.read().unwrap().clone();
            super::do_get_cards(new_time, cards_map)
        };

        unsafe { super::get_cards(ptr as i64, new_time as i64) };
        let len_due_cards = unsafe { u32::from_le_bytes(read_bytes(ptr)) };
        let len_non_due_cards = unsafe { u32::from_le_bytes(read_bytes(ptr.add(4))) };
        let len_new_cards = unsafe { u32::from_le_bytes(read_bytes(ptr.add(8))) };
        let type_next_card = unsafe { u8::from_le_bytes(read_bytes(ptr.add(12))) };
        let next_card = unsafe { u64::from_le_bytes(read_bytes(ptr.add(13))) };
        assert_eq!(ret.len_non_due_cards, 0);
        assert_eq!(ret.len_due_cards, 10);
        assert_eq!(ret.len_new_cards, 0);
        assert_eq!(ret.type_next_card, 1);
        assert_eq!(ret.next_card, 110);
        assert_eq!(len_non_due_cards, ret.len_non_due_cards);
        assert_eq!(len_due_cards, ret.len_due_cards);
        assert_eq!(len_new_cards, ret.len_new_cards);
        assert_eq!(type_next_card, ret.type_next_card.into());
        assert_eq!(next_card, ret.next_card);
        unsafe {
            super::dealloc(ptr as *mut c_void);
        }
    }

    #[test]
    fn f() {
        let ptr = unsafe { super::alloc(21) } as *mut u8;
        for i in 1..100 {
            add_card(100 + i);
        }
        let start_time = 30 * 24 * 60 * 60;

        for i in 0..11 {
            grade_card(100 + i, 1 + start_time + i, 1);
        }
        let new_time = 24 * 60 * 60 + 200 + start_time;
        let ret = {
            let set = &*super::CARDS_SET;
            let cards_map: HashMap<u64, Card> = set.read().unwrap().clone();
            super::do_get_cards(new_time, cards_map)
        };

        unsafe { super::get_cards(ptr as i64, new_time as i64) };
        let len_due_cards = unsafe { u32::from_le_bytes(read_bytes(ptr)) };
        let len_non_due_cards = unsafe { u32::from_le_bytes(read_bytes(ptr.add(4))) };
        let len_new_cards = unsafe { u32::from_le_bytes(read_bytes(ptr.add(8))) };
        let type_next_card = unsafe { u8::from_le_bytes(read_bytes(ptr.add(12))) };
        let next_card = unsafe { u64::from_le_bytes(read_bytes(ptr.add(13))) };
        assert_eq!(ret.len_non_due_cards, 0);
        assert_eq!(ret.len_due_cards, 10);
        assert_eq!(ret.len_new_cards, 10);
        assert_eq!(ret.type_next_card, 1);
        assert_eq!(ret.next_card, 110);
        assert_eq!(len_non_due_cards, ret.len_non_due_cards);
        assert_eq!(len_due_cards, ret.len_due_cards);
        assert_eq!(len_new_cards, ret.len_new_cards);
        assert_eq!(type_next_card, ret.type_next_card.into());
        assert_eq!(next_card, ret.next_card);
        unsafe {
            super::dealloc(ptr as *mut c_void);
        }
    }
    unsafe fn read_bytes<const N: usize>(p: *mut u8) -> [u8; N] {
        let mut m = [0; N];
        for i in 0..N {
            m[i] = unsafe { *p.add(i) };
        }
        m
    }
}
