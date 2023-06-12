struct HistogramResult
{
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

static const uint3 gl_WorkGroupSize = uint3(256u, 1u, 1u);

RWByteAddressBuffer histogram_data : register(u0, space0);
cbuffer AverageHistogramConfig : register(b1, space0)
{
    uint config_pixel_count : packoffset(c0);
    float config_min_log_luma : packoffset(c0.y);
    float config_log_luma_range : packoffset(c0.z);
    float config_dt : packoffset(c0.w);
    float config_low_percentile : packoffset(c1);
    float config_high_percentile : packoffset(c1.y);
    float config_low_adjust_speed : packoffset(c1.z);
    float config_high_adjust_speed : packoffset(c1.w);
    uint config_write_debug_output : packoffset(c2);
};

RWByteAddressBuffer histogram_result : register(u2, space0);
RWByteAddressBuffer debug_output : register(u3, space0);

static uint gl_LocalInvocationIndex;
struct SPIRV_Cross_Input
{
    uint gl_LocalInvocationIndex : SV_GroupIndex;
};

groupshared float HistogramShared[256];

float bin_to_luminosity(float bin, float min_log_luma, float log_luma_range)
{
    return exp2(((bin / 255.0f) * log_luma_range) + min_log_luma);
}

void comp_main()
{
    float count_for_this_bin = float(histogram_data.Load(gl_LocalInvocationIndex * 4 + 0));
    HistogramShared[gl_LocalInvocationIndex] = count_for_this_bin * float(gl_LocalInvocationIndex + 1u);
    GroupMemoryBarrierWithGroupSync();
    uint histogram_sample_index = 128u;
    for (;;)
    {
        if (histogram_sample_index > 0u)
        {
            if (gl_LocalInvocationIndex < histogram_sample_index)
            {
                HistogramShared[gl_LocalInvocationIndex] += HistogramShared[gl_LocalInvocationIndex + histogram_sample_index];
            }
            GroupMemoryBarrierWithGroupSync();
            histogram_sample_index = histogram_sample_index >> uint(1);
            continue;
        }
        else
        {
            break;
        }
    }
    if (gl_LocalInvocationIndex == 0u)
    {
        uint zero_pixel_count = uint(count_for_this_bin);
        uint max_bin = 0u;
        uint min_bin = 255u;
        uint pixels_seen = 0u;
        uint high_bin = 0u;
        uint high_pixel_thresh = uint((1.0f - config_high_percentile) * float(config_pixel_count - zero_pixel_count));
        uint low_bin = 0u;
        uint low_pixel_thresh = uint((1.0f - config_low_percentile) * float(config_pixel_count - zero_pixel_count));
        int i = 255;
        for (;;)
        {
            if (i >= 0)
            {
                uint data = histogram_data.Load(i * 4 + 0);
                if (data > 0u)
                {
                    max_bin = max(max_bin, uint(i));
                    min_bin = uint(i);
                }
                pixels_seen += data;
                if (pixels_seen > high_pixel_thresh)
                {
                    high_bin = max(high_bin, uint(i));
                }
                if (pixels_seen > low_pixel_thresh)
                {
                    low_bin = max(low_bin, uint(i));
                }
                i--;
                continue;
            }
            else
            {
                break;
            }
        }
        float average_bin_include_zero = (HistogramShared[0] / float(config_pixel_count)) - 1.0f;
        float non_zero_pixel_count = float(config_pixel_count) - float(zero_pixel_count);
        float average_bin_non_zero = (HistogramShared[0] - non_zero_pixel_count) / max(non_zero_pixel_count, 1.0f);
        float average_bin = average_bin_include_zero;
        histogram_result.Store(8, asuint(asfloat(histogram_result.Load(0))));
        histogram_result.Store(20, asuint(asfloat(histogram_result.Load(12))));
        histogram_result.Store(32, asuint(asfloat(histogram_result.Load(24))));
        histogram_result.Store(44, asuint(asfloat(histogram_result.Load(36))));
        histogram_result.Store(56, asuint(asfloat(histogram_result.Load(48))));
        float param = average_bin;
        float param_1 = config_min_log_luma;
        float param_2 = config_log_luma_range;
        histogram_result.Store(4, asuint(bin_to_luminosity(param, param_1, param_2)));
        float param_3 = float(min_bin);
        float param_4 = config_min_log_luma;
        float param_5 = config_log_luma_range;
        histogram_result.Store(16, asuint(bin_to_luminosity(param_3, param_4, param_5)));
        float param_6 = float(max_bin);
        float param_7 = config_min_log_luma;
        float param_8 = config_log_luma_range;
        histogram_result.Store(28, asuint(bin_to_luminosity(param_6, param_7, param_8)));
        float param_9 = float(low_bin);
        float param_10 = config_min_log_luma;
        float param_11 = config_log_luma_range;
        histogram_result.Store(40, asuint(bin_to_luminosity(param_9, param_10, param_11)));
        float param_12 = float(high_bin);
        float param_13 = config_min_log_luma;
        float param_14 = config_log_luma_range;
        histogram_result.Store(52, asuint(bin_to_luminosity(param_12, param_13, param_14)));
        float interp_high = clamp(1.0f - exp((-config_high_adjust_speed) * config_dt), 0.0f, 1.0f);
        float interp_low = clamp(1.0f - exp((-config_low_adjust_speed) * config_dt), 0.0f, 1.0f);
        histogram_result.Store(0, asuint(lerp(asfloat(histogram_result.Load(0)), asfloat(histogram_result.Load(4)), (asfloat(histogram_result.Load(0)) < asfloat(histogram_result.Load(4))) ? interp_high : interp_low)));
        histogram_result.Store(12, asuint(lerp(asfloat(histogram_result.Load(12)), asfloat(histogram_result.Load(16)), (asfloat(histogram_result.Load(12)) < asfloat(histogram_result.Load(16))) ? interp_high : interp_low)));
        histogram_result.Store(24, asuint(lerp(asfloat(histogram_result.Load(24)), asfloat(histogram_result.Load(28)), (asfloat(histogram_result.Load(24)) < asfloat(histogram_result.Load(28))) ? interp_high : interp_low)));
        histogram_result.Store(36, asuint(lerp(asfloat(histogram_result.Load(36)), asfloat(histogram_result.Load(40)), (asfloat(histogram_result.Load(36)) < asfloat(histogram_result.Load(40))) ? interp_high : interp_low)));
        histogram_result.Store(48, asuint(lerp(asfloat(histogram_result.Load(48)), asfloat(histogram_result.Load(52)), (asfloat(histogram_result.Load(48)) < asfloat(histogram_result.Load(52))) ? interp_high : interp_low)));
        histogram_result.Store(36, asuint(max(asfloat(histogram_result.Load(12)), asfloat(histogram_result.Load(36)))));
        histogram_result.Store(48, asuint(max(asfloat(histogram_result.Load(36)), asfloat(histogram_result.Load(48)))));
        histogram_result.Store(24, asuint(max(asfloat(histogram_result.Load(48)), asfloat(histogram_result.Load(24)))));
        histogram_result.Store(60, asuint(average_bin_include_zero));
        histogram_result.Store(64, asuint(average_bin_non_zero));
        histogram_result.Store(68, min_bin);
        histogram_result.Store(72, max_bin);
        histogram_result.Store(76, low_bin);
        histogram_result.Store(80, high_bin);
        if (config_write_debug_output != 0u)
        {
            HistogramResult _422;
            _422.average_luminosity_interpolated = asfloat(histogram_result.Load(0));
            _422.average_luminosity_this_frame = asfloat(histogram_result.Load(4));
            _422.average_luminosity_last_frame = asfloat(histogram_result.Load(8));
            _422.min_luminosity_interpolated = asfloat(histogram_result.Load(12));
            _422.min_luminosity_this_frame = asfloat(histogram_result.Load(16));
            _422.min_luminosity_last_frame = asfloat(histogram_result.Load(20));
            _422.max_luminosity_interpolated = asfloat(histogram_result.Load(24));
            _422.max_luminosity_this_frame = asfloat(histogram_result.Load(28));
            _422.max_luminosity_last_frame = asfloat(histogram_result.Load(32));
            _422.low_luminosity_interpolated = asfloat(histogram_result.Load(36));
            _422.low_luminosity_this_frame = asfloat(histogram_result.Load(40));
            _422.low_luminosity_last_frame = asfloat(histogram_result.Load(44));
            _422.high_luminosity_interpolated = asfloat(histogram_result.Load(48));
            _422.high_luminosity_this_frame = asfloat(histogram_result.Load(52));
            _422.high_luminosity_last_frame = asfloat(histogram_result.Load(56));
            _422.average_bin_include_zero = asfloat(histogram_result.Load(60));
            _422.average_bin_non_zero = asfloat(histogram_result.Load(64));
            _422.min_bin = histogram_result.Load(68);
            _422.max_bin = histogram_result.Load(72);
            _422.low_bin = histogram_result.Load(76);
            _422.high_bin = histogram_result.Load(80);
            debug_output.Store(0, asuint(_422.average_luminosity_interpolated));
            debug_output.Store(4, asuint(_422.average_luminosity_this_frame));
            debug_output.Store(8, asuint(_422.average_luminosity_last_frame));
            debug_output.Store(12, asuint(_422.min_luminosity_interpolated));
            debug_output.Store(16, asuint(_422.min_luminosity_this_frame));
            debug_output.Store(20, asuint(_422.min_luminosity_last_frame));
            debug_output.Store(24, asuint(_422.max_luminosity_interpolated));
            debug_output.Store(28, asuint(_422.max_luminosity_this_frame));
            debug_output.Store(32, asuint(_422.max_luminosity_last_frame));
            debug_output.Store(36, asuint(_422.low_luminosity_interpolated));
            debug_output.Store(40, asuint(_422.low_luminosity_this_frame));
            debug_output.Store(44, asuint(_422.low_luminosity_last_frame));
            debug_output.Store(48, asuint(_422.high_luminosity_interpolated));
            debug_output.Store(52, asuint(_422.high_luminosity_this_frame));
            debug_output.Store(56, asuint(_422.high_luminosity_last_frame));
            debug_output.Store(60, asuint(_422.average_bin_include_zero));
            debug_output.Store(64, asuint(_422.average_bin_non_zero));
            debug_output.Store(68, _422.min_bin);
            debug_output.Store(72, _422.max_bin);
            debug_output.Store(76, _422.low_bin);
            debug_output.Store(80, _422.high_bin);
            uint _426[256];
            [unroll]
            for (int _0ident = 0; _0ident < 256; _0ident++)
            {
                _426[_0ident] = histogram_data.Load(_0ident * 4 + 0);
            }
            debug_output.Store(84, _426[0]);
            debug_output.Store(88, _426[1]);
            debug_output.Store(92, _426[2]);
            debug_output.Store(96, _426[3]);
            debug_output.Store(100, _426[4]);
            debug_output.Store(104, _426[5]);
            debug_output.Store(108, _426[6]);
            debug_output.Store(112, _426[7]);
            debug_output.Store(116, _426[8]);
            debug_output.Store(120, _426[9]);
            debug_output.Store(124, _426[10]);
            debug_output.Store(128, _426[11]);
            debug_output.Store(132, _426[12]);
            debug_output.Store(136, _426[13]);
            debug_output.Store(140, _426[14]);
            debug_output.Store(144, _426[15]);
            debug_output.Store(148, _426[16]);
            debug_output.Store(152, _426[17]);
            debug_output.Store(156, _426[18]);
            debug_output.Store(160, _426[19]);
            debug_output.Store(164, _426[20]);
            debug_output.Store(168, _426[21]);
            debug_output.Store(172, _426[22]);
            debug_output.Store(176, _426[23]);
            debug_output.Store(180, _426[24]);
            debug_output.Store(184, _426[25]);
            debug_output.Store(188, _426[26]);
            debug_output.Store(192, _426[27]);
            debug_output.Store(196, _426[28]);
            debug_output.Store(200, _426[29]);
            debug_output.Store(204, _426[30]);
            debug_output.Store(208, _426[31]);
            debug_output.Store(212, _426[32]);
            debug_output.Store(216, _426[33]);
            debug_output.Store(220, _426[34]);
            debug_output.Store(224, _426[35]);
            debug_output.Store(228, _426[36]);
            debug_output.Store(232, _426[37]);
            debug_output.Store(236, _426[38]);
            debug_output.Store(240, _426[39]);
            debug_output.Store(244, _426[40]);
            debug_output.Store(248, _426[41]);
            debug_output.Store(252, _426[42]);
            debug_output.Store(256, _426[43]);
            debug_output.Store(260, _426[44]);
            debug_output.Store(264, _426[45]);
            debug_output.Store(268, _426[46]);
            debug_output.Store(272, _426[47]);
            debug_output.Store(276, _426[48]);
            debug_output.Store(280, _426[49]);
            debug_output.Store(284, _426[50]);
            debug_output.Store(288, _426[51]);
            debug_output.Store(292, _426[52]);
            debug_output.Store(296, _426[53]);
            debug_output.Store(300, _426[54]);
            debug_output.Store(304, _426[55]);
            debug_output.Store(308, _426[56]);
            debug_output.Store(312, _426[57]);
            debug_output.Store(316, _426[58]);
            debug_output.Store(320, _426[59]);
            debug_output.Store(324, _426[60]);
            debug_output.Store(328, _426[61]);
            debug_output.Store(332, _426[62]);
            debug_output.Store(336, _426[63]);
            debug_output.Store(340, _426[64]);
            debug_output.Store(344, _426[65]);
            debug_output.Store(348, _426[66]);
            debug_output.Store(352, _426[67]);
            debug_output.Store(356, _426[68]);
            debug_output.Store(360, _426[69]);
            debug_output.Store(364, _426[70]);
            debug_output.Store(368, _426[71]);
            debug_output.Store(372, _426[72]);
            debug_output.Store(376, _426[73]);
            debug_output.Store(380, _426[74]);
            debug_output.Store(384, _426[75]);
            debug_output.Store(388, _426[76]);
            debug_output.Store(392, _426[77]);
            debug_output.Store(396, _426[78]);
            debug_output.Store(400, _426[79]);
            debug_output.Store(404, _426[80]);
            debug_output.Store(408, _426[81]);
            debug_output.Store(412, _426[82]);
            debug_output.Store(416, _426[83]);
            debug_output.Store(420, _426[84]);
            debug_output.Store(424, _426[85]);
            debug_output.Store(428, _426[86]);
            debug_output.Store(432, _426[87]);
            debug_output.Store(436, _426[88]);
            debug_output.Store(440, _426[89]);
            debug_output.Store(444, _426[90]);
            debug_output.Store(448, _426[91]);
            debug_output.Store(452, _426[92]);
            debug_output.Store(456, _426[93]);
            debug_output.Store(460, _426[94]);
            debug_output.Store(464, _426[95]);
            debug_output.Store(468, _426[96]);
            debug_output.Store(472, _426[97]);
            debug_output.Store(476, _426[98]);
            debug_output.Store(480, _426[99]);
            debug_output.Store(484, _426[100]);
            debug_output.Store(488, _426[101]);
            debug_output.Store(492, _426[102]);
            debug_output.Store(496, _426[103]);
            debug_output.Store(500, _426[104]);
            debug_output.Store(504, _426[105]);
            debug_output.Store(508, _426[106]);
            debug_output.Store(512, _426[107]);
            debug_output.Store(516, _426[108]);
            debug_output.Store(520, _426[109]);
            debug_output.Store(524, _426[110]);
            debug_output.Store(528, _426[111]);
            debug_output.Store(532, _426[112]);
            debug_output.Store(536, _426[113]);
            debug_output.Store(540, _426[114]);
            debug_output.Store(544, _426[115]);
            debug_output.Store(548, _426[116]);
            debug_output.Store(552, _426[117]);
            debug_output.Store(556, _426[118]);
            debug_output.Store(560, _426[119]);
            debug_output.Store(564, _426[120]);
            debug_output.Store(568, _426[121]);
            debug_output.Store(572, _426[122]);
            debug_output.Store(576, _426[123]);
            debug_output.Store(580, _426[124]);
            debug_output.Store(584, _426[125]);
            debug_output.Store(588, _426[126]);
            debug_output.Store(592, _426[127]);
            debug_output.Store(596, _426[128]);
            debug_output.Store(600, _426[129]);
            debug_output.Store(604, _426[130]);
            debug_output.Store(608, _426[131]);
            debug_output.Store(612, _426[132]);
            debug_output.Store(616, _426[133]);
            debug_output.Store(620, _426[134]);
            debug_output.Store(624, _426[135]);
            debug_output.Store(628, _426[136]);
            debug_output.Store(632, _426[137]);
            debug_output.Store(636, _426[138]);
            debug_output.Store(640, _426[139]);
            debug_output.Store(644, _426[140]);
            debug_output.Store(648, _426[141]);
            debug_output.Store(652, _426[142]);
            debug_output.Store(656, _426[143]);
            debug_output.Store(660, _426[144]);
            debug_output.Store(664, _426[145]);
            debug_output.Store(668, _426[146]);
            debug_output.Store(672, _426[147]);
            debug_output.Store(676, _426[148]);
            debug_output.Store(680, _426[149]);
            debug_output.Store(684, _426[150]);
            debug_output.Store(688, _426[151]);
            debug_output.Store(692, _426[152]);
            debug_output.Store(696, _426[153]);
            debug_output.Store(700, _426[154]);
            debug_output.Store(704, _426[155]);
            debug_output.Store(708, _426[156]);
            debug_output.Store(712, _426[157]);
            debug_output.Store(716, _426[158]);
            debug_output.Store(720, _426[159]);
            debug_output.Store(724, _426[160]);
            debug_output.Store(728, _426[161]);
            debug_output.Store(732, _426[162]);
            debug_output.Store(736, _426[163]);
            debug_output.Store(740, _426[164]);
            debug_output.Store(744, _426[165]);
            debug_output.Store(748, _426[166]);
            debug_output.Store(752, _426[167]);
            debug_output.Store(756, _426[168]);
            debug_output.Store(760, _426[169]);
            debug_output.Store(764, _426[170]);
            debug_output.Store(768, _426[171]);
            debug_output.Store(772, _426[172]);
            debug_output.Store(776, _426[173]);
            debug_output.Store(780, _426[174]);
            debug_output.Store(784, _426[175]);
            debug_output.Store(788, _426[176]);
            debug_output.Store(792, _426[177]);
            debug_output.Store(796, _426[178]);
            debug_output.Store(800, _426[179]);
            debug_output.Store(804, _426[180]);
            debug_output.Store(808, _426[181]);
            debug_output.Store(812, _426[182]);
            debug_output.Store(816, _426[183]);
            debug_output.Store(820, _426[184]);
            debug_output.Store(824, _426[185]);
            debug_output.Store(828, _426[186]);
            debug_output.Store(832, _426[187]);
            debug_output.Store(836, _426[188]);
            debug_output.Store(840, _426[189]);
            debug_output.Store(844, _426[190]);
            debug_output.Store(848, _426[191]);
            debug_output.Store(852, _426[192]);
            debug_output.Store(856, _426[193]);
            debug_output.Store(860, _426[194]);
            debug_output.Store(864, _426[195]);
            debug_output.Store(868, _426[196]);
            debug_output.Store(872, _426[197]);
            debug_output.Store(876, _426[198]);
            debug_output.Store(880, _426[199]);
            debug_output.Store(884, _426[200]);
            debug_output.Store(888, _426[201]);
            debug_output.Store(892, _426[202]);
            debug_output.Store(896, _426[203]);
            debug_output.Store(900, _426[204]);
            debug_output.Store(904, _426[205]);
            debug_output.Store(908, _426[206]);
            debug_output.Store(912, _426[207]);
            debug_output.Store(916, _426[208]);
            debug_output.Store(920, _426[209]);
            debug_output.Store(924, _426[210]);
            debug_output.Store(928, _426[211]);
            debug_output.Store(932, _426[212]);
            debug_output.Store(936, _426[213]);
            debug_output.Store(940, _426[214]);
            debug_output.Store(944, _426[215]);
            debug_output.Store(948, _426[216]);
            debug_output.Store(952, _426[217]);
            debug_output.Store(956, _426[218]);
            debug_output.Store(960, _426[219]);
            debug_output.Store(964, _426[220]);
            debug_output.Store(968, _426[221]);
            debug_output.Store(972, _426[222]);
            debug_output.Store(976, _426[223]);
            debug_output.Store(980, _426[224]);
            debug_output.Store(984, _426[225]);
            debug_output.Store(988, _426[226]);
            debug_output.Store(992, _426[227]);
            debug_output.Store(996, _426[228]);
            debug_output.Store(1000, _426[229]);
            debug_output.Store(1004, _426[230]);
            debug_output.Store(1008, _426[231]);
            debug_output.Store(1012, _426[232]);
            debug_output.Store(1016, _426[233]);
            debug_output.Store(1020, _426[234]);
            debug_output.Store(1024, _426[235]);
            debug_output.Store(1028, _426[236]);
            debug_output.Store(1032, _426[237]);
            debug_output.Store(1036, _426[238]);
            debug_output.Store(1040, _426[239]);
            debug_output.Store(1044, _426[240]);
            debug_output.Store(1048, _426[241]);
            debug_output.Store(1052, _426[242]);
            debug_output.Store(1056, _426[243]);
            debug_output.Store(1060, _426[244]);
            debug_output.Store(1064, _426[245]);
            debug_output.Store(1068, _426[246]);
            debug_output.Store(1072, _426[247]);
            debug_output.Store(1076, _426[248]);
            debug_output.Store(1080, _426[249]);
            debug_output.Store(1084, _426[250]);
            debug_output.Store(1088, _426[251]);
            debug_output.Store(1092, _426[252]);
            debug_output.Store(1096, _426[253]);
            debug_output.Store(1100, _426[254]);
            debug_output.Store(1104, _426[255]);
        }
    }
}

[numthreads(256, 1, 1)]
void main(SPIRV_Cross_Input stage_input)
{
    gl_LocalInvocationIndex = stage_input.gl_LocalInvocationIndex;
    comp_main();
}
