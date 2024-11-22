#version 460 core

#pragma shader_stage(vertex)

layout(location = 0) in vec3 position;

layout(location = 0) out vec4 color;

struct HitInstanceData
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

layout(std430, binding = 1) readonly buffer HitInstanceDataSbo
{
    HitInstanceData instances[];
};

void main()
{
    HitInstanceData instanceData = instances[gl_InstanceIndex];

    // XXX: Maybe cheaper to have a separate pipeline for this.
    mat4 appliedRunnerTransform = instanceData.applyRunnerTransform * global.runnerTransform +
                            (1 - instanceData.applyRunnerTransform) * mat4(1.0);


    gl_Position = global.viewProj * appliedRunnerTransform * instanceData.model * vec4(position, 1.0);
    // gl_Position = global.viewProj * global.runnerTransform * instanceData.model * vec4(position, 1.0);

    // gl_Position = global.viewProj * global.runnerTransform * vec4(position, 1.0);
    // gl_Position = global.viewProj * vec4(position, 1.0);

    color = instanceData.color;
}
