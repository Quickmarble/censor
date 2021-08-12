use crate::colour::*;
use crate::palette::*;
use crate::text::*;
use crate::graph::*;
use crate::cache::PlotCacher;
use crate::widget::*;
use crate::metadata;

use std::f32::consts::PI;

pub fn analyse(colours: &Vec<RGB255>, T: f32, cacher: &mut PlotCacher, font: &Font, fname: String) {
    eprintln!("Starting analysis.");
    let ill = CAT16Illuminant::new(CIExy::from_T(T));
    let palette = Palette::new(colours.clone(), &ill);

    let w: i32 = 640;
    let h: i32 = 432;

    let mut graph = ImageGraph::new(w as u32, h as u32);
    graph.block(0, 0, w, h, palette.rgb[palette.bg]);
    eprintln!("Created the canvas.");

    let inner_x = 17;
    let inner_y = 16;
    let inner_w = 610;
    let inner_h = 406;
    graph.block(inner_x, inner_y, inner_w, inner_h, palette.rgb[palette.bl]);

    graph.text(&format!("= CENSOR v{} - PALETTE ANALYSER =", metadata::VERSION),
               w / 2, 2, TextAnchor::n(), font,
               palette.rgb[palette.tl]);
    graph.text(&format!("Unique colours in palette: {}", palette.n),
               2, 2, TextAnchor::nw(), font,
               palette.rgb[palette.tl]);
    graph.text("Colour difference: CAM16UCS",
               w - 2, 2, TextAnchor::ne(), font,
               palette.rgb[palette.tl]);
    graph.text(&format!("Illuminant: D(T={:.2}°K)", T),
               w - 2, 9, TextAnchor::ne(), font,
               palette.rgb[palette.tl]);

    let rect_JCh_w = 99;
    let rect_JCh_h = 96;
    let rect_JCh_C = [40, 10];
    for i in 0..rect_JCh_C.len() {
        let C = rect_JCh_C[i];
        let x = inner_x + 1 + (rect_JCh_w + 2) * i as i32;
        let y = inner_y + 1;
        graph.text(&format!("CHROMA: {}", C),
                   x, inner_y - 1, TextAnchor::sw(), font,
                   palette.rgb[palette.bl]);
        let rect_JCh = RectJChWidget::new(rect_JCh_w, rect_JCh_h, C as f32);
        rect_JCh.render(&mut graph, cacher, &palette, &ill, font, x, y);
    }

    let spectrum_w = 200;
    let spectrum_h = 6;
    let spectrum_y = inner_y + 100;
    graph.text("SPEC",
               inner_x - 1, spectrum_y + 1, TextAnchor::ne(), font,
               palette.rgb[palette.bl]);
    graph.text("C50%",
               inner_x - 1, spectrum_y + 1 + (spectrum_h + 1), TextAnchor::ne(), font,
               palette.rgb[palette.bl]);
    graph.text("J50%",
               inner_x - 1, spectrum_y + 1 + (spectrum_h + 1) * 2, TextAnchor::ne(), font,
               palette.rgb[palette.bl]);
    let spectrum = SpectrumWidget::new(spectrum_w, spectrum_h);
    spectrum.render(&mut graph, cacher, &palette, &ill, font, inner_x + 1, spectrum_y);

    let spectrobox_y = inner_y + 123;
    let spectrobox_w = 200;
    let spectrobox_h = 92;
    graph.text("SPEC",
               inner_x - 1, spectrobox_y + 1 + spectrobox_h / 2 - 3, TextAnchor::e(), font,
               palette.rgb[palette.bl]);
    graph.text("BOX",
               inner_x - 1, spectrobox_y + 1 + spectrobox_h / 2 + 3, TextAnchor::e(), font,
               palette.rgb[palette.bl]);
    let spectrobox = SpectroBoxWidget::new(spectrobox_w, spectrobox_h);
    spectrobox.render(&mut graph, cacher, &palette, &ill, font, inner_x + 1, spectrobox_y);

    let indexed_x = inner_x + 203;
    let indexed_y = inner_y + 1;
    graph.text("INDEXED PALETTE",
               indexed_x, inner_y - 1, TextAnchor::sw(), font,
               palette.rgb[palette.bl]);
    let indexed = IndexedWidget::new(32, 8, 3, 4);
    indexed.render(&mut graph, cacher, &palette, &ill, font, indexed_x, indexed_y);

    let close_n = 10;
    let close_d = 9;
    let close_x = inner_x + 203;
    let close_w = (close_d + 1) * close_n - 1;
    let close10_y = inner_y + 45;
    graph.text("close cols: 10% li-match",
               close_x + close_w / 2, close10_y, TextAnchor::s(), font,
               palette.rgb[palette.fg]);
    let close10 = CloseLiMatchWidget::new(close_d, close_d, close_n as usize, 0.1);
    close10.render(&mut graph, cacher, &palette, &ill, font, close_x, close10_y);

    let close70_y = close10_y + close_d * 2 + 2;
    graph.text("close cols: 70% li-match",
               close_x + close_w / 2, close70_y + close_d * 2 + 1, TextAnchor::n(), font,
               palette.rgb[palette.fg]);
    let close70 = CloseLiMatchWidget::new(close_d, close_d, close_n as usize, 0.7);
    close70.render(&mut graph, cacher, &palette, &ill, font, close_x, close70_y);

    let iss_x = inner_x + 203;
    let iss_y = inner_y + 92;
    let iss_w = 44;
    let iss_h = 24;
    let iss = ISSWidget::new(iss_w, iss_h, 2., 3.5);
    iss.render(&mut graph, cacher, &palette, &ill, font, iss_x, iss_y);

    let acyclic_x = inner_x + 259;
    let acyclic_y = inner_y + 92;
    let acyclic_w = 44;
    let acyclic_h = 24;
    let acyclic = AcyclicWidget::new(acyclic_w, acyclic_h);
    acyclic.render(&mut graph, cacher, &palette, &ill, font, acyclic_x, acyclic_y);

    let sdist_x = inner_x + 203;
    let sdist_y = inner_y + 124;
    let sdist_w = 100;
    let sdist_h = 36;
    graph.text("spectral distribution",
               sdist_x + sdist_w / 2, sdist_y, TextAnchor::s(), font,
               palette.rgb[palette.fg]);
    let sdist = SpectralDistributionWidget::new(sdist_w, sdist_h);
    sdist.render(&mut graph, cacher, &palette, &ill, font, sdist_x, sdist_y);

    let tdist_x = inner_x + 203;
    let tdist_y = inner_y + 170;
    let tdist_w = 100;
    let tdist_h = 36;
    graph.text("temperature",
               tdist_x + tdist_w / 2, tdist_y - 1, TextAnchor::s(), font,
               palette.rgb[palette.fg]);
    let tdist = TemperatureDistributionWidget::new(tdist_w, tdist_h);
    tdist.render(&mut graph, cacher, &palette, &ill, font, tdist_x, tdist_y);

    let limatch_x = inner_x + 305;
    let limatch_w = 34;
    let limatch_h = 214;
    graph.text("LI-MATCH",
               limatch_x + limatch_w / 2, inner_y - 1, TextAnchor::s(), font,
               palette.rgb[palette.bl]);
    let limatch = LiMatchGreyscaleWidget::new(limatch_w, limatch_h);
    limatch.render(&mut graph, cacher, &palette, &ill, font, limatch_x, inner_y + 1);

    let isocubes_x = inner_x + 355;
    let isocubes_ww = 80;
    let isocubes_dx = 7;
    graph.text("CAM16UCS COLOURSPACE",
               isocubes_x + isocubes_ww + isocubes_dx / 2, inner_y - 1, TextAnchor::s(), font,
               palette.rgb[palette.bl]);
    let isocubes = CAM16IsoCubesWidget::new(isocubes_ww, isocubes_dx);
    isocubes.render(&mut graph, cacher, &palette, &ill, font, isocubes_x, inner_y + 1);

    let chrlihue_x = inner_x + 352;
    let chrlihue_y = inner_y + 96;
    let chrlihue_w1 = 46;
    let chrlihue_hh1 = 37;
    let chrlihue_w2 = 130;
    let chrlihue_h2 = 109;
    let chrlihue = ChromaLightnessHueWidget::new(chrlihue_w1, chrlihue_hh1, chrlihue_w2, chrlihue_h2);
    chrlihue.render(&mut graph, cacher, &palette, &ill, font, chrlihue_x, chrlihue_y);

    let comps_x = inner_x + 533;
    let mut comps_y = inner_y + 7;
    let comps_w = 74;
    let mut comps_h = inner_h - 9;
    let mut comps_ty = inner_y + 3;

    if palette.n <= 64 {
        comps_y = inner_y + 82;
        comps_h = inner_h - 83;
        comps_ty = inner_y + 83;

        let mixes_x = inner_x + 533;
        let mixes_xn = 7;
        let mixes_yn = 7;
        let mixes_ww = 10;
        let mixes_hh = 9;
        graph.vtext("USEFUL MIXES",
                    inner_x + inner_w + 5, inner_y + 1,
                    HorizontalTextAnchor::Center, font,
                    palette.rgb[palette.bl]);
        let mixes = UsefulMixesWidget::new(mixes_xn, mixes_yn, mixes_ww, mixes_hh);
        mixes.render(&mut graph, cacher, &palette, &ill, font, mixes_x, inner_y + 1);
    }

    graph.vtext(
        "LIGHTNESS & CHROMA",
        inner_x + inner_w + 5, comps_ty,
        HorizontalTextAnchor::Center, font,
        palette.rgb[palette.bl]
    );
    let comps = LightnessChromaComponentsWidget::new(comps_w, comps_h);
    comps.render(&mut graph, cacher, &palette, &ill, font, comps_x, comps_y);

    let mainpal_y = inner_y + 234;
    let mainpal_w = 512;
    let mainpal_h = 10;
    graph.text("PAL",
               inner_x - 1, mainpal_y + 2, TextAnchor::ne(), font,
               palette.rgb[palette.bl]);
    let mainpal = MainPaletteWidget::new(mainpal_w, mainpal_h);
    mainpal.render(&mut graph, cacher, &palette, &ill, font, inner_x + 1, mainpal_y);

    if palette.n <= 64 {
        let neu_y = inner_y + 220;
        let neu_w = 512;
        let neu_h1 = 6;
        let neu_h2 = 7;
        graph.text("NEU",
                   inner_x - 1, neu_y, TextAnchor::ne(), font,
                   palette.rgb[palette.bl]);
        graph.text("GREY",
                   inner_x - 1, neu_y + 7, TextAnchor::ne(), font,
                   palette.rgb[palette.bl]);
        let neu = NeutralisersWidget::new(neu_w, neu_h1, neu_h2);
        neu.render(&mut graph, cacher, &palette, &ill, font, inner_x + 1, neu_y);
    }

    let rgb12bit_y = inner_y + 256;
    graph.text("12 BIT RGB",
               inner_x + 1, rgb12bit_y - 1, TextAnchor::sw(), font,
               palette.rgb[palette.fg]);
    let rgb12bit = RGB12BitWidget {};
    rgb12bit.render(&mut graph, cacher, &palette, &ill, font, inner_x + 1, rgb12bit_y);

    let huechroma_x = inner_x + 8;
    let huechroma_y = inner_y + 294;
    let huechroma_d = 104;
    graph.text("POLAR HUE-CHROMA",
               huechroma_x + huechroma_d / 2, inner_y + inner_h + 1, TextAnchor::n(), font,
               palette.rgb[palette.bl]);
    let huechroma = HueChromaPolarWidget::new(huechroma_d);
    huechroma.render(&mut graph, cacher, &palette, &ill, font, huechroma_x, huechroma_y);

    let hueli_x = inner_x + 137;
    let hueli_y = inner_y + 251;
    let hueli_C_low = 10.;
    let hueli_C_high = 50.;
    let hueli_d_small = 60;
    let hueli_d_big = 90;
    graph.text("POLAR HUE-LIGHTNESS",
               hueli_x + (hueli_d_small + hueli_d_big) / 2, inner_y + inner_h + 1,
               TextAnchor::n(), font, palette.rgb[palette.bl]);
    let hueli = HueLightnessPolarFilledGroupWidget::new(
        hueli_C_low, hueli_C_high, hueli_d_small, hueli_d_big
    );
    hueli.render(&mut graph, cacher, &palette, &ill, font, hueli_x, hueli_y);

    let comp_x = inner_x + 296;
    let comp_y = inner_y + 256;
    let comp_d = 70;
    let comp_dx = 2;
    let comp_dy = 9;
    let comp_C = 42.;
    graph.text("COMPLEMENTARIES/DESATURATION",
               comp_x + (comp_d * 3 + comp_dx * 2) / 2, inner_y + inner_h + 1,
               TextAnchor::n(), font, palette.rgb[palette.bl]);
    let comp_hues = [
        (0. * PI / 6., "purple/seaweed"),
        (1. * PI / 6., "red/cyan"),
        (2. * PI / 6., "orange/blue"),
        (3. * PI / 6., "olive/ultramarine"),
        (4. * PI / 6., "lime/violet"),
        (5. * PI / 6., "emerald/rose")
    ];
    let comp_data: Vec<_> = comp_hues.iter()
        .map(|&(a, _)| (comp_C * a.cos(), comp_C * a.sin()))
        .collect();
    for yi in 0..2 {
        for xi in 0..3 {
            let i = yi * 3 + xi;
            let (a, b) = comp_data[i];
            let title = comp_hues[i].1;
            let x = comp_x + (comp_d + comp_dx) * xi as i32;
            let y = comp_y + (comp_d + comp_dy) * yi as i32;
            graph.text(title,
                       x, y - 7,
                       TextAnchor::nw(), font, palette.rgb[palette.fg]);
            let comp = ComplementariesWidget::new(a, b, comp_d, comp_d);
            comp.render(&mut graph, cacher, &palette, &ill, font, x, y);
        }
    }

    eprintln!("Saving...");
    graph.save(fname).unwrap();
}
