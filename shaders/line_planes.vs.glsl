/**
 * Line rendered as quads/planes with smoothened egdes. Plane vertices are pre-computed and put into the vertex buffer.
 */

#version 460 core

#pragma shader_stage(vertex)

layout(location = 0) in vec3 position;

layout(location = 0) out vec4 outColor;
layout(location = 1) out flat float outThickness;
layout(location = 2) out float outWidthOffset;

layout(std140, binding = 0) uniform GlobalSceneUbo
{
    mat4 viewProj;
    mat4 runnerTransform;
}
global;


struct LineData
{
    float thickness;
};

struct LineInstanceData
{
    mat4 model;
    vec4 color;
    uint applyRunnerTransform;
};

layout(std430, binding = 1) readonly buffer LineDataSSBO
{
    LineData lines[];
};

layout(std430, binding = 2) readonly buffer LineInstanceDataSSBO
{
    LineInstanceData instances[];
};

const float aaRadius = 1.5;
const vec2 quad[6] = vec2[6](vec2(0, -1), vec2(1, -1), vec2(0, 1), vec2(0, 1), vec2(1, -1), vec2(1, 1));

void main()
{
    uint verticesPerInstance = gl_BaseInstance;
    uint instanceIndex = gl_VertexIndex / verticesPerInstance;

    LineData lineData = lines[instanceIndex];
    LineInstanceData instanceData = instances[instanceIndex];

    float width = lineData.thickness / 2.0 + aaRadius;
    int quadIndex = gl_VertexIndex % 6;
    vec2 quadVertex = quad[gl_VertexIndex % 6];
    float widthExtendAmount = quadVertex.y * width;

    // XXX: Maybe cheaper to have a separate pipeline for this.
    mat4 appliedRunnerTransform = instanceData.applyRunnerTransform * global.runnerTransform +
                            (1 - instanceData.applyRunnerTransform) * mat4(1.0);



    gl_Position = global.viewProj * appliedRunnerTransform * instanceData.model * vec4(position, 1.0);

    outColor = instanceData.color;
    outThickness = lineData.thickness;
    outWidthOffset = abs(widthExtendAmount);
}
