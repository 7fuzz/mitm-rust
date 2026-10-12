use eframe::egui::{self, RichText};
use mitm_core::utils::cvss::{
    AttackComplexity, AttackRequirements, AttackVector, CvssV3Base, CvssV4Base, CvssVersion, ImpactMetric,
    PrivilegesRequired, Scope, Severity, UserInteraction, UserInteractionV4,
};
use crate::ui::theme::{self, Tone};
use crate::widgets::pills::badge;
use super::CvssState;
use crate::ui::icons;

pub fn show(ui: &mut egui::Ui, state: &mut CvssState) {
    ui.horizontal(|ui| {
        for version in CvssVersion::ALL {
            if ui.selectable_label(state.prefs.version == version, version.label()).clicked() {
                state.set_version(version);
            }
        }
        ui.separator();

        let mut text = state.draft.clone().unwrap_or_else(|| state.vector());
        let response = ui.add(egui::TextEdit::singleline(&mut text).code_editor().desired_width(450.0));
        if response.changed() {
            state.edit_vector(text);
        }
        if ui.small_button(icons::COPY).on_hover_text("Copy vector").clicked() {
            ui.ctx().copy_text(state.vector());
        }
    });
    if let Some(err) = &state.error {
        ui.colored_label(theme::color(ui, Tone::Danger), err);
    }
    ui.separator();

    let score = state.score();
    ui.horizontal(|ui| {
        badge(ui, &format!("{:.1} {}", score.base_score, score.severity.label()), severity_tone(score.severity));
        if let Some(sub) = score.sub_scores {
            ui.add_space(8.0);
            ui.label(RichText::new(format!("Impact {:.1} · Exploitability {:.1}", sub.impact, sub.exploitability)).weak());
        }
    });
    ui.separator();

    let changed = egui::ScrollArea::vertical()
        .id_salt("cvss_metrics_scroll")
        .show(ui, |ui| match state.prefs.version {
            CvssVersion::V3_1 => v3_metrics(ui, &mut state.prefs.v3),
            CvssVersion::V4_0 => v4_metrics(ui, &mut state.prefs.v4),
        })
        .inner;
    if changed {
        state.discard_draft();
    }
}

fn v3_metrics(ui: &mut egui::Ui, b: &mut CvssV3Base) -> bool {
    let mut changed = false;
    group(ui, "Exploitability", |ui| {
        changed |= row(ui, "Attack Vector (AV)", &mut b.av, &AttackVector::ALL, AttackVector::label);
        changed |= row(ui, "Attack Complexity (AC)", &mut b.ac, &AttackComplexity::ALL, AttackComplexity::label);
        changed |= row(ui, "Privileges Required (PR)", &mut b.pr, &PrivilegesRequired::ALL, PrivilegesRequired::label);
        changed |= row(ui, "User Interaction (UI)", &mut b.ui, &UserInteraction::ALL, UserInteraction::label);
        changed |= row(ui, "Scope (S)", &mut b.s, &Scope::ALL, Scope::label);
    });
    group(ui, "Impact", |ui| {
        changed |= impact_rows(ui, ["Confidentiality (C)", "Integrity (I)", "Availability (A)"], [&mut b.c, &mut b.i, &mut b.a]);
    });
    changed
}

fn v4_metrics(ui: &mut egui::Ui, b: &mut CvssV4Base) -> bool {
    let mut changed = false;
    group(ui, "Exploitability", |ui| {
        changed |= row(ui, "Attack Vector (AV)", &mut b.av, &AttackVector::ALL, AttackVector::label);
        changed |= row(ui, "Attack Complexity (AC)", &mut b.ac, &AttackComplexity::ALL, AttackComplexity::label);
        changed |= row(ui, "Attack Requirements (AT)", &mut b.at, &AttackRequirements::ALL, AttackRequirements::label);
        changed |= row(ui, "Privileges Required (PR)", &mut b.pr, &PrivilegesRequired::ALL, PrivilegesRequired::label);
        changed |= row(ui, "User Interaction (UI)", &mut b.ui, &UserInteractionV4::ALL, UserInteractionV4::label);
    });
    group(ui, "Vulnerable System Impact", |ui| {
        changed |= impact_rows(ui, ["Confidentiality (VC)", "Integrity (VI)", "Availability (VA)"], [&mut b.vc, &mut b.vi, &mut b.va]);
    });
    group(ui, "Subsequent System Impact", |ui| {
        changed |= impact_rows(ui, ["Confidentiality (SC)", "Integrity (SI)", "Availability (SA)"], [&mut b.sc, &mut b.si, &mut b.sa]);
    });
    changed
}

fn group(ui: &mut egui::Ui, title: &str, add_rows: impl FnOnce(&mut egui::Ui)) {
    ui.label(RichText::new(title).strong());
    ui.add_space(4.0);
    egui::Grid::new(title).num_columns(2).spacing([16.0, 10.0]).show(ui, add_rows);
    ui.add_space(12.0);
}

fn impact_rows(ui: &mut egui::Ui, labels: [&str; 3], values: [&mut ImpactMetric; 3]) -> bool {
    let mut changed = false;
    for (label, value) in labels.into_iter().zip(values) {
        changed |= row(ui, label, value, &ImpactMetric::ALL, ImpactMetric::label);
    }
    changed
}

fn row<T: Copy + PartialEq>(ui: &mut egui::Ui, label: &str, value: &mut T, options: &[T], name: fn(T) -> &'static str) -> bool {
    ui.label(label);
    let changed = ui
        .horizontal(|ui| {
            let mut changed = false;
            for &opt in options {
                changed |= ui.selectable_value(value, opt, name(opt)).changed();
            }
            changed
        })
        .inner;
    ui.end_row();
    changed
}

fn severity_tone(severity: Severity) -> Tone {
    match severity {
        Severity::None => Tone::Neutral,
        Severity::Low => Tone::Success,
        Severity::Medium => Tone::Warn,
        Severity::High => Tone::Caution,
        Severity::Critical => Tone::Danger,
    }
}
