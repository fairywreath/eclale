#version 460 core

#pragma shader_stage(fragment)

layout(location = 0) in vec4 inColor;
layout(location = 1) in flat float thickness;
layout(location = 2) in float widthOffset;

layout(location = 0) out vec4 fragColor;

const float aaRadius = 1.5;

void main()
{
    vec4 color = inColor;

    float w = thickness / 2.0 - aaRadius;
    float d = abs(widthOffset) - w;
    if (d >= 0)
    {
        d /= aaRadius;
        // color.a *= exp(-d * d);
        color.rgb *= exp(-d * d);
    }

    fragColor = color;
}
