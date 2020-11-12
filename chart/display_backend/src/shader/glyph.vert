#version 300 es
layout (std140) uniform Transform {
    uniform mat3x2 uTransformationMatrix;
    uniform vec2 uOrigin;
    uniform vec2 uScale;    
    uniform float uGlyphScale;
};
uniform sampler2D uGlyphPaths;

in vec4 aPositionOffset;
in float aScale;
in uvec2 aBackgroundColor;
in uvec2 aBorderColor;
in uvec2 aForegroundColor;
in uvec4 aGlyphData; // (index, num_border_vertices, num_background_vertices, num_foreground_vertices)

flat out vec4 fColor;

vec2 transformPos(vec2 pos){
    return uOrigin + (vec2(1.0, -1.0) * uScale) * pos;
}

vec2 getPosition(vec4 position_offset){
    return transformPos(position_offset.xy) + uGlyphScale * position_offset.zw;
}


vec4 uintColorToVec4(uvec2 color){
    float field1 = float(color[0] & 255u)/255.0;
    color[0] >>= 4;
    float field2 = float(color[0] & 255u)/255.0;
    // color >>= 4;
    float field3 = float(color[1] & 255u)/255.0;
    color[1] >>= 4;
    float field4 = float(color[1] & 255u)/255.0;
    return vec4(field1, field2, field3, field4);
}

vec4 getValueByIndexFromTexture(sampler2D tex, uint index) {
    uint texWidth = uint(textureSize(tex, 0).x);
    int col = int(index % texWidth);
    int row = int(index / texWidth);
    return texelFetch(tex, ivec2(col, row), 0);
}

vec2 getVertexPosition() {
    uvec4 glyphData =  aGlyphData * 3u; // multiply by three to convert number of triangles to number of vertices.
    uint glyphIndex = glyphData[0];
    uint numBackgroundVertices = glyphData[1];
    uint numBorderVertices = glyphData[2];
    uint numForegroundVertices = glyphData[3];
    uint vertexID = uint(gl_VertexID);

    uint componentOffset = 0u;
    if(vertexID < numBackgroundVertices) {
        fColor = uintColorToVec4(aBackgroundColor);
        return getValueByIndexFromTexture(uGlyphPaths, glyphIndex + componentOffset + vertexID).xy * aScale;
    }
    vertexID -= numBackgroundVertices;
    componentOffset += numBackgroundVertices;
    // Border has position and normal.
    if(vertexID < numBorderVertices) {
        fColor = uintColorToVec4(aBorderColor);
        return getValueByIndexFromTexture(uGlyphPaths, glyphIndex + componentOffset + /* 2u * */ vertexID).xy * aScale;
    }
    vertexID -= numBorderVertices;
    componentOffset += /* 2u * */ numBorderVertices;

    if(vertexID < numForegroundVertices) {
        fColor = uintColorToVec4(aForegroundColor);
        return getValueByIndexFromTexture(uGlyphPaths, glyphIndex + componentOffset + vertexID).xy * aScale;
    }
    vertexID -= numForegroundVertices;
    return vec2(0.0, 0.0); // degenerate vertex
}

void main() {
    vec2 vertexPosition = getVertexPosition();
    vec2 center = getPosition(aPositionOffset);
    gl_Position = vec4(uTransformationMatrix * vec3(center + uGlyphScale * vertexPosition, 1.0), 0.0, 1.0);
}