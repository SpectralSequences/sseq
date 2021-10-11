#version 300 es
uniform mat3x2 uTransformationMatrix;
uniform vec2 uOrigin;
uniform vec2 uScale;
uniform ivec4 uChartRange; // (xmin, xmax, ymin, ymax)
uniform vec4 uScreenRange; // (xmin, xmax, ymin, ymax)
uniform ivec2 uGridStep; // (xGridStep, yGridStep)
uniform vec2 uGridOffset; // (xGridStep, yGridStep)

uniform float uThickness;
uniform vec4 uColor;

vec2 getCoord(int lineID, int axisMin, int axisStep, float axisOffset, float axisOrigin, float axisScale, vec2 screenRange){
    int vertexID = (gl_VertexID % 3) + gl_VertexID / 3;
    int chartCoord = axisMin + axisStep * lineID;
    float normalCoord = (float(chartCoord) + axisOffset) * axisScale + axisOrigin;
    float varyingCoord;

    if(vertexID/2 == 0){
        varyingCoord = screenRange.x;
    } else {
        varyingCoord = screenRange.y;
    }
    if(vertexID % 2 == 1){
        normalCoord += uThickness;
    } else {
        normalCoord -= uThickness;
    }

    return vec2(varyingCoord, normalCoord);
}


void main() {
    float chartXGridOffset = uGridOffset.x;
    float chartYGridOffset = uGridOffset.y;
    int chartXGridStep = uGridStep.x;
    int chartYGridStep = uGridStep.y;
    int chartXMin = uChartRange.x;
    int chartXMax = uChartRange.y;
    int chartYMin = uChartRange.z;
    int chartYMax = uChartRange.w;
    int numHorizontalGridLines = (chartYMax - chartYMin) / chartYGridStep + 1;
    vec2 screenXRange = uScreenRange.xy;
    vec2 screenYRange = uScreenRange.zw;
    
    vec2 position;
    if(gl_InstanceID < numHorizontalGridLines){
        // Horizontal lines
        int lineID = gl_InstanceID;
        position = getCoord(lineID, chartYMin, chartYGridStep, chartYGridOffset, uOrigin.y, -uScale.y, screenXRange).xy;
    } else {
        // Vertical lines
        int lineID = gl_InstanceID - numHorizontalGridLines;
        position = getCoord(lineID, chartXMin, chartXGridStep, chartXGridOffset, uOrigin.x, uScale.x, screenYRange).yx;
    }
    gl_Position = vec4(uTransformationMatrix * vec3(position, 1.0), 0.0, 1.0);
}