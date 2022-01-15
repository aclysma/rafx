use crate::pipelines::modern::JitterPattern;
lazy_static::lazy_static! {
    // From paper "Practical Hash-based Owen Scrambling"
    // https://jcgt.org/published/0009/04/01/paper.pdf
    // See also "Progressive Multi-Jittered Sample Sequences"
    // https://graphics.pixar.com/library/ProgressiveMultiJitteredSampling/paper.pdf
    //
    // The following is a 2D sequence generated using the genpoints program in the supplemental material for the first
    // paper
    //
    // For best results, use a 4^n number of samples
    pub static ref SOBOL_OWEN_JITTER_SAMPLES : [glam::Vec2; 64] = {[
        glam::Vec2::new(0.353826, 0.293137),
        glam::Vec2::new(0.880584, 0.730185),
        glam::Vec2::new(0.506151, 0.207731),
        glam::Vec2::new(0.13439, 0.964482),
        glam::Vec2::new(0.822768, 0.425304),
        glam::Vec2::new(0.390371, 0.569885),
        glam::Vec2::new(0.0428708, 0.0122574),
        glam::Vec2::new(0.674511, 0.87234),
        glam::Vec2::new(0.585205, 0.534357),
        glam::Vec2::new(0.191455, 0.463665),
        glam::Vec2::new(0.305637, 0.76955),
        glam::Vec2::new(0.948917, 0.091293),
        glam::Vec2::new(0.742843, 0.31442),
        glam::Vec2::new(0.116872, 0.64634),
        glam::Vec2::new(0.484685, 0.166215),
        glam::Vec2::new(0.798177, 0.906794),
        glam::Vec2::new(0.774677, 0.131658),
        glam::Vec2::new(0.458866, 0.884199),
        glam::Vec2::new(0.0791909, 0.350319),
        glam::Vec2::new(0.705774, 0.684359),
        glam::Vec2::new(0.270971, 0.120589),
        glam::Vec2::new(0.974779, 0.80992),
        glam::Vec2::new(0.615558, 0.497954),
        glam::Vec2::new(0.228014, 0.508024),
        glam::Vec2::new(0.326798, 0.717109),
        glam::Vec2::new(0.917997, 0.273814),
        glam::Vec2::new(0.543076, 0.980289),
        glam::Vec2::new(0.160187, 0.226163),
        glam::Vec2::new(0.0023893, 0.817498),
        glam::Vec2::new(0.6518, 0.0366651),
        glam::Vec2::new(0.431615, 0.378006),
        glam::Vec2::new(0.866239, 0.617835),
        glam::Vec2::new(0.559639, 0.303466),
        glam::Vec2::new(0.178365, 0.741463),
        glam::Vec2::new(0.338233, 0.199022),
        glam::Vec2::new(0.929716, 0.943479),
        glam::Vec2::new(0.0217765, 0.416646),
        glam::Vec2::new(0.638592, 0.589051),
        glam::Vec2::new(0.853046, 0.0259741),
        glam::Vec2::new(0.412387, 0.847859),
        glam::Vec2::new(0.0694994, 0.927955),
        glam::Vec2::new(0.689531, 0.181351),
        glam::Vec2::new(0.758812, 0.625168),
        glam::Vec2::new(0.438731, 0.335436),
        glam::Vec2::new(0.250606, 0.550724),
        glam::Vec2::new(0.989613, 0.449844),
        glam::Vec2::new(0.245932, 0.071281),
        glam::Vec2::new(0.596273, 0.755811),
        glam::Vec2::new(0.155275, 0.264629),
        glam::Vec2::new(0.517722, 0.701957),
        glam::Vec2::new(0.898325, 0.239684),
        glam::Vec2::new(0.374732, 0.99312),
        glam::Vec2::new(0.658978, 0.395027),
        glam::Vec2::new(0.055402, 0.607989),
        glam::Vec2::new(0.402262, 0.0532102),
        glam::Vec2::new(0.837034, 0.834392),
        glam::Vec2::new(0.732718, 0.901083),
        glam::Vec2::new(0.0954377, 0.142723),
        glam::Vec2::new(0.475437, 0.669633),
        glam::Vec2::new(0.786978, 0.360247),
        glam::Vec2::new(0.967937, 0.529673),
        glam::Vec2::new(0.289915, 0.477787),
        glam::Vec2::new(0.566065, 0.109029),
        glam::Vec2::new(0.203181, 0.79172)
    ]};

    // Classic halton(2, 3) that lots of people use
    // https://en.wikipedia.org/wiki/Halton_sequence
    //
    // TODO: Generate more points and skip the first handful of points in the sequence
    pub static ref HALTON_JITTER_SAMPLES : [glam::Vec2; 16] = {[
        glam::Vec2::new(0.500000, 0.333333),
        glam::Vec2::new(0.250000, 0.666667),
        glam::Vec2::new(0.750000, 0.111111),
        glam::Vec2::new(0.125000, 0.444444),
        glam::Vec2::new(0.625000, 0.777778),
        glam::Vec2::new(0.375000, 0.222222),
        glam::Vec2::new(0.875000, 0.555556),
        glam::Vec2::new(0.062500, 0.888889),
        glam::Vec2::new(0.562500, 0.037037),
        glam::Vec2::new(0.312500, 0.370370),
        glam::Vec2::new(0.812500, 0.703704),
        glam::Vec2::new(0.187500, 0.148148),
        glam::Vec2::new(0.687500, 0.481481),
        glam::Vec2::new(0.437500, 0.814815),
        glam::Vec2::new(0.937500, 0.259259),
        glam::Vec2::new(0.031250, 0.592593)
    ]};

    // silly test pattern
    pub static ref QUAD_JITTER_TEST_SAMPLES : [glam::Vec2; 4] = {[
        glam::Vec2::new(0.25, 0.25),
        glam::Vec2::new(0.25, 0.75),
        glam::Vec2::new(0.75, 0.75),
        glam::Vec2::new(0.75, 0.25)
    ]};
}

pub fn jitter_amount(
    frame_index: usize,
    pattern: JitterPattern,
    viewport_size: glam::Vec2,
) -> glam::Vec2 {
    let jitter_amount = match pattern {
        JitterPattern::SobolOwen16 => SOBOL_OWEN_JITTER_SAMPLES[frame_index % 16],
        JitterPattern::SobolOwen64 => SOBOL_OWEN_JITTER_SAMPLES[frame_index % 64],
        JitterPattern::Halton => HALTON_JITTER_SAMPLES[frame_index % HALTON_JITTER_SAMPLES.len()],
        JitterPattern::QuadJitterTest => {
            QUAD_JITTER_TEST_SAMPLES[frame_index % QUAD_JITTER_TEST_SAMPLES.len()]
        }
        JitterPattern::MAX => unimplemented!(),
    };

    return (jitter_amount * glam::Vec2::splat(2.0) - glam::Vec2::ONE) / viewport_size;
}
