static float4 gl_Position;
static float4 out_color;
static float4 in_color;
static float4 pos;

struct SPIRV_Cross_Input
{
    float4 pos : POSITION;
    float4 in_color : COLOR;
};

struct SPIRV_Cross_Output
{
    float4 out_color : TEXCOORD0;
    float4 gl_Position : SV_Position;
};

void vert_main()
{
    out_color = in_color;
    gl_Position = pos;
}

SPIRV_Cross_Output main(SPIRV_Cross_Input stage_input)
{
    in_color = stage_input.in_color;
    pos = stage_input.pos;
    vert_main();
    SPIRV_Cross_Output stage_output;
    stage_output.gl_Position = gl_Position;
    stage_output.out_color = out_color;
    return stage_output;
}
