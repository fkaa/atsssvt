#define MyRS1 "RootFlags( ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT | " \
                         "DENY_VERTEX_SHADER_ROOT_ACCESS), " \
              "CBV(b0) "

struct VSInput {
    float3 pos : POSITION;
    float3 normal : NORMAL;
    float2 texcoord : TEXCOORD;
};

struct VSOutput {
    float4 pos : SV_Position;
    float3 normal : NORMAL;
    float2 texcoord : TEXCOORD;
};

[RootSignature(MyRS1)]
VSOutput VS(VSInput input)
{
    VSOutput output;

    output.pos = float4(input.pos, 1);
    output.normal = input.normal;
    output.texcoord = input.texcoord;

    return output;
}

struct PSOutput {
    float4 color : SV_Target0;
};

[RootSignature(MyRS1)]
PSOutput PS(VSOutput input)
{
    PSOutput output;

    output.color = float4(input.pos.xyz, 1.f);

    return output;
}
