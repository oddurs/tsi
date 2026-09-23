//! Structured output: the data each command produces.
//!
//! Every command computes one serializable value, then shows it. `--output
//! json` prints it inside an [`Envelope`] that names the command and the
//! schema version; the pretty output is a [view](super::views) of the same
//! value. What a command knows and what it prints can't drift apart, because
//! both come from here.
//!
//! Field names carry their units (`delta_v_mps`, `propellant_kg`), as in the
//! library's serialized types.

use serde::Serialize;
use tsiolkovsky::engine::Engine;
use tsiolkovsky::optimizer::JSON_SCHEMA_VERSION;

/// Every JSON document tsi prints: which command, which schema version, and
/// the command's data.
#[derive(Serialize)]
pub struct Envelope<'a, T: Serialize> {
    pub schema_version: u32,
    pub command: &'a str,
    #[serde(flatten)]
    pub data: T,
}

impl<'a, T: Serialize> Envelope<'a, T> {
    pub fn new(command: &'a str, data: T) -> Self {
        Self {
            schema_version: JSON_SCHEMA_VERSION,
            command,
            data,
        }
    }

    /// Pretty-printed JSON.
    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(self)
    }
}

/// What `tsi calculate` works out for one stage.
#[derive(Debug, Clone, Serialize)]
pub struct CalculateReport {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engine: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engine_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub propellant: Option<String>,
    /// Vacuum Isp used for the delta-v
    pub isp_s: f64,
    pub mass_ratio: f64,
    /// Delta-v in vacuum, carrying nothing
    pub delta_v_mps: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub propellant_kg: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dry_mass_kg: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wet_mass_kg: Option<f64>,
    /// Total vacuum thrust
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thrust_n: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub burn_time_s: Option<f64>,
    /// Vacuum thrust over wet weight at ignition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub twr_vacuum: Option<f64>,
}

/// What `tsi engines` lists.
#[derive(Serialize)]
pub struct EnginesReport<'a> {
    pub engines: &'a [&'a Engine],
}
