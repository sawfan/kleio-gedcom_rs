//! Minimal source-preserving GEDCOM parser used by local authoring.
//!
//! This is intentionally separate from `crate::import::gedcom`, which maps GEDCOM
//! into normalized Kleio records through the optional `ged_io` dependency. Local
//! authoring needs raw source details such as notes and dangling family references
//! so it can generate reviewable files and import warnings without losing context.

use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct MinimalGedcomParseResult {
    pub(super) document: MinimalGedcomDocument,
    pub(super) parser: String,
    pub(super) warning: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct MinimalGedcomDocument {
    pub(super) individuals: BTreeMap<String, MinimalGedcomIndividual>,
    pub(super) families: BTreeMap<String, MinimalGedcomFamily>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct MinimalGedcomIndividual {
    pub(super) xref: String,
    pub(super) name: Option<String>,
    pub(super) sex: Option<String>,
    pub(super) events: Vec<MinimalGedcomEvent>,
    pub(super) notes: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct MinimalGedcomFamily {
    pub(super) husband: Option<String>,
    pub(super) wife: Option<String>,
    pub(super) children: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct MinimalGedcomEvent {
    pub(super) event_type: String,
    pub(super) date: Option<String>,
    pub(super) place: Option<String>,
    pub(super) notes: Vec<String>,
}

pub(super) fn parse_gedcom_document(text: &str) -> MinimalGedcomParseResult {
    MinimalGedcomParseResult {
        document: parse_minimal_gedcom(text),
        parser: "minimal".to_string(),
        warning: None,
    }
}

fn parse_minimal_gedcom(text: &str) -> MinimalGedcomDocument {
    let lines = text.lines().map(parse_gedcom_line).collect::<Vec<_>>();
    let mut document = MinimalGedcomDocument::default();
    let mut current_individual = None::<String>;
    let mut current_family = None::<String>;
    let mut current_event = None::<MinimalGedcomEvent>;
    let mut current_note_target = None::<GedcomNoteTarget>;

    for line in lines {
        let Some(line) = line else {
            continue;
        };

        if line.level == 0 {
            flush_event(
                &mut document,
                current_individual.as_deref(),
                &mut current_event,
            );
            current_individual = None;
            current_family = None;
            current_note_target = None;
            if line.tag == "INDI" {
                if let Some(xref) = line.xref {
                    let slug = gedcom_xref_slug(&xref);
                    document.individuals.insert(
                        xref.clone(),
                        MinimalGedcomIndividual {
                            xref: slug,
                            ..Default::default()
                        },
                    );
                    current_individual = Some(xref);
                }
            } else if line.tag == "FAM"
                && let Some(xref) = line.xref
            {
                document
                    .families
                    .insert(xref.clone(), MinimalGedcomFamily::default());
                current_family = Some(xref);
            }
            continue;
        }

        if let Some(xref) = current_individual.as_deref() {
            if line.level == 1 {
                flush_event(&mut document, Some(xref), &mut current_event);
                match line.tag.as_str() {
                    "NAME" => {
                        current_note_target = None;
                        if let Some(individual) = document.individuals.get_mut(xref) {
                            individual.name = line.value;
                        }
                    }
                    "SEX" => {
                        current_note_target = None;
                        if let Some(individual) = document.individuals.get_mut(xref) {
                            individual.sex = line.value.map(|value| match value.as_str() {
                                "M" => "male".to_string(),
                                "F" => "female".to_string(),
                                _ => "unknown".to_string(),
                            });
                        }
                    }
                    "BIRT" | "DEAT" | "RESI" => {
                        current_note_target = None;
                        current_event = Some(MinimalGedcomEvent {
                            event_type: match line.tag.as_str() {
                                "BIRT" => "birth".to_string(),
                                "DEAT" => "death".to_string(),
                                "RESI" => "residence".to_string(),
                                _ => "event".to_string(),
                            },
                            date: None,
                            place: None,
                            notes: Vec::new(),
                        });
                    }
                    "NOTE" => {
                        if let (Some(value), Some(individual)) =
                            (line.value, document.individuals.get_mut(xref))
                        {
                            individual.notes.push(value);
                            current_note_target = Some(GedcomNoteTarget::Individual);
                        }
                    }
                    _ => current_note_target = None,
                }
            } else if line.level == 2
                && let Some(event) = current_event.as_mut()
            {
                match line.tag.as_str() {
                    "DATE" => {
                        current_note_target = None;
                        event.date = line.value;
                    }
                    "PLAC" => {
                        current_note_target = None;
                        event.place = line.value;
                    }
                    "NOTE" => {
                        if let Some(value) = line.value {
                            event.notes.push(value);
                            current_note_target = Some(GedcomNoteTarget::Event);
                        }
                    }
                    _ => current_note_target = None,
                }
            } else if matches!(line.tag.as_str(), "CONT" | "CONC") {
                append_note_continuation(
                    &mut document,
                    xref,
                    &mut current_event,
                    current_note_target,
                    &line.tag,
                    line.value.as_deref().unwrap_or(""),
                );
            }
            continue;
        }

        if let Some(xref) = current_family.as_deref()
            && line.level == 1
            && let Some(family) = document.families.get_mut(xref)
        {
            match line.tag.as_str() {
                "HUSB" => family.husband = line.value.map(pointer_slug),
                "WIFE" => family.wife = line.value.map(pointer_slug),
                "CHIL" => {
                    if let Some(child) = line.value.map(pointer_slug) {
                        family.children.push(child);
                    }
                }
                _ => {}
            }
        }
    }

    flush_event(
        &mut document,
        current_individual.as_deref(),
        &mut current_event,
    );
    document
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GedcomNoteTarget {
    Individual,
    Event,
}

fn append_note_continuation(
    document: &mut MinimalGedcomDocument,
    individual_xref: &str,
    current_event: &mut Option<MinimalGedcomEvent>,
    target: Option<GedcomNoteTarget>,
    tag: &str,
    value: &str,
) {
    let Some(target) = target else {
        return;
    };

    match target {
        GedcomNoteTarget::Individual => {
            let Some(individual) = document.individuals.get_mut(individual_xref) else {
                return;
            };
            append_to_last_note(&mut individual.notes, tag, value);
        }
        GedcomNoteTarget::Event => {
            let Some(event) = current_event.as_mut() else {
                return;
            };
            append_to_last_note(&mut event.notes, tag, value);
        }
    }
}

fn append_to_last_note(notes: &mut [String], tag: &str, value: &str) {
    let Some(note) = notes.last_mut() else {
        return;
    };

    match tag {
        "CONT" => {
            note.push('\n');
            note.push_str(value);
        }
        "CONC" => note.push_str(value),
        _ => {}
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GedcomLine {
    level: u32,
    xref: Option<String>,
    tag: String,
    value: Option<String>,
}

fn parse_gedcom_line(line: &str) -> Option<GedcomLine> {
    let mut parts = line.split_whitespace();
    let level = parts.next()?.parse::<u32>().ok()?;
    let first = parts.next()?.to_string();
    let (xref, tag) = if first.starts_with('@') && first.ends_with('@') {
        (Some(strip_pointer(first)), parts.next()?.to_string())
    } else {
        (None, first)
    };
    let value = parts.collect::<Vec<_>>().join(" ");
    Some(GedcomLine {
        level,
        xref,
        tag,
        value: (!value.is_empty()).then_some(value),
    })
}

fn flush_event(
    document: &mut MinimalGedcomDocument,
    individual_xref: Option<&str>,
    current_event: &mut Option<MinimalGedcomEvent>,
) {
    let Some(event) = current_event.take() else {
        return;
    };
    let Some(xref) = individual_xref else {
        return;
    };
    if let Some(individual) = document.individuals.get_mut(xref) {
        individual.events.push(event);
    }
}

fn strip_pointer(value: String) -> String {
    value.trim_matches('@').to_string()
}

fn pointer_slug(value: String) -> String {
    gedcom_xref_slug(&strip_pointer(value))
}

fn gedcom_xref_slug(xref: &str) -> String {
    safe_slug(xref.trim_matches('@'))
}

fn safe_slug(value: &str) -> String {
    let mut slug = String::new();
    let mut last_dash = false;
    for ch in value.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            last_dash = false;
        } else if !last_dash {
            slug.push('-');
            last_dash = true;
        }
    }
    slug.trim_matches('-').to_string()
}
