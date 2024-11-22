#version 460 core

#pragma shader_stage(vertex)

layout(location = 0) in vec3 position;
layout(location = 1) in uint objectIndex;

layout(location = 0) out vec4 color;

struct ObjectData
{
    mat4 model;
    vec4 color;
    uint applyRunnerTransform;
};

layout(std140, binding = 0) uniform GlobalSceneUbo
{
    mat4 viewProj;
    mat4 runnerTransform;
}
global;

layout(std430, binding = 1) readonly buffer ObjectDataSsbo
{
    ObjectData allObjectData[];
};

void main()
{
    // ObjectData objectData = allObjectData[2];
    ObjectData objectData = allObjectData[objectIndex];

    // XXX FIXME: properly set the model matrix in cpu.
    // gl_Position = global.viewProj * global.runnerTransform * objectData.model * vec4(position, 1.0);

    gl_Position = global.viewProj * global.runnerTransform * vec4(position, 1.0);

    color = objectData.color;
    // color = vec4(0.5, 0.5, 0.5, 1.0);
}
