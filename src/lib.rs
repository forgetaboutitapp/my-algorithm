use crate::scheduler::from_scheduler::Response;
use crate::scheduler::response_correct::CardToView;
use crate::scheduler::{FromScheduler, ResponseCorrect, ResponseWrong, ToScheduler};
use chrono::{DateTime, Utc};
use core::{convert::From, ffi::c_void, time::Duration};
use prost::Message;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::any::Any;
use std::collections::HashSet;
use std::collections::{BTreeMap, HashMap};
use std::hash::{Hash, Hasher};


mod unit_tests;
mod integration_tests;

/// Split testing configuration for A/B testing different algorithm parameters
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct SplitTestConfig {
    /// Unique identifier for this experiment
    pub experiment_id: String,
    /// Percentage of users assigned to variant A (0-100)
    pub variant_a_percentage: u8,
    /// Algorithm parameters for variant A
    pub variant_a_params: AlgorithmParams,
    /// Algorithm parameters for variant B
    pub variant_b_params: AlgorithmParams,
    /// Start date for the experiment
    #[serde(serialize_with = "serialize_dt")]
    #[serde(deserialize_with = "deserialize_dt")]
    pub start_date: DateTime<Utc>,
    /// End date for the experiment
    #[serde(serialize_with = "serialize_dt")]
    #[serde(deserialize_with = "deserialize_dt")]
    pub end_date: DateTime<Utc>,
    /// Whether the experiment is currently active
    pub is_active: bool,
}

/// Algorithm parameters that can be varied in split tests
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct AlgorithmParams {
    /// Short-term interval multiplier (default: 4)
    pub short_term_multiplier: Option<f64>,
    /// Long-term interval base multiplier (default: 2.0)
    pub long_term_base_multiplier: Option<f64>,
    /// Long-term forgetting curve factor (default: 0.9)
    pub forgetting_curve_factor: Option<f64>,
    /// Maximum interval in days (default: 365)
    pub max_interval_days: Option<u16>,
    /// Wrong answer interval in seconds (default: 15)
    pub wrong_answer_interval: Option<u64>,
    /// Initial interval in seconds (default: 15 * 60)
    pub initial_interval: Option<u64>,
}

impl Default for AlgorithmParams {
    fn default() -> Self {
        Self {
            short_term_multiplier: Some(4.0),
            long_term_base_multiplier: Some(2.0),
            forgetting_curve_factor: Some(0.9),
            max_interval_days: Some(365),
            wrong_answer_interval: Some(15),
            initial_interval: Some(15 * 60),
        }
    }
}

/// Metrics collected during split testing
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct SplitTestMetrics {
    /// User identifier for consistent assignment
    pub user_id: String,
    /// Which variant the user was assigned to
    pub variant: Variant,
    /// Card performance metrics
    pub card_metrics: CardMetrics,
    /// Session metrics
    pub session_metrics: SessionMetrics,
    /// Timestamp when metrics were collected
    #[serde(serialize_with = "serialize_dt")]
    #[serde(deserialize_with = "deserialize_dt")]
    pub timestamp: DateTime<Utc>,
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub enum Variant {
    A,
    B,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct CardMetrics {
    /// Total cards reviewed
    pub total_cards: u32,
    /// Cards answered correctly
    pub correct_cards: u32,
    /// Cards answered incorrectly
    pub incorrect_cards: u32,
    /// Average response time in milliseconds
    pub avg_response_time_ms: Option<u64>,
    /// Cards that graduated to long-term
    pub graduated_cards: u32,
    /// Cards that were reset due to errors
    pub reset_cards: u32,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct SessionMetrics {
    /// Session duration in seconds
    pub duration_seconds: u64,
    /// Number of cards reviewed in this session
    pub cards_reviewed: u32,
    /// User engagement score (0-100)
    pub engagement_score: Option<u8>,
    /// Whether the session was completed
    pub completed: bool,
}

/// Split testing manager for handling experiments
pub struct SplitTestManager {
    configs: HashMap<String, SplitTestConfig>,
    user_assignments: HashMap<String, HashMap<String, Variant>>, // user_id -> experiment_id -> variant
}

impl SplitTestManager {
    pub fn new() -> Self {
        Self {
            configs: HashMap::new(),
            user_assignments: HashMap::new(),
        }
    }

    /// Add or update an experiment configuration
    pub fn add_experiment(&mut self, config: SplitTestConfig) {
        self.configs.insert(config.experiment_id.clone(), config);
    }

    /// Assign a user to a variant for a specific experiment
    pub fn assign_user_to_variant(&mut self, user_id: &str, experiment_id: &str) -> Option<Variant> {
        if let Some(config) = self.configs.get(experiment_id) {
            if !config.is_active {
                return None;
            }

            let now = Utc::now();
            if now < config.start_date || now > config.end_date {
                return None;
            }

            // Check if user already has an assignment
            if let Some(experiments) = self.user_assignments.get(user_id) {
                if let Some(variant) = experiments.get(experiment_id) {
                    return Some(variant.clone());
                }
            }

            // Generate consistent assignment based on user_id hash
            let variant = self.generate_variant_assignment(user_id, config);
            
            // Store the assignment
            self.user_assignments
                .entry(user_id.to_string())
                .or_insert_with(HashMap::new)
                .insert(experiment_id.to_string(), variant.clone());

            Some(variant)
        } else {
            None
        }
    }

    /// Generate a consistent variant assignment based on user_id
    fn generate_variant_assignment(&self, user_id: &str, config: &SplitTestConfig) -> Variant {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        user_id.hash(&mut hasher);
        config.experiment_id.hash(&mut hasher);
        
        let hash = hasher.finish();
        let assignment_value = hash % 100;
        
        if assignment_value < config.variant_a_percentage as u64 {
            Variant::A
        } else {
            Variant::B
        }
    }

    /// Get algorithm parameters for a user in a specific experiment
    pub fn get_algorithm_params(&self, user_id: &str, experiment_id: &str) -> Option<AlgorithmParams> {
        if let Some(variant) = self.user_assignments
            .get(user_id)
            .and_then(|experiments| experiments.get(experiment_id)) {
            
            if let Some(config) = self.configs.get(experiment_id) {
                match variant {
                    Variant::A => Some(config.variant_a_params.clone()),
                    Variant::B => Some(config.variant_b_params.clone()),
                }
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Record metrics for split testing analysis
    pub fn record_metrics(&mut self, metrics: SplitTestMetrics) {
        // In a real implementation, this would store metrics to a database
        // For now, we'll just validate the metrics structure
        println!("Recording metrics for user {} variant {:?}: {:?}", 
                metrics.user_id, metrics.variant, metrics.card_metrics);
    }
}

#[derive(Deserialize, Serialize)]
pub struct CustomArgs {
    /// # Max New Cards Per Day
    /// How many new cards should one do per day (max is 1024).
    max_amount_of_new_cards_per_day: Option<u32>,
    /// # Starting Hour
    /// At what hour of the day (where 0 is midnight and 23 is 11 PM) does the day start
    starting_hour: Option<u8>,
    /// # Exam Days
    /// What days will various tags be tested
    exam_days: Option<Vec<TagToDate>>,
    /// # Split Testing Configuration
    /// Configuration for A/B testing different algorithm parameters
    split_test_config: Option<SplitTestConfig>,
    /// # User ID for Split Testing
    /// Unique identifier for consistent experiment assignment
    user_id: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct TagToDate {
    tags: Vec<String>,
    #[serde(serialize_with = "serialize_dt")]
    #[serde(deserialize_with = "deserialize_dt")]
    date: DateTime<Utc>,
}
pub fn serialize_dt<S>(dt: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    dt.to_rfc3339().serialize(serializer)
}

pub fn deserialize_dt<'de, D>(
    deserializer: D,
) -> Result<DateTime<Utc>, <D as Deserializer<'de>>::Error>
where
    D: Deserializer<'de>,
{
    let buf = String::deserialize(deserializer)?;
    DateTime::parse_from_rfc3339(&buf)
        .map_err(|e| serde::de::Error::custom(e.to_string()))
        .map(|a| a.with_timezone(&Utc))
}

#[derive(Debug, Clone)]
struct ThickCard {
    id: u32,
    last_seen: Option<u64>,
    is_short_term: bool,
    pos: Option<u32>,
}

pub mod scheduler {
    use core::{concat, env, include};

    include!(concat!(env!("OUT_DIR"), "/scheduler.rs"));
}
type Timestamp = u64;
type CardId = u32;

#[derive(Clone, Debug)]
struct LogSnapshot {
    ts: Timestamp,
    is_correct: bool,
    id: CardId,
}

fn safe_proc_cards(
    cards: Vec<scheduler::Card>,
    tags_to_ask: HashSet<String>,
    n_unix: Timestamp,
    custom_args: CustomArgs,
    should_do_new_card: bool
) -> ResponseCorrect {
    let mut card_history: Vec<LogSnapshot> = Vec::new();
    let mut available_cards: HashSet<CardId> = HashSet::new();
    let mut when_was_card_first_done = HashMap::new();
    let cards_to_hash: HashMap<u32, Vec<String>> =
        cards.iter().map(|a| (a.id, a.tags.clone())).collect();
    for card in &cards {
        if card.tags.iter().any(|tag| tags_to_ask.contains(tag)) {
            available_cards.insert(card.id);
        }
        for ts in &card.card_records {
            let when = ts.when.unwrap().seconds as Timestamp;

            when_was_card_first_done
                .entry(card.id)
                .and_modify(|e| {
                    if *e > when {
                        *e = when
                    }
                })
                .or_insert(when);

            card_history.push(LogSnapshot {
                ts: when as Timestamp,
                is_correct: ts.correct,
                id: card.id,
            });
        }
    }
    // sort card history by chronological order, lowest to highest
    card_history.sort_by(|a, b| a.ts.cmp(&b.ts));

    let mut graded_cards: HashMap<CardId, Card> = HashMap::new();
    for log in &card_history {
        graded_cards.entry(log.id).or_insert(Card::new());
        graded_cards.entry(log.id).and_modify(|card| {
            card.grade(log.ts, log.is_correct);
        });
    }

    let mut due_cards: Vec<ThickCard> = vec![];
    let mut non_due_cards: Vec<ThickCard> = vec![];

    let how_many_new_cards_done_today = when_was_card_first_done
        .iter()
        .filter(|(_, s)| **s + 24 * 60 * 60 >= n_unix)
        .count();

    let mut future_card_heatmap: BTreeMap<u32, u64> = graded_cards
        .iter()
        .filter_map(|(id, c)| c.get_due_timestamp().map(|t| (*id, t)))
        .collect();
    for (k, v) in graded_cards {
        let t = v.get_due_timestamp();
        if let Some(timestamp) = t {
            future_card_heatmap.entry(k).or_insert(timestamp);
        }

        let new_thick_card = ThickCard {
            id: k,
            last_seen: v.last_seen(),
            is_short_term: v.is_short_term(),
            pos: v.get_pos(),
        };

        if let Some(t) = t {
            if n_unix > t {
                due_cards.push(new_thick_card);
            } else {
                non_due_cards.push(new_thick_card);
            }
        }
    }

    due_cards.sort_by(|a, b| a.last_seen.cmp(&b.last_seen));
    non_due_cards.sort_by(|a, b| a.last_seen.cmp(&b.last_seen));

    let new_cards: Vec<ThickCard> = cards
        .iter()
        .filter(|a| a.card_records.is_empty())
        .map(|a| ThickCard {
            id: a.id,
            last_seen: None,
            is_short_term: false,
            pos: None,
        })
        .collect();

    let new_cards: Vec<ThickCard> = new_cards
        .into_iter()
        .filter(|x| available_cards.contains(&x.id))
        .collect();
    let due_cards: Vec<ThickCard> = due_cards
        .into_iter()
        .filter(|x| available_cards.contains(&x.id))
        .collect();
    let non_due_cards: Vec<ThickCard> = non_due_cards
        .into_iter()
        .filter(|x| available_cards.contains(&x.id))
        .collect();

    let amnt_new_cards = new_cards.len().try_into().unwrap();
    let amnt_due_cards = due_cards.len().try_into().unwrap();
    let amnt_non_due_cards = non_due_cards.len().try_into().unwrap();
    let cards_left = if !should_do_new_card {
        custom_args
            .max_amount_of_new_cards_per_day
            .unwrap_or(10)
            .saturating_sub(how_many_new_cards_done_today as u32)
    } else {
        1
    };
    let len_new_cards = (new_cards.len() as u32).min(cards_left);
    let card_to_view = if amnt_due_cards > 0 {
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
                } else if a.id < b.id {
                    b
                } else {
                    a
                }
            })
            .clone();
        CardToView::DueCard(scheduler::Card {
            id: card.id,
            card_records: vec![],
            tags: cards_to_hash[&card.id].clone(),
        })
    } else if len_new_cards > 0 && (should_do_new_card || how_many_new_cards_done_today < 10) {
        let card: ThickCard = new_cards
            .iter()
            .fold(
                &new_cards[0],
                |a: &ThickCard, b: &ThickCard| {
                    if a.id > b.id { a } else { b }
                },
            )
            .clone();
        CardToView::NewCard(scheduler::Card {
            id: card.id,
            tags: cards_to_hash[&card.id].clone(),
            card_records: vec![],
        })
    } else if amnt_non_due_cards > 0 {
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

        CardToView::NonDueCard(scheduler::Card {
            id: card.id,
            tags: cards_to_hash[&card.id].clone(),
            card_records: vec![],
        })
    } else {
        CardToView::NoCard(Default::default())
    };
    let is_short_term = if let Some(card) = due_cards.first() {
        card.is_short_term
    } else {
        false
    };

    ResponseCorrect {
        amnt_new_cards,
        amnt_due_cards,
        amnt_non_due_cards,
        future_card_heatmap,
        card_to_view: Some(card_to_view),
        is_short_term,
    }
}
fn proc_cards(
    cards: Vec<scheduler::Card>,
    tags_to_ask: HashSet<String>,
    custom_params: String,
    n_unix: Timestamp,
    should_do_new_card: bool,
) -> Response {
    let custom_params: CustomArgs = if custom_params.is_empty() {
        CustomArgs {
            max_amount_of_new_cards_per_day: Default::default(),
            starting_hour: Default::default(),
            exam_days: None,
        }
    } else {
        match serde_json::from_str(&custom_params) {
            Ok(json) => json,
            Err(err) => {
                return Response::BadValue(ResponseWrong {
                    to_log: format!("Unable to parse custom params {custom_params:?}: {err:#?}"),
                    to_user: format!("Unable to parse custom params {custom_params:?}"),
                });
            }
        }
    };

    let res =
        std::panic::catch_unwind(|| safe_proc_cards(cards, tags_to_ask, n_unix, custom_params, should_do_new_card));
    match res {
        Ok(good) => Response::GoodValue(good),
        Err(err) => Response::BadValue(ResponseWrong {
            to_log: err_to_string(err),
            to_user: INTERNAL_ERROR.to_string(),
        }),
    }
}

fn err_to_string<T: Into<Box<dyn Any + Send>>>(err: T) -> String {
    let err = err.into();
    if let Some(message) = err.downcast_ref::<&str>() {
        message.to_string()
    } else if let Some(message) = err.downcast_ref::<String>() {
        message.to_string()
    } else {
        "Unknown error".to_string()
    }
}

const INTERNAL_ERROR: &str = "Internal Error, please look at the server logs for more information.";

#[unsafe(export_name = "get-schema")]
extern "C" fn get_schema(_p: u32, _l: u32) -> u64 {
    let s = "s";
    let buf = s.as_bytes();
    let p = unsafe { alloc(buf.len()) };
    unsafe { write_bytes(p as *mut u8, buf) };
    combine_u32_to_u64(p as u32, s.len() as u32)
}
#[unsafe(export_name = "get-cards")]
fn get_cards(ptr: u64, size: u32, time: u64) -> u64 {
    let v = unsafe { read_bytes(ptr as *mut u8, size) };
    let m = ToScheduler::decode(v.as_slice());
    match m {
        Ok(to_scheduler) => {
            let c = proc_cards(
                to_scheduler.cards,
                to_scheduler.tags_to_query.iter().cloned().collect(),
                to_scheduler.custom_params,
                time,
                to_scheduler.get_new_questions,
            );
            let return_val = FromScheduler { response: Some(c) };
            let mut buf = Vec::with_capacity(return_val.encoded_len());
            return_val.encode(&mut buf).unwrap();
            let p = unsafe { alloc(buf.len()) };
            unsafe { write_bytes(p as *mut u8, &buf) };
            combine_u32_to_u64(p as u32, buf.len() as u32)
        }
        Err(err) => {
            let return_val = FromScheduler {
                response: Some(Response::BadValue(ResponseWrong {
                    to_log: err.to_string(),
                    to_user: INTERNAL_ERROR.to_string(),
                })),
            };
            let mut buf = Vec::with_capacity(return_val.encoded_len());
            return_val.encode(&mut buf).unwrap();
            let p = unsafe { alloc(buf.len()) };
            unsafe { write_bytes(p as *mut u8, &buf) };
            combine_u32_to_u64(p as u32, size)
        }
    }
}

fn combine_u32_to_u64(high: u32, low: u32) -> u64 {
    ((high as u64) << 32) | (low as u64)
}

unsafe fn write_bytes(p: *mut u8, val: &[u8]) {
    for (i, v) in val.iter().enumerate() {
        unsafe { p.add(i).write_unaligned(*v) };
    }
}

unsafe fn read_bytes(p: *mut u8, size: u32) -> Vec<u8> {
    let mut v = Vec::with_capacity(size as usize);
    for i in 0..size {
        v.push(unsafe { p.add(i as usize).read() });
    }
    v
}

#[unsafe(export_name = "init")]
extern "C" fn init() {}

#[unsafe(export_name = "alloc")]
unsafe extern "C" fn alloc(size: usize) -> *mut c_void {
    unsafe { malloc_rust::rust_malloc(size) }
}

/// UnAllocate code
///
/// # Arguments
///
/// * `ptr`: the pointer to free
///
/// # Safety
/// ptr must be valid
#[unsafe(export_name = "dealloc")]
pub unsafe extern "C" fn dealloc(ptr: *mut c_void) {
    unsafe {
        malloc_rust::rust_free(ptr);
    }
}

#[derive(Debug, Clone)]
enum Card {
    None {
        multiplier: u64,
        last_check: Option<u64>,
        last_graded_check: Option<u64>,
    },
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
        Card::None {
            multiplier: 0,
            last_check: None,
            last_graded_check: None,
        }
    }
    fn is_short_term(&self) -> bool {
        match self {
            Card::None { .. } | Card::LongTerm { .. } => false,
            Card::ShortTerm { .. } | Card::Wrong { .. } => true,
        }
    }

    fn get_pos(&self) -> Option<u32> {
        match self {
            Card::None { .. } => None,
            Card::Wrong { pos, .. } | Card::ShortTerm { pos, .. } => Some((*pos).into()),
            Card::LongTerm { pos, .. } => Some(u32::from(*pos) + u32::from(u8::MAX)),
        }
    }
    fn last_seen(&self) -> Option<u64> {
        match self {
            Card::None { last_check, .. } => *last_check,
            Card::Wrong { last_check, .. }
            | Card::ShortTerm { last_check, .. }
            | Card::LongTerm { last_check, .. } => Some(*last_check),
        }
    }
    fn grade(&mut self, time: u64, correct: bool) {
        if !correct {
            let interval = self.get_interval();
            let multiplier = match self {
                Card::ShortTerm { multiplier, .. }
                | Card::Wrong { multiplier, .. }
                | Card::None { multiplier, .. } => *multiplier,
                Card::LongTerm {
                    multiplier,
                    last_check,
                    ..
                } => {
                    if time - *last_check > 2 * interval.unwrap().as_secs() {
                        *multiplier + 1
                    } else {
                        *multiplier
                    }
                }
            };
            let should_go_back = if let Card::LongTerm { last_check, .. } = self {
                time - *last_check > 2 * interval.unwrap().as_secs()
            } else {
                false
            };

            if should_go_back {
                *self = Card::None {
                    multiplier,
                    last_check: Some(time),
                    last_graded_check: Some(time),
                };
            } else {
                *self = Card::Wrong {
                    pos: 0,
                    multiplier,
                    last_check: time,
                    last_graded_check: time,
                };
            }
        } else {
            match *self {
                Card::None { multiplier, .. } => {
                    *self = Card::ShortTerm {
                        pos: 0,
                        multiplier,
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
                    } else {
                        *self = Card::Wrong {
                            pos: pos + 1,
                            multiplier,
                            last_check: time,
                            last_graded_check,
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
                    let due_time = (get_interval_secs_short_term(pos)) + last_graded_check;
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
                    let (new_pos, new_last_graded_check) = {
                        let due_time =
                            get_interval_secs_long_term(pos, multiplier) + last_graded_check;
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
        assert!(self.last_seen() >= self.last_graded_check());
    }

    fn last_graded_check(&self) -> Option<u64> {
        match *self {
            Card::None {
                last_graded_check, ..
            } => last_graded_check,
            Card::Wrong {
                last_graded_check, ..
            } => last_graded_check.into(),
            Card::ShortTerm {
                last_graded_check, ..
            } => last_graded_check.into(),
            Card::LongTerm {
                last_graded_check, ..
            } => last_graded_check.into(),
        }
    }
    fn get_interval(&self) -> Option<Duration> {
        match self {
            Card::None { .. } => None,
            Card::Wrong { .. } => Some(Duration::from_secs(15)),
            Card::ShortTerm { pos, .. } => {
                Some(Duration::from_secs(get_interval_secs_short_term(*pos)))
            }
            Card::LongTerm {
                pos, multiplier, ..
            } => Some(Duration::from_secs(get_interval_secs_long_term(
                *pos,
                *multiplier,
            ))),
        }
    }
    fn get_due_timestamp(&self) -> Option<u64> {
        dbg!(self);
        let interval = dbg!(self.get_interval());
        let interval: u64 = if let Some(interval) = interval {
            interval.as_secs()
        } else {
            return None;
        };
        match self {
            Card::None { .. } => None,
            Card::Wrong {
                last_graded_check, ..
            }
            | Card::ShortTerm {
                last_graded_check, ..
            }
            | Card::LongTerm {
                last_graded_check, ..
            } => Some(dbg!(last_graded_check) + interval),
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
