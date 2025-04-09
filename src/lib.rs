use core::{ffi::c_void, time::Duration};
use std::{
    collections::HashMap,
    sync::{LazyLock, RwLock, RwLockWriteGuard},
};

static CARDS_SET: LazyLock<RwLock<HashMap<u64, Card>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

static WHEN_WAS_CARD_FIRST_DONE: LazyLock<RwLock<HashMap<u64, u64>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

const DUE_CARD: u8 = 1;
const NEW_CARD: u8 = 2;
const NON_DUE_CARD: u8 = 3;
const ERROR_CARD: u8 = 4;
mod tests;

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

#[derive(Debug, Clone)]
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

#[derive(Debug, Eq, PartialEq)]
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

    let len_due_cards = due_cards.len().try_into().unwrap();
    let len_non_due_cards = non_due_cards.len().try_into().unwrap();
    let cards_left = 10_u32.saturating_sub(how_many_new_cards_done_today as u32);
    let len_new_cards = (new_cards.len() as u32).min(cards_left);
    let (type_next_card, next_card) = if len_due_cards > 0 {
        let card: ThickCard = due_cards
            .iter()
            .fold(&due_cards[0], |a: &ThickCard, b: &ThickCard| {
                if a.pos < b.pos {
                    a
                } else if a.pos > b.pos {
                    b
                } else if a.last_seen < b.last_seen {
                    a
                } else if a.last_seen > b.last_seen {
                    b
                } else if a.id > b.id {
                    a
                } else {
                    b
                }
            })
            .clone();
        (DUE_CARD, card.id)
    } else if len_new_cards > 0 && how_many_new_cards_done_today < 10 {
        let card: ThickCard = new_cards
            .iter()
            .fold(
                &new_cards[0],
                |a: &ThickCard, b: &ThickCard| {
                    if a.id > b.id { a } else { b }
                },
            )
            .clone();
        (NEW_CARD, card.id)
    } else if len_non_due_cards > 0 {
        let card: ThickCard = non_due_cards
            .iter()
            .fold(&non_due_cards[0], |a: &ThickCard, b: &ThickCard| {
                if a.last_seen < b.last_seen {
                    a
                } else if a.last_seen > b.last_seen {
                    b
                } else if a.id > b.id {
                    a
                } else {
                    b
                }
            })
            .clone();
        (NON_DUE_CARD, card.id)
    } else {
        (ERROR_CARD, 0)
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
    let set: &_ = &*CARDS_SET;
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
        write_bytes(
            my_ptr.add(13),
            if ret.is_short_term { 1 } else { 0_u8 }
                .to_le_bytes()
                .to_vec(),
        );
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
extern "C" fn init() {
    let mut c = CARDS_SET.write().unwrap();
    c.clear();
    assert_eq!(c.len(), 0);
    let mut c = WHEN_WAS_CARD_FIRST_DONE.write().unwrap();
    (*c).clear();
    assert_eq!(c.len(), 0);
}

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
    Wrong {
        pos: u8,
        multiplier: u64,
        last_check: u64,
        last_graded_check: u64,
    },
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
            Card::ShortTerm { .. } | Card::Wrong { .. } => true,
        }
    }
    fn get_pos(&self) -> Option<u32> {
        match self {
            Card::None => None,
            Card::Wrong { pos, .. } | Card::ShortTerm { pos, .. } => Some((*pos).into()),
            Card::LongTerm { pos, .. } => Some(u32::from(*pos) + u32::from(u8::MAX)),
        }
    }
    fn last_seen(&self) -> Option<u64> {
        match self {
            Card::None => None,
            Card::Wrong { last_check, .. }
            | Card::ShortTerm { last_check, .. }
            | Card::LongTerm { last_check, .. } => Some(*last_check),
        }
    }
    fn grade(&mut self, time: u64, correct: bool) {
        if !correct {
            let multiplyer = match self {
                Card::ShortTerm { multiplier, .. } | Card::Wrong { multiplier, .. } => *multiplier,
                Card::LongTerm {
                    multiplier: multiplyer,
                    ..
                } => *multiplyer + 1,
                Card::None => 0,
            };
            *self = Card::Wrong {
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
                Card::Wrong {
                    pos,
                    last_graded_check,
                    multiplier,
                    ..
                } => {
                    let due_time = 15 + last_graded_check;
                    if time > due_time {
                        *self = Card::ShortTerm {
                            pos: pos + 1,
                            multiplier,
                            last_check: time,
                            last_graded_check: time,
                        }
                    }
                }
                Card::ShortTerm {
                    pos,
                    multiplier,
                    last_graded_check,
                    ..
                } => {
                    // if we're not due, adjust last check and bail out
                    let due_time =
                        (get_interval_secs_short_term(pos)) + last_graded_check;
                    if due_time > time {
                        *self = Card::ShortTerm {
                            pos,
                            multiplier,
                            last_check: time,
                            last_graded_check,
                        };
                        return;
                    }
                    // if the interval will be more than 24 hours, move to long term
                    if get_interval_secs_short_term(pos + 1) > 24 * 60 * 60 {
                        *self = Card::LongTerm {
                            pos: 0,
                            multiplier,
                            last_check: time,
                            last_graded_check: time,
                        }
                    } else {
                        *self = Card::ShortTerm {
                            pos: pos + 1,
                            multiplier,
                            last_check: time,
                            last_graded_check: (time),
                        }
                    }
                }
                Card::LongTerm {
                    pos,
                    multiplier,
                    last_graded_check,
                    ..
                } => {
                    let (new_pos, new_last_graded_check) = {
                        let due_time = get_interval_secs_long_term(pos, multiplier)+last_graded_check;
                        if due_time > time {
                            (pos, last_graded_check)
                        } else {
                            (pos + 1, time)
                        }
                    };
                    *self = Card::LongTerm {
                        pos: new_pos,
                        multiplier,
                        last_graded_check: new_last_graded_check,
                        last_check: time,
                    }
                }
            }
        }
    }

    fn get_interval(&self) -> Option<Duration> {
        match self {
            Card::None => None,
            Card::Wrong { .. } => Some(Duration::from_secs(15)),
            Card::ShortTerm { pos, .. } => {
                Some(Duration::from_secs(get_interval_secs_short_term(*pos)))
            }
            Card::LongTerm {
                pos,
                multiplier: multiplyer,
                ..
            } => Some(Duration::from_secs(get_interval_secs_long_term(
                *pos,
                *multiplyer,
            ))),
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
            Card::Wrong {
                last_graded_check, ..
            }
            | Card::ShortTerm {
                last_graded_check, ..
            }
            | Card::LongTerm {
                last_graded_check, ..
            } => Some((last_graded_check) + (interval)),
        }
    }
}

fn get_interval_secs_short_term(pos: u8) -> u64 {
    u64::pow(4, u32::from(pos)) * 15 * 60
}

fn get_interval_secs_long_term(pos: u16, multiplier: u64) -> u64 {
    let power = (2.0_f64 * 0.9_f64.powf((multiplier as f64).powf(0.5))).powf(pos as f64);
    let days = 365_f64.min(power);
    (days * 86400.0).floor() as u64
}
