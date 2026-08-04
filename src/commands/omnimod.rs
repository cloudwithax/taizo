use crate::{Context, Error};
use base64::Engine;
use poise::serenity_prelude as serenity;
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;

const OMNIMOD_BASE_URL: &str = "https://omnimodapi.clxud.dev/v1/chat/completions";

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct OmnimodConfig {
    pub guild_id: i64,
    pub enabled: bool,
    pub pre_stage_threshold: f64,
    pub stage1_model: String,
    pub stage2_model: String,
    pub stage1_confidence_threshold: f64,
    pub stage2_confidence_threshold: f64,
    pub log_channel_id: Option<i64>,
}

impl OmnimodConfig {
    pub fn default(guild_id: i64) -> Self {
        OmnimodConfig {
            guild_id,
            enabled: false,
            pre_stage_threshold: 0.5,
            stage1_model: "/home/clxud/models/Qwen3.5-4B-Q4_K_M.gguf".to_string(),
            stage2_model: "/home/clxud/models/Qwen3.5-4B-Q4_K_M.gguf".to_string(),
            stage1_confidence_threshold: 0.5,
            stage2_confidence_threshold: 0.75,
            log_channel_id: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PreStageResult {
    pub flagged: bool,
    pub score: f64,
    pub matches: Vec<PatternMatch>,
}

#[derive(Debug, Clone)]
pub struct PatternMatch {
    pub category: String,
    pub weight: f64,
    pub matched: String,
}

#[derive(Debug, Clone)]
pub struct StageResult {
    pub label: String,
    pub confidence: f64,
    pub category: String,
    pub target: String,
    pub reason: String,
}

struct KeywordEntry {
    keyword: String,
    category: String,
    weight: f64,
}

struct RegexEntry {
    pattern: regex::Regex,
    category: String,
    weight: f64,
}

struct OmnimodState {
    keywords: Vec<KeywordEntry>,
    automaton: aho_corasick::AhoCorasick,
    raw_regex_patterns: Vec<RegexEntry>,
    normalized_regex_patterns: Vec<RegexEntry>,
}

fn get_keywords() -> Vec<KeywordEntry> {
    vec![
        KeywordEntry { keyword: "kill myself".to_string(), category: "self_harm_risk".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "end my life".to_string(), category: "self_harm_risk".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "end it all".to_string(), category: "self_harm_risk".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "end it".to_string(), category: "self_harm_risk".to_string(), weight: 1.5 },
        KeywordEntry { keyword: "not worth it".to_string(), category: "self_harm_risk".to_string(), weight: 1.5 },
        KeywordEntry { keyword: "better off dead".to_string(), category: "self_harm_risk".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "better off without me".to_string(), category: "self_harm_risk".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "wish i was dead".to_string(), category: "self_harm_risk".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "wish i wasnt born".to_string(), category: "self_harm_risk".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "wish i was never born".to_string(), category: "self_harm_risk".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "no one cares".to_string(), category: "self_harm_risk".to_string(), weight: 1.0 },
        KeywordEntry { keyword: "nobody cares".to_string(), category: "self_harm_risk".to_string(), weight: 1.0 },
        KeywordEntry { keyword: "nobody would notice".to_string(), category: "self_harm_risk".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "no one would notice".to_string(), category: "self_harm_risk".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "no one would miss me".to_string(), category: "self_harm_risk".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "nobody would miss me".to_string(), category: "self_harm_risk".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "everyone would be better off".to_string(), category: "self_harm_risk".to_string(), weight: 1.5 },
        KeywordEntry { keyword: "i am a burden".to_string(), category: "self_harm_risk".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "burden to everyone".to_string(), category: "self_harm_risk".to_string(), weight: 1.5 },
        KeywordEntry { keyword: "final goodbye".to_string(), category: "self_harm_risk".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "see you on the other side".to_string(), category: "self_harm_risk".to_string(), weight: 1.5 },
        KeywordEntry { keyword: "giving up".to_string(), category: "self_harm_risk".to_string(), weight: 1.0 },
        KeywordEntry { keyword: "cant go on".to_string(), category: "self_harm_risk".to_string(), weight: 1.5 },
        KeywordEntry { keyword: "over and out".to_string(), category: "self_harm_risk".to_string(), weight: 1.0 },
        KeywordEntry { keyword: "done with life".to_string(), category: "self_harm_risk".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "done with everything".to_string(), category: "self_harm_risk".to_string(), weight: 1.5 },
        KeywordEntry { keyword: "dont have a reason".to_string(), category: "self_harm_risk".to_string(), weight: 1.5 },
        KeywordEntry { keyword: "ill be gone".to_string(), category: "self_harm_risk".to_string(), weight: 1.5 },
        KeywordEntry { keyword: "i will be gone".to_string(), category: "self_harm_risk".to_string(), weight: 1.5 },
        KeywordEntry { keyword: "you wont have to worry".to_string(), category: "self_harm_risk".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "i wont be a problem".to_string(), category: "self_harm_risk".to_string(), weight: 1.5 },
        KeywordEntry { keyword: "ill take care of it".to_string(), category: "self_harm_risk".to_string(), weight: 1.0 },
        KeywordEntry { keyword: "whats the point".to_string(), category: "self_harm_risk".to_string(), weight: 1.5 },
        KeywordEntry { keyword: "what is the point anymore".to_string(), category: "self_harm_risk".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "made my peace".to_string(), category: "self_harm_risk".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "check out early".to_string(), category: "self_harm_risk".to_string(), weight: 1.5 },
        KeywordEntry { keyword: "exit plan".to_string(), category: "self_harm_risk".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "go sleep forever".to_string(), category: "self_harm_risk".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "unalive myself".to_string(), category: "self_harm_risk".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "unalive".to_string(), category: "self_harm_risk".to_string(), weight: 1.5 },
        KeywordEntry { keyword: "off myself".to_string(), category: "self_harm_risk".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "off yourself".to_string(), category: "encouraging_self_harm".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "take myself out".to_string(), category: "self_harm_risk".to_string(), weight: 1.5 },
        KeywordEntry { keyword: "sewerslide".to_string(), category: "self_harm_risk".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "find some rope".to_string(), category: "self_harm_risk".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "best way to rope".to_string(), category: "self_harm_risk".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "how to hang".to_string(), category: "self_harm_risk".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "how many pills".to_string(), category: "supplying_method".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "lethal dose".to_string(), category: "supplying_method".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "how much to".to_string(), category: "supplying_method".to_string(), weight: 1.5 },
        KeywordEntry { keyword: "deadly amount".to_string(), category: "supplying_method".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "method to".to_string(), category: "supplying_method".to_string(), weight: 1.5 },
        KeywordEntry { keyword: "step by step".to_string(), category: "supplying_method".to_string(), weight: 1.0 },
        KeywordEntry { keyword: "how much would be lethal".to_string(), category: "supplying_method".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "how many would it take".to_string(), category: "supplying_method".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "bus schedule".to_string(), category: "supplying_method".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "charcoal and".to_string(), category: "supplying_method".to_string(), weight: 1.5 },
        KeywordEntry { keyword: "exhaust pipe".to_string(), category: "supplying_method".to_string(), weight: 1.5 },
        KeywordEntry { keyword: "railroad tracks".to_string(), category: "supplying_method".to_string(), weight: 1.5 },
        KeywordEntry { keyword: "i will kill you".to_string(), category: "threat".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "im going to kill you".to_string(), category: "threat".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "im going to find you".to_string(), category: "threat".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "i know where you live".to_string(), category: "threat".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "i know your address".to_string(), category: "threat".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "ill find you".to_string(), category: "threat".to_string(), weight: 1.5 },
        KeywordEntry { keyword: "watch your back".to_string(), category: "threat".to_string(), weight: 1.5 },
        KeywordEntry { keyword: "watch what happens".to_string(), category: "threat".to_string(), weight: 1.5 },
        KeywordEntry { keyword: "youre dead".to_string(), category: "threat".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "you are dead".to_string(), category: "threat".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "im coming for you".to_string(), category: "threat".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "call the cops on".to_string(), category: "threat".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "send police to".to_string(), category: "threat".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "swat you".to_string(), category: "threat".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "swatted".to_string(), category: "threat".to_string(), weight: 1.5 },
        KeywordEntry { keyword: "you should die".to_string(), category: "encouraging_self_harm".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "just die".to_string(), category: "encouraging_self_harm".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "go die".to_string(), category: "encouraging_self_harm".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "kill yourself".to_string(), category: "encouraging_self_harm".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "kys".to_string(), category: "encouraging_self_harm".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "why dont you die".to_string(), category: "encouraging_self_harm".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "youd be better off dead".to_string(), category: "encouraging_self_harm".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "everyone dies".to_string(), category: "encouraging_self_harm".to_string(), weight: 1.0 },
        KeywordEntry { keyword: "do the world a favor".to_string(), category: "encouraging_self_harm".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "do everyone a favor".to_string(), category: "encouraging_self_harm".to_string(), weight: 1.5 },
        KeywordEntry { keyword: "just end it".to_string(), category: "encouraging_self_harm".to_string(), weight: 1.5 },
        KeywordEntry { keyword: "you should go".to_string(), category: "encouraging_self_harm".to_string(), weight: 1.0 },
        KeywordEntry { keyword: "permanent solution".to_string(), category: "encouraging_self_harm".to_string(), weight: 1.5 },
        KeywordEntry { keyword: "nobody would miss you".to_string(), category: "encouraging_self_harm".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "no one would miss you".to_string(), category: "encouraging_self_harm".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "go find some rope".to_string(), category: "encouraging_self_harm".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "go hang yourself".to_string(), category: "encouraging_self_harm".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "i know your phone".to_string(), category: "doxxing".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "your address is".to_string(), category: "doxxing".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "your workplace".to_string(), category: "doxxing".to_string(), weight: 1.5 },
        KeywordEntry { keyword: "your school".to_string(), category: "doxxing".to_string(), weight: 1.5 },
        KeywordEntry { keyword: "your employer".to_string(), category: "doxxing".to_string(), weight: 1.5 },
        KeywordEntry { keyword: "your car".to_string(), category: "doxxing".to_string(), weight: 1.5 },
        KeywordEntry { keyword: "license plate".to_string(), category: "doxxing".to_string(), weight: 1.5 },
        KeywordEntry { keyword: "social security".to_string(), category: "doxxing".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "his name is".to_string(), category: "doxxing".to_string(), weight: 1.5 },
        KeywordEntry { keyword: "her name is".to_string(), category: "doxxing".to_string(), weight: 1.5 },
        KeywordEntry { keyword: "their name is".to_string(), category: "doxxing".to_string(), weight: 1.5 },
        KeywordEntry { keyword: "works at".to_string(), category: "doxxing".to_string(), weight: 1.0 },
        KeywordEntry { keyword: "lives on".to_string(), category: "doxxing".to_string(), weight: 1.5 },
        KeywordEntry { keyword: "lives at".to_string(), category: "doxxing".to_string(), weight: 1.5 },
        KeywordEntry { keyword: "goes to school at".to_string(), category: "doxxing".to_string(), weight: 1.5 },
        KeywordEntry { keyword: "underage".to_string(), category: "minor_safety".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "how old are you really".to_string(), category: "minor_safety".to_string(), weight: 1.5 },
        KeywordEntry { keyword: "how old are u".to_string(), category: "minor_safety".to_string(), weight: 1.5 },
        KeywordEntry { keyword: "where do you live minor".to_string(), category: "minor_safety".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "dont tell mods".to_string(), category: "minor_safety".to_string(), weight: 1.5 },
        KeywordEntry { keyword: "dont tell the mods".to_string(), category: "minor_safety".to_string(), weight: 1.5 },
        KeywordEntry { keyword: "keep this between us".to_string(), category: "minor_safety".to_string(), weight: 1.0 },
        KeywordEntry { keyword: "dont report this".to_string(), category: "minor_safety".to_string(), weight: 1.0 },
        KeywordEntry { keyword: "dont tell anyone".to_string(), category: "minor_safety".to_string(), weight: 1.0 },
        KeywordEntry { keyword: "off platform".to_string(), category: "minor_safety".to_string(), weight: 1.5 },
        KeywordEntry { keyword: "move to discord".to_string(), category: "minor_safety".to_string(), weight: 1.5 },
        KeywordEntry { keyword: "move to dm".to_string(), category: "minor_safety".to_string(), weight: 1.5 },
        KeywordEntry { keyword: "move to dms".to_string(), category: "minor_safety".to_string(), weight: 1.5 },
        KeywordEntry { keyword: "mods cant see".to_string(), category: "minor_safety".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "add me on".to_string(), category: "minor_safety".to_string(), weight: 1.0 },
        KeywordEntry { keyword: "hate you".to_string(), category: "hate".to_string(), weight: 1.0 },
        KeywordEntry { keyword: "fucking die".to_string(), category: "hate".to_string(), weight: 1.5 },
        KeywordEntry { keyword: "i hope you suffer".to_string(), category: "hate".to_string(), weight: 1.5 },
        KeywordEntry { keyword: "i hope you rot".to_string(), category: "hate".to_string(), weight: 1.5 },
        KeywordEntry { keyword: "i hope you die".to_string(), category: "hate".to_string(), weight: 2.0 },
        KeywordEntry { keyword: "rot in hell".to_string(), category: "hate".to_string(), weight: 1.0 },
         KeywordEntry { keyword: "hentai".to_string(), category: "porn_link".to_string(), weight: 2.0 },
         KeywordEntry { keyword: "porn".to_string(), category: "porn_link".to_string(), weight: 3.0 },
         KeywordEntry { keyword: "pornhub".to_string(), category: "porn_link".to_string(), weight: 3.0 },
         KeywordEntry { keyword: "xvideos".to_string(), category: "porn_link".to_string(), weight: 3.0 },
         KeywordEntry { keyword: "xnxx".to_string(), category: "porn_link".to_string(), weight: 3.0 },
         KeywordEntry { keyword: "nsfw".to_string(), category: "porn_link".to_string(), weight: 2.0 },
         KeywordEntry { keyword: "nsfw link".to_string(), category: "porn_link".to_string(), weight: 2.5 },
         KeywordEntry { keyword: "nsfw content".to_string(), category: "porn_link".to_string(), weight: 2.5 },
         KeywordEntry { keyword: "rule 34".to_string(), category: "porn_link".to_string(), weight: 2.5 },
         KeywordEntry { keyword: "rule34".to_string(), category: "porn_link".to_string(), weight: 2.5 },
         KeywordEntry { keyword: "rule 34 link".to_string(), category: "porn_link".to_string(), weight: 3.0 },
         KeywordEntry { keyword: "rule34 link".to_string(), category: "porn_link".to_string(), weight: 3.0 },
         KeywordEntry { keyword: "adult content".to_string(), category: "porn_link".to_string(), weight: 2.0 },
         KeywordEntry { keyword: "18+".to_string(), category: "porn_link".to_string(), weight: 2.0 },
         KeywordEntry { keyword: "18 plus".to_string(), category: "porn_link".to_string(), weight: 2.0 },
         KeywordEntry { keyword: "xxx".to_string(), category: "porn_link".to_string(), weight: 2.0 },
         KeywordEntry { keyword: "xxx link".to_string(), category: "porn_link".to_string(), weight: 2.5 },
         KeywordEntry { keyword: "free porn".to_string(), category: "porn_link".to_string(), weight: 3.0 },
         KeywordEntry { keyword: "watch porn".to_string(), category: "porn_link".to_string(), weight: 3.0 },
         KeywordEntry { keyword: "download porn".to_string(), category: "porn_link".to_string(), weight: 3.0 },
         KeywordEntry { keyword: "stream porn".to_string(), category: "porn_link".to_string(), weight: 3.0 },
         KeywordEntry { keyword: "find porn".to_string(), category: "porn_link".to_string(), weight: 3.0 },
         KeywordEntry { keyword: "find hentai".to_string(), category: "porn_link".to_string(), weight: 3.0 },
         KeywordEntry { keyword: "look at hentai".to_string(), category: "porn_link".to_string(), weight: 3.0 },
         KeywordEntry { keyword: "check out hentai".to_string(), category: "porn_link".to_string(), weight: 3.0 },
         KeywordEntry { keyword: "check out porn".to_string(), category: "porn_link".to_string(), weight: 3.0 },
         KeywordEntry { keyword: "hentai video".to_string(), category: "porn_link".to_string(), weight: 3.0 },
         KeywordEntry { keyword: "porn video".to_string(), category: "porn_link".to_string(), weight: 3.0 },
         KeywordEntry { keyword: "porn image".to_string(), category: "porn_link".to_string(), weight: 3.0 },
         KeywordEntry { keyword: "hentai image".to_string(), category: "porn_link".to_string(), weight: 3.0 },
         KeywordEntry { keyword: "porn pic".to_string(), category: "porn_link".to_string(), weight: 3.0 },
         KeywordEntry { keyword: "hentai pic".to_string(), category: "porn_link".to_string(), weight: 3.0 },
         KeywordEntry { keyword: "porn gif".to_string(), category: "porn_link".to_string(), weight: 3.0 },
         KeywordEntry { keyword: "hentai gif".to_string(), category: "porn_link".to_string(), weight: 3.0 },
         KeywordEntry { keyword: "cum".to_string(), category: "porn_link".to_string(), weight: 2.0 },
         KeywordEntry { keyword: "boobs".to_string(), category: "porn_link".to_string(), weight: 2.0 },
         KeywordEntry { keyword: "pussy".to_string(), category: "porn_link".to_string(), weight: 2.0 },
         KeywordEntry { keyword: "anal".to_string(), category: "porn_link".to_string(), weight: 2.0 },
         KeywordEntry { keyword: "bdsm".to_string(), category: "porn_link".to_string(), weight: 2.0 },
         KeywordEntry { keyword: "fetish".to_string(), category: "porn_link".to_string(), weight: 2.0 },
         KeywordEntry { keyword: "slut".to_string(), category: "porn_link".to_string(), weight: 2.0 },
     ]
}

fn get_raw_regex_patterns() -> Vec<RegexEntry> {
    vec![
        RegexEntry {
            pattern: regex::Regex::new(r"[\u{200B}\u{200C}\u{200D}\u{FEFF}\u{2060}\u{2061}\u{2062}\u{2063}\u{2064}]").unwrap(),
            category: "evasion".to_string(),
            weight: 2.5,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"[\u{0300}-\u{036F}]").unwrap(),
            category: "evasion".to_string(),
            weight: 2.0,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"[\u{0430}\u{0435}\u{043E}\u{0440}\u{0441}\u{0443}\u{0445}\u{0456}]").unwrap(),
            category: "evasion".to_string(),
            weight: 1.5,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"(?i)\b[kk]\s*[i1!ìíîïı]\s*[l1!|ℓ]\s*[l1!|ℓ]\b").unwrap(),
            category: "evasion".to_string(),
            weight: 2.0,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"(?i)\b[dð]\s*[i1!ìíîïı]\s*[e3èéêëę]\s*[a4àáâãäå@]\s*[dð]\b").unwrap(),
            category: "evasion".to_string(),
            weight: 2.0,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"(?i)\b[s5$]\s*[uùúûüů]\s*[c¢©k]\s*[i1!ìíîïı]\s*[dð]\s*[e3èéêëę]?\b").unwrap(),
            category: "evasion".to_string(),
            weight: 2.0,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"(?i)\b[s5$]\s*[e3èéêëę]\s*[l1!|ℓ]\s*[fƒ]\s*[- ]?\s*[h#ℎ]\s*[a4àáâãäå@]\s*[rʀ]\s*[mм]?\b").unwrap(),
            category: "evasion".to_string(),
            weight: 2.0,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"(?i)\b[rʀ]\s*[o0òóôõöø@]\s*[pρ]\s*[e3èéêëę]\b").unwrap(),
            category: "self_harm_risk".to_string(),
            weight: 2.0,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"(?i)\b[h#ℎ]\s*[a4àáâãäå@]\s*[nñи]\s*[g9ɡĝ]\b").unwrap(),
            category: "self_harm_risk".to_string(),
            weight: 2.0,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"🔪|💀|☠️|🪢|💊|🔫|⚰️|🪦").unwrap(),
            category: "evasion".to_string(),
            weight: 1.5,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"(?i)\bk[i1!ìíîïı][l1!|ℓ][l1!|ℓ]\s*(?:y[o0òóôõöø@]u[rʀ]?[s5$][e3èéêëę]lf|u[rʀ]?self)\b").unwrap(),
            category: "encouraging_self_harm".to_string(),
            weight: 2.0,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"(?i)\b(?:k|k1|ky)[s5$]\b").unwrap(),
            category: "encouraging_self_harm".to_string(),
            weight: 2.0,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"\b\d{1,5}\s+(?:north|south|east|west|[NSEW]\.?)\s+[a-zA-Z]+\s+(?:st|street|ave|avenue|blvd|boulevard|dr|drive|rd|road|ln|lane|ct|court|pl|place|way|cir|circle|ter|terrace|pkwy|parkway)\b").unwrap(),
            category: "doxxing".to_string(),
            weight: 2.5,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"(?:\+?1[-.\s]?)?\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4}\b").unwrap(),
            category: "doxxing".to_string(),
            weight: 2.0,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"\b\d{3}[-.\s]?\d{2}[-.\s]?\d{4}\b").unwrap(),
            category: "doxxing".to_string(),
            weight: 2.5,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b").unwrap(),
            category: "doxxing".to_string(),
            weight: 2.0,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}").unwrap(),
            category: "doxxing".to_string(),
            weight: 1.5,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"\b[A-Z]{1,3}[- ]?\d{2,4}[- ]?[A-Z]{0,3}\b").unwrap(),
            category: "doxxing".to_string(),
            weight: 1.0,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"(?i)(?:bit\.ly|tinyurl\.com|t\.co|goo\.gl|is\.gd|cutt\.ly|ow\.ly|rebrand\.ly|shorturl\.at)/\S+").unwrap(),
            category: "evasion".to_string(),
            weight: 1.5,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"(?i)(?:v\.gd|tiny\.cc|clck\.ru|rb\.gd|adf\.ly|bc\.vc|cli\.gs|db\.tt|df\.ly|goo\.gl|hurl\.cat|lnkd\.in|nxy\.cn|ow\.ly|q\.gs|surl\.pro|t\.co|tiny\.cc|tiny\.url|tr\.im|u\.bb|url\.ie|url\.me|url\.shortened|urls\.com|ut\.ee|x\.co|yourls\.org)/\S+").unwrap(),
            category: "evasion".to_string(),
            weight: 1.5,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"(?i)https?://\s*old\.reddit\.com/r/(?:hentai|porn|HENTAI|PORN|gay|lesbians|nsfw|Rule34|rule34|sex|anal|boobs|pussy|cum|slut|bdsm|fetish|amateur|teen|teenager)/\S+").unwrap(),
            category: "porn_link".to_string(),
            weight: 3.0,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"(?i)https?://\s*www\.reddit\.com/r/(?:hentai|porn|HENTAI|PORN|gay|lesbians|nsfw|Rule34|rule34|sex|anal|boobs|pussy|cum|slut|bdsm|fetish|amateur|teen|teenager)/\S+").unwrap(),
            category: "porn_link".to_string(),
            weight: 3.0,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"(?i)reddit\.com/r/(?:hentai|porn|HENTAI|PORN|gay|lesbians|nsfw|Rule34|rule34|sex|anal|boobs|pussy|cum|slut|bdsm|fetish|amateur|teen|teenager)/\S+").unwrap(),
            category: "porn_link".to_string(),
            weight: 3.0,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"(?i)https?://\s*(?:www\.)?(?:pornhub|xvideos|xnxx|redtube|youporn|porndoe|spankbang|brazzers|twistys|mofos|naughtyamerica|realitykings|bangbros|milehigh|digitalplayground|evilangel|blacked|tushy|anal\sparty|mom\smass)\S+").unwrap(),
            category: "porn_link".to_string(),
            weight: 3.0,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"(?i)(?:pornhub|xvideos|xnxx|redtube|youporn|porndoe|spankbang|brazzers|twistys|mofos|naughtyamerica|realitykings|bangbros|milehigh|digitalplayground|evilangel|blacked|tushy)\.\S+").unwrap(),
            category: "porn_link".to_string(),
            weight: 3.0,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"(?i)https?://\s*\S*(?:hentai|porn|xxx|nsfw|sex|fuck|pussy|cum|slut|bdsm|fetish|anal|boobs|teen)\S*").unwrap(),
            category: "porn_link".to_string(),
            weight: 2.5,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"(?i)\b(?:find|check|look|get|see|watch|download|stream)\s+(?:free\s+)?(?:porn|hentai|sex|nsfw|xxx|fuck|pussy|cum|boobs|anal|teen)\b").unwrap(),
            category: "porn_link".to_string(),
            weight: 2.0,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"(?i)\b(?:porn|hentai|sex|nsfw|xxx|fuck|pussy|cum|boobs|anal|teen)\s+(?:link|site|url|address|page|video|content|download|stream)\b").unwrap(),
            category: "porn_link".to_string(),
            weight: 2.0,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"(?i)\bat\s+(?:bit\.ly|tinyurl|t\.co|goo\.gl|is\.gd|cutt\.ly|ow\.ly|v\.gd|tiny\.cc|clck\.ru|rb\.gd|adf\.ly|bc\.vc|cli\.gs|db\.tt|df\.ly|lnkd\.in|nxy\.cn|ow\.ly|q\.gs|surl\.pro|t\.co|tiny\.cc|tiny\.url|tr\.im|u\.bb|url\.ie|url\.me|ut\.ee|x\.co|yourls\.org)/\S+").unwrap(),
            category: "evasion".to_string(),
            weight: 1.5,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"(?i)\b(?:http|https|ftp|www)\s*:\s*/\s*/\s*\S+").unwrap(),
            category: "evasion".to_string(),
            weight: 1.0,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"(?i)\b(?:dot|period|@)\s*(?:com|org|net|ru|xyz|cc|tk|ml|ga|cf|gq|cf|ga|ml|tk|ml|ga|cf|gq)\b").unwrap(),
            category: "evasion".to_string(),
            weight: 1.5,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"(?i)\b(?:you\s+(?:can|can\s+find|will\s+find|can\s+get|can\s+see|can\s+watch|can\s+download|can\s+check)\s+(?:free\s+)?(?:porn|hentai|sex|nsfw|xxx|fuck|pussy|cum|boobs|anal|teen)|(?:porn|hentai|sex|nsfw|xxx|fuck|pussy|cum|boobs|anal|teen)\s+(?:is\s+available|can\s+be\s+found|is\s+here|is\s+on|is\s+at|is\s+free|is\s+available))").unwrap(),
            category: "porn_link".to_string(),
            weight: 2.5,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"(?i)\b(?:i\s+(?:found|discovered|located|got|have|just\s+found|just\s+got|just\s+discovered)\s+(?:a\s+)?(?:porn|hentai|sex|nsfw|xxx|fuck|pussy|cum|boobs|anal|teen)\s+(?:link|site|video|page|content|download|stream)|(?:porn|hentai|sex|nsfw|xxx|fuck|pussy|cum|boobs|anal|teen)\s+(?:link|site|video|page|content|download|stream)\s+(?:i\s+(?:found|discovered|located|got|have|just\s+found|just\s+got|just\s+discovered)))").unwrap(),
            category: "porn_link".to_string(),
            weight: 2.5,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"(?i)\b(?:check\s+out|look\s+at|click\s+(?:the\s+)?(?:link|here|this)|follow\s+(?:this|the)\s+link|go\s+(?:to|visit|check)\s+(?:the\s+)?(?:link|site|page|url))\s+(?:to\s+)?(?:find|get|see|watch|download|stream|view)\s+(?:free\s+)?(?:porn|hentai|sex|nsfw|xxx|fuck|pussy|cum|boobs|anal|teen)").unwrap(),
            category: "porn_link".to_string(),
            weight: 2.5,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"(?i)\b(?:discord\.gg|discord\.com|discord\.app)/\S*(?:porn|hentai|sex|nsfw|xxx|fuck|pussy|cum|boobs|anal|teen|adult|18\+|18\+|nsfw|rule\s*34)\S*").unwrap(),
            category: "porn_link".to_string(),
            weight: 3.0,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"(?i)\b(?:t\.me|telegram\.me|telegram\.dog)/\S*(?:porn|hentai|sex|nsfw|xxx|fuck|pussy|cum|boobs|anal|teen|adult|18\+|nsfw|rule\s*34)\S*").unwrap(),
            category: "porn_link".to_string(),
            weight: 3.0,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"(?i)(?:e621\.net|e621\.cc|e621\.org)/posts/\S*(?:tags.*(?:hentai|porn|sex|nsfw|xxx|anal|boobs|pussy|cum|slut|bdsm|fetish|teen|amateur)|(?:hentai|porn|sex|nsfw|xxx|anal|boobs|pussy|cum|slut|bdsm|fetish|teen|amateur))").unwrap(),
            category: "porn_link".to_string(),
            weight: 3.0,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"(?i)(?:danbooru\.donmai\.us|gelbooru\.com|rule34\.xxx|rule34\.paheal\.net|rule34\.us|safebooru\.org|xbooru\.com|rule34\.com|3dbooru\.com)/posts?\S*(?:tags.*(?:hentai|porn|sex|nsfw|xxx|anal|boobs|pussy|cum|slut|bdsm|fetish|teen|amateur)|(?:hentai|porn|sex|nsfw|xxx|anal|boobs|pussy|cum|slut|bdsm|fetish|teen|amateur))").unwrap(),
            category: "porn_link".to_string(),
            weight: 3.0,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"(?i)(?:e621|danbooru|gelbooru|rule34|safebooru|xbooru|3dbooru)\.\S+").unwrap(),
            category: "porn_link".to_string(),
            weight: 3.0,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"(?i)(?:imgur\.com|i\.imgur\.com)/\S*(?:hentai|porn|sex|nsfw|xxx|anal|boobs|pussy|cum|slut|bdsm|fetish|teen|amateur)\S*").unwrap(),
            category: "porn_link".to_string(),
            weight: 2.5,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"(?i)(?:mediafire\.com|mega\.nz|drive\.google\.com|dropbox\.com|pastebin\.com|rentry\.co|coil\.me|000webhost\.com|000web\.host|github\.io|gitlab\.io)/\S*(?:hentai|porn|sex|nsfw|xxx|fuck|pussy|cum|boobs|anal|teen|adult|18\+|nsfw|rule\s*34)\S*").unwrap(),
            category: "porn_link".to_string(),
            weight: 2.5,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"(?i)hxxps?://\S+").unwrap(),
            category: "evasion".to_string(),
            weight: 2.0,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"(?i)\b(?:http|https|ftp|www)\s*\[\s*\.\s*\]\s*\S+").unwrap(),
            category: "evasion".to_string(),
            weight: 2.0,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"(?i)\b(?:http|https|ftp|www)\s*\(\s*/\s*\)\s*\S+").unwrap(),
            category: "evasion".to_string(),
            weight: 2.0,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"(?i)\b(?:http|https|ftp|www)\s*dot\s*\S+\s*dot\s*(?:com|org|net|ru|xyz|cc|tk|ml|ga|cf|gq)\b").unwrap(),
            category: "evasion".to_string(),
            weight: 2.0,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"(?i)\b(?:0x[0-9a-f]{4,}|[A-Za-z0-9+/]{20,}={0,2})").unwrap(),
            category: "evasion".to_string(),
            weight: 1.0,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"(?i)\b(d|d0|d0x)(o|0)(x|xx|xing|xxed)\b").unwrap(),
            category: "doxxing".to_string(),
            weight: 2.0,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"(?i)\b[s5$][wω][a4àáâãäå@][t7][t7]\b").unwrap(),
            category: "threat".to_string(),
            weight: 2.0,
        },
    ]
}

fn get_normalized_regex_patterns() -> Vec<RegexEntry> {
    vec![
        RegexEntry {
            pattern: regex::Regex::new(r"\bkill\s+(?:my|your|him|her|them|ur|his|hers|their)\s*self\b").unwrap(),
            category: "self_harm_risk".to_string(),
            weight: 2.0,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"\b(?:how many|how much)\s+\w+\s+(?:would|will|does|do)\s+(?:it take|be lethal|kill|be enough|be fatal)\b").unwrap(),
            category: "supplying_method".to_string(),
            weight: 2.0,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"\b(?:best|easiest|quickest|fastest|most effective)\s+(?:way|method|means)\s+(?:to|of)\b").unwrap(),
            category: "supplying_method".to_string(),
            weight: 1.5,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"\b(?:lethal|fatal|deadly)\s+(?:dose|amount|quantity)\b").unwrap(),
            category: "supplying_method".to_string(),
            weight: 2.0,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"\bi\s+(?:will|am going to|gonna|will)\s+(?:kill|murder|shoot|stab|hurt|harm)\s+you\b").unwrap(),
            category: "threat".to_string(),
            weight: 2.0,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"\bi\s+know\s+(?:where|what)\s+you\s+(?:live|are|work|go)\b").unwrap(),
            category: "threat".to_string(),
            weight: 2.0,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"\b(?:how old|what age)\s+(?:are you|r u|ru)\b").unwrap(),
            category: "minor_safety".to_string(),
            weight: 1.5,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"\b(?:add me|dm me|message me)\s+(?:on|in)\s+(?:discord|snap|insta|snapchat|telegram|whatsapp|signal)\b").unwrap(),
            category: "minor_safety".to_string(),
            weight: 1.5,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"\b(?:dont|don t|do not)\s+(?:tell|report)\s+(?:the\s+)?(?:mods|moderators|admin|admins|staff)\b").unwrap(),
            category: "minor_safety".to_string(),
            weight: 2.0,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"\b(?:send|call)\s+(?:the\s+)?(?:police|cops|swat|fbi)\s+(?:to|on|at)\b").unwrap(),
            category: "threat".to_string(),
            weight: 2.0,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"\b(?:works at|goes to|lives at|lives on)\s+\w+\s+\w+\b").unwrap(),
            category: "doxxing".to_string(),
            weight: 1.5,
        },
        RegexEntry {
            pattern: regex::Regex::new(r"\b(?:his|her|their)\s+(?:name|real name)\s+is\s+\w+\b").unwrap(),
            category: "doxxing".to_string(),
            weight: 1.5,
        },
    ]
}

fn normalize_text(text: &str) -> String {
    let text = text.to_lowercase();
    let text: String = text.chars().map(|c| match c {
        '1' | '!' | '|' | '¡' | 'ℓ' => 'i',
        '3' | '€' => 'e',
        '4' | '@' => 'a',
        '5' | '$' => 's',
        '7' => 't',
        '0' | '°' => 'o',
        '8' => 'b',
        '9' | 'ɡ' | 'ĝ' => 'g',
        '2' | 'z' => 'z',
        '6' => 'b',
        '\u{0430}' => 'a',
        '\u{0435}' => 'e',
        '\u{043E}' => 'o',
        '\u{0440}' => 'p',
        '\u{0441}' => 'c',
        '\u{0443}' => 'y',
        '\u{0445}' => 'x',
        '\u{0456}' => 'i',
        '\u{0458}' => 'j',
        '\u{04BB}' => 'h',
        '\u{0455}' => 's',
        '\u{0442}' => 't',
        '\u{043C}' => 'm',
        '\u{043F}' => 'n',
        '\u{0432}' => 'b',
        '\u{043A}' => 'k',
        '\u{0434}' => 'd',
        '\u{0444}' => 'f',
        '\u{0433}' => 'g',
        '\u{043B}' => 'l',
        '\u{0438}' => 'u',
        '\u{043D}' => 'h',
        '\u{0446}' => 'c',
        '\u{0447}' => 'c',
        '\u{0448}' => 'w',
        '\u{0449}' => 'w',
        '\u{044F}' => 'a',
        '\u{044E}' => 'u',
        '\u{044B}' => 'y',
        '\u{044D}' => 'e',
        '\u{0457}' => 'i',
        '\u{0454}' => 'e',
        '\u{0491}' => 'g',
        '\u{045E}' => 'y',
        '\u{0406}' => 'i',
        '\u{0407}' => 'i',
        '\u{0404}' => 'e',
        '\u{0490}' => 'g',
        '\u{040E}' => 'y',
        '\u{0401}' => 'e',
        '\u{0436}' => 'z',
        '\u{0437}' => 'z',
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'æ' | 'ą' => 'a',
        'è' | 'é' | 'ê' | 'ë' | 'ę' => 'e',
        'ì' | 'í' | 'î' | 'ï' | 'ı' => 'i',
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' => 'o',
        'ù' | 'ú' | 'û' | 'ü' | 'ů' => 'u',
        'ñ' => 'n',
        'ð' => 'd',
        'þ' => 't',
        'ß' => 's',
        'ç' | '¢' | '©' => 'c',
        'ƒ' => 'f',
        'μ' => 'u',
        'π' => 'p',
        'ω' => 'w',
        'ρ' => 'p',
        'ʀ' => 'r',
        'ℎ' => 'h',
        _ => c,
    }).collect();
    let text: String = text.chars().map(|c| if c.is_alphanumeric() || c == ' ' { c } else { ' ' }).collect();
    let text: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    text.trim().to_string()
}

fn get_automaton() -> aho_corasick::AhoCorasick {
    let keywords = get_keywords();
    let patterns: Vec<String> = keywords.iter().map(|k| k.keyword.clone()).collect();
    aho_corasick::AhoCorasick::new(&patterns).expect("failed to build aho-corasick automaton")
}

fn get_state() -> Arc<RwLock<OmnimodState>> {
    let keywords = get_keywords();
    let automaton = get_automaton();
    let raw_regex_patterns = get_raw_regex_patterns();
    let normalized_regex_patterns = get_normalized_regex_patterns();
    Arc::new(RwLock::new(OmnimodState {
        keywords,
        automaton,
        raw_regex_patterns,
        normalized_regex_patterns,
    }))
}

lazy_static::lazy_static! {
    static ref OMNIMOD_STATE: Arc<RwLock<OmnimodState>> = get_state();
}

pub async fn run_pre_stage(text: &str, threshold: f64) -> PreStageResult {
    let normalized = normalize_text(text);
    let state = OMNIMOD_STATE.read().await;
    let mut matches = Vec::new();
    let mut score = 0.0;

    for re in &state.raw_regex_patterns {
        if re.pattern.is_match(text) {
            score += re.weight;
            matches.push(PatternMatch {
                category: re.category.clone(),
                weight: re.weight,
                matched: re.pattern.as_str().to_string(),
            });
        }
    }

    for mat in state.automaton.find_iter(&normalized) {
        let pattern = &state.keywords[mat.pattern()];
        score += pattern.weight;
        matches.push(PatternMatch {
            category: pattern.category.clone(),
            weight: pattern.weight,
            matched: pattern.keyword.clone(),
        });
    }

    for re in &state.normalized_regex_patterns {
        if re.pattern.is_match(&normalized) {
            score += re.weight;
            matches.push(PatternMatch {
                category: re.category.clone(),
                weight: re.weight,
                matched: re.pattern.as_str().to_string(),
            });
        }
    }

    let obfuscation_count = text.chars().filter(|c| is_obfuscation_char(*c)).count();
    if obfuscation_count >= 2 {
        score += 2.5;
        matches.push(PatternMatch {
            category: "obfuscation".to_string(),
            weight: 2.5,
            matched: format!("{} obfuscation chars", obfuscation_count),
        });
    }

    let foreign_count = text.chars().filter(|c| is_non_latin_letter(*c)).count();
    if foreign_count >= 4 {
        score += 1.5;
        matches.push(PatternMatch {
            category: "foreign_script".to_string(),
            weight: 1.5,
            matched: format!("{} non-latin letters", foreign_count),
        });
    }

    PreStageResult {
        flagged: score >= threshold,
        score,
        matches,
    }
}

fn is_obfuscation_char(c: char) -> bool {
    matches!(c,
        '\u{200B}'..='\u{200F}'
        | '\u{20E3}'
        | '\u{FEFF}'
        | '\u{0300}'..='\u{036F}'
        | '\u{2060}'..='\u{2064}'
    )
}

fn is_non_latin_letter(c: char) -> bool {
    if !c.is_alphabetic() {
        return false;
    }
    let cp = c as u32;
    !((0x0000..=0x024F).contains(&cp)
        || (0x1E00..=0x1EFF).contains(&cp)
        || (0x2C60..=0x2C7F).contains(&cp)
        || (0xA720..=0xA7FF).contains(&cp))
}

fn image_to_jpeg_base64(bytes: &[u8]) -> Result<String, Error> {
    let img = image::load_from_memory(bytes)?;
    let mut buf = Vec::new();
    let mut encoder = image::codecs::jpeg::JpegEncoder::new(&mut buf);
    encoder.encode_image(&img)?;
    Ok(base64::engine::general_purpose::STANDARD.encode(&buf))
}

#[derive(Deserialize)]
struct NovitaResponse {
    choices: Vec<NovitaChoice>,
}

#[derive(Deserialize)]
struct NovitaChoice {
    message: NovitaMessage,
}

#[derive(Deserialize)]
struct NovitaMessage {
    content: Option<String>,
    reasoning_content: Option<String>,
}

pub struct NovitaClient {
    client: reqwest::Client,
    api_key: String,
}

impl NovitaClient {
    pub fn new(api_key: String) -> Self {
        NovitaClient {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .unwrap_or_default(),
            api_key,
        }
    }

    pub async fn call_stage1(&self, message: &str) -> Result<String, Error> {
        let model = "/home/clxud/models/Qwen3.5-4B-Q4_K_M.gguf";
        let system_prompt = "You are a triage filter for a chat community. You do not punish anyone. You decide only whether a human-grade reviewer should look at a message.

You are looking for messages that LOOK clean on the surface but are not. Clean vocabulary is not evidence of innocence. Judge what the message is DOING, not which words it contains.

Messages may be in ANY language or script, including transliterated or obfuscated text (keycap letters, combining marks, leetspeak, homoglyphs, spaces between letters). Translate it in your head; do not let non-English characters or evasion tricks make a harmful message look innocent.

Escalate if any of these are plausible — plausible, not proven:

1. The author may be at risk. Hopelessness, worthlessness, feeling like a burden, finality or goodbye tone, giving away possessions, sudden calm after a rough patch, asking about a method or a quantity, referencing self-harm by euphemism.
2. The message may be telling someone else to die or harm themselves, however indirect, sarcastic, or joke-framed. Suggestions phrased as helpful advice, as a question, or as a comment about a method count.
3. Someone is supplying method detail, dosage, or lethality information to another user.
4. A threat, or a vague menacing reference to meeting someone or knowing where they are.
5. Real-world identifying information about a person: name plus workplace, street, school, employer, vehicle.
6. An adult seeking a minor's age, contact, privacy, or a move off-platform; any request to keep something from moderators.
7. Deliberate filter evasion: letter substitution, spacing, homoglyphs, invented euphemism, or an in-joke that seems to stand in for something worse.
8. A message that is mild alone but is the latest in a run aimed at the same person.

Do NOT escalate ordinary rudeness, profanity, insults, arguments, dark humor about oneself with no ideation, grief, gaming or work hyperbole (\"this is killing me\", \"I'd rather die than sit through this\"), or good-faith discussion of hard topics.

When torn, escalate. A wasted review costs nothing. A miss can cost a great deal.

Output exactly one word, nothing else: ALLOW or ESCALATE";

        let body = serde_json::json!({
            "model": model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": message}
            ],
            "temperature": 0.1,
            "max_tokens": 50,
            "chat_template_kwargs": {"enable_thinking": false}
        });

        let response = self.client
            .post(OMNIMOD_BASE_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let text = response.text().await?;
        let parsed: NovitaResponse = serde_json::from_str(&text)?;
        let msg = parsed.choices.first().map(|c| &c.message);
        let content = msg
            .and_then(|m| m.content.as_deref().or(m.reasoning_content.as_deref()))
            .unwrap_or("")
            .trim()
            .to_uppercase();

        if content.contains("ESCALATE") {
            Ok("ESCALATE".to_string())
        } else {
            Ok("ALLOW".to_string())
        }
    }

    pub async fn call_stage2(&self, message: &str) -> Result<StageResult, Error> {
        let model = "/home/clxud/models/Qwen3.5-4B-Q4_K_M.gguf";
        let system_prompt = "You are the adjudicating moderator for a chat community. A cheap filter flagged this message; most flags are false alarms, so do not assume guilt. Your specialty is implicit harm — messages with no slurs, no explicit threats, and no obvious rule-breaking words whose actual function is to wound, to encourage harm, or to evade moderation.

## Language

Messages may be in ANY language, script, or encoding. Translate and judge the meaning, never the surface characters. Obfuscation (keycap letters, combining marks, zero-width chars, leetspeak, homoglyphs, spaces between letters, transliteration) does not make a message innocent — evaluate what the message says once decoded.

DECODING IS MANDATORY. Before judging, strip combining marks (keycap, diacritics, zero-width), remove spaces between letters, and transliterate. A message that looks like gibberish is often an obfuscated slur or threat in another language. If you cannot decode it to verify it is harmless, do NOT ALLOW it — output REVIEW. \"no discernible meaning\" is never a reason to ALLOW.

## Output format (MANDATORY)

Your ENTIRE response must be exactly one JSON object. Start with { and end with }. No thinking tags, no explanation, no markdown, no extra text. Just the JSON.

Example: {\"label\":\"ALLOW\",\"confidence\":0.9,\"category\":\"none\",\"target\":\"none\",\"reason\":\"hyperbolic work frustration\"}

## What to check

1. What is the message DOING? Asking, joking, informing, wounding, recruiting, instructing, or signalling distress.
2. Who is the target? THE CONTEXT LINE TELLS YOU WHO IS BEING ADDRESSED. If context mentions a person and the message addresses them, the target is OTHER, not SELF.
3. Euphemisms: \"rope\" = hanging, \"sewerslide\" = suicide, \"bus schedule\" = being hit by a bus, \"find some rope\" = hang yourself, \"do the world a favor\" = die.
4. Strip joke framing. \"/j\", \"lol\", \"jk\" do NOT neutralize harmful payloads. \"how many of my meds would it take, asking for no reason lol\" is still asking about lethal dosage.
5. Direction is everything.

## Hyperbole carve-out

\"kms\", \"kill me\", \"i'd rather die\", \"this is killing me\", \"murdered me\" when clearly about work stress, gaming, deadlines, meetings, or technical frustration -> ALLOW.

## CRISIS indicators (author at risk)

These ALWAYS -> CRISIS:
- Asking about method, quantity, or lethality of own harm (\"how many of my meds\", \"whats the best way to rope\")
- Hopelessness, feeling like a burden, \"nobody would notice\", \"what's the point anymore\"
- Giving away possessions or accounts (\"giving my setup to my brother\")
- Finality or goodbye tone, \"made my peace with things\"
- \"sewerslide\", \"self-harm\" euphemisms about self
Ambiguity resolves to CRISIS. When in doubt, CRISIS.

## Direction

- Author about THEMSELVES -> CRISIS. Never punish.
- Pointing at ANOTHER person suggesting they die or harm themselves -> BAN. Even oblique, sarcastic, or joke-framed.
- Supplying method detail to distressed user -> BAN.

## Context rules

If context says \"@lily said she's struggling\" and message says \"nobody would miss you\" -> BAN (directed AT lily).
If context says \"@kai posted art\" and message says \"lol who let you have a tablet\" -> REVIEW (mocking directed at kai).
If context says \"user said they feel worthless\" and message says \"have you considered the bus schedule is very reliable\" -> BAN (suggesting method AT that user).
A message that is mild alone but is the latest in a run aimed at the same person -> REVIEW.

## Grooming

Asks age + asks to move to DMs + asks to keep from moderators -> BAN (all three = grooming).
Only asks to move platform without age/secrecy -> REVIEW.

## Doxxing

Real name + workplace/street/school/location -> BAN. Always severe.

## Threats

\"watch what happens next time you're at the meetup\" -> REVIEW. \"i know what street you live on\" -> BAN.

## The REVIEW principle

REVIEW is the safe default for anything ambiguous. When you are torn between ALLOW and REVIEW, choose REVIEW. When you are torn between BAN and REVIEW, choose REVIEW. A wasted review costs nothing. A miss can cost a great deal. This applies especially to:
- Mocking or dismissing someone's creative work when context shows it's directed at them
- Unsolicited negative advice about someone's interests or hobbies
- Requests to move to other platforms that could be grooming-adjacent
- Mild but potentially part of a harassment pattern

## Labels

- ALLOW — rudeness, profanity, arguments, dark humor, grief, gaming/work hyperbole, good-faith hard topics.
- REVIEW — plausibly harmful, genuinely ambiguous, or part of a harassment pattern. Use when torn.
- REMOVE — violating but no evident malice. Delete, no ban.
- BAN — targeted, deliberate, severe. Encouraging harm, supplying method, threats, doxxing, grooming, telling someone to die.
- CRISIS — author at risk. Support route, not punishment. Overrides all other labels.

## Confidence

If BAN confidence < 0.75, output REVIEW instead. No floor for CRISIS.";

        let body = serde_json::json!({
            "model": model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": message}
            ],
            "temperature": 0.1,
            "max_tokens": 2000,
            "chat_template_kwargs": {"enable_thinking": false}
        });

        let response = self.client
            .post(OMNIMOD_BASE_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let text = response.text().await?;
        let parsed: NovitaResponse = serde_json::from_str(&text)?;
        let msg = parsed.choices.first().map(|c| &c.message);
        let content = msg
            .and_then(|m| m.content.as_deref().filter(|s| !s.is_empty()))
            .unwrap_or("")
            .trim()
            .to_string();

        if content.is_empty() {
            tracing::warn!("omnimod: stage2 returned empty content");
            return Ok(StageResult {
                label: "REVIEW".to_string(),
                confidence: 0.5,
                category: "empty_model_output".to_string(),
                target: "none".to_string(),
                reason: "model returned no verdict".to_string(),
            });
        }

        parse_stage2_output(&content)
    }

    pub async fn call_stage2_with_image(&self, message: &str, image_base64: &str) -> Result<StageResult, Error> {
        let model = "/home/clxud/models/Qwen3.5-4B-Q4_K_M.gguf";
        let system_prompt = "You are an image content adjudicator for a Discord community moderation system. A triage filter has flagged this image. You must decide the final action.

You WILL see the image. Judge it directly against ALL community rules.

## What to check

### Sexual content
- Nudity: exposed genitalia, female-presenting nipples, buttocks, pubic area
- Sexual activity: any depiction of sex, oral, masturbation, penetration
- Hentai, anime porn, drawn sexual content
- Sexually suggestive poses, especially with minors or minor-appearing characters
- Sex toys, bondage in sexual context
- Cum or sexual fluids

### Hate and extremism
- Nazi symbols: swastikas, SS runes, Reichsadler, iron cross in Nazi context
- White supremacist, KKK, Confederate, or fascist imagery
- Racial/ethnic/religious hate propaganda
- Extremist group imagery: ISIS, neo-Nazi, KKK

### Violence and gore
- Graphic injuries, wounds, dismemberment
- Dead bodies, corpses, death scenes
- Animal abuse
- Self-harm or suicide depictions
- Torture or abuse

### Illegal content
- Drug use or manufacturing
- Weapons in threatening context
- Child exploitation

### Harassment
- Doxxing: personal info of private individuals
- Revenge porn or non-consensual intimate images
- Threatening or intimidation imagery

### Dangerous content
- Bomb/weapon making instructions
- Dangerous challenges

## Labels

- ALLOW — benign: landscape, pet, food, non-offensive meme, clothed non-sexual, educational
- REVIEW — ambiguous, need human eyes
- REMOVE — violating but no evident malice. Delete, no ban.
- BAN — deliberate, severe violation: porn, hate symbols, graphic violence, doxxing
- CRISIS — author at risk. Support route, not punishment.

## Confidence

Output a JSON object: {\"label\":\"...\",\"confidence\":0.0,\"category\":\"...\",\"target\":\"...\",\"reason\":\"...\"}

If confidence < 0.75 for BAN, output REVIEW instead.";

        let body = serde_json::json!({
            "model": model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": [
                    {"type": "text", "text": message},
                    {"type": "image_url", "image_url": {"url": format!("data:image/jpeg;base64,{}", image_base64)}}
                ]}
            ],
            "temperature": 0.1,
            "max_tokens": 2000,
            "chat_template_kwargs": {"enable_thinking": false}
        });

        let response = self.client
            .post(OMNIMOD_BASE_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let text = response.text().await?;
        let parsed: Result<NovitaResponse, _> = serde_json::from_str(&text);
        match parsed {
            Ok(parsed) => {
                let msg = parsed.choices.first().map(|c| &c.message);
                let content = msg
                    .and_then(|m| m.content.as_deref().filter(|s| !s.is_empty()))
                    .unwrap_or("")
                    .trim()
                    .to_string();

                if content.is_empty() {
                    tracing::warn!("omnimod: image stage2 returned empty content");
                    return Ok(StageResult {
                        label: "REVIEW".to_string(),
                        confidence: 0.5,
                        category: "empty_model_output".to_string(),
                        target: "none".to_string(),
                        reason: "model returned no verdict".to_string(),
                    });
                }

                parse_stage2_output(&content)
            }
            Err(e) => {
                tracing::warn!("omnimod: image stage2 parse error: {} response: {}", e, text);
                Ok(StageResult {
                    label: "REVIEW".to_string(),
                    confidence: 0.5,
                    category: "parse_error".to_string(),
                    target: "none".to_string(),
                    reason: "failed to parse model output".to_string(),
                })
            }
        }
    }

    pub async fn call_stage1_with_image(&self, message: &str, image_base64: &str) -> Result<String, Error> {
        let model = "/home/clxud/models/Qwen3.5-4B-Q4_K_M.gguf";
        let system_prompt = "You are an image content reviewer for a Discord community moderation system. You receive images that need review.

Your ONLY job: determine if this image violates community rules.

ESCALATE if the image contains ANY of the following:

## Sexual content
- Nudity: exposed genitalia, breasts (female-presenting), buttocks, or pubic area
- Sexual activity: intercourse, oral sex, masturbation, foreplay, penetration
- Hentai, anime porn, drawn sexual content, manga with explicit scenes
- Pornographic content: any depiction of real or simulated sexual acts
- Sexually suggestive poses, especially with minors or minor-appearing characters
- Sex toys, bondage equipment in sexual context
- Close-up sexual body parts (genitalia, anus, female presenting nipples in sexual context)
- Cum, sexual fluids, or bodily fluids in sexual context
- Nudity even if not fully explicit (topless, nude selfies, locker room photos)

## Hate and extremism
- Nazi symbols: swastikas, SS runes, Reichsadler, iron cross in Nazi context
- Confederate flags, KKK imagery, white supremacist symbols
- Hateful propaganda targeting race, religion, ethnicity, sexual orientation
- Extremist group imagery: ISIS, neo-Nazi, white power, Ku Klux Klan
- Racial slurs or ethnic slurs in image form
- Holocaust denial or minimization imagery
- Black sun, totenkopf, or other fascist symbols
- Mein Kampf excerpts or Nazi salute imagery

## Violence and gore
- Physical assault, beating, stabbing, shooting victims
- Graphic injuries, open wounds, broken bones, dismemberment
- Dead bodies, corpses, death scenes
- Animal abuse or cruelty imagery
- Self-harm: cutting, burns, suicide attempts depicted
- Graphic car accidents, fatal injuries
- Torture or abuse imagery

## Illegal content
- Drug use: injecting, smoking, ingesting illegal substances
- Drug manufacturing: labs, grow operations, pill presses
- Weapons in threatening context
- Child exploitation material (任何内容)
- Stolen goods, fraud evidence

## Harassment and doxxing
- Screenshots of private messages used for harassment
- Personal information: faces, addresses, phone numbers of private individuals
- Revenge porn or intimate images shared without consent
- Threatening messages or intimidation imagery
- Swatting or doxxing evidence

## Dangerous content
- Instructions for making weapons, bombs, or dangerous substances
- Dangerous challenges or stunts that could cause harm
- Content promoting eating disorders or self-starvation

ALLOW ONLY if the image is completely benign:
- Landscape, nature, pet, animal photos
- Food, cooking, recipes
- Screenshots of text, code, memes (non-offensive)
- Artistic content without hate symbols or sexual activity
- Fashion, clothing, everyday life
- Educational or scientific content
- Non-offensive humor or memes

When in doubt, ALWAYS ESCALATE. A false positive is acceptable. Missing harmful content is not.

Output exactly one word: ALLOW or ESCALATE";

        let body = serde_json::json!({
            "model": model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": [
                    {"type": "text", "text": message},
                    {"type": "image_url", "image_url": {"url": format!("data:image/jpeg;base64,{}", image_base64)}}
                ]}
            ],
            "temperature": 0.0,
            "max_tokens": 10,
            "chat_template_kwargs": {"enable_thinking": false}
        });

        let response = self.client
            .post(OMNIMOD_BASE_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let text = response.text().await?;
        let parsed: Result<NovitaResponse, _> = serde_json::from_str(&text);
        match parsed {
            Ok(parsed) => {
                let msg = parsed.choices.first().map(|c| &c.message);
                let content = msg
                    .and_then(|m| m.content.as_deref().or(m.reasoning_content.as_deref()))
                    .unwrap_or("")
                    .trim()
                    .to_uppercase();

                if content.contains("ESCALATE") {
                    Ok("ESCALATE".to_string())
                } else {
                    Ok("ALLOW".to_string())
                }
            }
            Err(e) => {
                tracing::warn!("omnimod: image LLM stage1 parse error: {} response: {}", e, text);
                Ok("ALLOW".to_string())
            }
        }
    }
}

fn parse_stage2_output(text: &str) -> Result<StageResult, Error> {
    let json_start = text.find('{').unwrap_or(0);
    let json_end = text.rfind('}').map(|i| i + 1).unwrap_or(text.len());
    let json_str = &text[json_start..json_end];
    let parsed: serde_json::Value = serde_json::from_str(json_str)?;

    let label = parsed.get("label").and_then(|v| v.as_str()).unwrap_or("REVIEW").to_string();
    let confidence = parsed.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.5);
    let category = parsed.get("category").and_then(|v| v.as_str()).unwrap_or("none").to_string();
    let target = parsed.get("target").and_then(|v| v.as_str()).unwrap_or("none").to_string();
    let reason = parsed.get("reason").and_then(|v| v.as_str()).unwrap_or("").to_string();

    Ok(StageResult {
        label,
        confidence,
        category,
        target,
        reason,
    })
}

pub async fn get_omnimod_config(db: &sqlx::PgPool, guild_id: i64) -> Result<OmnimodConfig, Error> {
    let row = sqlx::query_as::<_, (bool, f64, String, String, f64, f64, Option<i64>)>(
        "SELECT enabled, pre_stage_threshold, stage1_model, stage2_model, stage1_confidence_threshold, stage2_confidence_threshold, log_channel_id FROM omnimod_config WHERE guild_id = $1",
    )
    .bind(guild_id)
    .fetch_optional(db)
    .await?;

    match row {
        Some(r) => Ok(OmnimodConfig {
            guild_id,
            enabled: r.0,
            pre_stage_threshold: r.1,
            stage1_model: r.2,
            stage2_model: r.3,
            stage1_confidence_threshold: r.4,
            stage2_confidence_threshold: r.5,
            log_channel_id: r.6,
        }),
        None => {
            sqlx::query(
                "INSERT INTO omnimod_config (guild_id, enabled) VALUES ($1, false)",
            )
            .bind(guild_id)
            .execute(db)
            .await?;
            Ok(OmnimodConfig::default(guild_id))
        }
    }
}

pub async fn set_omnimod_enabled(db: &sqlx::PgPool, guild_id: i64, enabled: bool) -> Result<(), Error> {
    sqlx::query("UPDATE omnimod_config SET enabled = $1 WHERE guild_id = $2")
        .bind(enabled)
        .bind(guild_id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn set_omnimod_threshold(db: &sqlx::PgPool, guild_id: i64, threshold: f64) -> Result<(), Error> {
    sqlx::query("UPDATE omnimod_config SET pre_stage_threshold = $1 WHERE guild_id = $2")
        .bind(threshold)
        .bind(guild_id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn set_omnimod_models(db: &sqlx::PgPool, guild_id: i64, stage1: String, stage2: String) -> Result<(), Error> {
    sqlx::query("UPDATE omnimod_config SET stage1_model = $1, stage2_model = $2 WHERE guild_id = $3")
        .bind(&stage1)
        .bind(&stage2)
        .bind(guild_id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn set_omnimod_log_channel(db: &sqlx::PgPool, guild_id: i64, channel_id: Option<i64>) -> Result<(), Error> {
    sqlx::query("UPDATE omnimod_config SET log_channel_id = $1 WHERE guild_id = $2")
        .bind(channel_id)
        .bind(guild_id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn log_omnimod_flag(
    db: &sqlx::PgPool,
    guild_id: i64,
    channel_id: i64,
    message_id: i64,
    author_id: i64,
    content: &str,
    stage: &str,
    label: Option<&str>,
    confidence: Option<f64>,
    reason: Option<&str>,
    action_taken: Option<&str>,
) -> Result<i64, Error> {
    let content_truncated = if content.len() > 1000 {
        &content[..1000]
    } else {
        content
    };
    let row: (i64,) = sqlx::query_as(
        "INSERT INTO omnimod_flags (guild_id, channel_id, message_id, author_id, content, stage, label, confidence, reason, action_taken, case_number) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, (SELECT COALESCE(MAX(case_number), 0) + 1 FROM omnimod_flags WHERE guild_id = $1)) RETURNING case_number"
    )
    .bind(guild_id)
    .bind(channel_id)
    .bind(message_id)
    .bind(author_id)
    .bind(content_truncated)
    .bind(stage)
    .bind(label)
    .bind(confidence)
    .bind(reason)
    .bind(action_taken)
    .fetch_one(db)
    .await?;
    Ok(row.0)
}

pub async fn get_recent_flags(db: &sqlx::PgPool, guild_id: i64, limit: i32) -> Result<Vec<(i64, i64, String, Option<String>, chrono::DateTime<chrono::Utc>)>, Error> {
    let rows = sqlx::query_as::<_, (i64, i64, String, Option<String>, chrono::DateTime<chrono::Utc>)>(
        "SELECT case_number, message_id, content, label, created_at FROM omnimod_flags WHERE guild_id = $1 ORDER BY created_at DESC LIMIT $2"
    )
    .bind(guild_id)
    .bind(limit)
    .fetch_all(db)
    .await?;
    Ok(rows)
}

pub async fn handle_message(
    http: &serenity::Http,
    db: &sqlx::PgPool,
    msg: &serenity::Message,
) {
    if msg.author.bot {
        return;
    }

    let guild_id = match msg.guild_id {
        Some(g) => g.get() as i64,
        None => return,
    };

    let config = match get_omnimod_config(db, guild_id).await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("omnimod config fetch error: {:?}", e);
            return;
        }
    };

    if !config.enabled {
        return;
    }

    let content = msg.content.trim();

    if !msg.attachments.is_empty() && crate::image_classifier::CLASSIFIER.is_some() {
        for attachment in &msg.attachments {
            if let Ok(response) = reqwest::get(&attachment.url).await {
                if let Ok(bytes) = response.bytes().await {
                    let is_gif = attachment.filename.to_lowercase().ends_with(".gif");
                    let result = if is_gif {
                        crate::image_classifier::classify_gif(&bytes)
                    } else {
                        crate::image_classifier::classify_image(&bytes)
                    };
                    if let Ok(result) = result {
                        tracing::info!("omnimod: image classification: {} (score: {:.3})", result.dominant_class, result.nsfw_score);
if result.is_nsfw {
                             let _ = msg.delete(http).await;
                             let _ = log_omnimod_flag(
                                 db,
                                 guild_id,
                                 msg.channel_id.get() as i64,
                                 msg.id.get() as i64,
                                 msg.author.id.get() as i64,
                                 content,
                                 "image",
                                 Some(&result.dominant_class),
                                 Some(result.nsfw_score as f64),
                                 Some(&format!("nsfw image: {} (score: {:.3})", result.dominant_class, result.nsfw_score)),
                                 Some("image_nsfw_deleted"),
                             ).await;
                             if let Some(log_channel_id) = config.log_channel_id {
                                 let _ = send_image_log_embed(http, log_channel_id, msg, &result, attachment).await;
                             }
                             return;
                         }
                         
                         // Score below threshold but not negligible — send to LLM for multimodal review
                         if result.nsfw_score >= 0.001 && result.nsfw_score < 0.4 {
                             if let Ok(api_key) = std::env::var("OMNIMOD_API_KEY") {
                                 if !api_key.is_empty() {
                                     if let Ok(base64_image) = image_to_jpeg_base64(&bytes) {
                                         let client = NovitaClient::new(api_key);
                                         let image_message = format!("[Image NSFW score: {:.3}, class: {}]. Review this image for policy violations.", result.nsfw_score, result.dominant_class);
                                         
                                         match client.call_stage1_with_image(&image_message, &base64_image).await {
                                             Ok(llm_result) => {
                                                 tracing::info!("omnimod: image LLM review: {}", llm_result);
                                                 
                                                 if llm_result == "ESCALATE" {
                                                     // Run stage2 for adjudication (with image)
                                                     match client.call_stage2_with_image(&image_message, &base64_image).await {
                                                         Ok(stage2_result) => {
                                                             tracing::info!("omnimod: image LLM stage2={} confidence={:.2}", stage2_result.label, stage2_result.confidence);
                                                             
                                                             let action_taken = match stage2_result.label.as_str() {
                                                                 "BAN" => {
                                                                     let gid = serenity::GuildId::new(guild_id as u64);
                                                                     if let Ok(guild) = gid.to_partial_guild(http).await {
                                                                         let _ = guild.ban_with_reason(http, msg.author.id, 7, "omnimod: banned by image LLM review").await;
                                                                     }
                                                                     let _ = msg.delete(http).await;
                                                                     "image_llm_banned_and_deleted"
                                                                 }
                                                                 "REMOVE" => {
                                                                     let _ = msg.delete(http).await;
                                                                     "image_llm_message_deleted"
                                                                 }
                                                                 "CRISIS" => {
                                                                     let _ = send_crisis_dm(http, msg).await;
                                                                     "image_llm_crisis_dm_sent"
                                                                 }
                                                                 _ => "image_llm_logged_for_review",
                                                             };
                                                             
                                                             let _ = log_omnimod_flag(
                                                                 db,
                                                                 guild_id,
                                                                 msg.channel_id.get() as i64,
                                                                 msg.id.get() as i64,
                                                                 msg.author.id.get() as i64,
                                                                 content,
                                                                 "image_llm_review",
                                                                 Some(&stage2_result.label),
                                                                 Some(stage2_result.confidence),
                                                                 Some(&format!("image LLM review: {} (score: {:.3}, class: {})", stage2_result.label, result.nsfw_score, result.dominant_class)),
                                                                 Some(action_taken),
                                                             ).await;
                                                         }
                                                         Err(e) => {
                                                             tracing::warn!("omnimod: image LLM stage2 error: {}", e);
                                                         }
                                                     }
                                                 }
                                             }
                                             Err(e) => {
                                                 tracing::warn!("omnimod: image LLM stage1 error: {}", e);
                                             }
                                         }
                                     }
                                 }
                             }
                         }
                     }
                     
                     // OCR: Extract text from image
                    let ocr_text = if is_gif {
                        crate::ocr::extract_text_from_gif(&bytes).unwrap_or_default()
                    } else {
                        match image::load_from_memory(&bytes) {
                            Ok(img) => crate::ocr::extract_text_from_image(&img).unwrap_or_default(),
                            Err(_) => String::new(),
                        }
                    };
                    
                    if !ocr_text.is_empty() {
                        tracing::info!("omnimod: OCR extracted {} chars from image", ocr_text.len());
                        let ocr_pre_result = run_pre_stage(&ocr_text, config.pre_stage_threshold).await;
                        
                        if ocr_pre_result.flagged {
                            tracing::info!("omnimod: OCR text flagged: score={:.2} matches={}", ocr_pre_result.score, ocr_pre_result.matches.len());
                            
                            let _ = log_omnimod_flag(
                                db,
                                guild_id,
                                msg.channel_id.get() as i64,
                                msg.id.get() as i64,
                                msg.author.id.get() as i64,
                                &ocr_text,
                                "ocr_pre_stage",
                                Some("FLAGGED"),
                                Some(ocr_pre_result.score),
                                Some(&format!("OCR text flagged: {:?}", ocr_pre_result.matches.iter().map(|m| &m.category).collect::<Vec<_>>())),
                                None,
                            ).await;
                            
                            // Escalate to LLM for review
                            if let Ok(api_key) = std::env::var("OMNIMOD_API_KEY") {
                                if !api_key.is_empty() {
                                    let client = NovitaClient::new(api_key);
                                    let ocr_message = format!("[Image OCR text]: {}", ocr_text);
                                    
                                    match client.call_stage1(&ocr_message).await {
                                        Ok(stage1_result) => {
                                            tracing::info!("omnimod: OCR stage1={}", stage1_result);
                                            
                                            if stage1_result == "ESCALATE" {
                                                match client.call_stage2(&ocr_message).await {
                                                    Ok(stage2_result) => {
                                                        tracing::info!("omnimod: OCR stage2={} confidence={:.2}", stage2_result.label, stage2_result.confidence);
                                                        
                                                        let action_taken = match stage2_result.label.as_str() {
                                                            "BAN" => {
                                                                let gid = serenity::GuildId::new(guild_id as u64);
                                                                if let Ok(guild) = gid.to_partial_guild(http).await {
                                                                    let _ = guild.ban_with_reason(http, msg.author.id, 7, "omnimod: banned by OCR text moderation").await;
                                                                }
                                                                let _ = msg.delete(http).await;
                                                                "ocr_banned_and_deleted"
                                                            }
                                                            "REMOVE" => {
                                                                let _ = msg.delete(http).await;
                                                                "ocr_message_deleted"
                                                            }
                                                            "CRISIS" => {
                                                                let _ = send_crisis_dm(http, msg).await;
                                                                "ocr_crisis_dm_sent"
                                                            }
                                                            _ => "ocr_logged_for_review",
                                                        };
                                                        
                                                        let case_number = log_omnimod_flag(
                                                            db,
                                                            guild_id,
                                                            msg.channel_id.get() as i64,
                                                            msg.id.get() as i64,
                                                            msg.author.id.get() as i64,
                                                            &ocr_text,
                                                            "ocr_stage2",
                                                            Some(&stage2_result.label),
                                                            Some(stage2_result.confidence),
                                                            Some(&stage2_result.reason),
                                                            Some(action_taken),
                                                        ).await.unwrap_or(0);
                                                        
                                                        if let Some(log_channel_id) = config.log_channel_id {
                                                            let _ = send_ocr_log_embed(http, log_channel_id, msg, &stage2_result, &ocr_pre_result, case_number, action_taken, &ocr_text).await;
                                                        }
                                                    }
                                                    Err(e) => {
                                                        tracing::error!("omnimod: OCR stage2 error: {:?}", e);
                                                    }
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            tracing::error!("omnimod: OCR stage1 error: {:?}", e);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if content.is_empty() {
        return;
    }

    let pre_result = run_pre_stage(content, config.pre_stage_threshold).await;
    tracing::info!("omnimod: score={:.2} flagged={} matches={}", pre_result.score, pre_result.flagged, pre_result.matches.len());

    if !pre_result.flagged {
        return;
    }

    let _ = log_omnimod_flag(
        db,
        guild_id,
        msg.channel_id.get() as i64,
        msg.id.get() as i64,
        msg.author.id.get() as i64,
        content,
        "pre_stage",
        Some("FLAGGED"),
        Some(pre_result.score),
        Some(&format!("pre_stage matches: {:?}", pre_result.matches.iter().map(|m| &m.category).collect::<Vec<_>>())),
        None,
    ).await;

let api_key = match std::env::var("OMNIMOD_API_KEY") {
            Ok(k) if !k.is_empty() => k,
            _ => {
                tracing::info!("omnimod: OMNIMOD_API_KEY not set, skipping LLM stages");
                return;
            }
        };

    let client = NovitaClient::new(api_key);
    let message_text = msg.content.clone();

    let direct_to_stage2 = pre_result.matches.iter().any(|m| {
        m.category == "obfuscation" || m.category == "foreign_script"
    });

    if !direct_to_stage2 {
        let stage1_result = match client.call_stage1(&message_text).await {
            Ok(r) => {
                tracing::info!("omnimod: stage1={}", r);
                r
            }
            Err(e) => {
                tracing::error!("stage1 novita error: {:?}", e);
                return;
            }
        };

        if stage1_result == "ALLOW" {
            let _ = log_omnimod_flag(
                db,
                guild_id,
                msg.channel_id.get() as i64,
                msg.id.get() as i64,
                msg.author.id.get() as i64,
                content,
                "stage1",
                Some("ALLOW"),
                Some(1.0),
                Some("stage1 allowed"),
                None,
            ).await;
            return;
        }
    }

    let stage2_result = match client.call_stage2(&message_text).await {
        Ok(r) => {
            tracing::info!("omnimod: stage2={} confidence={:.2}", r.label, r.confidence);
            r
        }
        Err(e) => {
            tracing::error!("stage2 novita error: {:?}", e);
            return;
        }
    };

    let mut stage2_result = stage2_result;
    if stage2_result.label == "ALLOW"
        && pre_result.matches.iter().any(|m| m.category == "obfuscation" || m.category == "foreign_script")
    {
        tracing::info!("omnimod: obfuscation/foreign content allowed by stage2, downgrading to REVIEW");
        stage2_result.label = "REVIEW".to_string();
        stage2_result.reason = format!("{} (model allowed, obfuscated content)", stage2_result.reason);
    }

    let label = stage2_result.label.clone();
    let confidence = stage2_result.confidence;

    let action_taken = match label.as_str() {
        "CRISIS" => {
            let _ = send_crisis_dm(http, msg).await;
            "crisis_dm_sent"
        }
        "BAN" => {
            let gid = serenity::GuildId::new(guild_id as u64);
            if let Ok(guild) = gid.to_partial_guild(http).await {
                let _ = guild.ban_with_reason(http, msg.author.id, 7, "omnimod: banned by automated moderation").await;
            }
            let _ = msg.delete(http).await;
            "banned_and_deleted"
        }
        "REMOVE" => {
            let _ = msg.delete(http).await;
            "message_deleted"
        }
        "REVIEW" => {
            "logged_for_review"
        }
        _ => "no_action",
    };

    let case_number = log_omnimod_flag(
        db,
        guild_id,
        msg.channel_id.get() as i64,
        msg.id.get() as i64,
        msg.author.id.get() as i64,
        content,
        "stage2",
        Some(&label),
        Some(confidence),
        Some(&stage2_result.reason),
        Some(action_taken),
    ).await.unwrap_or(0);

    if let Some(log_channel_id) = config.log_channel_id {
        let _ = send_log_embed(http, log_channel_id, msg, &stage2_result, &pre_result, case_number, action_taken).await;
    }
}

async fn send_crisis_dm(
    http: &serenity::Http,
    msg: &serenity::Message,
) -> Result<(), Error> {
    let _ = msg.author.dm(http, serenity::CreateMessage::new()
        .embed(
            serenity::CreateEmbed::new()
                .title("we noticed something")
                .description("someone on this server cares about you. if you're going through a hard time, there are people who want to help.")
                .field("resources", "988 suicide & crisis lifeline: call or text 988\ncrisis text line: text home to 741741", false)
                .color(0x80F291)
        )
    ).await;
    Ok(())
}

async fn send_log_embed(
    http: &serenity::Http,
    log_channel_id: i64,
    msg: &serenity::Message,
    stage2: &StageResult,
    pre_stage: &PreStageResult,
    case_number: i64,
    action_taken: &str,
) -> Result<(), Error> {
    let channel = serenity::ChannelId::new(log_channel_id as u64);
    let embed = serenity::CreateEmbed::new()
        .title(format!("case #{} — {} — {}", case_number, action_taken.replace('_', " "), stage2.label))
        .field("author", format!("<@{}>", msg.author.id), true)
        .field("message", msg.content.chars().take(500).collect::<String>(), false)
        .field("label", &stage2.label, true)
        .field("confidence", format!("{:.2}", stage2.confidence), true)
        .field("category", &stage2.category, true)
        .field("target", &stage2.target, true)
        .field("pre_stage_score", format!("{:.2}", pre_stage.score), true)
        .field("reason", &stage2.reason, false)
        .color(0xF28080)
        .timestamp(chrono::Utc::now());

    let _ = channel.send_message(http, serenity::CreateMessage::new().embed(embed)).await;
    Ok(())
}

async fn send_image_log_embed(
    http: &serenity::Http,
    log_channel_id: i64,
    msg: &serenity::Message,
    result: &crate::image_classifier::NsfwResult,
    attachment: &serenity::Attachment,
) -> Result<(), Error> {
    let channel = serenity::ChannelId::new(log_channel_id as u64);
    let embed = serenity::CreateEmbed::new()
        .title(format!("case #{} — image nsfw — {}", 0, result.dominant_class))
        .field("author", format!("<@{}>", msg.author.id), true)
        .field("message", msg.content.chars().take(500).collect::<String>(), false)
        .field("image", attachment.url.as_str(), false)
        .field("nsfw_score", format!("{:.3}", result.nsfw_score), true)
        .field("dominant_class", &result.dominant_class, true)
        .field("reason", format!("image classification: {} (score: {:.3})", result.dominant_class, result.nsfw_score), false)
        .color(0xF28080)
        .timestamp(chrono::Utc::now());

    let _ = channel.send_message(http, serenity::CreateMessage::new().embed(embed)).await;
    Ok(())
}

async fn send_ocr_log_embed(
    http: &serenity::Http,
    log_channel_id: i64,
    msg: &serenity::Message,
    stage2: &StageResult,
    pre_stage: &PreStageResult,
    case_number: i64,
    action_taken: &str,
    ocr_text: &str,
) -> Result<(), Error> {
    let channel = serenity::ChannelId::new(log_channel_id as u64);
    let embed = serenity::CreateEmbed::new()
        .title(format!("case #{} — ocr text — {}", case_number, stage2.label))
        .field("author", format!("<@{}>", msg.author.id), true)
        .field("action", action_taken.replace('_', " "), true)
        .field("ocr text", ocr_text.chars().take(1000).collect::<String>(), false)
        .field("label", &stage2.label, true)
        .field("confidence", format!("{:.2}", stage2.confidence), true)
        .field("category", &stage2.category, true)
        .field("target", &stage2.target, true)
        .field("pre_stage_score", format!("{:.2}", pre_stage.score), true)
        .field("reason", &stage2.reason, false)
        .color(0xF28080)
        .timestamp(chrono::Utc::now());

    let _ = channel.send_message(http, serenity::CreateMessage::new().embed(embed)).await;
    Ok(())
}

#[poise::command(slash_command, category = "omnimod", subcommands("enable", "disable", "status", "setthreshold", "setmodels", "setlogchannel", "flags", "addpattern", "removepattern", "test"))]
pub async fn omnimod(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("omnimod subcommands: `enable`, `disable`, `status`, `setthreshold`, `setmodels`, `setlogchannel`, `flags`, `addpattern`, `removepattern`, `test`").await?;
    Ok(())
}

#[poise::command(slash_command, category = "omnimod", required_permissions = "MANAGE_GUILD")]
pub async fn enable(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("must be used in a guild")?.get() as i64;

    let existing = get_omnimod_config(&ctx.data().db, guild_id).await?;
    if existing.enabled {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("omnimod is already enabled for this server. use `/omnimod disable` first.")
                    .color(0xF2D380),
            ),
        )
        .await?;
        return Ok(());
    }

    ctx.send(
        poise::CreateReply::default()
            .embed(
                serenity::CreateEmbed::new()
                    .title("confirmation")
                    .description(
                        "omnimod uses machine learning to scan messages and classify them by risk. \
                         it is a moderator assistant, not a replacement for human judgment.\n\n\
                         by enabling, you agree that:\n\
                         • all messages will be scanned in real time via novita.ai\n\
                         • flagged content may be automatically deleted or result in bans\n\
                         • message content is stored in an audit log for admin review\n\
                         • no system is perfect — false positives/negatives will occur\n\
                         • you can disable this at any time with `/omnimod disable`\n\n\
                         by clicking enable, you confirm you have authority to enable this on behalf of the server.",
                    )
                    .color(0x5865F2),
            )
            .components(vec![
                serenity::CreateActionRow::Buttons(vec![
                    serenity::CreateButton::new(format!("omnimod_confirm_enable:{}", guild_id))
                        .label("enable")
                        .style(serenity::ButtonStyle::Success),
                    serenity::CreateButton::new(format!("omnimod_cancel_enable:{}", guild_id))
                        .label("cancel")
                        .style(serenity::ButtonStyle::Danger),
                ]),
            ]),
    )
    .await?;
    Ok(())
}

pub async fn handle_omnimod_enable_button(
    ctx: &serenity::Context,
    component: &serenity::ComponentInteraction,
    db: &sqlx::PgPool,
) -> Result<(), Error> {
    let custom_id = &component.data.custom_id;

    if custom_id.starts_with("omnimod_confirm_enable:") {
        let guild_id: i64 = custom_id.split(':').nth(1).unwrap_or("0").parse().unwrap_or(0);
        if guild_id == 0 {
            let _ = component.create_response(
                ctx,
                serenity::CreateInteractionResponse::UpdateMessage(
                    serenity::CreateInteractionResponseMessage::new()
                        .content("")
                        .embed(
                            serenity::CreateEmbed::new()
                                .description("error: could not determine guild")
                                .color(0xF28080),
                        )
                        .components(vec![]),
                ),
            ).await;
            return Ok(());
        }

        set_omnimod_enabled(db, guild_id, true).await?;

        let _ = component.create_response(
            ctx,
            serenity::CreateInteractionResponse::UpdateMessage(
                serenity::CreateInteractionResponseMessage::new()
                    .content("")
                    .embed(
                        serenity::CreateEmbed::new()
                            .description("omnimod enabled for this server")
                            .color(0x80F291),
                    )
                    .components(vec![]),
            ),
        ).await;
    } else if custom_id.starts_with("omnimod_cancel_enable:") {
        let _ = component.create_response(
            ctx,
            serenity::CreateInteractionResponse::UpdateMessage(
                serenity::CreateInteractionResponseMessage::new()
                    .content("")
                    .embed(
                        serenity::CreateEmbed::new()
                            .description("omnimod enable cancelled")
                            .color(0xF2D380),
                    )
                    .components(vec![]),
            ),
        ).await;
    }

    Ok(())
}

#[poise::command(slash_command, category = "omnimod", required_permissions = "MANAGE_GUILD")]
pub async fn disable(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("must be used in a guild")?.get() as i64;
    set_omnimod_enabled(&ctx.data().db, guild_id, false).await?;
    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .description("omnimod disabled for this server")
                .color(0xF28080),
        ),
    )
    .await?;
    Ok(())
}

#[poise::command(slash_command, category = "omnimod", required_permissions = "MANAGE_GUILD")]
pub async fn status(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("must be used in a guild")?.get() as i64;
    let config = get_omnimod_config(&ctx.data().db, guild_id).await?;

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .title("omnimod status")
                .field("enabled", config.enabled.to_string(), true)
                .field("pre_stage_threshold", format!("{:.2}", config.pre_stage_threshold), true)
                .field("stage1_model", &config.stage1_model, true)
                .field("stage2_model", &config.stage2_model, true)
                .field("stage1_confidence", format!("{:.2}", config.stage1_confidence_threshold), true)
                .field("stage2_confidence", format!("{:.2}", config.stage2_confidence_threshold), true)
                .field("log_channel", config.log_channel_id.map(|id| format!("<#{}>", id)).unwrap_or("not set".to_string()), true)
                .color(0x5865F2),
        ),
    )
    .await?;
    Ok(())
}

#[poise::command(slash_command, category = "omnimod", required_permissions = "MANAGE_GUILD")]
pub async fn setthreshold(
    ctx: Context<'_>,
    #[description = "pre-stage score threshold (0.0-1.0)"] threshold: f64,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("must be used in a guild")?.get() as i64;
    let threshold = threshold.clamp(0.0, 1.0);
    set_omnimod_threshold(&ctx.data().db, guild_id, threshold).await?;
    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .description(format!("pre-stage threshold set to **{:.2}**", threshold))
                .color(0x80F291),
        ),
    )
    .await?;
    Ok(())
}

#[poise::command(slash_command, category = "omnimod", required_permissions = "MANAGE_GUILD")]
pub async fn setmodels(
    ctx: Context<'_>,
    #[description = "stage 1 model id"] stage1: String,
    #[description = "stage 2 model id"] stage2: String,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("must be used in a guild")?.get() as i64;
    set_omnimod_models(&ctx.data().db, guild_id, stage1, stage2).await?;
    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .description("models updated")
                .color(0x80F291),
        ),
    )
    .await?;
    Ok(())
}

#[poise::command(slash_command, category = "omnimod", required_permissions = "MANAGE_GUILD")]
pub async fn setlogchannel(
    ctx: Context<'_>,
    #[description = "channel to send omnimod logs to"] channel: Option<serenity::Channel>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("must be used in a guild")?.get() as i64;
    let channel_id = channel.map(|c| c.id().get() as i64);
    set_omnimod_log_channel(&ctx.data().db, guild_id, channel_id).await?;
    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .description(format!("log channel {}", channel_id.map(|id| format!("<#{}>", id)).unwrap_or("cleared".to_string())))
                .color(0x80F291),
        ),
    )
    .await?;
    Ok(())
}

#[poise::command(slash_command, category = "omnimod", required_permissions = "MANAGE_GUILD")]
pub async fn flags(ctx: Context<'_>, #[description = "number of flags to show"] limit: Option<u32>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("must be used in a guild")?.get() as i64;
    let limit = limit.unwrap_or(10).min(25) as i32;
    let flags = get_recent_flags(&ctx.data().db, guild_id, limit).await?;

    if flags.is_empty() {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("no flags recorded yet")
                    .color(0x5865F2),
            ),
        )
        .await?;
        return Ok(());
    }

    let mut description = String::new();
    for (case_number, msg_id, content, label, created_at) in &flags {
        let content_preview = content.chars().take(100).collect::<String>();
        description.push_str(&format!(
            "`case #{}` | {} | {} | {}\n{}\n\n",
            case_number,
            label.as_deref().unwrap_or("unknown"),
            created_at.format("%m/%d %H:%M"),
            msg_id,
            content_preview,
        ));
    }

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .title(format!("recent flags ({} shown)", flags.len()))
                .description(&description)
                .color(0xF28080),
        ),
    )
    .await?;
    Ok(())
}

#[poise::command(slash_command, category = "omnimod", required_permissions = "MANAGE_GUILD")]
pub async fn addpattern(
    ctx: Context<'_>,
    #[description = "pattern to match"] pattern: String,
    #[description = "pattern category"] category: Option<String>,
    #[description = "match weight (0.1-5.0)"] weight: Option<f64>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("must be used in a guild")?.get() as i64;
    let category = category.unwrap_or("general".to_string());
    let weight = weight.unwrap_or(1.0).clamp(0.1, 5.0);

    sqlx::query(
        "INSERT INTO omnimod_patterns (guild_id, pattern, category, weight, regex) VALUES ($1, $2, $3, $4, false)"
    )
    .bind(guild_id)
    .bind(&pattern)
    .bind(&category)
    .bind(weight)
    .execute(&ctx.data().db)
    .await?;

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .description(format!("added keyword pattern `{}` (category: {}, weight: {:.1})", pattern, category, weight))
                .color(0x80F291),
        ),
    )
    .await?;
    Ok(())
}

#[poise::command(slash_command, category = "omnimod", required_permissions = "MANAGE_GUILD")]
pub async fn removepattern(ctx: Context<'_>, #[description = "pattern id to remove"] id: i32) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("must be used in a guild")?.get() as i64;

    let result = sqlx::query("DELETE FROM omnimod_patterns WHERE id = $1 AND guild_id = $2")
        .bind(id)
        .bind(guild_id)
        .execute(&ctx.data().db)
        .await?;

    if result.rows_affected() == 0 {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("pattern not found")
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .description(format!("removed pattern `{}`", id))
                .color(0x80F291),
        ),
    )
    .await?;
    Ok(())
}

#[poise::command(slash_command, category = "omnimod", required_permissions = "MANAGE_GUILD")]
pub async fn test(
    ctx: Context<'_>,
    #[description = "message to test against the filter"] message: String,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("must be used in a guild")?.get() as i64;
    let config = get_omnimod_config(&ctx.data().db, guild_id).await?;

    let pre_result = run_pre_stage(&message, config.pre_stage_threshold).await;

    let mut description = String::new();
    description.push_str(&format!("pre-stage score: **{:.2}** (threshold: {:.2})\n", pre_result.score, config.pre_stage_threshold));
    description.push_str(&format!("pre-stage flagged: **{}**\n\n", pre_result.flagged));

    if !pre_result.matches.is_empty() {
        description.push_str("**matches:**\n");
        for m in &pre_result.matches {
            description.push_str(&format!("  - {} ({} weight: {:.1})\n", m.matched, m.category, m.weight));
        }
        description.push('\n');
    }

    if pre_result.flagged {
        if let Ok(api_key) = std::env::var("OMNIMOD_API_KEY") {
            if !api_key.is_empty() {
                let client = NovitaClient::new(api_key);
                match client.call_stage1(&message).await {
                    Ok(stage1) => {
                        description.push_str(&format!("stage1 result: **{}**\n", stage1));
                        if stage1 == "ESCALATE" {
                            match client.call_stage2(&message).await {
                                Ok(stage2) => {
                                    description.push_str(&format!("stage2 label: **{}**\n", stage2.label));
                                    description.push_str(&format!("stage2 confidence: **{:.2}**\n", stage2.confidence));
                                    description.push_str(&format!("stage2 category: **{}**\n", stage2.category));
                                    description.push_str(&format!("stage2 target: **{}**\n", stage2.target));
                                    description.push_str(&format!("stage2 reason: **{}**\n", stage2.reason));
                                }
                                Err(_) => {
                                    description.push_str("stage2: error running model\n");
                                }
                            }
                        }
                    }
                    Err(_) => {
                        description.push_str("stage1: error running model\n");
                    }
                }
            } else {
                description.push_str("stage1/stage2: skipped (no omnimod api key in env)\n");
            }
        } else {
            description.push_str("stage1/stage2: skipped (no omnimod api key in env)\n");
        }
    }

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .title("omnimod test result")
                .description(&description)
                .color(if pre_result.flagged { 0xF28080 } else { 0x80F291 }),
        ),
    )
    .await?;
    Ok(())
}