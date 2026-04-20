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
            name: "Progress Pride Flag",
            program_text: include_str!("../gallery/01-progress-pride-flag.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Luca 02",
            program_text: include_str!("../gallery/02-luca-deltapalette.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Luca 03",
            program_text: include_str!("../gallery/03-luca-deltapalette-b.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Luca 04",
            program_text: include_str!("../gallery/04-luca-deltapalette-c.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Luca 05 gaborish",
            program_text: include_str!("../gallery/05-luca-gaborish.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Luca 06 Rec2100",
            program_text: include_str!("../gallery/06-luca-rec2100.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Luca 07 RCT 0",
            program_text: include_str!("../gallery/07-luca-rct0.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Luca 08 RCT 20",
            program_text: include_str!("../gallery/08-luca-rct20.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Luca 09 Rec2100 PQ",
            program_text: include_str!("../gallery/09-luca-rec2100-pq.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Luca 10 RCT 2",
            program_text: include_str!("../gallery/10-luca-rct2.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Luca 11 Noise XYB",
            program_text: include_str!("../gallery/11-luca-noise-xyb.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Luca 12 Upsample 4",
            program_text: include_str!("../gallery/12-luca-upsample4.jxlart"),
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
            name: "Luca 15 RCT 13",
            program_text: include_str!("../gallery/15-luca-rct13.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Luca 16 Rec2100 PQ + FramePos",
            program_text: include_str!("../gallery/16-luca-rec2100-pq-fp.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Luca 17 NE predictor",
            program_text: include_str!("../gallery/17-luca-nepred.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Luca 18 RCT 27",
            program_text: include_str!("../gallery/18-luca-rct27.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Luca 19 RCT 13 b",
            program_text: include_str!("../gallery/19-luca-rct13-b.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Luca 20 WW",
            program_text: include_str!("../gallery/20-luca-deltapalette-ww.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Luca 21 NW-N",
            program_text: include_str!("../gallery/21-luca-deltapalette-nwn.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Luca 22 composite",
            program_text: include_str!("../gallery/22-luca-deltapalette-composite.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Luca 23 Spline",
            program_text: include_str!("../gallery/23-luca-spline.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Luca 24 Spline RCT 6",
            program_text: include_str!("../gallery/24-luca-spline-rct6.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Luca 25 WGH",
            program_text: include_str!("../gallery/25-luca-wgh.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Luca 26 Select + Gradient",
            program_text: include_str!("../gallery/26-luca-select-gradient.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Luca 27 NE + AvgW+N",
            program_text: include_str!("../gallery/27-luca-ne-avgwn.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Luca 28 Gradient",
            program_text: include_str!("../gallery/28-luca-deltapalette-gradient.jxlart"),
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
            name: "Luca 33 Rec2100 + Noise",
            program_text: include_str!("../gallery/33-luca-rec2100-noise.jxlart"),
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
            name: "Luca 37 minimal",
            program_text: include_str!("../gallery/37-luca-minimal.jxlart"),
            size: 0,
        },
        GalleryEntry {
            name: "Luca 38 FramePos",
            program_text: include_str!("../gallery/38-luca-framepos.jxlart"),
            size: 0,
        },
    ]
}
