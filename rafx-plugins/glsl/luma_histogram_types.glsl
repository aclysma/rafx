
struct HistogramResult {
    float average_luminosity_interpolated;
    float average_luminosity_this_frame;
    float average_luminosity_last_frame;

    float min_luminosity_interpolated;
    float min_luminosity_this_frame;
    float min_luminosity_last_frame;

    float max_luminosity_interpolated;
    float max_luminosity_this_frame;
    float max_luminosity_last_frame;

    float low_luminosity_interpolated;
    float low_luminosity_this_frame;
    float low_luminosity_last_frame;

    float high_luminosity_interpolated;
    float high_luminosity_this_frame;
    float high_luminosity_last_frame;

    float average_bin_include_zero;
    float average_bin_non_zero;
    uint min_bin;
    uint max_bin;
    uint low_bin;
    uint high_bin;
};
