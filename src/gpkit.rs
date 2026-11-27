//! The drum kit as Guitar Pro writes it into a file: its elements and their articulations, in
//! Guitar Pro's order (a drum note names its sound by its place in this list). Read from the
//! kit definition in a file Guitar Pro saved. Staff lines and output sounds are in `drums.rs`.

/// One way of playing an element.
pub struct Articulation {
    /// The sound's number (`drums.rs`).
    pub midi: u8,
    pub name: &'static str,
    /// Noteheads for short, half and whole notes.
    pub heads: &'static str,
    /// Where the technique sign goes, and the sign (empty for none).
    pub place: &'static str,
    pub symbol: &'static str,
    /// The sound in Guitar Pro's own sound engine.
    pub rse: &'static str,
}

pub struct Element {
    pub name: &'static str,
    pub kind: &'static str,
    pub soundbank: &'static str,
    pub articulations: &'static [Articulation],
}

const fn a(midi: u8, name: &'static str, heads: &'static str, place: &'static str, symbol: &'static str, rse: &'static str) -> Articulation {
    Articulation { midi, name, heads, place, symbol, rse }
}

#[rustfmt::skip]
pub const KIT: &[Element] = &[
    Element { name: "Snare", kind: "snare", soundbank: "Master-Snare", articulations: &[
        a(38, "Snare (hit)", "noteheadBlack noteheadHalf noteheadWhole", "outside", "", "stick.hit.hit"),
        a(37, "Snare (side stick)", "noteheadXBlack noteheadXBlack noteheadXBlack", "outside", "", "stick.hit.sidestick"),
        a(91, "Snare (rim shot)", "noteheadDiamondWhite noteheadDiamondWhite noteheadDiamondWhite", "outside", "", "stick.hit.rimshot"),
    ] },
    Element { name: "Charley", kind: "hiHat", soundbank: "Master-Hihat", articulations: &[
        a(42, "Hi-Hat (closed)", "noteheadXBlack noteheadXBlack noteheadXBlack", "outside", "", "stick.hit.closed"),
        a(92, "Hi-Hat (half)", "noteheadCircleSlash noteheadCircleSlash noteheadCircleSlash", "outside", "", "stick.hit.half"),
        a(46, "Hi-Hat (open)", "noteheadCircleX noteheadCircleX noteheadCircleX", "outside", "", "stick.hit.open"),
        a(44, "Pedal Hi-Hat (hit)", "noteheadXBlack noteheadXBlack noteheadXBlack", "outside", "", "pedal.hit.pedal"),
    ] },
    Element { name: "Acoustic Kick Drum", kind: "kickDrum", soundbank: "AcousticKick-Percu", articulations: &[
        a(35, "Kick (hit)", "noteheadBlack noteheadHalf noteheadWhole", "outside", "", "pedal.hit.hit"),
    ] },
    Element { name: "Kick Drum", kind: "kickDrum", soundbank: "Master-Kick", articulations: &[
        a(36, "Kick (hit)", "noteheadBlack noteheadHalf noteheadWhole", "outside", "", "pedal.hit.hit"),
    ] },
    Element { name: "Tom Very High", kind: "tom", soundbank: "Master-Tom05", articulations: &[
        a(50, "High Floor Tom (hit)", "noteheadBlack noteheadHalf noteheadWhole", "outside", "", "stick.hit.hit"),
    ] },
    Element { name: "Tom High", kind: "tom", soundbank: "Master-Tom04", articulations: &[
        a(48, "High Tom (hit)", "noteheadBlack noteheadHalf noteheadWhole", "outside", "", "stick.hit.hit"),
    ] },
    Element { name: "Tom Medium", kind: "tom", soundbank: "Master-Tom03", articulations: &[
        a(47, "Mid Tom (hit)", "noteheadBlack noteheadHalf noteheadWhole", "outside", "", "stick.hit.hit"),
    ] },
    Element { name: "Tom Low", kind: "tom", soundbank: "Master-Tom02", articulations: &[
        a(45, "Low Tom (hit)", "noteheadBlack noteheadHalf noteheadWhole", "outside", "", "stick.hit.hit"),
    ] },
    Element { name: "Tom Very Low", kind: "tom", soundbank: "Master-Tom01", articulations: &[
        a(43, "Very Low Tom (hit)", "noteheadBlack noteheadHalf noteheadWhole", "outside", "", "stick.hit.hit"),
    ] },
    Element { name: "Ride", kind: "ride", soundbank: "Master-Ride", articulations: &[
        a(93, "Ride (edge)", "noteheadXBlack noteheadXBlack noteheadXBlack", "above", "pictEdgeOfCymbal", "stick.hit.edge"),
        a(51, "Ride (middle)", "noteheadXBlack noteheadXBlack noteheadXBlack", "outside", "", "stick.hit.mid"),
        a(53, "Ride (bell)", "noteheadDiamondWhite noteheadDiamondWhite noteheadDiamondWhite", "outside", "", "stick.hit.bell"),
        a(94, "Ride (choke)", "noteheadXBlack noteheadXBlack noteheadXBlack", "outside", "articStaccatoAbove", "stick.hit.choke"),
    ] },
    Element { name: "Splash", kind: "splash", soundbank: "Master-Splash", articulations: &[
        a(55, "Splash (hit)", "noteheadXBlack noteheadXBlack noteheadXBlack", "outside", "", "stick.hit.hit"),
        a(95, "Splash (choke)", "noteheadXBlack noteheadXBlack noteheadXBlack", "outside", "articStaccatoAbove", "stick.hit.choke"),
    ] },
    Element { name: "China", kind: "china", soundbank: "Master-China", articulations: &[
        a(52, "China (hit)", "noteheadHeavyXHat noteheadHeavyXHat noteheadHeavyXHat", "outside", "", "stick.hit.hit"),
        a(96, "China (choke)", "noteheadHeavyXHat noteheadHeavyXHat noteheadHeavyXHat", "outside", "", "stick.hit.choke"),
    ] },
    Element { name: "Crash High", kind: "crash", soundbank: "Master-Crash02", articulations: &[
        a(49, "Crash high (hit)", "noteheadHeavyX noteheadHeavyX noteheadHeavyX", "outside", "", "stick.hit.hit"),
        a(97, "Crash high (choke)", "noteheadHeavyX noteheadHeavyX noteheadHeavyX", "outside", "articStaccatoAbove", "stick.hit.choke"),
    ] },
    Element { name: "Crash Medium", kind: "crash", soundbank: "Master-Crash01", articulations: &[
        a(57, "Crash medium (hit)", "noteheadHeavyX noteheadHeavyX noteheadHeavyX", "outside", "", "stick.hit.hit"),
        a(98, "Crash medium (choke)", "noteheadHeavyX noteheadHeavyX noteheadHeavyX", "outside", "articStaccatoAbove", "stick.hit.choke"),
    ] },
    Element { name: "Cowbell Low", kind: "cowbell", soundbank: "CowbellBig-Percu", articulations: &[
        a(99, "Cowbell low (hit)", "noteheadTriangleUpBlack noteheadTriangleUpHalf noteheadTriangleUpWhole", "outside", "", "stick.hit.hit"),
        a(100, "Cowbell low (tip)", "noteheadXBlack noteheadXHalf noteheadXWhole", "outside", "", "stick.hit.tip"),
    ] },
    Element { name: "Cowbell Medium", kind: "cowbell", soundbank: "CowbellMid-Percu", articulations: &[
        a(56, "Cowbell medium (hit)", "noteheadTriangleUpBlack noteheadTriangleUpHalf noteheadTriangleUpWhole", "outside", "", "stick.hit.hit"),
        a(101, "Cowbell medium (tip)", "noteheadXBlack noteheadXHalf noteheadXWhole", "outside", "", "stick.hit.tip"),
    ] },
    Element { name: "Cowbell High", kind: "cowbell", soundbank: "CowbellSmall-Percu", articulations: &[
        a(102, "Cowbell high (hit)", "noteheadTriangleUpBlack noteheadTriangleUpHalf noteheadTriangleUpWhole", "outside", "", "stick.hit.hit"),
        a(103, "Cowbell high (tip)", "noteheadXBlack noteheadXHalf noteheadXWhole", "outside", "", "stick.hit.tip"),
    ] },
    Element { name: "Woodblock Low", kind: "woodblock", soundbank: "WoodblockLow-Percu", articulations: &[
        a(77, "Woodblock low (hit)", "noteheadTriangleUpBlack noteheadTriangleUpBlack noteheadTriangleUpBlack", "outside", "", "stick.hit.hit"),
    ] },
    Element { name: "Woodblock High", kind: "woodblock", soundbank: "WoodblockHigh-Percu", articulations: &[
        a(76, "Woodblock high (hit)", "noteheadTriangleUpBlack noteheadTriangleUpBlack noteheadTriangleUpBlack", "outside", "", "stick.hit.hit"),
    ] },
    Element { name: "Bongo High", kind: "bongo", soundbank: "BongoHigh-Percu", articulations: &[
        a(60, "Hand (hit)", "noteheadBlack noteheadHalf noteheadWhole", "outside", "", "hand.hit.hit"),
        a(104, "Hand (mute)", "noteheadBlack noteheadHalf noteheadWhole", "inside", "noteheadParenthesis", "hand.hit.mute"),
        a(105, "Hand (slap)", "noteheadXBlack noteheadXBlack noteheadXBlack", "outside", "", "hand.hit.slap"),
    ] },
    Element { name: "Bongo Low", kind: "bongo", soundbank: "BongoLow-Percu", articulations: &[
        a(61, "Hand (hit)", "noteheadBlack noteheadHalf noteheadWhole", "outside", "", "hand.hit.hit"),
        a(106, "Hand (mute)", "noteheadBlack noteheadHalf noteheadWhole", "inside", "noteheadParenthesis", "hand.hit.mute"),
        a(107, "Hand (slap)", "noteheadXBlack noteheadXBlack noteheadXBlack", "outside", "", "hand.hit.slap"),
    ] },
    Element { name: "Timbale Low", kind: "timbale", soundbank: "TimbaleLow-Percu", articulations: &[
        a(66, "Timbale low (hit)", "noteheadBlack noteheadHalf noteheadWhole", "outside", "", "stick.hit.hit"),
    ] },
    Element { name: "Timbale High", kind: "timbale", soundbank: "TimbaleHigh-Percu", articulations: &[
        a(65, "Timbale high (hit)", "noteheadBlack noteheadHalf noteheadWhole", "outside", "", "stick.hit.hit"),
    ] },
    Element { name: "Agogo Low", kind: "agogo", soundbank: "AgogoLow-Percu", articulations: &[
        a(68, "Agogo low (hit)", "noteheadBlack noteheadHalf noteheadWhole", "outside", "", "stick.hit.hit"),
    ] },
    Element { name: "Agogo High", kind: "agogo", soundbank: "AgogoHigh-Percu", articulations: &[
        a(67, "Agogo high (hit)", "noteheadBlack noteheadHalf noteheadWhole", "outside", "", "stick.hit.hit"),
    ] },
    Element { name: "Conga Low", kind: "conga", soundbank: "CongaLow-Percu", articulations: &[
        a(64, "Conga low (hit)", "noteheadBlack noteheadHalf noteheadWhole", "outside", "", "hand.hit.hit"),
        a(108, "Conga low (slap)", "noteheadXBlack noteheadXBlack noteheadXBlack", "outside", "", "hand.hit.slap"),
        a(109, "Conga low (mute)", "noteheadBlack noteheadHalf noteheadWhole", "inside", "noteheadParenthesis", "hand.hit.mute"),
    ] },
    Element { name: "Conga High", kind: "conga", soundbank: "CongaHigh-Percu", articulations: &[
        a(63, "Conga high (hit)", "noteheadBlack noteheadHalf noteheadWhole", "outside", "", "hand.hit.hit"),
        a(110, "Conga high (slap)", "noteheadXBlack noteheadXBlack noteheadXBlack", "outside", "", "hand.hit.slap"),
        a(62, "Conga high (mute)", "noteheadBlack noteheadHalf noteheadWhole", "inside", "noteheadParenthesis", "hand.hit.mute"),
    ] },
    Element { name: "Whistle Low", kind: "whistle", soundbank: "WhistleLow-Percu", articulations: &[
        a(72, "Whistle low (hit)", "noteheadBlack noteheadHalf noteheadWhole", "outside", "", "blow.hit.hit"),
    ] },
    Element { name: "Whistle High", kind: "whistle", soundbank: "WhistleHigh-Percu", articulations: &[
        a(71, "Whistle high (hit)", "noteheadBlack noteheadHalf noteheadWhole", "outside", "", "blow.hit.hit"),
    ] },
    Element { name: "Guiro", kind: "guiro", soundbank: "Guiro-Percu", articulations: &[
        a(73, "Guiro (hit)", "noteheadBlack noteheadHalf noteheadWhole", "outside", "", "stick.hit.hit"),
        a(74, "Guiro (scrap-return)", "noteheadBlack noteheadHalf noteheadWhole", "outside", "", "stick.scrape.return"),
    ] },
    Element { name: "Surdo", kind: "surdo", soundbank: "Surdo-Percu", articulations: &[
        a(86, "Surdo (hit)", "noteheadBlack noteheadHalf noteheadWhole", "outside", "", "brush.hit.hit"),
        a(87, "Surdo (mute)", "noteheadXBlack noteheadXBlack noteheadXBlack", "inside", "noteheadParenthesis", "brush.hit.mute"),
    ] },
    Element { name: "Tambourine", kind: "tambourine", soundbank: "Tambourine-Percu", articulations: &[
        a(54, "Tambourine (hit)", "noteheadTriangleUpBlack noteheadTriangleUpBlack noteheadTriangleUpBlack", "outside", "", "hand.hit.hit"),
        a(111, "Tambourine (return)", "noteheadTriangleUpBlack noteheadTriangleUpBlack noteheadTriangleUpBlack", "above", "stringsUpBow", "hand.hit.return"),
        a(112, "Tambourine (roll)", "noteheadTriangleUpBlack noteheadTriangleUpBlack noteheadTriangleUpBlack", "above", "stringsDownBow", "hand.hit.roll"),
        a(113, "Tambourine (hand)", "noteheadXBlack noteheadXBlack noteheadXBlack", "outside", "", "hand.hit.handhit"),
    ] },
    Element { name: "Cuica", kind: "cuica", soundbank: "Cuica-Percu", articulations: &[
        a(79, "Cuica (open)", "noteheadBlack noteheadHalf noteheadWhole", "outside", "", "hand.hit.hit"),
        a(78, "Cuica (mute)", "noteheadXBlack noteheadXBlack noteheadXBlack", "outside", "", "hand.hit.mute"),
    ] },
    Element { name: "Vibraslap", kind: "vibraslap", soundbank: "Vibraslap-Percu", articulations: &[
        a(58, "Vibraslap (hit)", "noteheadBlack noteheadHalf noteheadWhole", "outside", "", "hand.hit.hit"),
    ] },
    Element { name: "Triangle", kind: "triangle", soundbank: "Triangle-Percu", articulations: &[
        a(81, "Triangle (hit)", "noteheadBlack noteheadHalf noteheadWhole", "outside", "", "stick.hit.hit"),
        a(80, "Triangle (mute)", "noteheadXBlack noteheadXBlack noteheadXBlack", "inside", "noteheadParenthesis", "stick.hit.mute"),
    ] },
    Element { name: "Grancassa", kind: "grancassa", soundbank: "Grancassa-Percu", articulations: &[
        a(114, "Grancassa (hit)", "noteheadBlack noteheadHalf noteheadWhole", "outside", "", "mallet.hit.hit"),
    ] },
    Element { name: "Piatti", kind: "piatti", soundbank: "Piatti-Percu", articulations: &[
        a(115, "Piatti (hit)", "noteheadBlack noteheadHalf noteheadWhole", "outside", "", "hand.hit.hit"),
        a(116, "Piatti (hand)", "noteheadXBlack noteheadXBlack noteheadXBlack", "outside", "", "hand.hit.hit"),
    ] },
    Element { name: "Cabasa", kind: "cabasa", soundbank: "Cabasa-Percu", articulations: &[
        a(69, "Cabasa (hit)", "noteheadBlack noteheadHalf noteheadWhole", "outside", "", "hand.hit.hit"),
        a(117, "Cabasa (return)", "noteheadBlack noteheadHalf noteheadWhole", "outside", "stringsUpBow", "hand.hit.return"),
    ] },
    Element { name: "Castanets", kind: "castanets", soundbank: "Castanets-Percu", articulations: &[
        a(85, "Castanets (hit)", "noteheadBlack noteheadHalf noteheadWhole", "outside", "", "hand.hit.hit"),
    ] },
    Element { name: "Claves", kind: "claves", soundbank: "Claves-Percu", articulations: &[
        a(75, "Claves (hit)", "noteheadBlack noteheadHalf noteheadWhole", "outside", "", "stick.hit.hit"),
    ] },
    Element { name: "Left Maraca", kind: "maraca", soundbank: "Maracas-Percu", articulations: &[
        a(70, "Left Maraca (hit)", "noteheadBlack noteheadHalf noteheadWhole", "outside", "", "hand.hit.hit"),
        a(118, "Left Maraca (return)", "noteheadBlack noteheadHalf noteheadWhole", "outside", "stringsUpBow", "hand.hit.return"),
    ] },
    Element { name: "Right Maraca", kind: "maraca", soundbank: "Maracas-Percu", articulations: &[
        a(119, "Right Maraca (hit)", "noteheadBlack noteheadHalf noteheadWhole", "outside", "", "hand.hit.hit"),
        a(120, "Right Maraca (return)", "noteheadBlack noteheadHalf noteheadWhole", "outside", "stringsUpBow", "hand.hit.return"),
    ] },
    Element { name: "Orchestral Snare", kind: "snare", soundbank: "SnareDrum-Percu", articulations: &[
        a(121, "Snare (hit)", "noteheadBlack noteheadHalf noteheadWhole", "outside", "", "stick.hit.hit"),
    ] },
    Element { name: "Shaker", kind: "shaker", soundbank: "ShakerStudio-Percu", articulations: &[
        a(82, "Shaker (hit)", "noteheadBlack noteheadHalf noteheadWhole", "outside", "", "hand.hit.hit"),
        a(122, "Shaker (return)", "noteheadBlack noteheadHalf noteheadWhole", "outside", "stringsUpBow", "hand.hit.return"),
    ] },
    Element { name: "Bell Tree", kind: "bellTree", soundbank: "BellTree-Percu", articulations: &[
        a(84, "Bell Tree (hit)", "noteheadBlack noteheadHalf noteheadWhole", "outside", "", "stick.hit.hit"),
        a(123, "Bell Tree (return)", "noteheadBlack noteheadHalf noteheadWhole", "outside", "stringsUpBow", "stick.hit.return"),
    ] },
    Element { name: "Jingle Bell", kind: "jingleBell", soundbank: "JingleBell-Percu", articulations: &[
        a(83, "Jingle Bell (hit)", "noteheadBlack noteheadHalf noteheadWhole", "outside", "", "stick.hit.hit"),
    ] },
    Element { name: "Golpe", kind: "unpitched", soundbank: "Golpe-Percu", articulations: &[
        a(124, "Golpe (thumb)", "noteheadNull noteheadNull noteheadNull", "below", "guitarGolpe", "thumb.hit.body"),
        a(125, "Golpe (finger)", "noteheadNull noteheadNull noteheadNull", "above", "guitarGolpe", "finger4.hit.body"),
    ] },
    Element { name: "Hand Clap", kind: "handClap", soundbank: "GroupHandClap-Percu", articulations: &[
        a(39, "Hand Clap (hit)", "noteheadBlack noteheadHalf noteheadWhole", "outside", "", "hand.hit.hit"),
    ] },
    Element { name: "Electric Snare", kind: "snare", soundbank: "ElectricSnare-Percu", articulations: &[
        a(40, "Snare (hit)", "noteheadBlack noteheadHalf noteheadWhole", "outside", "", "stick.hit.hit"),
    ] },
    Element { name: "Sticks", kind: "snare", soundbank: "Stick-Percu", articulations: &[
        a(31, "Snare (side stick)", "noteheadSlashedBlack2 noteheadSlashedBlack2 noteheadSlashedBlack2", "outside", "", "stick.hit.sidestick"),
    ] },
    Element { name: "Very Low Floor Tom", kind: "tom", soundbank: "LowFloorTom-Percu", articulations: &[
        a(41, "Low Floor Tom (hit)", "noteheadBlack noteheadHalf noteheadWhole", "outside", "", "stick.hit.hit"),
    ] },
    Element { name: "Ride Cymbal 2", kind: "ride", soundbank: "Ride-Percu", articulations: &[
        a(59, "Ride (edge)", "noteheadXBlack noteheadXBlack noteheadXBlack", "above", "pictEdgeOfCymbal", "stick.hit.edge"),
        a(126, "Ride (middle)", "noteheadXBlack noteheadXBlack noteheadXBlack", "outside", "", "stick.hit.mid"),
        a(127, "Ride (bell)", "noteheadDiamondWhite noteheadDiamondWhite noteheadDiamondWhite", "outside", "", "stick.hit.bell"),
        a(29, "Ride (choke)", "noteheadXBlack noteheadXBlack noteheadXBlack", "outside", "articStaccatoAbove", "stick.hit.choke"),
    ] },
    Element { name: "Reverse Cymbal", kind: "crash", soundbank: "Reverse-Cymbal", articulations: &[
        a(30, "Cymbal (hit)", "noteheadXBlack noteheadXBlack noteheadXBlack", "outside", "", "stick.hit.hit"),
    ] },
    Element { name: "Metronome", kind: "snare", soundbank: "Metronome-Percu", articulations: &[
        a(33, "Snare (side stick)", "noteheadXBlack noteheadXBlack noteheadXBlack", "outside", "", "stick.hit.sidestick"),
        a(34, "Snare (hit)", "noteheadBlack noteheadBlack noteheadBlack", "outside", "", "stick.hit.hit"),
    ] },
];

/// The name of the standard notation, and the sounds it lists, by element.
pub const NOTATION: &str = "Drumkit-Standard";

#[rustfmt::skip]
pub const NOTATION_SOUNDS: &[(&str, &[u8])] = &[
    ("Snare", &[38, 37, 91]),
    ("Charley", &[42, 92, 46, 44]),
    ("Acoustic Kick Drum", &[35]),
    ("Kick Drum", &[36]),
    ("Tom Very High", &[50]),
    ("Tom High", &[48]),
    ("Tom Medium", &[47]),
    ("Tom Low", &[45]),
    ("Tom Very Low", &[43]),
    ("Ride", &[93, 51, 53]),
    ("Splash", &[55]),
    ("China", &[52]),
    ("Crash High", &[49]),
    ("Crash Medium", &[57]),
    ("Cowbell Low", &[99]),
    ("Cowbell Medium", &[56]),
    ("Cowbell High", &[102]),
];
