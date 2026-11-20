//! Sampler Pad Actions.
//!
//! A pad (per Sampler Bank, or on every bank) can run an action instead of playing a sample:
//! start an application, open a URL / file, or run GoXLR commands (including quick switches for
//! effect presets and profiles). The configuration lives in `sampler-actions.json`.
//!
//! Anything that starts programs on the machine is disabled while network access to the daemon is
//! enabled, otherwise anybody who can reach the web UI could run commands on this computer.

use crate::config_store::DeviceConfigStore;
use anyhow::{Context, Result, bail};
use goxlr_ipc::{GoXLRCommand, PadActionKind, SamplerAction, SamplerActionConfig};
use goxlr_types::{SampleBank, SampleButtons};
use log::debug;
use std::path::PathBuf;
use std::process::{Command, Stdio};

pub const SAMPLER_ACTIONS_FILE_NAME: &str = "sampler-actions.json";

pub type SamplerActionsHandle = DeviceConfigStore<SamplerActionConfig>;

/// Finds the action for a pad, an action for the exact bank wins over an 'every bank' one.
pub fn find_action(
    config: &SamplerActionConfig,
    bank: SampleBank,
    button: SampleButtons,
) -> Option<&SamplerAction> {
    let matching = |a: &&SamplerAction| a.button == button;
    config
        .actions
        .iter()
        .filter(matching)
        .find(|a| a.bank == Some(bank))
        .or_else(|| {
            config
                .actions
                .iter()
                .filter(matching)
                .find(|a| a.bank.is_none())
        })
}

/// The pads that have an action on the given bank (used for lighting). Programs are left out
/// when `programs_allowed` is false so blocked pads don't look usable.
pub fn pads_for_bank(
    config: &SamplerActionConfig,
    bank: SampleBank,
    programs_allowed: bool,
) -> Vec<SampleButtons> {
    if !config.enabled {
        return vec![];
    }

    [
        SampleButtons::TopLeft,
        SampleButtons::TopRight,
        SampleButtons::BottomLeft,
        SampleButtons::BottomRight,
    ]
    .into_iter()
    .filter(|button| {
        find_action(config, bank, *button)
            .map(|a| programs_allowed || !a.action.runs_programs())
            .unwrap_or(false)
    })
    .collect()
}

/// Replaces (or adds) the action for exactly this bank / pad combination.
pub fn upsert(
    config: &mut SamplerActionConfig,
    bank: Option<SampleBank>,
    button: SampleButtons,
    action: PadActionKind,
) {
    if let Some(existing) = config
        .actions
        .iter_mut()
        .find(|a| a.bank == bank && a.button == button)
    {
        existing.action = action;
    } else {
        config.actions.push(SamplerAction {
            bank,
            button,
            action,
        });
    }
}

pub fn remove(config: &mut SamplerActionConfig, bank: Option<SampleBank>, button: SampleButtons) {
    config
        .actions
        .retain(|a| !(a.bank == bank && a.button == button));
}

/// Checks an action before it is stored.
pub fn validate(action: &PadActionKind) -> Result<()> {
    match action {
        PadActionKind::Launch { command_line, .. } => {
            if split_command_line(command_line)?.is_empty() {
                bail!("The command to launch is empty");
            }
        }
        PadActionKind::Open { target } => {
            if target.trim().is_empty() {
                bail!("The target to open is empty");
            }
        }
        PadActionKind::Macro(commands) if commands.is_empty() => {
            bail!("The macro doesn't contain any commands");
        }
        PadActionKind::SwitchProfile { name, .. } if name.trim().is_empty() => {
            bail!("No profile name specified");
        }
        _ => {}
    }

    // Pad actions must not be able to edit (or trigger) pad actions, a pad pressing itself in a
    // loop, or rewriting the configuration, helps nobody.
    for command in action.commands() {
        if matches!(
            command,
            GoXLRCommand::SetSamplerActionsEnabled(_)
                | GoXLRCommand::SetSamplerAction(..)
                | GoXLRCommand::ClearSamplerAction(..)
                | GoXLRCommand::TestSamplerAction(..)
        ) {
            bail!("Sampler actions can't run Sampler Action commands");
        }
    }
    Ok(())
}

/// Splits a command line into the program and its arguments, an empty result means no program.
fn split_command_line(line: &str) -> Result<Vec<String>> {
    if line.trim().is_empty() {
        return Ok(vec![]);
    }

    #[cfg(unix)]
    let parts = shell_words::split(line).context("The command line couldn't be parsed")?;
    #[cfg(not(unix))]
    let parts: Vec<String> = windows_args::Args::parse_cmd(line).collect();

    // Don't accept a blank program name (`""`)
    if parts
        .first()
        .is_some_and(|program| program.trim().is_empty())
    {
        return Ok(vec![]);
    }
    Ok(parts)
}

/// Starts an application, fully detached so it outlives the daemon.
pub fn launch(command_line: &str, working_dir: Option<&str>) -> Result<()> {
    let parts = split_command_line(command_line)?;
    let Some((program, args)) = parts.split_first() else {
        bail!("The command to launch is empty");
    };

    debug!("Launching {program} {args:?}");
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    if let Some(dir) = working_dir.filter(|d| !d.trim().is_empty()) {
        command.current_dir(PathBuf::from(dir));
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP
        command.creation_flags(0x0000_0008 | 0x0000_0200);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // Own process group, so it isn't tied to the daemon's.
        command.process_group(0);
    }

    command
        .spawn()
        .with_context(|| format!("Unable to start {program}"))?;
    Ok(())
}

/// Opens a URL or file with the default application.
pub fn open_target(target: &str) -> Result<()> {
    open::that(target).with_context(|| format!("Unable to open {target}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn macro_action() -> PadActionKind {
        PadActionKind::SwitchEffectPreset(goxlr_types::EffectBankPresets::Preset2)
    }

    #[test]
    fn exact_bank_wins_over_every_bank() {
        let mut config = SamplerActionConfig {
            enabled: true,
            actions: vec![],
        };
        upsert(&mut config, None, SampleButtons::TopLeft, macro_action());
        upsert(
            &mut config,
            Some(SampleBank::B),
            SampleButtons::TopLeft,
            PadActionKind::Open {
                target: "https://example.org".into(),
            },
        );

        let on_a = find_action(&config, SampleBank::A, SampleButtons::TopLeft).unwrap();
        assert!(on_a.bank.is_none());
        let on_b = find_action(&config, SampleBank::B, SampleButtons::TopLeft).unwrap();
        assert_eq!(on_b.bank, Some(SampleBank::B));
        assert!(find_action(&config, SampleBank::A, SampleButtons::TopRight).is_none());
    }

    #[test]
    fn upsert_replaces_and_remove_clears() {
        let mut config = SamplerActionConfig::default();
        upsert(&mut config, None, SampleButtons::TopLeft, macro_action());
        upsert(&mut config, None, SampleButtons::TopLeft, macro_action());
        assert_eq!(config.actions.len(), 1);
        remove(&mut config, None, SampleButtons::TopLeft);
        assert!(config.actions.is_empty());
    }

    #[test]
    fn programs_are_left_out_when_blocked() {
        let mut config = SamplerActionConfig {
            enabled: true,
            actions: vec![],
        };
        upsert(&mut config, None, SampleButtons::TopLeft, macro_action());
        upsert(
            &mut config,
            None,
            SampleButtons::TopRight,
            PadActionKind::Launch {
                command_line: "notepad".into(),
                working_dir: None,
            },
        );

        assert_eq!(pads_for_bank(&config, SampleBank::A, true).len(), 2);
        assert_eq!(
            pads_for_bank(&config, SampleBank::A, false),
            vec![SampleButtons::TopLeft]
        );

        config.enabled = false;
        assert!(pads_for_bank(&config, SampleBank::A, true).is_empty());
    }

    #[test]
    fn validation() {
        assert!(validate(&macro_action()).is_ok());
        assert!(
            validate(&PadActionKind::Launch {
                command_line: "  ".into(),
                working_dir: None
            })
            .is_err()
        );
        assert!(validate(&PadActionKind::Macro(vec![])).is_err());
        assert!(
            validate(&PadActionKind::Command(Box::new(
                GoXLRCommand::ClearSamplerAction(None, SampleButtons::TopLeft)
            )))
            .is_err()
        );
    }
}
