//! Persisted settings, difficulty presets and agent prompt configuration.

use gambiit_agent::AgentConfig;
use serde::{Deserialize, Serialize};

use crate::{bluff, credentials, style};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct CustomTheme {
    pub(crate) bg: String,
    pub(crate) panel: String,
    pub(crate) border: String,
    pub(crate) text: String,
    pub(crate) muted: String,
    pub(crate) accent: String,
    pub(crate) board_light: String,
    pub(crate) board_dark: String,
    pub(crate) board_border: String,
}

impl Default for CustomTheme {
    fn default() -> Self {
        Self {
            bg: "#14110e".to_string(),
            panel: "#201a15".to_string(),
            border: "#3d3126".to_string(),
            text: "#efe6d5".to_string(),
            muted: "#a89a86".to_string(),
            accent: "#c9a227".to_string(),
            board_light: "#e8d3b0".to_string(),
            board_dark: "#8b5a2b".to_string(),
            board_border: "#3b2a1a".to_string(),
        }
    }
}

fn default_piece_set() -> String {
    "cburnett".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct CustomPersona {
    pub(crate) name: String,
    pub(crate) accent: String,
    pub(crate) theatricality: u8,
    pub(crate) confidence: u8,
    pub(crate) warmth: u8,
    pub(crate) humor: u8,
    pub(crate) aggression: u8,
    pub(crate) verbosity: u8,
    pub(crate) deception: u8,
}

impl Default for CustomPersona {
    fn default() -> Self {
        Self {
            name: "Custom Persona".to_string(),
            accent: "#c9a227".to_string(),
            theatricality: 7,
            confidence: 6,
            warmth: 5,
            humor: 5,
            aggression: 5,
            verbosity: 5,
            deception: 4,
        }
    }
}

impl Settings {
    pub(crate) fn persona_name(&self) -> String {
        if self.persona == "Custom" && !self.custom_persona.name.trim().is_empty() {
            self.custom_persona.name.trim().to_string()
        } else {
            self.persona.clone()
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_agent_mode() -> String {
    "prompt".to_string()
}

fn default_persona_style() -> String {
    "subtle".to_string()
}

fn default_time_control() -> String {
    "rapid".to_string()
}

fn default_clock_initial() -> u64 {
    600_000
}

fn default_clock_increment() -> u64 {
    5_000
}

fn default_bluff_intensity() -> u8 {
    50
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Settings {
    pub(crate) model: String,
    pub(crate) persona: String,
    pub(crate) ui_language: String,
    pub(crate) agent_language: String,
    pub(crate) agent_language_name: String,
    pub(crate) response_mode: String,
    #[serde(default = "default_agent_mode")]
    pub(crate) agent_mode: String,
    #[serde(default = "default_persona_style")]
    pub(crate) persona_style: String,
    #[serde(default)]
    pub(crate) custom_persona: CustomPersona,
    #[serde(default = "default_bluff_intensity")]
    pub(crate) bluff_intensity: u8,
    pub(crate) stockfish_depth: u32,
    pub(crate) multipv: usize,
    pub(crate) difficulty: String,
    pub(crate) theme: String,
    #[serde(default)]
    pub(crate) custom_theme: CustomTheme,
    #[serde(default = "default_piece_set")]
    pub(crate) piece_set: String,
    #[serde(default = "default_true")]
    pub(crate) chat_enabled: bool,
    #[serde(default = "default_true")]
    pub(crate) coach_enabled: bool,
    #[serde(default = "default_true")]
    pub(crate) draw_offers_enabled: bool,
    #[serde(default)]
    pub(crate) clock_enabled: bool,
    #[serde(default = "default_time_control")]
    pub(crate) time_control: String,
    #[serde(default = "default_clock_initial")]
    pub(crate) clock_initial_ms: u64,
    #[serde(default = "default_clock_increment")]
    pub(crate) clock_increment_ms: u64,
    pub(crate) sound: bool,
    pub(crate) animations: bool,
    pub(crate) font_scale: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            model: "opencode-go/deepseek-v4.1-flash".to_string(),
            persona: "Theater Master".to_string(),
            ui_language: "en".to_string(),
            agent_language: "en".to_string(),
            agent_language_name: "English".to_string(),
            response_mode: "auto".to_string(),
            agent_mode: default_agent_mode(),
            persona_style: default_persona_style(),
            custom_persona: CustomPersona::default(),
            bluff_intensity: default_bluff_intensity(),
            stockfish_depth: 11,
            multipv: 4,
            difficulty: "intermediate".to_string(),
            theme: "salon".to_string(),
            custom_theme: CustomTheme::default(),
            piece_set: default_piece_set(),
            chat_enabled: true,
            coach_enabled: true,
            draw_offers_enabled: true,
            clock_enabled: false,
            time_control: default_time_control(),
            clock_initial_ms: default_clock_initial(),
            clock_increment_ms: default_clock_increment(),
            sound: true,
            animations: true,
            font_scale: 1.2,
        }
    }
}

pub(crate) fn normalize_settings(mut settings: Settings) -> Settings {
    let defaults = Settings::default();
    if settings.model.trim().is_empty() {
        settings.model = defaults.model;
    }
    if settings.persona.trim().is_empty() {
        settings.persona = defaults.persona;
    }
    if settings.ui_language.trim().is_empty() {
        settings.ui_language = defaults.ui_language;
    }
    if settings.agent_language.trim().is_empty() {
        settings.agent_language = defaults.agent_language;
    }
    if settings.agent_language_name.trim().is_empty() {
        settings.agent_language_name = match settings.agent_language.as_str() {
            "et" => "Estonian".to_string(),
            _ => defaults.agent_language_name,
        };
    }
    let mapped = match settings.difficulty.as_str() {
        "theater" => "beginner",
        "standard" => "intermediate",
        "coach" => "master",
        other => other,
    };
    settings.difficulty = match mapped {
        "beginner" | "casual" | "intermediate" | "strong" | "master" | "custom" => {
            mapped.to_string()
        }
        _ => defaults.difficulty,
    };
    if let Some((multipv, depth, _)) = preset_params(&settings.difficulty) {
        settings.multipv = multipv;
        settings.stockfish_depth = depth;
    } else {
        settings.multipv = settings.multipv.clamp(1, 6);
        settings.stockfish_depth = settings.stockfish_depth.clamp(5, 20);
    }
    if !(0.9..=1.5).contains(&settings.font_scale) {
        settings.font_scale = defaults.font_scale;
    }
    if !matches!(
        settings.theme.as_str(),
        "salon" | "tournament" | "midnight" | "custom"
    ) {
        settings.theme = defaults.theme;
    }
    if settings.piece_set.trim().is_empty() {
        settings.piece_set = default_piece_set();
    }
    if !matches!(settings.agent_mode.as_str(), "prompt" | "mcp") {
        settings.agent_mode = default_agent_mode();
    }
    if !matches!(
        settings.persona_style.as_str(),
        "off" | "subtle" | "theatrical" | "strong"
    ) {
        settings.persona_style = default_persona_style();
    }
    let persona_defaults = CustomPersona::default();
    let custom_name = settings
        .custom_persona
        .name
        .lines()
        .next()
        .unwrap_or("")
        .trim();
    settings.custom_persona.name = if custom_name.is_empty() {
        persona_defaults.name.clone()
    } else {
        custom_name.chars().take(40).collect()
    };
    if !settings.custom_persona.accent.starts_with('#') || settings.custom_persona.accent.len() != 7
    {
        settings.custom_persona.accent = persona_defaults.accent.clone();
    }
    settings.custom_persona.theatricality = settings.custom_persona.theatricality.min(10);
    settings.custom_persona.confidence = settings.custom_persona.confidence.min(10);
    settings.custom_persona.warmth = settings.custom_persona.warmth.min(10);
    settings.custom_persona.humor = settings.custom_persona.humor.min(10);
    settings.custom_persona.aggression = settings.custom_persona.aggression.min(10);
    settings.custom_persona.verbosity = settings.custom_persona.verbosity.min(10);
    settings.custom_persona.deception = settings.custom_persona.deception.min(10);
    settings.bluff_intensity = settings.bluff_intensity.min(100);
    if let Some((initial, increment)) = clock_preset(&settings.time_control) {
        settings.clock_initial_ms = initial;
        settings.clock_increment_ms = increment;
    } else {
        settings.time_control = "custom".to_string();
        settings.clock_initial_ms = settings.clock_initial_ms.clamp(10_000, 10_800_000);
        settings.clock_increment_ms = settings.clock_increment_ms.clamp(0, 60_000);
    }
    settings
}

fn clock_preset(id: &str) -> Option<(u64, u64)> {
    match id {
        "bullet" => Some((60_000, 0)),
        "blitz" => Some((180_000, 2_000)),
        "rapid" => Some((600_000, 5_000)),
        "classical" => Some((1_800_000, 0)),
        _ => None,
    }
}

fn preset_params(difficulty: &str) -> Option<(usize, u32, u8)> {
    match difficulty {
        "beginner" => Some((6, 5, 0)),
        "casual" => Some((5, 8, 5)),
        "intermediate" => Some((4, 11, 10)),
        "strong" => Some((3, 15, 15)),
        "master" => Some((2, 19, 20)),
        _ => None,
    }
}

pub(crate) fn engine_params(settings: &Settings) -> (usize, u32, u8) {
    preset_params(&settings.difficulty).unwrap_or((
        settings.multipv.clamp(1, 6),
        settings.stockfish_depth.clamp(5, 20),
        20,
    ))
}

pub(crate) fn director_agent_config(
    settings: &Settings,
    env: Vec<(String, String)>,
) -> AgentConfig {
    AgentConfig {
        model: settings.model.clone(),
        persona: "Director".to_string(),
        language: settings.ui_language.clone(),
        language_name: ui_language_name(&settings.ui_language).to_string(),
        timeout_secs: 180,
        retries: 0,
        env,
        style_instruction: String::new(),
        persona_prompt: String::new(),
        bluff_instruction: String::new(),
    }
}

pub(crate) fn ui_language_name(code: &str) -> &'static str {
    match code {
        "et" => "Estonian",
        _ => "English",
    }
}

pub(crate) fn coach_agent_config(settings: &Settings, env: Vec<(String, String)>) -> AgentConfig {
    AgentConfig {
        model: settings.model.clone(),
        persona: "Coach".to_string(),
        language: settings.agent_language.clone(),
        language_name: settings.agent_language_name.clone(),
        timeout_secs: 120,
        retries: 0,
        env,
        style_instruction: String::new(),
        persona_prompt: String::new(),
        bluff_instruction: String::new(),
    }
}

pub(crate) fn agent_config(settings: &Settings, env: Vec<(String, String)>) -> AgentConfig {
    AgentConfig {
        model: settings.model.clone(),
        persona: settings.persona.clone(),
        language: settings.agent_language.clone(),
        language_name: settings.agent_language_name.clone(),
        timeout_secs: 180,
        retries: 1,
        env,
        persona_prompt: if settings.persona == "Custom" {
            custom_persona_instruction(&settings.custom_persona)
        } else {
            String::new()
        },
        bluff_instruction: bluff::instruction(settings.bluff_intensity),
        style_instruction: if settings.persona_style == "off" {
            String::new()
        } else if settings.persona == "Custom" {
            style::instruction_custom(&settings.custom_persona)
        } else {
            style::instruction(&settings.persona)
        },
    }
}

fn custom_persona_instruction(persona: &CustomPersona) -> String {
    format!(
        "Persona: {name}. Theatricality {theatricality}/10, confidence {confidence}/10, warmth {warmth}/10, humor {humor}/10, aggression {aggression}/10, verbosity {verbosity}/10, deception {deception}/10. Stay in character. The persona affects commentary and bounded move style, never rules or move legality. Deception colours how theatrical your exaggerations are when bluffing is allowed; never lie about the board or the rules.",
        name = persona.name,
        theatricality = persona.theatricality,
        confidence = persona.confidence,
        warmth = persona.warmth,
        humor = persona.humor,
        aggression = persona.aggression,
        verbosity = persona.verbosity,
        deception = persona.deception,
    )
}

pub(crate) fn credential_env_for_model(model: &str) -> Vec<(String, String)> {
    let provider = model.split('/').next().unwrap_or("").trim();
    credentials::env_for_provider(provider)
        .into_iter()
        .collect()
}
