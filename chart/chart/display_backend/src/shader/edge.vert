#version 300 es
#define M_PI 3.1415926535897932384626433832795
#define ANGLE_RES 180 // must be the same as ANGLE_RESOLUTION in convex_hull.rs

// this variant counts each pixel as 4 distinct floats.
float getValueByIndexFrom4ChannelTexture(sampler2D tex, int index){
    int texWidth = textureSize(tex, 0).x;
    int channel = index % 4;
    int texOffset = index / 4;
    int col = texOffset % texWidth;
    int row = texOffset / texWidth;
    return texelFetch(tex, ivec2(col, row), 0)[channel];
}

vec4 getValueByIndexFromTexture(sampler2D tex, int index) {
    int texWidth = textureSize(tex, 0).x;
    int col = index % texWidth;
    int row = index / texWidth;
    return texelFetch(tex, ivec2(col, row), 0);
}

// Coordinate system
layout (std140) uniform Transform {
    uniform mat3x2 uTransformationMatrix;
    uniform vec2 uOrigin;
    uniform vec2 uScale;
    uniform float uGlyphScale;
};

uniform sampler2D uGlyphConvexHulls;
uniform sampler2D uArrowMetrics;
uniform sampler2D uArrowPaths;

// Apparently some webgl2 implementations only allow at most 8 attributes.
// Here we have 7, which is why they are all packed so carefully. I think an ivec4 is only guaranteed
// to take i16s whereas a vec4 is guaranteed to take f32s (maybe?) so you could perhaps pack
// more integers into vec4's and then use "floatBitsToInt" if you were really desperate / needed to mix floats and ints.
in vec4 aColor;
in vec4 aStartPositionOffset; // (start_position, start_offset)
in vec4 aEndPositionOffset; // (end_position, end_offset)
in vec4 aGlyphScales_angle_thickness; // (start_glyph_scale, end_glyph_scale, angle, thickness)
in ivec4 aStart; // (startGlyph, vec3 startArrow = (NumVertices, HeaderIndex, VerticesIndex) )
in ivec4 aEnd; // (endGlyph, vec3 endArrow = (NumVertices, HeaderIndex, VerticesIndex) )
in ivec4 aDashPattern; // (dash_length, dash_index, dash_offset, _ )

out vec4 fColor;
flat out float fHalfThickness;
// In frag shader, we test whether fCurvature is zero to decide whether we draw an arc or a line.
// In the case of an arc, fCurvature > 0 means curve right, < 0 means curve left.
flat out float fCurvature;
// Needed for arcs
flat out vec2 fP0; // Position of start of arc
flat out vec2 fN0; // Normal to start of arc

// Needed to apply dash pattern
flat out ivec4 fDashPattern;
out vec2 vPosition;
// Needed to apply dash pattern to an arc.
flat out vec2 fCenter;
flat out float fInitialAngle;

// Transform from chart coordinate system to pixels
vec2 transformPos(vec2 pos){
    return uOrigin + (vec2(1.0, -1.0) * uScale) * pos;
}

vec2 getPosition(vec4 position_offset){
    return transformPos(position_offset.xy) + uGlyphScale * position_offset.zw;
}


vec4 reverseTangent(vec4 pos_tan){
    return pos_tan * vec4(1.0, 1.0, -1.0, -1.0);
}

// Rotate 90 degrees counterclockwise
vec2 normalVector(vec2 direction){
    return direction.yx * vec2(-1.0, 1.0);
}

// Input must be a unit vector. If the input vector is at an angle theta from the x axis, give the matrix rotating by theta.
mat2 rotationMatrix(vec2 direction){
    return mat2(direction, normalVector(direction));
}

// Returns distance from center of glyph to boundary in direction angle.
float glyphBoundaryPoint(int glyph, float angle){
    int glyph_index = (int(angle / (2.0 * M_PI) * float(ANGLE_RES)) + ANGLE_RES) % ANGLE_RES;
    int total_index = ANGLE_RES * glyph + glyph_index;
    return uGlyphScale * getValueByIndexFrom4ChannelTexture(uGlyphConvexHulls, total_index);
}


int arrowNumVertices(ivec3 arrow){
    return arrow[0];
}

float arrowTipEnd(ivec3 arrow){
    int headerIndex = arrow[1];
    float tip_end = getValueByIndexFrom4ChannelTexture(uArrowMetrics, headerIndex);
    return tip_end;
}

vec2 arrowEnds(ivec3 arrow){
    int headerIndex = arrow[1];
    float tip_end = getValueByIndexFrom4ChannelTexture(uArrowMetrics, headerIndex);
    float back_end = getValueByIndexFrom4ChannelTexture(uArrowMetrics, headerIndex + 1);
    return vec2(tip_end, back_end);
}

vec2 arrowVisualEnds(ivec3 arrow){
    int headerIndex = arrow[1];
    float visual_tip_end = getValueByIndexFrom4ChannelTexture(uArrowMetrics, headerIndex + 2);
    float visual_back_end = getValueByIndexFrom4ChannelTexture(uArrowMetrics, headerIndex + 3);
    return vec2(visual_tip_end, visual_back_end);
}

float arrowLineEnd(ivec3 arrow){
    int headerIndex = arrow[1];
    return getValueByIndexFrom4ChannelTexture(uArrowMetrics, headerIndex + 4);
}

vec2 getArrowVertex(ivec3 arrow, int vertexIndex) {
    int verticesIndex = arrow[2];
    return getValueByIndexFromTexture(uArrowPaths, verticesIndex + vertexIndex).xy;
}


// TriangleStrip patterning : [0, 1, 2, 1, 2, 3, 2, 3, 4, ...]
int triangleStripPattern(int index){
    return (index % 3) + (index / 3);
}



// This is the special case of circleOffset when position = (0, 0) and tangent = (1, 0).
// In that case the circle looks like the graph of r = sin(theta).
// Needs: epsilon < dist * abs(curvature) / 2 < 1 - epsilon (upper bound comes from dist < diameter).
// curvature -- curvature of circle (1/radius). If curvature > 0 it curves leftward, if curvature < 0 it curves rightward.
// dist -- distance to move along circle
// direction -- does circle curve to the left or to the right of the tangent vector.
vec4 circleOffsetHelper(float curvature, float dist) {
    // If dist == 0, tangent, vector we want to normalize to get tangent is (0, 0).
    // Thus, it's necessary to special case distance == 0. (What about distance small?)
    if(dist == 0.0){
        return vec4(0.0, 0.0, 1.0, 0.0);
    }
    // ??
    // if(dist < epsilon){
    //     return vec4(epsilon, 0.0, 1.0, 0.0);
    // }
    float x = dist;
    float C = -curvature;
    float cx_over_2 = C*x/2.0;
    float om_cx_over_2_sq = 1.0 - cx_over_2 * cx_over_2;
    // position = x(sqrt(1 - (Cx/2)^2), Cx/2)
    // tangent = position * (4(1 - (Cx/2)^2) - 2, 4(1 - (Cx/2)^2))
    vec2 position = x * vec2(sqrt(om_cx_over_2_sq), cx_over_2);
    vec2 tangent_factor = (4.0 * om_cx_over_2_sq) * vec2(1.0, 1.0) - vec2(2.0, 0.0);
    // tangent_factor * position is the double angle formula applied to position.
    // The secant for the doubled angle is the tangent at the angle.
    vec2 tangent = normalize(tangent_factor * position);
    if(dist < 0.0){
        tangent *= -1.0;
    }
    return vec4(position, tangent);
}

// There is a unique circle through pos with tangent vector tan at pos and given curvature that curves to the left.
// start_pos_tan -- start position and tangent.
// curvature -- 1/radius, with sign: if curvature > 0 it curves leftward, if curvature < 0 it curves rightward.
// dist -- secand length along circle.
// Needs: epsilon < dist * abs(curvature) / 2 < 1 - epsilon (upper bound comes from dist < diameter).
vec4 circleOffset(vec4 start_pos_tan, float curvature, float dist){
    vec2 start_pos = start_pos_tan.xy;
    vec2 start_tan = start_pos_tan.zw;
    vec4 helper_pos_tan = circleOffsetHelper(curvature, dist);
    vec2 helper_pos = helper_pos_tan.xy;
    vec2 helper_tan = helper_pos_tan.zw;
    mat2 rotation = rotationMatrix(start_tan);
    vec2 result_pos = start_pos + rotation * helper_pos;
    vec2 result_tan = rotation * helper_tan;
    return vec4(result_pos, result_tan);
}


float glyphOffsetLinear(int glyph, float scale, float angle){
    return scale * glyphBoundaryPoint(glyph, angle);
}

float glyphOffsetCurvedHelper(int glyph, float scale, vec2 tangent){
    float angle = atan(tangent.y, tangent.x);
    return scale * glyphBoundaryPoint(glyph, angle);
}

vec4 glyphOffsetCurved(int glyph, float scale, float extra, vec4 pos_tan, float curvature){
    float offset = glyphOffsetCurvedHelper(glyph, scale, pos_tan.zw) + extra;
    // return circleOffset(pos_tan, curvature, offset);
    vec2 secant = circleOffset(pos_tan, curvature, offset).zw;
    // Try again with more accurate angle.
    offset = glyphOffsetCurvedHelper(glyph, scale, secant) + extra;
    return circleOffset(pos_tan, curvature, offset);
}


// The main function in the case where we are drawing a straight line.
vec2 vertexPositionLinear(){
    vec2 startPos = getPosition(aStartPositionOffset);
    vec2 endPos = getPosition(aEndPositionOffset);
    vec2 tangent = normalize(endPos - startPos);
    // Will use angle to decide how far to offset from start / end glyph
    float angle = atan(tangent.y, tangent.x);

    // Load input attributes.
    int startGlyph = aStart.x;
    int endGlyph = aEnd.x;
    float startGlyphScale = aGlyphScales_angle_thickness.x;
    float endGlyphScale = aGlyphScales_angle_thickness.y;
    float thickness = aGlyphScales_angle_thickness.w;
    fN0 = normalVector(tangent); // For dash pattern to calculate distance along line

    ivec3 startArrow = aStart.yzw;
    ivec3 endArrow = aEnd.yzw;
    float startArrowTipEnd = arrowTipEnd(startArrow);
    float endArrowTipEnd = arrowTipEnd(startArrow);

    // Offset start and end position to boundary of glyph
    // We want the tip end to be exactly at the boundary of the glyph so we also adjust by tip end.
    startPos += tangent * (glyphOffsetLinear(startGlyph, startGlyphScale, angle) + startArrowTipEnd);
    endPos -= tangent * (glyphOffsetLinear(endGlyph, endGlyphScale, angle + M_PI) + endArrowTipEnd);

    int vertexID = gl_VertexID;
    // The line
    if(vertexID < 6){
        fDashPattern = aDashPattern;
        // Further shorten by lineEnd
        startPos -= tangent * arrowLineEnd(startArrow);
        endPos += tangent * arrowLineEnd(endArrow);
        fP0 = startPos;

        int vertexIndex = triangleStripPattern(vertexID);
        vec2 normal = normalVector(tangent);
        if(vertexIndex % 2 == 1){
            normal = - normal;
        }
        vec2 pos;
        if(vertexIndex/2 == 0){
            pos = startPos;
        } else {
            pos = endPos;
        }
        pos += thickness/2.0 * normal;
        vPosition = pos; // for dash pattern to compute distance along line.
        return pos;
    }
    vertexID -= 6;

    mat2 rotationMatrix = rotationMatrix(tangent);
    // Position start arrow tip
    if(vertexID < arrowNumVertices(startArrow)) {
        return startPos - rotationMatrix * getArrowVertex(startArrow, vertexID);
    }
    vertexID -= arrowNumVertices(startArrow);

    // End arrow tip
    if(vertexID < arrowNumVertices(endArrow)) {
        return endPos + rotationMatrix * getArrowVertex(endArrow, vertexID).xy;
    }
    vertexID -= arrowNumVertices(endArrow);

    // Extra throw-away vertices
    return vec2(0.0, 0.0);
}


vec2 positionCurvedArrrow(ivec3 arrow, int glyph, float glyphScale, vec4 posTan, float curvature, int vertexID){
    vec2 ends = arrowVisualEnds(arrow);
    float visualTipEnd = ends[0];
    float visualBackEnd = ends[1];
    // Find position for visual tip end and visual back end. We want both the visual tip end and the visual back end to be on the circle.
    // (We are "flexing" the arrow in the language of the tikz manual...)
    vec4 tipEndPosTan = glyphOffsetCurved(glyph, glyphScale, visualTipEnd, posTan, curvature);
    vec4 backEndPosTan = circleOffset(tipEndPosTan, curvature, -visualTipEnd + visualBackEnd);
    vec2 secant = normalize(tipEndPosTan.xy - backEndPosTan.xy);
    mat2 rotationMatrix = rotationMatrix(secant);
    return tipEndPosTan.xy - rotationMatrix * getArrowVertex(arrow, vertexID);
}

// The main function in the case where we are drawing an arc (angle != 0)
vec2 vertexPositionCurved(){
    // Load input attributes, compute tangents, curvature.
    vec2 startPos = getPosition(aStartPositionOffset);
    vec2 endPos = getPosition(aEndPositionOffset);
    vec2 displacement = endPos.xy - startPos.xy;
    float displacement_length = length(displacement);
    float angle = aGlyphScales_angle_thickness.z;
    float curvature = 2.0 * sin(angle) / displacement_length;

    float segment_angle = atan(displacement.y, displacement.x);

    float start_tangent_angle = segment_angle + angle;
    vec2 start_tangent = vec2(cos(start_tangent_angle), sin(start_tangent_angle));
    vec4 startPosTan = vec4(startPos, start_tangent);

    float end_tangent_angle = segment_angle - angle;
    vec2 end_tangent = vec2(cos(end_tangent_angle), sin(end_tangent_angle));
    vec4 endPosTan = vec4(endPos, end_tangent);


    // A couple of sign distinctions depend on whether we curve left or right.
    bool curvesLeft = angle < 0.0;
    float thickness = aGlyphScales_angle_thickness.w;
    float startGlyphScale = aGlyphScales_angle_thickness.x;
    float endGlyphScale = aGlyphScales_angle_thickness.y;
    int startGlyph = aStart.x;
    int endGlyph = aEnd.x;
    ivec3 startArrow = aStart.yzw;
    ivec3 endArrow = aEnd.yzw;

    int vertexID = gl_VertexID;

    // Arc. It's our responsibility here to draw triangles such that:
    // (1) Triangles cover entire arc once each
    // (2) When arc passes out of our region, the edge is perpendicular to the tangent
    // We use two trapezoids. Would be possible to accommodate angle < 90 degrees with just one. This deals with angle <
    if(vertexID < 12){
        fDashPattern = aDashPattern;
        // Save original positions. We want to use angle in a bit, but we're not sure what the angle between the updated start and end position are.
        vec4 origStartPosTan = startPosTan;
        vec4 origEndPosTan = endPosTan;
        float startSetback = arrowTipEnd(startArrow) - arrowLineEnd(startArrow);
        float endSetback = arrowTipEnd(endArrow) - arrowLineEnd(endArrow);
        // Compute start and endpoints
        startPosTan = glyphOffsetCurved(startGlyph, startGlyphScale, startSetback, startPosTan, curvature);
        // Negate curvature and reverse tangent directions to move backward along the curve (would suffice to negate distance, but we get the
        // distance to glyph boundary inside glyphOffsetCurved so this is the easiest way)
        endPosTan = reverseTangent(glyphOffsetCurved(endGlyph, endGlyphScale, endSetback, reverseTangent(endPosTan), -curvature));

        // Parameters for fragment shader
        fCurvature = curvature;
        fP0 = startPosTan.xy;
        fN0 = normalVector(startPosTan.zw);
        fHalfThickness = thickness / 2.0;

        int vidx = triangleStripPattern(vertexID);
        bool inside = vidx % 2 == 0;
        inside = inside != curvesLeft;
        int angle_idx = vidx / 2;
        // recall displacement is (origEndPosTan.xy - origStartPosTan.xy);
        // midpoint normal is the normal to the secant.
        vec2 midNormal = normalVector(normalize(displacement));
        vec2 midPos = (origStartPosTan.xy + origEndPosTan.xy) / 2.0 + (displacement_length/2.0 * tan(angle/2.0)) * midNormal;

        vec2 pos;
        vec2 normal;
        switch(angle_idx){
            case 0:
                pos = startPosTan.xy;
                normal = normalVector(startPosTan.zw);
                break;
            case 1:
                pos = midPos;
                normal = midNormal;
                break;
            case 2:
                pos = endPosTan.xy;
                normal = normalVector(endPosTan.zw);
                break;
        }

        float thickness_scale = 2.0;
        float offset;
        if(inside){
            // Inside just needs to account for thickness. We double the thickness to be sure to avoid clipping at the end points.
            offset = -thickness_scale * thickness;
        } else {
            // Make sure to go out far enough that the line between outer points doesn't clip the midpoints of the segments (at 1/4 and 3/4 of angle)
            float magnitude = length(midPos - origStartPosTan.xy)/2.0 * abs(tan(angle/4.0))/cos(angle/2.0);
            offset = magnitude + thickness_scale * thickness;
        }
        if(curvesLeft){
            offset = -offset;
        }
        pos += offset * normal;
        vPosition = pos;
        // Parameters for fragment shader dash pattern (needs to compute arclength)
        if(aDashPattern.x != 0 && abs(curvature) > 0.0001){
            vec2 initialNormal = normalVector(startPosTan.zw) / curvature;
            fCenter = startPosTan.xy - initialNormal;
            // make sure to use normal vector scaled by curvature here so that it points in the correct direction
            // (depends on sign of curvature)
            fInitialAngle = atan(initialNormal.y, initialNormal.x);
        }
        return pos;
    }
    vertexID -= 12;

    // Start arrow
    if(vertexID < arrowNumVertices(startArrow)) {
        return positionCurvedArrrow(startArrow, startGlyph, startGlyphScale, startPosTan, curvature, vertexID);
    }
    vertexID -= arrowNumVertices(startArrow);

    // End arrow
    if(vertexID < arrowNumVertices(endArrow)) {
        return positionCurvedArrrow(endArrow, endGlyph, endGlyphScale, reverseTangent(endPosTan), -curvature, vertexID);
    }
    vertexID -= arrowNumVertices(endArrow);

    // Extra throw-away vertices
    return vec2(0.0, 0.0);
}

void main() {
    fColor = aColor;
    float angle = aGlyphScales_angle_thickness.z;
    vec2 position;
    if(angle == 0.0){
        position = vertexPositionLinear();
    } else {
        position = vertexPositionCurved();
    }
    gl_Position = vec4(uTransformationMatrix * vec3(position, 1.0), 0.0, 1.0);
}
