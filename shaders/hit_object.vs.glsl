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


    gl_Position = global.viewProj * global.runnerTransform * instanceData.model * vec4(position, 1.0);

    // XXX: Remove this divergence
    if (instanceData.applyRunnerTransform == 0)
    {
        gl_Position = global.viewProj * instanceData.model * vec4(position, 1.0);
    }

    color = instanceData.color;
}
