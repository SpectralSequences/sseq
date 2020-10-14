#version 300 es
#define ANGLE_RES 180 // should be same as ANGLE_RESOLUTION
layout (std140) uniform Transform {
    uniform mat3x2 uTransformationMatrix;
    uniform vec2 uOrigin;
    uniform vec2 uScale;
    uniform float uGlyphScale;
};
uniform sampler2D uGlyphBoundaryTexture;

in vec4 aPositionOffset;
in float aScale;
in uvec2 aGlyphIndex;  // (index, padding)

flat out vec4 fColor;
out vec2 vPosition;

vec2 transformPos(vec2 pos){
    return uOrigin + (vec2(1.0, -1.0) * uScale) * pos;
}

vec2 getPosition(vec4 position_offset){
    return transformPos(position_offset.xy) + uGlyphScale * position_offset.zw;
}

vec2 getVec2ByIndexFrom4ChannelTexture(sampler2D tex, int index){
    int texWidth = textureSize(tex, 0).x;
    int channel = index % 2;
    int texOffset = index / 2;
    int col = texOffset % texWidth;
    int row = texOffset / texWidth;
    vec4 pixel = texelFetch(tex, ivec2(col, row), 0);
    if(channel == 0) {
        return pixel.xy;
    } else {
        return pixel.zw;
    }
}

vec2 glyphBoundaryPointVertex(uint glyph, int vertexID){
    int total_index = ANGLE_RES * int(glyph) + vertexID;
    return uGlyphScale * aScale * getVec2ByIndexFrom4ChannelTexture(uGlyphBoundaryTexture, total_index);
}

vec4 getColor(){
    int vertexID = gl_InstanceID + 1;
    int r = vertexID & 0xFF;
    int g = (vertexID >> 8) & 0xFF;
    int b = (vertexID >> 16) & 0xFF;
    int a = (vertexID >> 24) & 0xFF;
    return vec4(float(r)/255., float(g)/255., float(b)/255., float(a)/255.);
}

void main() {
    vec2 vertexPosition = glyphBoundaryPointVertex(aGlyphIndex.x, gl_VertexID);
    vPosition = vertexPosition;
    vec2 center = getPosition(aPositionOffset);
    fColor = getColor();
    gl_Position = vec4(uTransformationMatrix * vec3(center + vertexPosition, 1.0), 0.0, 1.0);
    length(vPosition);
}