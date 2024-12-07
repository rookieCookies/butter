// language: metal1.0
#include <metal_stdlib>
#include <simd/simd.h>

using metal::uint;

struct VertexInput {
    metal::float3 position;
    metal::float2 texture_coord;
};
struct VertexOutput {
    metal::float4 clip_position;
    metal::float2 uv;
};

struct vs_mainInput {
    metal::float3 position [[attribute(0)]];
    metal::float2 texture_coord [[attribute(1)]];
};
struct vs_mainOutput {
    metal::float4 clip_position [[position]];
    metal::float2 uv [[user(loc0), center_perspective]];
};
vertex vs_mainOutput vs_main(
  vs_mainInput varyings [[stage_in]]
, constant metal::float4x4& proj [[user(fake0)]]
) {
    const VertexInput model = { varyings.position, varyings.texture_coord };
    VertexOutput out = {};
    out.uv = model.texture_coord;
    metal::float4x4 _e6 = proj;
    out.clip_position = _e6 * metal::float4(model.position, 1.0);
    VertexOutput _e11 = out;
    const auto _tmp = _e11;
    return vs_mainOutput { _tmp.clip_position, _tmp.uv };
}


struct fs_mainInput {
    metal::float2 uv [[user(loc0), center_perspective]];
};
struct fs_mainOutput {
    metal::float4 member_1 [[color(0)]];
};
fragment fs_mainOutput fs_main(
  fs_mainInput varyings_1 [[stage_in]]
, metal::float4 clip_position [[position]]
, metal::texture2d<float, metal::access::sample> texture [[user(fake0)]]
, metal::sampler samp [[user(fake0)]]
, constant metal::float4& modulate [[user(fake0)]]
) {
    const VertexOutput in = { clip_position, varyings_1.uv };
    metal::float4 _e4 = texture.sample(samp, in.uv);
    metal::float4 _e6 = modulate;
    return fs_mainOutput { _e4 * _e6 };
}
