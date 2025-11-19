use std::path::Path;

use anyhow::{Context, Result};
use serde::Serialize;

#[derive(Serialize)]
struct Environment<'a> {
    os: &'a str,
}

#[derive(Serialize)]
struct Cpu<'a> {
    arch: &'a str,
    vendor: &'a str,
    brand: &'a str,
}

#[derive(Serialize)]
struct Screen<'a> {
    width: u32,
    height: u32,
    format: &'a str,
}

#[derive(Serialize)]
struct Options {
    quantity: u32,
    sizes: Vec<String>,
    repeat: u32,
}

#[derive(Serialize)]
pub struct JsonRecord {
    #[serde(rename = "test")]
    pub test_name: String,
    #[serde(rename = "compOp")]
    pub comp_op: String,
    #[serde(rename = "style")]
    pub style: String,
    pub rcpms: Vec<String>,
}

#[derive(Serialize)]
struct Run<'a> {
    name: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<&'a str>,
    records: &'a [JsonRecord],
}

#[derive(Serialize)]
struct Root<'a> {
    environment: Environment<'a>,
    cpu: Cpu<'a>,
    screen: Screen<'a>,
    options: Options,
    runs: Vec<Run<'a>>,
}

pub struct JsonWriter {
    runs: Vec<(String, Option<String>, Vec<JsonRecord>)>,
    screen_w: u32,
    screen_h: u32,
    quantity: u32,
    repeat: u32,
    sizes: Vec<u32>,
}

impl JsonWriter {
    pub fn new(screen_w: u32, screen_h: u32, quantity: u32, repeat: u32, sizes: Vec<u32>) -> Self {
        Self {
            runs: Vec::new(),
            screen_w,
            screen_h,
            quantity,
            repeat,
            sizes,
        }
    }

    pub fn push_run(&mut self, name: impl Into<String>, version: Option<String>, records: Vec<JsonRecord>) {
        self.runs.push((name.into(), version, records));
    }

    pub fn write(&self, path: &Path) -> Result<()> {
        let mut run_refs = Vec::new();
        for (name, version, records) in &self.runs {
            run_refs.push(Run {
                name,
                version: version.as_deref(),
                records,
            });
        }
        let root = Root {
            environment: Environment { os: os_name() },
            cpu: Cpu {
                arch: arch_name(),
                vendor: "unknown",
                brand: "unknown",
            },
            screen: Screen {
                width: self.screen_w,
                height: self.screen_h,
                format: "prgb32",
            },
            options: Options {
                quantity: self.quantity,
                sizes: self
                    .sizes
                    .iter()
                    .map(|s| format!("{}x{}", s, s))
                    .collect(),
                repeat: self.repeat,
            },
            runs: run_refs,
        };
        let data = serde_json::to_string_pretty(&root).context("serialize benchmark JSON")?;
        std::fs::write(path, data).with_context(|| format!("write {}", path.display()))
    }
}

fn os_name() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "osx"
    }
    #[cfg(target_os = "linux")]
    {
        "linux"
    }
    #[cfg(target_os = "windows")]
    {
        "windows"
    }
    #[cfg(all(not(target_os = "macos"), not(target_os = "linux"), not(target_os = "windows"), target_os = "ios"))]
    {
        "ios"
    }
    #[cfg(all(not(target_os = "macos"), not(target_os = "linux"), not(target_os = "windows"), not(target_os = "ios")))]
    {
        "unknown"
    }
}

fn arch_name() -> &'static str {
    #[cfg(target_arch = "x86_64")]
    {
        "x86_64"
    }
    #[cfg(target_arch = "x86")]
    {
        "x86"
    }
    #[cfg(target_arch = "aarch64")]
    {
        "aarch64"
    }
    #[cfg(target_arch = "arm")]
    {
        "aarch32"
    }
    #[cfg(all(
        not(target_arch = "x86_64"),
        not(target_arch = "x86"),
        not(target_arch = "aarch64"),
        not(target_arch = "arm")
    ))]
    {
        "unknown"
    }
}
