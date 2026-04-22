pub struct GalleryEntry {
    pub name: &'static str,
    pub program_text: &'static str,
    /// 0 = native (program's declared width/height).
    pub size: u32,
}

pub fn entries() -> Vec<GalleryEntry> {
    vec![
        GalleryEntry {
            name: "Sky and grass",
            program_text: include_str!("../gallery/00-sky-and-grass.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Luca 02",
            program_text: include_str!("../gallery/02-luca-deltapalette.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Luca 11 Noise XYB",
            program_text: include_str!("../gallery/11-luca-noise-xyb.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Luca 13 WGH",
            program_text: include_str!("../gallery/13-luca-deltapalette-wgh.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Surma 14",
            program_text: include_str!("../gallery/14-surma-deltapalette.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Luca 27 NE + AvgW+N",
            program_text: include_str!("../gallery/27-luca-ne-avgwn.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Luca 29 Prev errors",
            program_text: include_str!("../gallery/29-luca-prev-errors.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Luca 30 Bitdepth 1",
            program_text: include_str!("../gallery/30-luca-bitdepth1.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Surma 31 Squeeze + RCT 16",
            program_text: include_str!("../gallery/31-surma-squeeze-rct16.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Surma 32 Splines",
            program_text: include_str!("../gallery/32-surma-splines.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Surma 34 complex",
            program_text: include_str!("../gallery/34-surma-complex.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Hidden channels",
            program_text: include_str!("../gallery/35-hidden-channels.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Simple gradient",
            program_text: include_str!("../gallery/36-simple-gradient.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Tropical Island Sunset with JXL logo overlay",
            program_text: include_str!(
                "../gallery/p1-001-tropical-island-sunset-with-jxl-logo-overlay.jxlart"
            ),
            size: 0,
        },
        GalleryEntry {
            name: "Rubin golden vase 1",
            program_text: include_str!("../gallery/p1-006-rubin-golden-vase-1.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Lucifer's Dominion: Synthesis",
            program_text: include_str!("../gallery/p1-008-lucifers-dominion-synthesis.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Simplified Diagonalization Argument",
            program_text: include_str!(
                "../gallery/bg-001-simplified-diagonalization-argument.jxlart"
            ),
            size: 0,
        },
        GalleryEntry {
            name: "Cyborg Brain",
            program_text: include_str!("../gallery/bg-002-cyborg-brain.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Lovestorm",
            program_text: include_str!("../gallery/bg-003-lovestorm.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Approximation of a perfect sunset",
            program_text: include_str!(
                "../gallery/bg-004-approximation-of-a-perfect-sunset.jxlart"
            ),
            size: 0,
        },
        GalleryEntry {
            name: "Ceci n'est pas un arbre",
            program_text: include_str!("../gallery/bg-005-ceci-nest-pas-un-arbre.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Peace Flag",
            program_text: include_str!("../gallery/bg-006-peace-flag.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Abstract Haze",
            program_text: include_str!("../gallery/bg-007-abstract-haze.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "lossless",
            program_text: include_str!("../gallery/bg-008-lossless.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Color and Texture",
            program_text: include_str!("../gallery/bg-009-color-and-texture.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Yet Another Sierpinski",
            program_text: include_str!("../gallery/bg-010-yet-another-sierpinski.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Staircased",
            program_text: include_str!("../gallery/bg-011-staircased.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Structure out of the blue",
            program_text: include_str!("../gallery/bg-012-structure-out-of-the-blue.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Fake sky with fake rainbow",
            program_text: include_str!("../gallery/bg-013-fake-sky-with-fake-rainbow.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Hi!",
            program_text: include_str!("../gallery/bg-014-hi.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Minimalist XOR pattern",
            program_text: include_str!("../gallery/bg-015-minimalist-xor-pattern.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Endoplasmic Reticulum",
            program_text: include_str!("../gallery/bg-016-endoplasmic-reticulum.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Endoplasmic Reticulum 2",
            program_text: include_str!("../gallery/bg-017-endoplasmic-reticulum-2.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Endoplasmic Reticulum 2",
            program_text: include_str!("../gallery/bg-018-endoplasmic-reticulum-2.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Misty Ruins",
            program_text: include_str!("../gallery/bg-019-misty-ruins.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "This is fine.",
            program_text: include_str!("../gallery/bg-020-this-is-fine.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Rainbow Cubicles",
            program_text: include_str!("../gallery/bg-021-rainbow-cubicles.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "The Atrocities of War",
            program_text: include_str!("../gallery/bg-022-the-atrocities-of-war.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Magnificent Magnification",
            program_text: include_str!("../gallery/bg-023-magnificent-magnification.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Erosion and shadows",
            program_text: include_str!("../gallery/bg-024-erosion-and-shadows.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Peace",
            program_text: include_str!("../gallery/bg-025-peace.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Random or regular?",
            program_text: include_str!("../gallery/bg-026-random-or-regular.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "A New Style",
            program_text: include_str!("../gallery/bg-027-a-new-style.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "A Maze of Broken Shards",
            program_text: include_str!("../gallery/bg-028-a-maze-of-broken-shards.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Colored Aurora",
            program_text: include_str!("../gallery/bg-029-colored-aurora.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Disintegration Or Genesis?",
            program_text: include_str!("../gallery/bg-030-disintegration-or-genesis.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Sunset in Paradise",
            program_text: include_str!("../gallery/bg-031-sunset-in-paradise.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Sunset in Paradise",
            program_text: include_str!("../gallery/bg-032-sunset-in-paradise.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Sierpinski Meteor Apocalypse (oil on canvas)",
            program_text: include_str!(
                "../gallery/bg-033-sierpinski-meteor-apocalypse-oil-on-canvas.jxlart"
            ),
            size: 0,
        },
        GalleryEntry {
            name: "Inexplicable shapes",
            program_text: include_str!("../gallery/bg-034-inexplicable-shapes.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Minimal pride flag",
            program_text: include_str!("../gallery/bg-035-minimal-pride-flag.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Kryptonite Refinery",
            program_text: include_str!("../gallery/bg-036-kryptonite-refinery.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Cyberpunk Moods",
            program_text: include_str!("../gallery/bg-037-cyberpunk-moods.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Halal",
            program_text: include_str!("../gallery/bg-038-halal.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Binary trees",
            program_text: include_str!("../gallery/bg-039-binary-trees.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Hell is starting to freeze over",
            program_text: include_str!("../gallery/bg-040-hell-is-starting-to-freeze-over.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Cosmic background radiation",
            program_text: include_str!("../gallery/bg-041-cosmic-background-radiation.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Lovebirds",
            program_text: include_str!("../gallery/bg-042-lovebirds.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "build your own polygons",
            program_text: include_str!("../gallery/bg-043-build-your-own-polygons.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Golden matrix",
            program_text: include_str!("../gallery/bg-044-golden-matrix.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Summer abstractions",
            program_text: include_str!("../gallery/bg-045-summer-abstractions.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Summer abstractions",
            program_text: include_str!("../gallery/bg-046-summer-abstractions.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Sponge",
            program_text: include_str!("../gallery/bg-047-sponge.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Autumn Macroblocks",
            program_text: include_str!("../gallery/bg-048-autumn-macroblocks.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "North+Northwest",
            program_text: include_str!("../gallery/bg-049-northnorthwest.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "North+Northeast",
            program_text: include_str!("../gallery/bg-050-northnortheast.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "West+Northwest",
            program_text: include_str!("../gallery/bg-051-westnorthwest.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Weighted",
            program_text: include_str!("../gallery/bg-052-weighted.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "AvgAll",
            program_text: include_str!("../gallery/bg-053-avgall.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "NorthWest",
            program_text: include_str!("../gallery/bg-054-northwest.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Northeast",
            program_text: include_str!("../gallery/bg-055-northeast.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Hairy Selection",
            program_text: include_str!("../gallery/bg-056-hairy-selection.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "4K Moiré Curtains",
            program_text: include_str!("../gallery/bg-057-4k-moiré-curtains.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Carrosserie Brillante",
            program_text: include_str!("../gallery/bg-058-carrosserie-brillante.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Kafkaesque Blue Corridors",
            program_text: include_str!("../gallery/bg-059-kafkaesque-blue-corridors.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Shimmer",
            program_text: include_str!("../gallery/bg-060-shimmer.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "24-bite Chocolate Bar",
            program_text: include_str!("../gallery/bg-061-24-bite-chocolate-bar.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "24-bite Chocolate Bar",
            program_text: include_str!("../gallery/bg-062-24-bite-chocolate-bar.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Lies, damned lies, and statistics",
            program_text: include_str!("../gallery/bg-063-lies-damned-lies-and-statistics.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Hardware",
            program_text: include_str!("../gallery/bg-064-hardware.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Ceci n'est pas 3D",
            program_text: include_str!("../gallery/bg-066-ceci-nest-pas-3d.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Ceci n'est pas 3D",
            program_text: include_str!("../gallery/bg-067-ceci-nest-pas-3d.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Folding an old-fashioned map",
            program_text: include_str!("../gallery/bg-068-folding-an-old-fashioned-map.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Color Bars",
            program_text: include_str!("../gallery/bg-069-color-bars.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Curvature",
            program_text: include_str!("../gallery/bg-070-curvature.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Batteries?",
            program_text: include_str!("../gallery/bg-071-batteries.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Simple rule, unexpected outcome",
            program_text: include_str!("../gallery/bg-072-simple-rule-unexpected-outcome.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Simple rule, unexpected outcome (2)",
            program_text: include_str!("../gallery/bg-073-simple-rule-unexpected-outcome-2.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Simple rule, unexpected outcome (3)",
            program_text: include_str!("../gallery/bg-074-simple-rule-unexpected-outcome-3.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Simple rule, unexpected outcome (4)",
            program_text: include_str!("../gallery/bg-075-simple-rule-unexpected-outcome-4.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Simple rule, unexpected outcome (5)",
            program_text: include_str!("../gallery/bg-076-simple-rule-unexpected-outcome-5.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Simple rule, unexpected outcome (6)",
            program_text: include_str!("../gallery/bg-077-simple-rule-unexpected-outcome-6.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "The Big Puzzle",
            program_text: include_str!("../gallery/bg-078-the-big-puzzle.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Festivities",
            program_text: include_str!("../gallery/bg-079-festivities.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Diagram",
            program_text: include_str!("../gallery/bg-080-diagram.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "The decay of gold",
            program_text: include_str!("../gallery/bg-081-the-decay-of-gold.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "The decay of gold",
            program_text: include_str!("../gallery/bg-082-the-decay-of-gold.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Mondrian Revisited",
            program_text: include_str!("../gallery/bg-083-mondrian-revisited.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Untitled",
            program_text: include_str!("../gallery/bg-084-untitled.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Chessboard Madness",
            program_text: include_str!("../gallery/bg-085-chessboard-madness.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Untitled",
            program_text: include_str!("../gallery/bg-086-untitled.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Untitled",
            program_text: include_str!("../gallery/bg-087-untitled.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Lines",
            program_text: include_str!("../gallery/bg-088-lines.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "$$$$$",
            program_text: include_str!("../gallery/bg-089-untitled.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Feathers",
            program_text: include_str!("../gallery/bg-090-feathers.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Skyscrapers",
            program_text: include_str!("../gallery/bg-091-skyscrapers.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Iron Curtains",
            program_text: include_str!("../gallery/bg-092-iron-curtains.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "The New Variant (simplified)",
            program_text: include_str!("../gallery/bg-093-the-new-variant-simplified.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "What is this",
            program_text: include_str!("../gallery/bg-094-what-is-this.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Or this?",
            program_text: include_str!("../gallery/bg-095-or-this.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "cave crystals",
            program_text: include_str!("../gallery/bg-096-cave-crystals.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Untitled",
            program_text: include_str!("../gallery/bg-097-untitled.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Count the triangles in this picture",
            program_text: include_str!(
                "../gallery/bg-098-count-the-triangles-in-this-picture.jxlart"
            ),
            size: 0,
        },
        GalleryEntry {
            name: "Signals",
            program_text: include_str!("../gallery/bg-099-signals.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Rift",
            program_text: include_str!("../gallery/bg-100-rift.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Advanced Trigonometry",
            program_text: include_str!("../gallery/bg-101-advanced-trigonometry.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Advanced Trigonometry",
            program_text: include_str!("../gallery/bg-102-advanced-trigonometry.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Ripples",
            program_text: include_str!("../gallery/bg-103-ripples.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Zigzag",
            program_text: include_str!("../gallery/bg-104-zigzag.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Mountain peaks?",
            program_text: include_str!("../gallery/bg-105-mountain-peaks.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Moore's Moiré",
            program_text: include_str!("../gallery/bg-106-moores-moiré.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Blue (da ba dee)",
            program_text: include_str!("../gallery/bg-107-blue-da-ba-dee.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Misty Glass Towers",
            program_text: include_str!("../gallery/bg-108-misty-glass-towers.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Industrial Revolution",
            program_text: include_str!("../gallery/bg-109-industrial-revolution.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Barcode synthesis",
            program_text: include_str!("../gallery/bg-110-barcode-synthesis.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Emergent Behavior",
            program_text: include_str!("../gallery/bg-111-emergent-behavior.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Pharmaceutical Abstraction",
            program_text: include_str!("../gallery/bg-112-pharmaceutical-abstraction.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Spliney",
            program_text: include_str!("../gallery/bg-113-spliney.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Drapeau Franco-Belge",
            program_text: include_str!("../gallery/bg-114-drapeau-franco-belge.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Structure (1)",
            program_text: include_str!("../gallery/bg-115-structure-1.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Structure (2)",
            program_text: include_str!("../gallery/bg-116-structure-2.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Structure (3)",
            program_text: include_str!("../gallery/bg-117-structure-3.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Structure (4)",
            program_text: include_str!("../gallery/bg-118-structure-4.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Nebulosity",
            program_text: include_str!("../gallery/bg-119-nebulosity.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Gradient 444",
            program_text: include_str!("../gallery/bg-120-gradient-444.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Gradient 444",
            program_text: include_str!("../gallery/bg-121-gradient-444.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Sun in the sky",
            program_text: include_str!("../gallery/bg-122-sun-in-the-sky.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Sun in the sky",
            program_text: include_str!("../gallery/bg-123-sun-in-the-sky.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Flag of Seychelles",
            program_text: include_str!("../gallery/bg-124-flag-of-seychelles.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Flag of Sweden",
            program_text: include_str!("../gallery/bg-125-flag-of-sweden.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Frozen Sierpiński",
            program_text: include_str!("../gallery/bg-126-frozen-sierpiński.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Painting with light",
            program_text: include_str!("../gallery/bg-127-painting-with-light.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "New Wallpaper",
            program_text: include_str!("../gallery/bg-128-new-wallpaper.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Barfing Chips",
            program_text: include_str!("../gallery/bg-129-barfing-chips.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Stage Spotlight",
            program_text: include_str!("../gallery/bg-130-stage-spotlight.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Irregular diagonal lines",
            program_text: include_str!("../gallery/bg-131-irregular-diagonal-lines.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Completely Bonkers",
            program_text: include_str!("../gallery/bg-132-completely-bonkers.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "RayBeams",
            program_text: include_str!("../gallery/bg-133-raybeams.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "RayBeams 2",
            program_text: include_str!("../gallery/bg-134-raybeams-2.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Flag of Austria",
            program_text: include_str!("../gallery/bg-135-flag-of-austria.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Swiss Flag",
            program_text: include_str!("../gallery/bg-136-swiss-flag.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Unpredictability",
            program_text: include_str!("../gallery/bg-137-unpredictability.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Chromatic Stripes",
            program_text: include_str!("../gallery/bg-138-chromatic-stripes.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Some fucked up shit idk",
            program_text: include_str!("../gallery/bg-139-some-fucked-up-shit-idk.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "German flag",
            program_text: include_str!("../gallery/bg-140-german-flag.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Funky iPhone 16 wallpaper",
            program_text: include_str!("../gallery/bg-141-funky-iphone-16-wallpaper.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Minimalism",
            program_text: include_str!("../gallery/bg-142-minimalism.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Bruges Lace",
            program_text: include_str!("../gallery/bg-143-bruges-lace.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Bruges Lace",
            program_text: include_str!("../gallery/bg-144-bruges-lace.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Shattered palette",
            program_text: include_str!("../gallery/bg-145-shattered-palette.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "something",
            program_text: include_str!("../gallery/bg-146-something.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Directions",
            program_text: include_str!("../gallery/bg-147-directions.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Untitled curly stuff",
            program_text: include_str!("../gallery/bg-148-untitled-curly-stuff.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Pixelation, Interpolation",
            program_text: include_str!("../gallery/bg-149-pixelation-interpolation.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "The Organ",
            program_text: include_str!("../gallery/bg-150-the-organ.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Merry jXl-mas!",
            program_text: include_str!("../gallery/bg-151-merry-jxl-mas.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Merry jXl-mas!",
            program_text: include_str!("../gallery/bg-152-merry-jxl-mas.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Interference",
            program_text: include_str!("../gallery/bg-153-interference.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "A new phone wallpaper",
            program_text: include_str!("../gallery/bg-154-a-new-phone-wallpaper.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "MicroRough Surface",
            program_text: include_str!("../gallery/bg-155-microrough-surface.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Untitled",
            program_text: include_str!("../gallery/bg-156-untitled.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "something",
            program_text: include_str!("../gallery/bg-157-something.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Relatively interesting for 21 bytes",
            program_text: include_str!(
                "../gallery/bg-158-relatively-interesting-for-21-bytes.jxlart"
            ),
            size: 0,
        },
        GalleryEntry {
            name: "The rocket",
            program_text: include_str!("../gallery/bg-159-the-rocket.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Unexpected marble and gold",
            program_text: include_str!("../gallery/bg-160-unexpected-marble-and-gold.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Corruption of order",
            program_text: include_str!("../gallery/bg-161-corruption-of-order.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Spilling and smearing",
            program_text: include_str!("../gallery/bg-162-spilling-and-smearing.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Brushed plastic",
            program_text: include_str!("../gallery/bg-163-brushed-plastic.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "The Epic Siege of the Nether Castle",
            program_text: include_str!(
                "../gallery/bg-164-the-epic-siege-of-the-nether-castle.jxlart"
            ),
            size: 0,
        },
        GalleryEntry {
            name: "melting crystal",
            program_text: include_str!("../gallery/bg-165-melting-crystal.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "DeltaPalette + Squeeze = ?",
            program_text: include_str!("../gallery/bg-166-deltapalette-squeeze.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Set 0",
            program_text: include_str!("../gallery/bg-167-set-0.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Set 1",
            program_text: include_str!("../gallery/bg-168-set-1.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Set 2",
            program_text: include_str!("../gallery/bg-169-set-2.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Set 4",
            program_text: include_str!("../gallery/bg-170-set-4.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Set 6",
            program_text: include_str!("../gallery/bg-171-set-6.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Tablecloth",
            program_text: include_str!("../gallery/bg-172-tablecloth.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Messy mosaic",
            program_text: include_str!("../gallery/bg-173-messy-mosaic.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "How to tame this thing?",
            program_text: include_str!("../gallery/bg-174-how-to-tame-this-thing.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Noisy pattern",
            program_text: include_str!("../gallery/bg-175-noisy-pattern.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Progress Pride Flag v1",
            program_text: include_str!("../gallery/bg-176-progress-pride-flag-v1.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Wonky frame corners",
            program_text: include_str!("../gallery/bg-177-wonky-frame-corners.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Étude nº19",
            program_text: include_str!("../gallery/bg-178-étude-nº19.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Étude nº20",
            program_text: include_str!("../gallery/bg-179-étude-nº20.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Étude nº22",
            program_text: include_str!("../gallery/bg-180-étude-nº22.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Étude nº28",
            program_text: include_str!("../gallery/bg-181-étude-nº28.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Default delta metallic",
            program_text: include_str!("../gallery/bg-182-default-delta-metallic.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "A selection of Occam's razors",
            program_text: include_str!("../gallery/bg-183-a-selection-of-occams-razors.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Alien Muscle Tissue Sample",
            program_text: include_str!("../gallery/bg-184-alien-muscle-tissue-sample.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Entropie Émergente (Étude nº 241)",
            program_text: include_str!("../gallery/bg-185-entropie-émergente-étude-nº-241.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Stairs mutating",
            program_text: include_str!("../gallery/bg-186-stairs-mutating.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Entropie Émergente (Étude nº -225)",
            program_text: include_str!("../gallery/bg-187-entropie-émergente-étude-nº-225.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "broken prism",
            program_text: include_str!("../gallery/bg-188-broken-prism.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "the smiling face",
            program_text: include_str!("../gallery/bg-189-the-smiling-face.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "The Snake",
            program_text: include_str!("../gallery/bg-190-the-snake.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "More Delta Palette Abstract Stuff",
            program_text: include_str!(
                "../gallery/bg-191-more-delta-palette-abstract-stuff.jxlart"
            ),
            size: 0,
        },
        GalleryEntry {
            name: "Waterfall Brushes",
            program_text: include_str!("../gallery/bg-192-waterfall-brushes.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Flag of Cornwall",
            program_text: include_str!("../gallery/bg-193-flag-of-cornwall.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Flag of Cornwall",
            program_text: include_str!("../gallery/bg-194-flag-of-cornwall.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Chroma and luma (Rec2100 PQ)",
            program_text: include_str!("../gallery/hdr-001-chroma-and-luma-rec2100-pq.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "freshly baked tray of neoncookieli",
            program_text: include_str!(
                "../gallery/hdr-002-freshly-baked-tray-of-neoncookieli.jxlart"
            ),
            size: 0,
        },
        GalleryEntry {
            name: "neon fractal",
            program_text: include_str!("../gallery/hdr-003-neon-fractal.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Sierpinski's shadow",
            program_text: include_str!("../gallery/hdr-004-sierpinskis-shadow.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Something something neon HDR",
            program_text: include_str!("../gallery/hdr-005-something-something-neon-hdr.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Nuggets of default delta palette in HDR",
            program_text: include_str!(
                "../gallery/hdr-006-nuggets-of-default-delta-palette-in-hdr.jxlart"
            ),
            size: 0,
        },
        GalleryEntry {
            name: "HDR test image (10 bit, full range PQ)",
            program_text: include_str!(
                "../gallery/hdr-007-hdr-test-image-10-bit-full-range-pq.jxlart"
            ),
            size: 0,
        },
        GalleryEntry {
            name: "ITU-R BT.2111-3 (2K, 10 bit, full range PQ)",
            program_text: include_str!(
                "../gallery/hdr-008-itu-r-bt2111-3-2k-10-bit-full-range-pq.jxlart"
            ),
            size: 0,
        },
        GalleryEntry {
            name: "Heat",
            program_text: include_str!("../gallery/hdr-009-heat.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Flares",
            program_text: include_str!("../gallery/hdr-010-flares.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Surma extra 1",
            program_text: include_str!("../gallery/surma-001-surma-extra-1.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Surma extra 2",
            program_text: include_str!("../gallery/surma-002-surma-extra-2.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Surma extra 3",
            program_text: include_str!("../gallery/surma-003-surma-extra-3.jxlart"),
            size: 0,
        },
    ]
}
