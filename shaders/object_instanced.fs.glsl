#version 460 core

#pragma shader_stage(fragment)

layout(location = 0) in vec4 color;

layout(location = 0) out vec4 outFragColor;

void main()
{
    outFragColor = color;
}
