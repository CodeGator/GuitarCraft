//! The dockable panel layout: which panels there are, where they start, and how the
//! arrangement is saved and brought back. What a panel shows lives with the code that owns
//! it (`palette.rs`, `side.rs`, `bottom.rs`, `actions.rs`); the dock only places them.
//!
//! The score is a pane too, but it stays where it is: the middle of the main window. The other
//! panes can be dragged to any edge of the window, stacked as tabs, floated inside the window
//! (egui_dock does that when a tab is dropped outside the dock), or popped out into a window
//! of their own (see `float.rs`). Panes popped out are kept in [`Layout::popped`], not in the
//! dock.
//!
//! The arrangement is saved in `layout.json` in GuitarCraft's settings folder.

use super::home::app_dir;
use egui_dock::{DockState, Node, NodeIndex, NodePath, Split, SurfaceIndex};
use serde::{Deserialize, Serialize};

/// Everything that can sit in a dock tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Pane {
    /// The open song: the one pane that is never closed, floated or moved out of the middle.
    Score,
    Palette,
    Inspector,
    /// The tracks and mixer.
    Tracks,
    Fretboard,
    Keyboard,
    /// A pane an older build had. Layouts that name one still load; the tab is dropped.
    #[serde(other)]
    Retired,
}

impl Pane {
    /// The panels the View menu and the toolbar switch on and off.
    pub const PANELS: [Pane; 5] = [Pane::Palette, Pane::Inspector, Pane::Tracks, Pane::Fretboard, Pane::Keyboard];

    pub fn title(self) -> &'static str {
        match self {
            Pane::Score => "Score",
            Pane::Palette => "Editing Palette",
            Pane::Inspector => "Inspector",
            Pane::Tracks => "Tracks",
            Pane::Fretboard => "Fretboard",
            Pane::Keyboard => "Keyboard",
            Pane::Retired => "",
        }
    }

    /// The size of the window when the pane is popped out (width, height).
    pub fn window_size(self) -> [f32; 2] {
        match self {
            Pane::Palette => [240.0, 640.0],
            Pane::Inspector => [280.0, 640.0],
            Pane::Tracks => [820.0, 260.0],
            Pane::Fretboard => [900.0, 190.0],
            Pane::Keyboard => [900.0, 150.0],
            Pane::Score | Pane::Retired => [800.0, 600.0],
        }
    }

    /// Only the score is fixed in the middle; every panel can be closed.
    pub fn closeable(self) -> bool {
        self != Pane::Score
    }
}

/// What is saved between runs: the dock, and the panels popped out into windows of their own.
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Layout {
    pub dock: DockState<Pane>,
    pub popped: Vec<Pane>,
}

impl Default for Layout {
    fn default() -> Self {
        Layout { dock: default_layout(), popped: Vec::new() }
    }
}

/// Share of the width the palette starts with.
const PALETTE_SHARE: f32 = 0.16;
/// Share of the width the inspector starts with.
const INSPECTOR_SHARE: f32 = 0.19;
/// Share of the height the tracks panel starts with.
const TRACKS_SHARE: f32 = 0.24;

/// The arrangement GuitarCraft starts with, and what View > Reset Panel Layout returns to: the
/// score in the middle, the palette on the left, the inspector on the right and the tracks along
/// the bottom. The fretboard and keyboard are off until switched on.
///
/// In a split, `fraction` is the share of the *first* child (the left or top one): the new panel
/// for `split_left`, the existing one for `split_right` and `split_below`.
pub fn default_layout() -> DockState<Pane> {
    let mut state = DockState::new(vec![Pane::Score]);
    let tree = state.main_surface_mut();
    let [upper, _tracks] = tree.split_below(NodeIndex::root(), 1.0 - TRACKS_SHARE, vec![Pane::Tracks]);
    let [middle, _palette] = tree.split_left(upper, PALETTE_SHARE, vec![Pane::Palette]);
    let [_score, _inspector] = tree.split_right(middle, 1.0 - INSPECTOR_SHARE / (1.0 - PALETTE_SHARE), vec![Pane::Inspector]);
    state
}

pub fn is_open(dock: &DockState<Pane>, pane: Pane) -> bool {
    dock.find_tab(&pane).is_some()
}

/// Removes `pane`'s tab, if it has one.
pub fn close(dock: &mut DockState<Pane>, pane: Pane) {
    if let Some(at) = dock.find_tab(&pane) {
        dock.remove_tab(at);
    }
}

/// Brings `pane`'s tab to the front of its tab group, if it is open.
pub fn bring_to_front(dock: &mut DockState<Pane>, pane: Pane) {
    if let Some(at) = dock.find_tab(&pane) {
        let _ = dock.set_active_tab(at);
    }
}

/// Adds `pane` where it belongs by default. A panel that is already open stays where the person
/// put it.
pub fn open(dock: &mut DockState<Pane>, pane: Pane) {
    if is_open(dock, pane) {
        return;
    }
    match pane {
        Pane::Score => dock.push_to_first_leaf(pane),
        Pane::Palette => split_score(dock, Split::Left, PALETTE_SHARE, pane),
        Pane::Inspector => split_score(dock, Split::Right, 1.0 - INSPECTOR_SHARE, pane),
        // Along the bottom, beside the tracks if they are there.
        Pane::Fretboard | Pane::Keyboard => {
            let beside = if pane == Pane::Fretboard { [Pane::Tracks, Pane::Keyboard] } else { [Pane::Tracks, Pane::Fretboard] };
            if !join(dock, &beside, pane) {
                dock.main_surface_mut().split_below(NodeIndex::root(), 1.0 - TRACKS_SHARE, vec![pane]);
            }
        }
        Pane::Tracks => {
            if !join(dock, &[Pane::Fretboard, Pane::Keyboard], pane) {
                dock.main_surface_mut().split_below(NodeIndex::root(), 1.0 - TRACKS_SHARE, vec![pane]);
            }
        }
        Pane::Retired => {}
    }
}

/// Puts `pane` beside the score's tab group, on the main surface. `fraction` is the share of the
/// first (left or top) child, as in `default_layout`.
fn split_score(dock: &mut DockState<Pane>, side: Split, fraction: f32, pane: Pane) {
    match dock.find_tab(&Pane::Score) {
        Some(at) => {
            dock.split(NodePath { surface: at.surface, node: at.node }, side, fraction, Node::leaf(pane));
        }
        None => dock.push_to_first_leaf(pane),
    }
}

/// Adds `pane` to the tab group of the first open member of `group`. False if none is open.
fn join(dock: &mut DockState<Pane>, group: &[Pane], pane: Pane) -> bool {
    let Some(at) = group.iter().find_map(|p| dock.find_tab(p)) else {
        return false;
    };
    let path = NodePath { surface: at.surface, node: at.node };
    dock.set_focused_node_and_surface(path);
    dock.push_to_focused_leaf(pane);
    true
}

fn layout_file() -> Option<std::path::PathBuf> {
    Some(app_dir()?.join("layout.json"))
}

/// The saved layout, or the default when there is none or it can't be read.
pub fn load() -> Layout {
    layout_file()
        .and_then(|f| std::fs::read(f).ok())
        .and_then(|d| serde_json::from_slice::<Layout>(&d).ok())
        .map(sanitize)
        .unwrap_or_default()
}

#[derive(Serialize)]
struct LayoutRef<'a> {
    dock: &'a DockState<Pane>,
    popped: &'a [Pane],
}

/// The layout as the text that is saved.
pub fn to_json(dock: &DockState<Pane>, popped: &[Pane]) -> String {
    serde_json::to_string_pretty(&LayoutRef { dock, popped }).unwrap_or_default()
}

pub fn save(json: &str) {
    if let Some(f) = layout_file() {
        if let Some(dir) = f.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(f, json);
    }
}

/// The score's tab bar shows only when something is stacked with the score; alone, the
/// document tabs above are all the person needs.
pub fn tidy_score_bar(dock: &mut DockState<Pane>) {
    for (_, leaf) in dock.iter_leaves_mut() {
        let only_score = leaf.tabs.len() == 1 && leaf.tabs[0] == Pane::Score;
        leaf.tab_bar_hidden = only_score;
    }
}

/// Makes a restored layout safe to use: tabs of retired panes are dropped (and any floating
/// window they leave empty, which the dock cannot draw); a pane appearing twice keeps its first
/// tab; a popped-out pane isn't also docked; and a layout with no score, or with the score in a
/// floating window, is replaced by the default.
pub fn sanitize(mut layout: Layout) -> Layout {
    layout.popped.retain(|p| p.closeable() && *p != Pane::Retired);
    layout.popped.dedup();
    let popped = layout.popped.clone();
    let mut seen: Vec<Pane> = Vec::new();
    layout.dock.retain_tabs(|p| {
        if *p == Pane::Retired || popped.contains(p) || seen.contains(p) {
            false
        } else {
            seen.push(*p);
            true
        }
    });
    let emptied: Vec<SurfaceIndex> = (1..layout.dock.surfaces_count())
        .map(SurfaceIndex)
        .filter(|&i| layout.dock.get_surface(i).and_then(|s| s.node_tree()).is_some_and(|tree| tree.num_tabs() == 0))
        .collect();
    for surface in emptied.into_iter().rev() {
        layout.dock.remove_surface(surface);
    }
    let score_on_main = layout.dock.find_main_surface_tab(&Pane::Score).is_some();
    if !score_on_main {
        return Layout::default();
    }
    tidy_score_bar(&mut layout.dock);
    layout
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Blank;

    impl egui_dock::TabViewer for Blank {
        type Tab = Pane;
        fn id(&mut self, tab: &mut Pane) -> egui::Id {
            egui::Id::new(("pane", *tab))
        }
        fn title(&mut self, tab: &mut Pane) -> egui::WidgetText {
            tab.title().into()
        }
        fn ui(&mut self, _ui: &mut egui::Ui, _tab: &mut Pane) {}
    }

    /// Draws the dock once, headless, as the app does: until then its rectangles are not
    /// numbers, which a saved file can't hold.
    fn drawn(mut l: Layout) -> Layout {
        let ctx = egui::Context::default();
        let input = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200.0, 800.0))), ..Default::default() };
        let _ = ctx.run_ui(input, |ui| {
            egui_dock::DockArea::new(&mut l.dock).show_inside(ui, &mut Blank);
        });
        l
    }

    fn panes(dock: &DockState<Pane>) -> Vec<Pane> {
        dock.iter_all_tabs().map(|(_, p)| *p).collect()
    }

    #[test]
    fn the_default_layout_has_every_starting_panel_once() {
        let d = default_layout();
        let mut p = panes(&d);
        p.sort_by_key(|p| p.title());
        assert_eq!(p.len(), 4);
        for want in [Pane::Score, Pane::Palette, Pane::Inspector, Pane::Tracks] {
            assert_eq!(p.iter().filter(|x| **x == want).count(), 1, "{want:?}");
        }
        assert!(!is_open(&d, Pane::Fretboard) && !is_open(&d, Pane::Keyboard));
    }

    #[test]
    fn opening_and_closing_panels_works_and_keeps_the_score() {
        let mut d = default_layout();
        open(&mut d, Pane::Fretboard);
        open(&mut d, Pane::Keyboard);
        assert!(is_open(&d, Pane::Fretboard) && is_open(&d, Pane::Keyboard));
        open(&mut d, Pane::Fretboard);
        assert_eq!(panes(&d).iter().filter(|p| **p == Pane::Fretboard).count(), 1);
        for p in Pane::PANELS {
            close(&mut d, p);
        }
        assert_eq!(panes(&d), vec![Pane::Score]);
        // Closed panels come back, even with only the score left.
        for p in Pane::PANELS {
            open(&mut d, p);
            assert!(is_open(&d, p), "{p:?}");
        }
    }

    #[test]
    fn panels_start_with_sensible_proportions() {
        let l = drawn(Layout::default());
        let tree = l.dock.main_surface();
        let width = |p: Pane| tree[l.dock.find_tab(&p).unwrap().node].rect().unwrap().width();
        let height = |p: Pane| tree[l.dock.find_tab(&p).unwrap().node].rect().unwrap().height();
        assert!(width(Pane::Palette) < width(Pane::Score) / 3.0, "the palette is a narrow side panel");
        assert!(width(Pane::Inspector) < width(Pane::Score) / 3.0, "so is the inspector");
        assert!(height(Pane::Tracks) < height(Pane::Score) / 2.0, "the tracks are a strip along the bottom");
    }

    #[test]
    fn the_layout_survives_saving_and_loading() {
        let mut l = Layout::default();
        open(&mut l.dock, Pane::Keyboard);
        close(&mut l.dock, Pane::Palette);
        l.popped.push(Pane::Palette);
        let l = drawn(l);
        let json = to_json(&l.dock, &l.popped);
        let back = sanitize(serde_json::from_str::<Layout>(&json).unwrap());
        let mut a = panes(&l.dock);
        let mut b = panes(&back.dock);
        a.sort_by_key(|p| p.title());
        b.sort_by_key(|p| p.title());
        assert_eq!(a, b);
        assert_eq!(back.popped, vec![Pane::Palette]);
    }

    #[test]
    fn odd_layouts_are_repaired_or_replaced() {
        // A popped-out pane isn't docked as well; a retired one is dropped.
        let mut l = Layout::default();
        l.popped.push(Pane::Tracks);
        let fixed = sanitize(drawn(l));
        assert!(!is_open(&fixed.dock, Pane::Tracks));
        let l = drawn(Layout::default());
        let old: Layout = serde_json::from_str(&to_json(&l.dock, &l.popped).replace("\"Tracks\"", "\"Gone\"")).unwrap();
        assert!(!is_open(&sanitize(old).dock, Pane::Retired));
        // No score: the default comes back.
        let mut none = Layout::default();
        none.dock.retain_tabs(|p| *p != Pane::Score);
        assert!(is_open(&sanitize(none).dock, Pane::Score));
        // The score can't be popped out.
        let mut p = Layout::default();
        p.popped.push(Pane::Score);
        assert!(sanitize(p).popped.is_empty());
    }

    #[test]
    fn the_score_bar_hides_only_while_the_score_is_alone() {
        let mut d = default_layout();
        tidy_score_bar(&mut d);
        let at = d.find_tab(&Pane::Score).unwrap();
        assert!(d.main_surface()[at.node].is_tab_bar_hidden());
        let at = d.find_tab(&Pane::Score).unwrap();
        d.set_focused_node_and_surface(NodePath { surface: at.surface, node: at.node });
        d.push_to_focused_leaf(Pane::Keyboard);
        tidy_score_bar(&mut d);
        let at = d.find_tab(&Pane::Score).unwrap();
        assert!(!d.main_surface()[at.node].is_tab_bar_hidden());
    }
}
