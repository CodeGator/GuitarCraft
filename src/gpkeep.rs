//! What a .gp file holds that this app doesn't use, kept so it can be written back when the song
//! is saved. The design and its rules are in `docs/gp-keep-design.md`.
//!
//! The settings of the file as a whole, of each track, of each bar, and the layout and views. A
//! kept setting is written back until the thing it depends on changes, so what the song held when
//! it was read is recorded beside it. A change over time names a bar by its number, so it is kept
//! on that bar and moves with it; one at the very start of the song sets how the song begins, and
//! stays at the start.

use crate::gpstyle::{self, Raw, V};
use crate::model::{Bar, Instrument, MasterBar, Song, Track};
use crate::style::{HeaderItem, Stylesheet};
use roxmltree::{Document, Node};
use std::io::Read;
use std::sync::Arc;

/// Archive entries written back as they were.
const FILES: [&str; 2] = ["Content/Preferences.json", "meta.json"];

/// Archive entries that make up the score's views: which staves each shows and how its bars are
/// laid out.
fn of_the_views(name: &str) -> bool {
    !name.ends_with('/')
        && (name.starts_with("Content/ScoreViews/")
            || name.starts_with("Content/Stylesheets/")
            || matches!(name, "Content/LayoutConfiguration" | "Content/PartConfiguration"))
}

/// The newer stylesheet of the whole score, which repeats settings of the other one.
const SCORE_STYLE: &str = "Content/Stylesheets/score.gpss";

/// What the views depend on: the number of bars, and each track in order with what it shows.
#[derive(Debug, PartialEq, Default)]
struct Shape {
    bars: usize,
    tracks: Vec<(String, Instrument, [bool; 5])>,
}

fn shape(song: &Song) -> Shape {
    let track = |t: &Track| {
        let shows = [t.visible, t.multirest, t.show_std, t.show_tab, t.grand()];
        (t.name.clone(), t.instrument, shows)
    };
    Shape { bars: song.bar_count(), tracks: song.tracks.iter().map(track).collect() }
}

/// The views of the file.
#[derive(Debug, PartialEq, Default)]
struct Views {
    /// <ScoreViews> in the score (empty if the file had none).
    element: String,
    files: Vec<(String, Vec<u8>)>,
    opened: Shape,
}

/// A change over time (<Automation>): its text before and after the number of its bar.
#[derive(Clone, Debug, PartialEq)]
pub struct Change {
    /// Which list of changes it belongs to, and its place among those of one bar.
    list: usize,
    order: usize,
    before: String,
    after: String,
    /// The channel strip number it changes, for a mixer or effect change.
    param: Option<usize>,
    /// The path of the sound it switches to.
    sound: Option<String>,
    /// It happens as its bar begins.
    on_the_bar: bool,
}

impl Change {
    fn at(&self, bar: usize) -> String {
        format!("{}{bar}{}", self.before, self.after)
    }
}

/// Changes as read, parted into those at the very start of the song and the ones on bars.
fn part_start(changes: Vec<(usize, Change)>) -> (Vec<Change>, Vec<(usize, Change)>) {
    let (start, later): (Vec<_>, Vec<_>) = changes.into_iter().partition(|(bar, c)| *bar == 0 && c.on_the_bar);
    (start.into_iter().map(|(_, c)| c).collect(), later)
}

/// The changes at the start and those on the bars, each with the number its bar has now. A bar's
/// change that has come to the start takes the place of the one that was there.
fn together<'a>(start: &'a [Change], on_bars: impl Iterator<Item = (usize, &'a Change)>) -> Vec<(usize, &'a Change)> {
    let mut out: Vec<(usize, &Change)> = on_bars.collect();
    let replaced = |s: &Change| out.iter().any(|(bar, c)| *bar == 0 && c.on_the_bar && c.list == s.list && c.param == s.param);
    let kept: Vec<_> = start.iter().filter(|s| !replaced(s)).map(|s| (0, s)).collect();
    out.extend(kept);
    out
}

/// An element with each list of changes taken out, and a mark left where it was.
#[derive(Debug, PartialEq)]
struct Template(String);

fn mark(list: usize) -> String {
    format!("\u{1}{list}\u{1}")
}

impl Template {
    /// Takes the lists of changes out of `element` (which may itself be one). Returns each change
    /// with the number of its bar.
    fn split(xml: &str, element: Node) -> (Template, Vec<(usize, Change)>) {
        let start = element.range().start;
        let mut text_left = xml[element.range()].to_string();
        let mut changes = vec![];
        let lists: Vec<Node> = element.descendants().filter(|n| n.has_tag_name("Automations")).collect();
        // From the last to the first, so the places before stay where they were.
        for (list, node) in lists.iter().enumerate().rev() {
            for (order, a) in node.children().filter(|c| c.has_tag_name("Automation")).enumerate() {
                let Some(bar) = child(a, "Bar").and_then(|b| b.first_child()).filter(|t| t.is_text()) else { continue };
                let Ok(number) = xml[bar.range()].trim().parse::<usize>() else { continue };
                let of = |tag: &str| child(a, tag).map(text).unwrap_or_default();
                let kind = of("Type");
                changes.push((
                    number,
                    Change {
                        list,
                        order,
                        before: xml[a.range().start..bar.range().start].to_string(),
                        after: xml[bar.range().end..a.range().end].to_string(),
                        param: kind.strip_prefix("DSPParam_").and_then(|n| n.parse().ok()),
                        sound: (kind == "Sound").then(|| of("Value").split(';').next().unwrap_or_default().to_string()),
                        on_the_bar: matches!(of("Position").as_str(), "" | "0"),
                    },
                ));
            }
            let r = node.range();
            text_left.replace_range(r.start - start..r.end - start, &mark(list));
        }
        changes.sort_by_key(|(bar, c)| (c.list, *bar, c.order));
        (Template(text_left), changes)
    }

    /// The element with its changes put back: `changes` pairs each with the number its bar has
    /// now. A list without any is left out.
    fn fill<'a>(&self, changes: impl Iterator<Item = (usize, &'a Change)>) -> String {
        let mut changes: Vec<(usize, &Change)> = changes.collect();
        changes.sort_by_key(|(bar, c)| (c.list, *bar, c.order));
        let mut out = self.0.clone();
        let mut list = 0;
        while out.contains(&mark(list)) {
            let here: String = changes.iter().filter(|(_, c)| c.list == list).map(|(bar, c)| c.at(*bar)).collect();
            let element = if here.is_empty() { here } else { format!("<Automations>{here}</Automations>") };
            out = out.replacen(&mark(list), &element, 1);
            list += 1;
        }
        out
    }
}

/// What a master bar of a .gp file holds that this app doesn't use.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct KeptMasterBar {
    /// How its key is spelled (<TransposeAs>), with the key it was read with.
    transpose_as: Option<(String, i8)>,
    /// Changes of the master effects at this bar.
    effects: Vec<Change>,
}

impl KeptMasterBar {
    /// The choice of flats or sharps, while the key is the one it was made for.
    pub fn transpose_as(&self, bar: &MasterBar) -> Option<&str> {
        self.transpose_as.as_ref().filter(|(_, key)| *key == bar.key).map(|(element, _)| element.as_str())
    }
}

/// What a bar of one track holds that this app doesn't use.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct KeptBar {
    /// The bar's further settings (<XProperties>), on each staff.
    settings: [Option<String>; 2],
    /// Changes of the mixer and of the sound at this bar.
    mixer: Vec<Change>,
    sounds: Vec<Change>,
}

impl KeptBar {
    pub fn settings(&self, staff: usize) -> Option<&str> {
        self.settings.get(staff)?.as_deref()
    }
}

fn kept_master(bar: &mut MasterBar) -> &mut KeptMasterBar {
    Arc::make_mut(bar.kept.get_or_insert_default())
}

fn kept_bar(bar: &mut Bar) -> &mut KeptBar {
    Arc::make_mut(bar.kept.get_or_insert_default())
}

/// Keeps a master bar's own settings, as read from `node`.
pub fn read_master_bar(node: Node, xml: &str, bar: &mut MasterBar) {
    if let Some(spelling) = child(node, "Key").and_then(|k| child(k, "TransposeAs")) {
        kept_master(bar).transpose_as = Some((xml[spelling.range()].to_string(), bar.key));
    }
}

/// Keeps a bar's own settings, as read from staff `staff` of `node`.
pub fn read_bar(node: Node, xml: &str, staff: usize, bar: &mut Bar) {
    if let Some(settings) = child(node, "XProperties") {
        kept_bar(bar).settings[staff] = Some(xml[settings.range()].to_string());
    }
}

/// Puts a track's changes over time on the bars they name.
pub fn place_changes(bars: &mut [Bar], changes: TrackChanges) {
    for (i, change) in changes.mixer {
        if let Some(bar) = bars.get_mut(i) {
            kept_bar(bar).mixer.push(change);
        }
    }
    for (i, change) in changes.sounds {
        if let Some(bar) = bars.get_mut(i) {
            kept_bar(bar).sounds.push(change);
        }
    }
}

/// A track's changes over time, each with the number of its bar.
#[derive(Default)]
pub struct TrackChanges {
    mixer: Vec<(usize, Change)>,
    sounds: Vec<(usize, Change)>,
}

/// The page header and footer texts, elements of <Score>.
const PAGE_TEXTS: [&str; 4] = ["FirstPageHeader", "FirstPageFooter", "PageHeader", "PageFooter"];

#[derive(Debug, PartialEq, Default)]
pub struct KeptSong {
    /// Archive entries by name.
    pub files: Vec<(String, Vec<u8>)>,
    /// The stylesheet as read.
    stylesheet: Vec<Raw>,
    /// This app's stylesheet settings as they were when the file was read.
    opened_style: Vec<(String, V)>,
    /// Whole elements of <Score>.
    page_texts: Vec<(&'static str, String)>,
    opened_headers: [Vec<HeaderItem>; 4],
    multi_voice: Option<String>,
    /// <MasterTrack>'s effects, without their changes over time (those on bars are on the
    /// master bars), and the changes at the very start.
    master_effects: Option<(Template, Vec<Change>)>,
    opened_bars: usize,
    /// The file had no title: the song was given a name to show, which isn't written back.
    untitled: Option<String>,
    /// How many bars Guitar Pro puts on each line: elements of <Score>.
    bars_per_line: Option<String>,
    /// The zoom, elements of <Score>.
    zoom: Option<String>,
    views: Views,
}

fn headers(st: &Stylesheet) -> [&Vec<HeaderItem>; 4] {
    [&st.first_header, &st.first_footer, &st.other_header, &st.other_footer]
}

fn child<'a, 'i>(n: Node<'a, 'i>, tag: &str) -> Option<Node<'a, 'i>> {
    n.children().find(|c| c.has_tag_name(tag))
}

/// An element's own text, trimmed.
fn text(n: Node) -> String {
    n.children().filter(|c| c.is_text()).filter_map(|c| c.text()).collect::<String>().trim().to_string()
}

/// The named children of `parent` as written, one after another.
fn elements(xml: &str, parent: Node, tags: &[&str]) -> String {
    tags.iter().filter_map(|tag| child(parent, tag)).map(|n| &xml[n.range()]).collect()
}

/// Reads what is to be kept from a .gp archive. `song` is the song as read from it, with all it
/// takes from the archive already set. Also marks a pickup bar on the song itself, so that it
/// stays with its bar.
pub fn read<R: Read + std::io::Seek>(zip: &mut zip::ZipArchive<R>, xml: &str, song: &mut Song) -> KeptSong {
    let mut kept = KeptSong {
        opened_style: gpstyle::entries(&song.style),
        opened_headers: headers(&song.style).map(Vec::clone),
        opened_bars: song.bar_count(),
        ..KeptSong::default()
    };
    let names: Vec<String> = zip.file_names().filter(|n| of_the_views(n)).map(String::from).collect();
    kept.views.opened = shape(song);
    let mut entry = |name: &str| {
        let mut data = vec![];
        zip.by_name(name).ok()?.read_to_end(&mut data).ok()?;
        Some(data)
    };
    for name in FILES {
        if let Some(data) = entry(name) {
            kept.files.push((name.to_string(), data));
        }
    }
    for name in names {
        if let Some(data) = entry(&name) {
            kept.views.files.push((name, data));
        }
    }
    kept.views.files.sort();
    if let Some(raw) = entry("Content/BinaryStylesheet").and_then(|d| gpstyle::raw(&d)) {
        kept.stylesheet = raw;
    }
    let Ok(doc) = Document::parse(xml) else { return kept };
    let root = doc.root_element();
    let source = |n: Node| xml[n.range()].to_string();
    if let Some(score) = child(root, "Score") {
        if child(score, "Title").is_none_or(|t| text(t).is_empty()) {
            kept.untitled = Some(song.title.clone());
        }
        for tag in PAGE_TEXTS {
            if let Some(n) = child(score, tag) {
                kept.page_texts.push((tag, source(n)));
            }
        }
        kept.multi_voice = child(score, "MultiVoice").map(source);
        let lines = elements(xml, score, &["ScoreSystemsDefaultLayout", "ScoreSystemsLayout"]);
        kept.bars_per_line = (!lines.is_empty()).then_some(lines);
        kept.zoom = Some(elements(xml, score, &["ScoreZoomPolicy", "ScoreZoom"]));
    }
    kept.views.element = elements(xml, root, &["ScoreViews"]);
    if let Some(master) = child(root, "MasterTrack") {
        if child(master, "Anacrusis").is_some()
            && let Some(first) = song.master.first_mut()
        {
            first.pickup = true;
        }
        if let Some(rse) = child(master, "RSE") {
            let (template, changes) = Template::split(xml, rse);
            let (start, later) = part_start(changes);
            for (i, change) in later {
                if let Some(bar) = song.master.get_mut(i) {
                    kept_master(bar).effects.push(change);
                }
            }
            kept.master_effects = Some((template, start));
        }
    }
    kept
}

impl KeptSong {
    /// The title to write: empty if the file had none and the song's hasn't been changed.
    pub fn title<'a>(&self, song: &'a Song) -> &'a str {
        if self.untitled.as_deref() == Some(song.title.as_str()) { "" } else { &song.title }
    }

    /// A page header or footer text, while the header and footer settings are as they were.
    pub fn page_text(&self, tag: &str, song: &Song) -> Option<&str> {
        let same = headers(&song.style).iter().zip(&self.opened_headers).all(|(now, then)| *now == then);
        same.then(|| self.page_texts.iter().find(|t| t.0 == tag).map(|t| t.1.as_str())).flatten()
    }

    /// How many bars go on each line, while the song has as many bars as it did.
    pub fn bars_per_line(&self, song: &Song) -> Option<&str> {
        self.bars_per_line.as_deref().filter(|_| song.bar_count() == self.opened_bars)
    }

    pub fn zoom(&self) -> Option<&str> {
        self.zoom.as_deref()
    }

    /// Whether the file's views still describe the song: the same bars, and the same tracks in
    /// the same order showing the same staves.
    fn views_fit(&self, song: &Song) -> bool {
        !self.views.files.is_empty() && shape(song) == self.views.opened
    }

    /// <ScoreViews> to write in the score; None when the app's own views are to be written.
    pub fn views_element(&self, song: &Song) -> Option<&str> {
        self.views_fit(song).then_some(&self.views.element)
    }

    /// The archive entries of the views, while they fit. The score's newer stylesheet is left
    /// out once a style setting has changed, as it would repeat the old one.
    pub fn view_files(&self, song: &Song) -> Option<Vec<(&str, &[u8])>> {
        let restyled = || gpstyle::entries(&song.style) != self.opened_style;
        self.views_fit(song).then(|| {
            let files = self.views.files.iter().filter(|(name, _)| name != SCORE_STYLE || !restyled());
            files.map(|(name, data)| (name.as_str(), data.as_slice())).collect()
        })
    }

    pub fn multi_voice(&self) -> Option<&str> {
        self.multi_voice.as_deref()
    }

    /// The master effects, with their changes over time where their bars now are.
    pub fn master_effects(&self, song: &Song) -> Option<String> {
        let changes = song.master.iter().enumerate().flat_map(|(i, bar)| {
            bar.kept.iter().flat_map(move |k| k.effects.iter().map(move |c| (i, c)))
        });
        let (element, start) = self.master_effects.as_ref()?;
        Some(element.fill(together(start, changes).into_iter()))
    }

    /// The stylesheet file: as read, except for the settings changed since. None when the file
    /// that was opened had no stylesheet that could be read.
    pub fn stylesheet(&self, song: &Song) -> Option<Vec<u8>> {
        (!self.stylesheet.is_empty()).then(|| gpstyle::write_over(&self.stylesheet, &self.opened_style, &song.style))
    }
}

/// The mixer numbers this app has a control for, as places in the channel strip.
fn modelled(place: usize) -> bool {
    crate::model::EQ_GPIF_INDEX.contains(&place) || matches!(place, 9 | 11 | 12)
}

/// A track's mixer: <RSE> as read, without its changes over time, and the changes at the very
/// start.
#[derive(Debug, PartialEq)]
struct Mixer {
    element: Template,
    start: Vec<Change>,
    /// The channel strip's numbers as written, the text they are in, and what this app's
    /// controls stood at when the file was read.
    numbers: String,
    opened: [f32; 16],
}

/// What a track of a .gp file holds that this app doesn't use.
#[derive(Debug, PartialEq)]
pub struct KeptTrack {
    instrument: Instrument,
    opened_bars: usize,
    /// The instrument set, and the notation that goes with it.
    instrument_set: Option<String>,
    /// The instrument set's sounds in order (a drum note names its sound by its place).
    kit: Vec<Option<u8>>,
    /// For an instrument Guitar Pro tunes and this app doesn't: each staff's open strings.
    tunings: Vec<Vec<u8>>,
    transpose: Option<String>,
    forced_sound: Option<String>,
    sounds: Option<String>,
    midi: Option<String>,
    engine: Option<String>,
    one_channel: Option<String>,
    /// The list of changes of sound, without those on bars, and the changes at the very start.
    automations: Option<(Template, Vec<Change>)>,
    mixer: Option<Mixer>,
    /// How many bars Guitar Pro puts on each line when the track is shown alone.
    bars_per_line: Option<String>,
}

/// Reads what is to be kept of a track. `track` is the song's track as read from `node`; the
/// changes over time are for its bars (see `place_changes`).
pub fn read_track(node: Node, xml: &str, track: &Track, bars: usize) -> (KeptTrack, TrackChanges) {
    let element = |tag: &str| child(node, tag).map(|n| xml[n.range()].to_string());
    // Earlier versions of this app named sounds "Midi/...", which Guitar Pro doesn't have, and
    // wrote instrument sets it can't use: nothing about such a track's instrument is kept.
    let ours = node.descendants().any(|n| n.has_tag_name("Path") && text(n).starts_with("Midi/"));
    let theirs = |tag: &str| if ours { None } else { element(tag) };
    let mut changes = TrackChanges::default();
    let mixer = child(node, "RSE").and_then(|rse| {
        let numbers = rse.descendants().find(|n| n.has_tag_name("Parameters"))?;
        let numbers = numbers.first_child().filter(|c| c.is_text())?;
        let (element, over_time) = Template::split(xml, rse);
        let (start, later) = part_start(over_time);
        changes.mixer = later;
        Some(Mixer { element, start, numbers: xml[numbers.range()].to_string(), opened: crate::gpifwrite::strip_values(track) })
    });
    let automations = child(node, "Automations").filter(|_| !ours).map(|all| {
        let (element, over_time) = Template::split(xml, all);
        let (start, later) = part_start(over_time);
        changes.sounds = later;
        (element, start)
    });
    let kept = KeptTrack {
        instrument: track.instrument,
        opened_bars: bars,
        instrument_set: theirs("InstrumentSet").map(|set| set + &element("NotationPatch").unwrap_or_default()),
        kit: if ours { vec![] } else { crate::gpif::kit_sounds(node) },
        tunings: if ours || crate::gpinstr::pitched(track.instrument).is_none() { vec![] } else { crate::gpif::staff_strings(node) },
        transpose: theirs("Transpose"),
        forced_sound: theirs("ForcedSound"),
        sounds: theirs("Sounds"),
        midi: theirs("MidiConnection"),
        engine: theirs("AudioEngineState"),
        one_channel: theirs("UseOneChannelPerString"),
        automations,
        mixer,
        bars_per_line: Some(elements(xml, node, &["SystemsDefautLayout", "SystemsLayout"])).filter(|e| !e.is_empty()),
    };
    (kept, changes)
}

impl KeptTrack {
    /// Whether the track still has the instrument its settings belong to.
    fn fits(&self, track: &Track) -> bool {
        self.instrument == track.instrument
    }

    /// The instrument set (with its notation). A drum kit is only kept while it has every sound
    /// the track uses.
    pub fn instrument_set(&self, track: &Track) -> Option<&str> {
        let complete = || {
            let used = track.bars.iter().flat_map(|b| (0..crate::model::VOICES).flat_map(move |v| b.voice(v).iter())).flat_map(|b| &b.notes);
            used.map(|n| track.pitch(n)).all(|p| self.kit.contains(&Some(p)))
        };
        let usable = self.fits(track) && (track.instrument != Instrument::Drums || complete());
        self.instrument_set.as_deref().filter(|_| usable)
    }

    /// The sounds of the kept drum kit, in order; None when ours is written.
    pub fn kit(&self, track: &Track) -> Option<&[Option<u8>]> {
        (track.instrument == Instrument::Drums && self.instrument_set(track).is_some()).then_some(&self.kit[..])
    }

    /// The open strings of staff `staff`, lowest first.
    pub fn tuning(&self, track: &Track, staff: usize) -> Option<&[u8]> {
        self.tunings.get(staff).map(Vec::as_slice).filter(|t| self.fits(track) && t.iter().any(|&s| s > 0))
    }

    pub fn transpose(&self, track: &Track) -> Option<&str> {
        self.transpose.as_deref().filter(|_| self.fits(track))
    }

    /// Semitones from a sounding note to the written one, from the kept transposition.
    pub fn written_shift(&self, track: &Track) -> Option<i32> {
        let element = self.transpose(track)?;
        let doc = Document::parse(element).ok()?;
        let number = |tag: &str| child(doc.root_element(), tag).and_then(|n| text(n).parse::<i32>().ok());
        Some(-(number("Chromatic")? + 12 * number("Octave")?))
    }

    pub fn forced_sound(&self, track: &Track) -> Option<&str> {
        self.forced_sound.as_deref().filter(|_| self.fits(track))
    }

    /// The sounds with their effects. They and the changes of sound belong together.
    pub fn sounds(&self, track: &Track) -> Option<&str> {
        self.sounds.as_deref().filter(|_| self.fits(track))
    }

    /// The changes of sound, where their bars now are. One to a sound the track doesn't have is
    /// left out.
    pub fn automations(&self, track: &Track) -> Option<String> {
        let sounds = self.sounds(track)?;
        let (element, start) = self.automations.as_ref()?;
        let has = |c: &&Change| c.sound.as_ref().is_none_or(|path| sounds.contains(&format!("<Path>{path}</Path>")));
        let later = track.bars.iter().enumerate().flat_map(|(i, bar)| {
            bar.kept.iter().flat_map(move |k| k.sounds.iter().map(move |c| (i, c)))
        });
        Some(element.fill(together(start, later).into_iter().filter(|(_, c)| has(c))))
    }

    pub fn midi(&self, track: &Track) -> Option<&str> {
        self.midi.as_deref().filter(|_| self.fits(track))
    }

    /// Which of Guitar Pro's sound engines plays the track.
    pub fn engine(&self, track: &Track) -> Option<&str> {
        self.engine.as_deref().filter(|_| self.fits(track))
    }

    pub fn one_channel(&self, track: &Track) -> Option<&str> {
        self.one_channel.as_deref().filter(|_| self.fits(track))
    }

    /// How many bars go on each line, while the track has as many bars as it did.
    pub fn bars_per_line(&self, track: &Track) -> Option<&str> {
        self.bars_per_line.as_deref().filter(|_| track.bars.len() == self.opened_bars)
    }

    /// The mixer as read, except for the controls moved since, with its changes over time where
    /// their bars now are. A moved control loses its changes over time, which would undo it.
    pub fn mixer(&self, track: &Track) -> Option<String> {
        let m = self.mixer.as_ref()?;
        let now = crate::gpifwrite::strip_values(track);
        let moved = |i: usize| match (now.get(i), m.opened.get(i)) {
            (Some(&a), Some(&b)) => modelled(i) && (a - b).abs() > 1e-6,
            _ => false,
        };
        let numbers: Vec<String> = m
            .numbers
            .split_whitespace()
            .enumerate()
            .map(|(i, was)| if moved(i) { format!("{:.6}", now[i]) } else { was.to_string() })
            .collect();
        let changes = track.bars.iter().enumerate().flat_map(|(i, bar)| {
            bar.kept.iter().flat_map(move |k| k.mixer.iter().map(move |c| (i, c)))
        });
        let element = m.element.fill(together(&m.start, changes).into_iter().filter(|(_, c)| !c.param.is_some_and(moved)));
        Some(element.replacen(&m.numbers, &numbers.join(" "), 1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Bar, Instrument, MasterBar, Track};
    use std::collections::BTreeSet;

    /// What saving an opened file unchanged still loses or rewrites, each with the step of the
    /// design that is to deal with it. The test fails on anything not listed here, and on a line
    /// here that no longer happens: take it out.
    const ACCEPTED: &[&str] = &[
        // Stays: we say which version wrote the file.
        "changed GPRevision",
        "changed GPVersion",
        "changed Encoding/EncodingDescription",
        // Stays: the same value written another way, and a value Guitar Pro adds itself.
        "changed MasterBars/MasterBar/Fermatas/Fermata/Offset",
        "lost Notes/Note/InstrumentArticulation",
        "lost Notes/Note/Properties/Property[Fret]",
        "lost Notes/Note/Properties/Property[Fret]/Fret",
        "lost Notes/Note/Properties/Property[String]",
        "lost Notes/Note/Properties/Property[String]/String",
        // Stays: Guitar Pro adds these to a drum track itself.
        "lost Tracks/Track/Staves/Staff/Properties/Name",
        "lost Tracks/Track/Staves/Staff/Properties/Property[CapoFret]",
        "lost Tracks/Track/Staves/Staff/Properties/Property[FretCount]",
        "lost Tracks/Track/Staves/Staff/Properties/Property[PartialCapoFret]",
        "lost Tracks/Track/Staves/Staff/Properties/Property[PartialCapoStringFlags]",
        "lost Tracks/Track/Staves/Staff/Properties/Property[Tuning]",
        // Step 5, per-beat settings: not planned.
        "lost Beats/Beat/ConcertPitchStemOrientation",
        "lost Beats/Beat/TransposedPitchStemOrientation",
        "lost Beats/Beat/Properties",
        "lost Beats/Beat/Properties/Property[PrimaryPickupTone]",
        "lost Beats/Beat/Properties/Property[PrimaryPickupTone]/Float",
        "lost Beats/Beat/Properties/Property[PrimaryPickupVolume]",
        "lost Beats/Beat/Properties/Property[PrimaryPickupVolume]/Float",
    ];

    /// Parts of the score whose entries are shared and renumbered, so only their kinds are compared.
    const SHARED: [&str; 5] = ["Bars", "Voices", "Beats", "Notes", "Rhythms"];

    fn name(n: Node) -> String {
        let tag = n.tag_name().name();
        match tag {
            "Property" => format!("Property[{}]", n.attribute("name").unwrap_or("")),
            "XProperty" => format!("XProperty[{}]", n.attribute("id").unwrap_or("")),
            "Automation" => format!("Automation[{}]", child(n, "Type").and_then(|t| t.text()).unwrap_or("").trim()),
            _ => tag.to_string(),
        }
    }

    /// Walks two trees side by side, noting what the second lacks or says differently.
    fn compare(a: Node, b: Node, path: &str, out: &mut BTreeSet<String>) {
        // Lists of references to the shared entries are renumbered too.
        let reference = path == "MasterBars/MasterBar/Bars";
        if !reference && (text(a) != text(b) || a.attributes().any(|x| b.attribute(x.name()) != Some(x.value()))) {
            out.insert(format!("changed {path}"));
        }
        let mut seen = std::collections::HashMap::new();
        for ca in a.children().filter(|c| c.is_element()) {
            let key = name(ca);
            let i = seen.entry(key.clone()).or_insert(0usize);
            let at = format!("{path}/{key}");
            match b.children().filter(|c| c.is_element() && name(*c) == key).nth(*i) {
                Some(cb) => compare(ca, cb, &at, out),
                None => {
                    out.insert(format!("lost {at}"));
                }
            }
            *i += 1;
        }
    }

    fn kinds(n: Node, path: &str, out: &mut BTreeSet<String>) {
        for c in n.children().filter(|c| c.is_element()) {
            let at = format!("{path}/{}", name(c));
            out.insert(at.clone());
            kinds(c, &at, out);
        }
    }

    fn entry(data: &[u8], name: &str) -> Option<Vec<u8>> {
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(data)).ok()?;
        let mut out = vec![];
        zip.by_name(name).ok()?.read_to_end(&mut out).ok()?;
        Some(out)
    }

    /// Everything in the file `a` that the file `b` has lost or changed.
    fn differences(a: &[u8], b: &[u8]) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        let zip = zip::ZipArchive::new(std::io::Cursor::new(a)).unwrap();
        for file in zip.file_names().filter(|f| !f.ends_with('/') && *f != "Content/score.gpif") {
            match entry(b, file) {
                None => {
                    // The score views and their stylesheets are numbered: one line for them all.
                    let numbered = ["ScoreViews/", "scoreview"].iter().any(|n| file.contains(n));
                    let name: String = if numbered {
                        let mut parts = file.split(|c: char| c.is_ascii_digit());
                        format!("{}N{}", parts.next().unwrap_or(""), parts.next_back().unwrap_or(""))
                    } else {
                        file.to_string()
                    };
                    out.insert(format!("lost file {name}"));
                }
                Some(now) if file != "Content/BinaryStylesheet" && Some(&now) != entry(a, file).as_ref() => {
                    out.insert(format!("changed file {file}"));
                }
                _ => {}
            }
        }
        let style = |d: &[u8]| entry(d, "Content/BinaryStylesheet").and_then(|s| gpstyle::raw(&s)).unwrap_or_default();
        let now = style(b);
        for old in style(a) {
            match now.iter().find(|r| r.key == old.key) {
                None => {
                    out.insert("lost stylesheet settings".into());
                }
                Some(r) if *r != old => {
                    out.insert(format!("changed stylesheet {}", old.key));
                }
                _ => {}
            }
        }
        let xml = |d: &[u8]| String::from_utf8(entry(d, "Content/score.gpif").unwrap()).unwrap();
        let (a, b) = (xml(a), xml(b));
        let (a, b) = (Document::parse(&a).unwrap(), Document::parse(&b).unwrap());
        for part in a.root_element().children().filter(|c| c.is_element()) {
            let key = name(part);
            let Some(other) = child(b.root_element(), &key) else {
                out.insert(format!("lost {key}"));
                continue;
            };
            if SHARED.contains(&key.as_str()) {
                let (mut had, mut has) = (BTreeSet::new(), BTreeSet::new());
                kinds(part, &key, &mut had);
                kinds(other, &key, &mut has);
                out.extend(had.difference(&has).map(|p| format!("lost {p}")));
            } else {
                compare(part, other, &key, &mut out);
            }
        }
        out
    }

    /// Every .gp file kept for the tests.
    fn files() -> Vec<std::path::PathBuf> {
        let tests = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
        let mut all: Vec<_> = [tests.join("fixtures/gp8"), tests]
            .iter()
            .flat_map(|d| std::fs::read_dir(d).expect("the test files"))
            .map(|f| f.expect("a file").path())
            .filter(|p| p.extension().is_some_and(|e| e == "gp"))
            .collect();
        all.sort();
        all
    }

    fn resave(data: &[u8]) -> Vec<u8> {
        crate::gpifwrite::export_gp(&crate::gpif::import_gp(data).expect("opens")).expect("saves")
    }

    #[test]
    fn saving_an_opened_file_loses_only_what_is_listed() {
        let mut found = BTreeSet::new();
        for path in files() {
            let data = std::fs::read(&path).unwrap();
            found.extend(differences(&data, &resave(&data)));
        }
        let accepted: BTreeSet<String> = ACCEPTED.iter().map(|s| s.to_string()).collect();
        let new: Vec<_> = found.difference(&accepted).collect();
        let gone: Vec<_> = accepted.difference(&found).collect();
        assert!(new.is_empty() && gone.is_empty(), "not listed: {new:#?}\nno longer happens: {gone:#?}");
    }

    const BASELINE: &str = "gp8/Morning Walk Gp8.gp";

    fn opened() -> (Vec<u8>, Song) {
        let data = crate::fixture(BASELINE);
        let song = crate::gpif::import_gp(&data).unwrap();
        (data, song)
    }

    #[test]
    fn file_settings_survive_a_second_save() {
        let (data, _) = opened();
        let twice = resave(&resave(&data));
        assert_eq!(entry(&twice, "Content/BinaryStylesheet"), entry(&data, "Content/BinaryStylesheet"));
        for name in FILES {
            assert_eq!(entry(&twice, name), entry(&data, name), "{name}");
        }
    }

    #[test]
    fn a_changed_style_setting_is_written_and_the_others_stay() {
        let (data, mut song) = opened();
        song.style.page_w = 148.0;
        let saved = crate::gpifwrite::export_gp(&song).unwrap();
        let raw = |d: &[u8]| gpstyle::raw(&entry(d, "Content/BinaryStylesheet").unwrap()).unwrap();
        let (old, new) = (raw(&data), raw(&saved));
        assert_eq!(old.len(), new.len(), "no setting is dropped or added");
        let changed: Vec<_> = old.iter().zip(&new).filter(|(a, b)| a != b).map(|(a, _)| a.key.as_str()).collect();
        assert_eq!(changed, vec!["PageSetup/pagePortraitWidth"]);
        let read = gpstyle::read(&entry(&saved, "Content/BinaryStylesheet").unwrap()).unwrap();
        assert_eq!(read["PageSetup/pagePortraitWidth"], V::F(148.0));
    }

    #[test]
    fn a_file_without_a_title_is_not_given_one() {
        let data = crate::fixture("gp8/Morning Walk - track eq default.gp");
        let mut song = crate::gpif::import_gp(&data).unwrap();
        assert_eq!(song.title, "Untitled", "a name to show");
        assert!(crate::gpifwrite::gpif_xml(&song).contains("<Title><![CDATA[]]></Title>"));
        song.title = "Mine".into();
        assert!(crate::gpifwrite::gpif_xml(&song).contains("<Title><![CDATA[Mine]]></Title>"));
    }

    /// The first track's part of the saved score.
    fn first_track(song: &Song) -> String {
        let xml = crate::gpifwrite::gpif_xml(song);
        let a = xml.find("<Track id=\"0\">").expect("a track");
        xml[a..a + xml[a..].find("</Track>").expect("closed")].to_string()
    }

    /// The drum file Guitar Pro 8 saved with the track on its own sound engine.
    fn drum_check() -> Song {
        crate::gpif::import_gp(&crate::fixture("gp8/Drum check.gp")).unwrap()
    }

    #[test]
    fn a_track_keeps_its_sound_and_engine_until_the_instrument_changes() {
        let (_, mut song) = opened();
        let kept = first_track(&song);
        assert!(kept.contains("<SoundbankPatch>D-Steel</SoundbankPatch>"), "{kept}");
        // A new name, tuning or volume leaves them.
        song.tracks[0].name = "Other".into();
        song.tracks[0].volume = 0.25;
        assert!(first_track(&song).contains("D-Steel"));
        song.tracks[0].set_instrument(Instrument::Banjo);
        let changed = first_track(&song);
        assert!(!changed.contains("D-Steel") && changed.contains("<Type>banjo</Type>"), "{changed}");
        assert!(changed.contains("<AudioEngineState>MIDI</AudioEngineState>"));
        // Changing back brings them back.
        song.tracks[0].set_instrument(Instrument::SteelGuitar);
        assert!(first_track(&song).contains("D-Steel"));
        // The engine chosen in Guitar Pro stays.
        assert!(first_track(&drum_check()).contains("<AudioEngineState>RSE</AudioEngineState>"));
    }

    #[test]
    fn moving_one_mixer_control_changes_only_its_number() {
        let (_, mut song) = opened();
        let numbers = |song: &Song| {
            let t = first_track(song);
            let a = t.find("<Parameters>").unwrap() + "<Parameters>".len();
            t[a..a + t[a..].find("</Parameters>").unwrap()].split_whitespace().map(String::from).collect::<Vec<_>>()
        };
        let before = numbers(&song);
        song.tracks[0].volume = 0.25;
        let after = numbers(&song);
        let changed: Vec<usize> = (0..before.len()).filter(|&i| before[i] != after[i]).collect();
        assert_eq!((changed, after[12].as_str()), (vec![12], "0.250000"));
    }

    #[test]
    fn a_duplicated_track_keeps_its_settings_and_a_new_one_has_ours() {
        let (_, mut song) = opened();
        let copy = song.tracks[0].clone();
        song.tracks.insert(1, copy);
        song.tracks.push(Track::of_instrument(Instrument::Bass, "New"));
        song.normalize();
        let xml = crate::gpifwrite::gpif_xml(&song);
        assert_eq!(xml.matches("<SoundbankPatch>D-Steel</SoundbankPatch>").count(), 2);
        assert!(xml.contains("<Path>Midi/Electric Bass</Path>"));
    }

    #[test]
    fn a_drum_note_names_its_place_in_the_kit_that_is_written() {
        // A kit of two sounds, the other way round from ours.
        let set = "<InstrumentSet><Type>drumKit</Type><Elements><Element><Articulations><Articulation><InputMidiNumbers>38</InputMidiNumbers></Articulation><Articulation><InputMidiNumbers>36</InputMidiNumbers></Articulation></Articulations></Element></Elements></InstrumentSet>";
        let xml = format!("<Track>{set}</Track>");
        let doc = Document::parse(&xml).unwrap();
        let mut song = Song::new_blank();
        let mut drums = Track::of_instrument(Instrument::Drums, "Drums");
        let hit = |p: u8| crate::model::Beat { notes: vec![crate::model::Note::new(0, p)], ..crate::model::Beat::rest(crate::model::Dur::Whole) };
        drums.bars = vec![Bar::new(vec![hit(36)])];
        drums.kept = Some(Arc::new(read_track(doc.root_element(), &xml, &drums, 1).0));
        song.master = vec![MasterBar::default()];
        song.tracks = vec![drums];
        let saved = crate::gpifwrite::gpif_xml(&song);
        assert!(saved.contains(set) && saved.contains("<InstrumentArticulation>1</InstrumentArticulation>"), "{saved}");
        // A sound the kept kit lacks: the whole of our kit is written instead.
        song.tracks[0].bars[0].beats[0] = hit(42);
        let saved = crate::gpifwrite::gpif_xml(&song);
        assert!(!saved.contains(set) && saved.contains("<InstrumentArticulation>3</InstrumentArticulation>"));
    }

    #[test]
    fn nothing_of_the_instrument_is_kept_from_our_own_older_files() {
        let xml = "<Track><InstrumentSet><Type>leadVoice</Type></InstrumentSet><Sounds><Sound><Path>Midi/Lead Voice</Path></Sound></Sounds><AudioEngineState>MIDI</AudioEngineState></Track>";
        let doc = Document::parse(xml).unwrap();
        let track = Track::of_instrument(Instrument::LeadVocals, "Voice");
        let kept = read_track(doc.root_element(), xml, &track, 1).0;
        assert_eq!((kept.instrument_set(&track), kept.sounds(&track), kept.engine(&track)), (None, None, None));
    }

    #[test]
    fn a_trumpet_keeps_its_transposition_and_is_written_to_match() {
        let song = crate::gpif::import_gp(&crate::fixture("gp8/Instruments.gp")).unwrap();
        let k = song.tracks.iter().position(|t| t.instrument == Instrument::Trumpet).unwrap();
        let kept = song.tracks[k].kept.as_ref().unwrap();
        assert_eq!(kept.written_shift(&song.tracks[k]), Some(2));
        let xml = crate::gpifwrite::gpif_xml(&song);
        assert!(xml.contains("<Chromatic>-2</Chromatic>"));
        // The file's note: it sounds G and is written A.
        assert!(xml.contains("<Property name=\"ConcertPitch\"><Pitch><Step>G</Step><Accidental></Accidental><Octave>6</Octave></Pitch></Property><Property name=\"TransposedPitch\"><Pitch><Step>A</Step>"));
    }

    /// The names of a saved file's entries, and its score.
    fn saved(song: &Song) -> (Vec<String>, String) {
        let data = crate::gpifwrite::export_gp(song).unwrap();
        let zip = zip::ZipArchive::new(std::io::Cursor::new(&data[..])).unwrap();
        let mut names: Vec<String> = zip.file_names().map(String::from).collect();
        names.sort();
        (names, String::from_utf8(entry(&data, "Content/score.gpif").unwrap()).unwrap())
    }

    #[test]
    fn bars_per_line_stay_while_the_number_of_bars_does() {
        let (_, mut song) = opened();
        let lines = "<ScoreSystemsLayout>4 4 4</ScoreSystemsLayout>";
        song.tracks[0].bars[0].beats[0].dotted = true;
        assert!(saved(&song).1.contains(lines), "an edit within a bar leaves them");
        song.master.push(MasterBar::default());
        song.normalize();
        let xml = saved(&song).1;
        assert!(!xml.contains(lines) && xml.contains("<ScoreSystemsLayout>4</ScoreSystemsLayout>"));
    }

    #[test]
    fn the_views_stay_while_the_tracks_and_bars_do() {
        let (_, song) = opened();
        let (names, xml) = saved(&song);
        assert!(names.contains(&SCORE_STYLE.to_string()) && names.contains(&"Content/ScoreViews/2.gpsv".to_string()), "{names:?}");
        assert!(xml.contains("<ScoreViews>"));
        // Showing other staves, hiding, renaming or reordering tracks: our own views.
        let edits: [fn(&mut Song); 4] = [
            |s| s.tracks[0].visible = false,
            |s| s.tracks[1].show_tab = !s.tracks[1].show_tab,
            |s| s.tracks[0].name = "Other".into(),
            |s| s.tracks.swap(0, 1),
        ];
        for edit in edits {
            let mut changed = song.clone();
            edit(&mut changed);
            let names = saved(&changed).0;
            assert!(!names.iter().any(|n| n.starts_with("Content/Stylesheets/")), "{names:?}");
            assert!(names.contains(&"Content/PartConfiguration".to_string()));
        }
        // Another page size: the views stay, the newer stylesheet that repeats the old size goes.
        let mut restyled = song.clone();
        restyled.style.page_w = 148.0;
        let names = saved(&restyled).0;
        assert!(!names.contains(&SCORE_STYLE.to_string()) && names.contains(&"Content/ScoreViews/2.gpsv".to_string()));
    }

    #[test]
    fn the_zoom_of_the_file_stays() {
        let mut song = crate::gpif::import_gp(&crate::fixture("gp8/Instruments.gp")).unwrap();
        song.master.push(MasterBar::default());
        song.normalize();
        assert!(saved(&song).1.contains("<ScoreZoom>1.5</ScoreZoom>"));
    }

    #[test]
    fn page_texts_go_when_the_header_changes() {
        let (_, mut song) = opened();
        let mut kept = KeptSong { page_texts: vec![("PageHeader", "<PageHeader>kept</PageHeader>".into())], ..KeptSong::default() };
        kept.opened_headers = headers(&song.style).map(Vec::clone);
        assert_eq!(kept.page_text("PageHeader", &song), Some("<PageHeader>kept</PageHeader>"));
        song.style.first_header[0].on = !song.style.first_header[0].on;
        assert_eq!(kept.page_text("PageHeader", &song), None);
    }

    /// The baseline file with changes over time added: a master effect's at the start and at
    /// bars 3 and 6, the first track's volume and pan at the start and at bar 3, and its sound
    /// again at bar 6. Bar 2 is spelled with flats and has a setting of its own.
    fn with_changes() -> Song {
        let data = crate::fixture(BASELINE);
        let mut xml = String::from_utf8(entry(&data, "Content/score.gpif").unwrap()).unwrap();
        let change = |kind: &str, bar: usize, value: &str| format!("<Automation><Type>{kind}</Type><Linear>false</Linear><Bar>{bar}</Bar><Position>0</Position><Visible>true</Visible><Value>{value}</Value></Automation>");
        let insert = |xml: &mut String, after: &str, before: &str, what: &str| {
            let from = xml.find(after).unwrap_or_else(|| panic!("{after}"));
            let at = from + xml[from..].find(before).unwrap_or_else(|| panic!("{before}"));
            xml.insert_str(at, what);
        };
        let master = format!("<Automations>{}{}{}</Automations>", change("DSPParam_00", 0, "0.8"), change("DSPParam_00", 2, "0.1"), change("DSPParam_00", 5, "0.2"));
        insert(&mut xml, "<MasterTrack>", "</Effect>", &master);
        let mixer = format!(
            "<Automations>{}{}{}{}</Automations>",
            change("DSPParam_12", 0, "0.6"),
            change("DSPParam_11", 0, "0.5"),
            change("DSPParam_12", 2, "0.3"),
            change("DSPParam_11", 2, "0.4")
        );
        insert(&mut xml, "<ChannelStrip", "</ChannelStrip>", &mixer);
        let path = "Stringed/Acoustic Guitars/Steel Guitar";
        let from = xml.find("<ChannelStrip").unwrap();
        let sounds = from + xml[from..].find("<Type>Sound</Type>").unwrap();
        let end = sounds + xml[sounds..].find("</Automations>").unwrap();
        xml.insert_str(end, &(change("Sound", 5, &format!("{path};;Factory")) + &change("Sound", 6, "No/Such/Sound;;Factory")));
        // The third master bar (index 1) in flats, and its first track's bar with a setting.
        let second = xml.match_indices("<MasterBar>").nth(1).unwrap().0;
        let at = second + xml[second..].find("<TransposeAs>").unwrap();
        let end = at + xml[at..].find("</TransposeAs>").unwrap();
        xml.replace_range(at..end, "<TransposeAs>Flats");
        let bar = xml.find("<Bars>").unwrap();
        let opening = xml[bar..bar + 6].to_string();
        insert(&mut xml, &opening, "</Bar>", "<XProperties><XProperty id=\"7\"><Int>1</Int></XProperty></XProperties>");
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(vec![]));
        let options = zip::write::SimpleFileOptions::default();
        let mut old = zip::ZipArchive::new(std::io::Cursor::new(&data[..])).unwrap();
        for i in 0..old.len() {
            let name = old.by_index(i).unwrap().name().to_string();
            if name.ends_with('/') {
                continue;
            }
            zip.start_file(name.as_str(), options).unwrap();
            let bytes = if name == "Content/score.gpif" { xml.clone().into_bytes() } else { entry(&data, &name).unwrap() };
            std::io::Write::write_all(&mut zip, &bytes).unwrap();
        }
        crate::gpif::import_gp(&zip.finish().unwrap().into_inner()).unwrap()
    }

    /// The bar numbers of the saved changes of one kind, in the order written.
    fn bars_of(xml: &str, kind: &str) -> Vec<usize> {
        let doc = Document::parse(xml).unwrap();
        doc.descendants()
            .filter(|n| n.has_tag_name("Automation") && child(*n, "Type").is_some_and(|t| text(t) == kind))
            .map(|n| text(child(n, "Bar").unwrap()).parse().unwrap())
            .collect()
    }

    fn insert_bar(song: &mut Song, at: usize) {
        song.master.insert(at, MasterBar::default());
        for t in &mut song.tracks {
            t.bars.insert(at, Bar::empty());
        }
    }

    fn delete_bar(song: &mut Song, at: usize) {
        song.master.remove(at);
        for t in &mut song.tracks {
            t.bars.remove(at);
        }
    }

    #[test]
    fn changes_over_time_move_with_their_bars() {
        let mut song = with_changes();
        let check = |song: &Song, effects: &[usize], volume: &[usize], sounds: &[usize]| {
            let xml = crate::gpifwrite::gpif_xml(song);
            assert_eq!(bars_of(&xml, "DSPParam_00"), effects, "master effect");
            assert_eq!(bars_of(&xml, "DSPParam_12"), volume, "volume");
            assert_eq!(bars_of(&xml, "DSPParam_11"), volume, "pan");
            // Two tracks' sounds at the start, and the first track's change; not the one to a
            // sound the track doesn't have.
            assert_eq!(bars_of(&xml, "Sound"), sounds, "sound");
        };
        check(&song, &[0, 2, 5], &[0, 2], &[0, 5, 0]);
        // A new first bar: what sets how the song begins stays at the start.
        insert_bar(&mut song, 0);
        check(&song, &[0, 3, 6], &[0, 3], &[0, 6, 0]);
        delete_bar(&mut song, 3);
        check(&song, &[0, 5], &[0], &[0, 5, 0]);
        // The first bar goes: the points at the start stay.
        delete_bar(&mut song, 0);
        check(&song, &[0, 4], &[0], &[0, 4, 0]);
    }

    #[test]
    fn a_change_that_comes_to_the_start_replaces_the_one_there() {
        let mut song = with_changes();
        delete_bar(&mut song, 1);
        delete_bar(&mut song, 0);
        let xml = crate::gpifwrite::gpif_xml(&song);
        // The points of what was bar 3 are now the song's first, one for each control.
        assert_eq!((bars_of(&xml, "DSPParam_12"), bars_of(&xml, "DSPParam_11")), (vec![0], vec![0]));
        assert!(xml.contains("<Value>0.3</Value>") && !xml.contains("<Value>0.6</Value>"));
        assert_eq!(bars_of(&xml, "DSPParam_00"), vec![0, 3]);
    }

    #[test]
    fn a_moved_control_loses_its_changes_over_time() {
        let mut song = with_changes();
        song.tracks[0].volume = 0.25;
        let xml = crate::gpifwrite::gpif_xml(&song);
        assert_eq!(bars_of(&xml, "DSPParam_12"), Vec::<usize>::new(), "volume moved");
        assert_eq!(bars_of(&xml, "DSPParam_11"), vec![0, 2], "pan not");
    }

    #[test]
    fn the_flats_choice_stays_with_its_key_and_a_bars_settings_with_the_bar() {
        let mut song = with_changes();
        let saved = |song: &Song| crate::gpifwrite::gpif_xml(song);
        let flats = |xml: &str| xml.matches("<TransposeAs>Flats</TransposeAs>").count();
        assert_eq!(flats(&saved(&song)), 1);
        assert_eq!(saved(&song).matches("<XProperty id=\"7\">").count(), 1);
        insert_bar(&mut song, 0);
        assert_eq!((flats(&saved(&song)), saved(&song).matches("<XProperty id=\"7\">").count()), (1, 1));
        // Read back, the choice is on the bar that moved.
        let back = crate::gpif::parse_gpif(&saved(&song)).unwrap();
        assert!(back.master[2].kept.as_ref().unwrap().transpose_as(&back.master[2]).unwrap().contains("Flats"));
        song.master[2].key = 3;
        assert_eq!(flats(&saved(&song)), 0, "another key has its own spelling");
    }

    #[test]
    fn a_pickup_bar_is_written_while_it_is_still_first() {
        let mut song = Song::new_blank();
        song.master[0].pickup = true;
        let xml = crate::gpifwrite::gpif_xml(&song);
        assert!(xml.contains("<MasterTrack><Anacrusis />"));
        song.master.insert(0, MasterBar::default());
        song.tracks[0].bars.insert(0, Bar::empty());
        assert!(!crate::gpifwrite::gpif_xml(&song).contains("Anacrusis"));
    }
}
