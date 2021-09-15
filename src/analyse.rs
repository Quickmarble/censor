use crossbeam_channel::{Receiver, Sender};

use crate::colour::*;
use crate::palette::*;
use crate::loader::LoadedPalette;
use crate::text::*;
use crate::graph::*;
use crate::cache::*;
use crate::widget::*;
use crate::metadata;

use std::f32::consts::PI;
use std::sync::{Arc, RwLock};
use std::rc::Rc;

pub fn analyse_multithreaded(
            colours: &LoadedPalette, T: f32,
            cp_req_send: Sender<()>, cp_recv: Receiver<MultithreadedCacheProvider>,
            font: Arc<Font>, grey_ui: bool,
            fname: String, verbose: bool) {
    use crossbeam_utils::thread;
    if verbose { eprintln!("Starting analysis."); }
    let ill = CAT16Illuminant::new(CIExy::from_T(T));
    let palette = Palette::new(colours.colours.clone(), &ill, grey_ui);

    let w: i32 = 640;
    let h: i32 = 432;

    let mut graph = ImageGraph::new(w as u32, h as u32);
    if let Some(ref profile) = colours.icc_profile {
        graph = graph.with_icc_profile(profile.clone());
    }
    graph.block(0, 0, w, h, palette.bg_rgb);

    let inner_x = 17;
    let inner_y = 16;
    let inner_w = 610;
    let inner_h = 406;

    graph.block(inner_x, inner_y, inner_w, inner_h, palette.bl_rgb);

    graph.text(&format!("= CENSOR v{} - PALETTE ANALYSER =", metadata::VERSION),
               w / 2, 2, TextAnchor::n(), font.as_ref(),
               palette.tl_rgb);
    graph.text(&format!("Unique colours in palette: {}", palette.n),
               2, 2, TextAnchor::nw(), font.as_ref(),
               palette.tl_rgb);
    graph.text("Colour difference: CAM16UCS",
               w - 2, 2, TextAnchor::ne(), font.as_ref(),
               palette.tl_rgb);
    graph.text(&format!("Illuminant: D(T={:.2}°K)", T),
               w - 2, 9, TextAnchor::ne(), font.as_ref(),
               palette.tl_rgb);
    graph.text(metadata::REPO,
               w - 3, h - 2, TextAnchor::se(), font.as_ref(),
               palette.tl_rgb);

    fn init_state<'a, T: GraphPixel>(
                graph: &'a mut ImageGraph,
                cp_req_send: Sender<()>, cp_recv: Receiver<MultithreadedCacheProvider>,
                palette: Arc<Palette>,
                ill: Arc<CAT16Illuminant>,
                font: Arc<Font>
                ) -> (
                    Vec<Box<dyn FnOnce()+Send>>,
                    GraphHoster<'a, T>,
                    Sender<()>, Receiver<MultithreadedCacheProvider>,
                    Arc<Palette>,
                    Arc<CAT16Illuminant>, Arc<Font>) {
        (
            vec![], GraphHoster::new(graph, palette.as_ref().clone(),
            font.as_ref().clone()), cp_req_send, cp_recv, palette, ill, font
        )
    }
    fn run_multithreaded<'a, T: GraphPixel+'a>(
                mut state: (
                    Vec<Box<dyn FnOnce()+'a+Send>>,
                    GraphHoster<'a, T>,
                    Sender<()>, Receiver<MultithreadedCacheProvider>,
                    Arc<Palette>, Arc<CAT16Illuminant>, Arc<Font>
                ),
                mut f: Box<dyn FnMut(
                    Arc<RwLock<MultithreadedGraphProvider<T>>>,
                    Arc<RwLock<MultithreadedCacheProvider>>,
                    Arc<Palette>, Arc<CAT16Illuminant>, Arc<Font>
                )+Send>) -> (
                    Vec<Box<dyn FnOnce()+'a+Send>>,
                    GraphHoster<'a, T>,
                    Sender<()>, Receiver<MultithreadedCacheProvider>,
                    Arc<Palette>, Arc<CAT16Illuminant>, Arc<Font>
                ) {
        let graph_sender = state.1.register();
        let graph_provider = MultithreadedGraphProvider::new(graph_sender);
        state.2.send(()).unwrap();
        let cache_provider = match state.3.recv() {
            Ok(x) => { x }
            Err(e) => {
                panic!("{}", e);
            }
        };
        let palette = state.4.clone();
        let ill = state.5.clone();
        let font = state.6.clone();
        let g = move || {
            f(
                Arc::new(RwLock::new(graph_provider)),
                Arc::new(RwLock::new(cache_provider)),
                palette, ill, font
            );
        };
        state.0.push(Box::new(g));
        return state;
    }
    fn run_all<'a, T: GraphPixel>(
                state: (
                    Vec<Box<dyn FnOnce()+'a+Send>>,
                    GraphHoster<'a, T>,
                    Sender<()>, Receiver<MultithreadedCacheProvider>,
                    Arc<Palette>, Arc<CAT16Illuminant>, Arc<Font>
                )) {
        let mut hoster = state.1;
        let funcs = state.0;
        std::mem::drop(state.2);
        thread::scope(|s| {
            for f in funcs {
                s.spawn(move |_| { f(); });
            }
            s.spawn(move |_| { hoster.process() });
        }).unwrap();
    }

    analyse_main(
        palette.n,
        ||{init_state(&mut graph, cp_req_send, cp_recv, Arc::new(palette), Arc::new(ill), font)},
        run_multithreaded,
        run_all,
        verbose
    );

    if verbose { eprintln!("Saving..."); }
    graph.save(fname).unwrap();
}

pub fn analyse_singlethreaded<CP: CacheProvider, C: AsRef<RwLock<CP>>+Clone, FR: AsRef<Font>+Clone>(
            colours: &LoadedPalette, T: f32,
            cache: C, font: FR, grey_ui: bool,
            fname: String, verbose: bool) {
    if verbose { eprintln!("Starting analysis."); }
    let ill = CAT16Illuminant::new(CIExy::from_T(T));
    let palette = Palette::new(colours.colours.clone(), &ill, grey_ui);

    let w: i32 = 640;
    let h: i32 = 432;

    let mut graph = ImageGraph::new(w as u32, h as u32);
    if let Some(ref profile) = colours.icc_profile {
        graph = graph.with_icc_profile(profile.clone());
    }
    graph.block(0, 0, w, h, palette.bg_rgb);

    let inner_x = 17;
    let inner_y = 16;
    let inner_w = 610;
    let inner_h = 406;

    graph.block(inner_x, inner_y, inner_w, inner_h, palette.bl_rgb);

    graph.text(&format!("= CENSOR v{} - PALETTE ANALYSER =", metadata::VERSION),
               w / 2, 2, TextAnchor::n(), font.as_ref(),
               palette.tl_rgb);
    graph.text(&format!("Unique colours in palette: {}", palette.n),
               2, 2, TextAnchor::nw(), font.as_ref(),
               palette.tl_rgb);
    graph.text("Colour difference: CAM16UCS",
               w - 2, 2, TextAnchor::ne(), font.as_ref(),
               palette.tl_rgb);
    graph.text(&format!("Illuminant: D(T={:.2}°K)", T),
               w - 2, 9, TextAnchor::ne(), font.as_ref(),
               palette.tl_rgb);
    graph.text(metadata::REPO,
               w - 3, h - 2, TextAnchor::se(), font.as_ref(),
               palette.tl_rgb);

    let graph_rw = Rc::new(RwLock::new(graph));
    analyse_main(
        palette.n,
        ||{(graph_rw.clone(), cache, Rc::new(palette), Rc::new(ill), font)},
        just_run,
        |_|{},
        verbose
    );

    if verbose { eprintln!("Saving..."); }
    graph_rw.write().unwrap().save(fname).unwrap();
}

fn just_run<CP: CacheProvider, C: AsRef<RwLock<CP>>+Clone, FR: AsRef<Font>+Clone>(
            state: (Rc<RwLock<ImageGraph>>, C, Rc<Palette>, Rc<CAT16Illuminant>, FR),
            mut f: Box<dyn FnMut(Rc<RwLock<ImageGraph>>, C, Rc<Palette>, Rc<CAT16Illuminant>, FR)+Send>)
                -> (Rc<RwLock<ImageGraph>>, C, Rc<Palette>, Rc<CAT16Illuminant>, FR) {
    f(
        state.0.clone(),
        state.1.clone(),
        state.2.clone(),
        state.3.clone(),
        state.4.clone()
    );
    return state;
}

trait ComputationInit<S>: FnOnce() -> S {}
impl<S, T: FnOnce() -> S> ComputationInit<S> for T {}

trait ComputationWrapper
    <S, GP: GraphProvider<RGB255>, G: AsRef<RwLock<GP>>+Clone,
    CP: CacheProvider, C: AsRef<RwLock<CP>>, PR: AsRef<Palette>+Clone,
    I: AsRef<CAT16Illuminant>, F: AsRef<Font>+Clone>:
            Fn(S, Box<dyn FnMut(G, C, PR, I, F)+Send>)->S {}
impl<S, GP: GraphProvider<RGB255>, G: AsRef<RwLock<GP>>+Clone,
    CP: CacheProvider, C: AsRef<RwLock<CP>>, PR: AsRef<Palette>+Clone,
    I: AsRef<CAT16Illuminant>, F: AsRef<Font>+Clone,
    T: Fn(S, Box<dyn FnMut(G, C, PR, I, F)+Send>)->S>
            ComputationWrapper<S, GP, G, CP, C, PR, I, F> for T {}

trait ComputationEnd<S>: Fn(S) {}
impl<S, T: Fn(S)> ComputationEnd<S> for T {}

fn analyse_main
    <CP: CacheProvider, C: AsRef<RwLock<CP>>+Clone, GP: GraphProvider<RGB255>, G: AsRef<RwLock<GP>>+Clone,
    PR: AsRef<Palette>+Clone, I: AsRef<CAT16Illuminant>+Clone, F: AsRef<Font>+Clone,
    S, CI: ComputationInit<S>, CW: ComputationWrapper<S, GP, G, CP, C, PR, I, F>, CE: ComputationEnd<S>>(
            palette_n: usize,
            init: CI, compute: CW, end: CE,
            _verbose: bool) {

    let mut state: S = init();

    let _w: i32 = 640;
    let _h: i32 = 432;

    let inner_x = 17;
    let inner_y = 16;
    let inner_w = 610;
    let inner_h = 406;

    let rect_JCh_w = 99;
    let rect_JCh_h = 96;
    let rect_JCh_C = [40, 10];
    for i in 0..rect_JCh_C.len() {
        let C = rect_JCh_C[i];
        let x = inner_x + 1 + (rect_JCh_w + 2) * i as i32;
        let y = inner_y + 1;
        state = compute(state, Box::new(move |graph, cache, palette, ill, font| {
            graph.as_ref().write().unwrap().text(&format!("CHROMA: {}", C),
                x, inner_y - 1, TextAnchor::sw(), font.as_ref(),
                palette.as_ref().bl_rgb);
            let rect_JCh = RectJChWidget::new(rect_JCh_w, rect_JCh_h, C as f32);
            rect_JCh.render(graph, cache, palette, ill, font, x, y);
        }));
    }

    let spectrum_w = 200;
    let spectrum_h = 6;
    let spectrum_y = inner_y + 100;
    state = compute(state, Box::new(move |graph, cache, palette, ill, font| {
        graph.as_ref().write().unwrap().text("SPEC",
            inner_x - 1, spectrum_y + 1, TextAnchor::ne(), font.as_ref(),
            palette.as_ref().bl_rgb);
        graph.as_ref().write().unwrap().text("C50%",
            inner_x - 1, spectrum_y + 1 + (spectrum_h + 1), TextAnchor::ne(), font.as_ref(),
            palette.as_ref().bl_rgb);
        graph.as_ref().write().unwrap().text("J50%",
            inner_x - 1, spectrum_y + 1 + (spectrum_h + 1) * 2, TextAnchor::ne(), font.as_ref(),
            palette.as_ref().bl_rgb);
        let spectrum = SpectrumWidget::new(spectrum_w, spectrum_h);
        spectrum.render(graph, cache, palette, ill, font, inner_x + 1, spectrum_y);
    }));

    let spectrobox_y = inner_y + 123;
    let spectrobox_w = 200;
    let spectrobox_h = 92;
    state = compute(state, Box::new(move |graph, cache, palette, ill, font| {
        graph.as_ref().write().unwrap().text("SPEC",
            inner_x - 1, spectrobox_y + 1 + spectrobox_h / 2 - 3, TextAnchor::e(), font.as_ref(),
            palette.as_ref().bl_rgb);
        graph.as_ref().write().unwrap().text("BOX",
            inner_x - 1, spectrobox_y + 1 + spectrobox_h / 2 + 3, TextAnchor::e(), font.as_ref(),
            palette.as_ref().bl_rgb);
        let spectrobox = SpectroBoxWidget::new(spectrobox_w, spectrobox_h);
        spectrobox.render(graph, cache, palette, ill, font, inner_x + 1, spectrobox_y);
    }));

    let indexed_x = inner_x + 203;
    let indexed_y = inner_y + 1;
    state = compute(state, Box::new(move |graph, cache, palette, ill, font| {
        graph.as_ref().write().unwrap().text("INDEXED PALETTE",
            indexed_x, inner_y - 1, TextAnchor::sw(), font.as_ref(),
            palette.as_ref().bl_rgb);
        let indexed = IndexedWidget::new(32, 8, 3, 4);
        indexed.render(graph, cache, palette, ill, font, indexed_x, indexed_y);
    }));

    let close_n = 10;
    let close_d = 9;
    let close_x = inner_x + 203;
    let close_w = (close_d + 1) * close_n - 1;
    let close10_y = inner_y + 45;
    state = compute(state, Box::new(move |graph, cache, palette, ill, font| {
        graph.as_ref().write().unwrap().text("close cols: 10% li-match",
            close_x + close_w / 2, close10_y, TextAnchor::s(), font.as_ref(),
            palette.as_ref().fg_rgb);
        let close10 = CloseLiMatchWidget::new(close_d, close_d, close_n as usize, 0.1);
        close10.render(graph, cache, palette, ill, font, close_x, close10_y);
    }));

    let close70_y = close10_y + close_d * 2 + 2;
    state = compute(state, Box::new(move |graph, cache, palette, ill, font| {
        graph.as_ref().write().unwrap().text("close cols: 70% li-match",
            close_x + close_w / 2, close70_y + close_d * 2 + 1, TextAnchor::n(), font.as_ref(),
            palette.as_ref().fg_rgb);
        let close70 = CloseLiMatchWidget::new(close_d, close_d, close_n as usize, 0.7);
        close70.render(graph, cache, palette, ill, font, close_x, close70_y);
    }));

    let iss_x = inner_x + 203;
    let iss_y = inner_y + 92;
    let iss_w = 44;
    let iss_h = 24;
    let iss = ISSWidget::new(iss_w, iss_h, 2., 3.5);
    state = compute(state, Box::new(move |graph, cache, palette, ill, font| {
        iss.render(graph, cache, palette, ill, font, iss_x, iss_y);
    }));

    let acyclic_x = inner_x + 259;
    let acyclic_y = inner_y + 92;
    let acyclic_w = 44;
    let acyclic_h = 24;
    let acyclic = AcyclicWidget::new(acyclic_w, acyclic_h);
    state = compute(state, Box::new(move |graph, cache, palette, ill, font| {
        acyclic.render(graph, cache, palette, ill, font, acyclic_x, acyclic_y);
    }));

    let sdist_x = inner_x + 203;
    let sdist_y = inner_y + 124;
    let sdist_w = 100;
    let sdist_h = 36;
    let sdist = SpectralDistributionWidget::new(sdist_w, sdist_h);
    state = compute(state, Box::new(move |graph, cache, palette, ill, font| {
        sdist.render(graph.clone(), cache, palette.clone(), ill, font.clone(), sdist_x, sdist_y);
        graph.as_ref().write().unwrap().text("spectral distribution",
            sdist_x + sdist_w / 2, sdist_y, TextAnchor::s(), font.as_ref(),
            palette.as_ref().fg_rgb);
    }));

    let tdist_x = inner_x + 203;
    let tdist_y = inner_y + 170;
    let tdist_w = 100;
    let tdist_h = 36;
    let tdist = TemperatureDistributionWidget::new(tdist_w, tdist_h);
    state = compute(state, Box::new(move |graph, cache, palette, ill, font| {
        tdist.render(graph.clone(), cache, palette.clone(), ill, font.clone(), tdist_x, tdist_y);
        graph.as_ref().write().unwrap().text("temperature",
            tdist_x + tdist_w / 2, tdist_y - 1, TextAnchor::s(), font.as_ref(),
            palette.as_ref().fg_rgb);
    }));

    let limatch_x = inner_x + 305;
    let limatch_w = 34;
    let limatch_h = 214;
    state = compute(state, Box::new(move |graph, cache, palette, ill, font| {
        graph.as_ref().write().unwrap().text("LI-MATCH",
            limatch_x + limatch_w / 2, inner_y - 1, TextAnchor::s(), font.as_ref(),
           palette.as_ref().bl_rgb);
        let limatch = LiMatchGreyscaleWidget::new(limatch_w, limatch_h);
        limatch.render(graph, cache, palette, ill, font, limatch_x, inner_y + 1);
    }));

    let isocubes_x = inner_x + 355;
    let isocubes_ww = 80;
    let isocubes_dx = 7;
    state = compute(state, Box::new(move |graph, cache, palette, ill, font| {
        graph.as_ref().write().unwrap().text("CAM16UCS COLOURSPACE",
            isocubes_x + isocubes_ww + isocubes_dx / 2, inner_y - 1, TextAnchor::s(), font.as_ref(),
            palette.as_ref().bl_rgb);
        let isocubes = CAM16IsoCubesWidget::new(isocubes_ww, isocubes_dx);
        isocubes.render(graph, cache, palette, ill, font, isocubes_x, inner_y + 1);
    }));

    let chrlihue_x = inner_x + 352;
    let chrlihue_y = inner_y + 96;
    let chrlihue_w1 = 46;
    let chrlihue_hh1 = 37;
    let chrlihue_w2 = 130;
    let chrlihue_h2 = 109;
    let chrlihue = ChromaLightnessHueWidget::new(chrlihue_w1, chrlihue_hh1, chrlihue_w2, chrlihue_h2);
    state = compute(state, Box::new(move |graph, cache, palette, ill, font| {
        chrlihue.render(graph, cache, palette, ill, font, chrlihue_x, chrlihue_y);
    }));

    let comps_x = inner_x + 533;
    let mut comps_y = inner_y + 7;
    let comps_w = 74;
    let mut comps_h = inner_h - 9;
    let mut comps_ty = inner_y + 3;

    if palette_n <= 64 {
        comps_y = inner_y + 82;
        comps_h = inner_h - 83;
        comps_ty = inner_y + 83;

        let mixes_x = inner_x + 533;
        let mixes_xn = 7;
        let mixes_yn = 7;
        let mixes_ww = 10;
        let mixes_hh = 9;
        state = compute(state, Box::new(move |graph, cache, palette, ill, font| {
            graph.as_ref().write().unwrap().vtext("USEFUL MIXES",
                inner_x + inner_w + 5, inner_y + 1,
                HorizontalTextAnchor::Center, font.as_ref(),
                palette.as_ref().bl_rgb);
            let mixes = UsefulMixesWidget::new(mixes_xn, mixes_yn, mixes_ww, mixes_hh);
            mixes.render(graph, cache, palette, ill, font, mixes_x, inner_y + 1);
        }));
    }

    state = compute(state, Box::new(move |graph, cache, palette, ill, font| {
        graph.as_ref().write().unwrap().vtext(
            "LIGHTNESS & CHROMA",
            inner_x + inner_w + 5, comps_ty,
            HorizontalTextAnchor::Center, font.as_ref(),
            palette.as_ref().bl_rgb
        );
        let comps = LightnessChromaComponentsWidget::new(comps_w, comps_h);
        comps.render(graph, cache, palette, ill, font, comps_x, comps_y);
    }));

    let mainpal_y = inner_y + 234;
    let mainpal_w = 512;
    let mainpal_h = 10;
    state = compute(state, Box::new(move |graph, cache, palette, ill, font| {
        graph.as_ref().write().unwrap().text("PAL",
            inner_x - 1, mainpal_y + 2, TextAnchor::ne(), font.as_ref(),
            palette.as_ref().bl_rgb);
        let mainpal = MainPaletteWidget::new(mainpal_w, mainpal_h);
        mainpal.render(graph, cache, palette, ill, font, inner_x + 1, mainpal_y);
    }));

    if palette_n <= 64 {
        let neu_y = inner_y + 220;
        let neu_w = 512;
        let neu_h1 = 6;
        let neu_h2 = 7;
        state = compute(state, Box::new(move |graph, cache, palette, ill, font| {
            graph.as_ref().write().unwrap().text("NEU",
                inner_x - 1, neu_y, TextAnchor::ne(), font.as_ref(),
                palette.as_ref().bl_rgb);
            graph.as_ref().write().unwrap().text("GREY",
                inner_x - 1, neu_y + 7, TextAnchor::ne(), font.as_ref(),
                palette.as_ref().bl_rgb);
            let neu = NeutralisersWidget::new(neu_w, neu_h1, neu_h2);
            neu.render(graph, cache, palette, ill, font, inner_x + 1, neu_y);
        }));
    }

    let rgb12bit_y = inner_y + 256;
    state = compute(state, Box::new(move |graph, cache, palette, ill, font| {
        graph.as_ref().write().unwrap().text("12 BIT RGB",
            inner_x + 1, rgb12bit_y - 1, TextAnchor::sw(), font.as_ref(),
            palette.as_ref().fg_rgb);
        let rgb12bit = RGB12BitWidget {};
        rgb12bit.render(graph, cache, palette, ill, font, inner_x + 1, rgb12bit_y);
    }));

    let huechroma_x = inner_x + 8;
    let huechroma_y = inner_y + 291;
    let huechroma_d = 105;
    state = compute(state, Box::new(move |graph, cache, palette, ill, font| {
        graph.as_ref().write().unwrap().text("POLAR HUE-CHROMA",
            huechroma_x + huechroma_d / 2, inner_y + inner_h + 1, TextAnchor::n(), font.as_ref(),
            palette.as_ref().bl_rgb);
        let huechroma = HueChromaPolarWidget::new(huechroma_d);
        huechroma.render(graph, cache, palette, ill, font, huechroma_x, huechroma_y);
    }));

    let hueli_x = inner_x + 137;
    let hueli_y = inner_y + 251;
    let hueli_C_low = 10.;
    let hueli_C_high = 50.;
    let hueli_d_small = 60;
    let hueli_d_big = 90;
    state = compute(state, Box::new(move |graph, cache, palette, ill, font| {
        graph.as_ref().write().unwrap().text("POLAR HUE-LIGHTNESS",
            hueli_x + (hueli_d_small + hueli_d_big) / 2, inner_y + inner_h + 1,
            TextAnchor::n(), font.as_ref(), palette.as_ref().bl_rgb);
        let hueli = HueLightnessPolarFilledGroupWidget::new(
            hueli_C_low, hueli_C_high, hueli_d_small, hueli_d_big
        );
        hueli.render(graph, cache, palette, ill, font, hueli_x, hueli_y);
    }));

    let comp_x = inner_x + 296;
    let comp_y = inner_y + 256;
    let comp_d = 70;
    let comp_dx = 2;
    let comp_dy = 9;
    let comp_C = 42.;
    state = compute(state, Box::new(move |graph, _cache, palette, _ill, font| {
        graph.as_ref().write().unwrap().text("COMPLEMENTARIES/DESATURATION",
            comp_x + (comp_d * 3 + comp_dx * 2) / 2, inner_y + inner_h + 1,
            TextAnchor::n(), font.as_ref(), palette.as_ref().bl_rgb);
    }));
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
            state = compute(state, Box::new(move |graph, cache, palette, ill, font| {
                graph.as_ref().write().unwrap().text(title,
                    x, y - 7,
                    TextAnchor::nw(), font.as_ref(), palette.as_ref().fg_rgb);
                let comp = ComplementariesWidget::new(a, b, comp_d, comp_d);
                comp.render(graph, cache, palette, ill, font, x, y);
            }));
        }
    }

    end(state);
}
