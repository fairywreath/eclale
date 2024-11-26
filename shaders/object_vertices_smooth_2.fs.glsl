#version 460 core

#pragma shader_stage(fragment)

layout(location = 0) in vec4 color;
layout(location = 2) in float distanceToCenter;

layout(location = 0) out vec4 outFragColor;

void main()
{

    // XXX TODO: Make this configurable from outside.
    float width = 0.05;
    float aaWidth = 0.035;

    float alpha = smoothstep(width + aaWidth, width, abs(distanceToCenter));
    outFragColor = vec4(color.rgb * alpha, 1.0);

    outFragColor = color;
}

