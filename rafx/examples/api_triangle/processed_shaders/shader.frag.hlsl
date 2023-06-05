cbuffer UniformData : register(b0, space0)
{
    float4 uniform_data_uniform_color : packoffset(c0);
};


static float4 out_color;
static float4 in_color;

struct SPIRV_Cross_Input
{
    float4 in_color : TEXCOORD0;
};

struct SPIRV_Cross_Output
{
    float4 out_color : SV_Target0;
};

void frag_main()
{
    out_color = uniform_data_uniform_color;
}

SPIRV_Cross_Output main(SPIRV_Cross_Input stage_input)
{
    in_color = stage_input.in_color;
    frag_main();
    SPIRV_Cross_Output stage_output;
    stage_output.out_color = out_color;
    return stage_output;
}
