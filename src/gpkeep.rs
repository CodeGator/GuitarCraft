//! What a .gp file holds that this app doesn't use, kept so it can be written back when the song
//! is saved. The design and its rules are in `docs/gp-keep-design.md`.
//!
//! Built so far: the settings of the file as a whole. A kept setting is written back until the
//! thing it depends on changes, so what the song held when it was read is recorded beside it.

use crate::gpstyle::{self, Raw, V};
use crate::model::Song;
use crate::style::{HeaderItem, Stylesheet};
use roxmltree::{Document, Node};
use std::io::Read;

/// Archive entries written back as they were.
const FILES: [&str; 2] = ["Content/Preferences.json", "meta.json"];

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
    /// <MasterTrack>'s effects, with their changes over time and without them.
    master_effects: Option<(String, String)>,
    opened_bars: usize,
    /// The file had no title: the song was given a name to show, which isn't written back.
    untitled: Option<String>,
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

/// Reads what is to be kept from a .gp archive. `song` is the song as read from it. Also marks
/// a pickup bar on the song itself, so that it stays with its bar.
pub fn read<R: Read + std::io::Seek>(zip: &mut zip::ZipArchive<R>, xml: &str, song: &mut Song) -> KeptSong {
    let mut kept = KeptSong {
        opened_style: gpstyle::entries(&song.style),
        opened_headers: headers(&song.style).map(Vec::clone),
        opened_bars: song.bar_count(),
        ..KeptSong::default()
    };
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
    }
    if let Some(master) = child(root, "MasterTrack") {
        if child(master, "Anacrusis").is_some()
            && let Some(first) = song.master.first_mut()
        {
            first.pickup = true;
        }
        kept.master_effects = child(master, "RSE").map(|rse| {
            let all = source(rse);
            // The same without the changes over time, which name bars by number.
            let mut without = all.clone();
            let start = rse.range().start;
            let mut cuts: Vec<_> = rse.descendants().filter(|n| n.has_tag_name("Automations")).map(|n| n.range()).collect();
            cuts.sort_by_key(|r| std::cmp::Reverse(r.start));
            for r in cuts {
                without.replace_range(r.start - start..r.end - start, "");
            }
            (all, without)
        });
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

    pub fn multi_voice(&self) -> Option<&str> {
        self.multi_voice.as_deref()
    }

    /// The master effects. Their changes over time go once bars have been added or removed.
    pub fn master_effects(&self, song: &Song) -> Option<&str> {
        let (all, without) = self.master_effects.as_ref()?;
        Some(if song.bar_count() == self.opened_bars { all } else { without })
    }

    /// The stylesheet file: as read, except for the settings changed since. None when the file
    /// that was opened had no stylesheet that could be read.
    pub fn stylesheet(&self, song: &Song) -> Option<Vec<u8>> {
        (!self.stylesheet.is_empty()).then(|| gpstyle::write_over(&self.stylesheet, &self.opened_style, &song.style))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Bar, MasterBar};
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
        // Step 2, track settings.
        "changed Tracks/Track/AudioEngineState",
        "changed Tracks/Track/Automations/Automation[Sound]/Value",
        "changed Tracks/Track/RSE/ChannelStrip/Parameters",
        "changed Tracks/Track/Sounds/Sound/Label",
        "changed Tracks/Track/Sounds/Sound/Name",
        "changed Tracks/Track/Sounds/Sound/Path",
        "changed Tracks/Track/Sounds/Sound/Role",
        "lost Tracks/Track/Sounds/Sound/RSE",
        "lost Tracks/Track/UseOneChannelPerString",
        "changed Tracks/Track/MidiConnection/PrimaryChannel",
        "changed Tracks/Track/MidiConnection/SecondaryChannel",
        "changed Tracks/Track/Sounds/Sound/MIDI/Program",
        "changed Tracks/Track/Transpose/Chromatic",
        "lost Tracks/Track/Sounds/Sound/RSE/EffectChain",
        "lost Tracks/Track/Sounds/Sound/RSE/ElementsSettings",
        // Step 2 too: what Guitar Pro 8 adds to a drum track, and its names for the bongos.
        "changed Tracks/Track/InstrumentSet/Elements/Element/Articulations/Articulation/Name",
        "lost Tracks/Track/Staves/Staff/Properties/Name",
        "lost Tracks/Track/Staves/Staff/Properties/Property[CapoFret]",
        "lost Tracks/Track/Staves/Staff/Properties/Property[FretCount]",
        "lost Tracks/Track/Staves/Staff/Properties/Property[PartialCapoFret]",
        "lost Tracks/Track/Staves/Staff/Properties/Property[PartialCapoStringFlags]",
        "lost Tracks/Track/Staves/Staff/Properties/Property[Tuning]",
        // Step 3, settings tied to bars.
        "changed MasterBars/MasterBar/Key/TransposeAs",
        // Step 4, layout and views.
        "changed Score/ScoreSystemsLayout",
        "changed Tracks/Track/SystemsLayout",
        "changed file Content/LayoutConfiguration",
        "changed file Content/PartConfiguration",
        "lost ScoreViews",
        "lost file Content/ScoreViews/N.gpsv",
        "lost file Content/Stylesheets/score.gpss",
        "lost file Content/Stylesheets/scoreviewN.gpss",
        "changed Score/ScoreZoom",
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

    #[test]
    fn page_texts_go_when_the_header_changes() {
        let (_, mut song) = opened();
        let mut kept = KeptSong { page_texts: vec![("PageHeader", "<PageHeader>kept</PageHeader>".into())], ..KeptSong::default() };
        kept.opened_headers = headers(&song.style).map(Vec::clone);
        assert_eq!(kept.page_text("PageHeader", &song), Some("<PageHeader>kept</PageHeader>"));
        song.style.first_header[0].on = !song.style.first_header[0].on;
        assert_eq!(kept.page_text("PageHeader", &song), None);
    }

    #[test]
    fn master_effect_changes_over_time_go_when_the_bars_change() {
        let xml = "<GPIF><Score/><MasterTrack><RSE><Master><Effect id=\"E\"><Automations><Automation/></Automations></Effect></Master></RSE></MasterTrack></GPIF>";
        let mut song = Song::new_blank();
        let mut buf = std::io::Cursor::new(vec![]);
        zip::ZipWriter::new(&mut buf).finish().unwrap();
        let kept = read(&mut zip::ZipArchive::new(buf).unwrap(), xml, &mut song);
        assert!(kept.master_effects(&song).unwrap().contains("<Automations>"));
        song.master.push(MasterBar::default());
        song.tracks[0].bars.push(Bar::empty());
        let after = kept.master_effects(&song).unwrap();
        assert!(after.contains("<Effect id=\"E\">") && !after.contains("Automation"), "{after}");
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
