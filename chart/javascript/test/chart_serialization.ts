import { expect } from 'chai';
import { parse as parseJSON } from "../src/sseq/json_utils";
import { SseqChart } from "../src/sseq/SseqChart";
import * as fs from 'fs';
import * as path from 'path';

// new folder absolute path
const dirPath = path.join(__dirname, '/views');

function assertSerializeParse(obj : any){
    expect(parseJSON(JSON.stringify(obj))).to.deep.equal(obj);
}

describe('SseqChart', function () {
    describe('parse', function () {
        it('Test chart parse', function () {
            let chart : SseqChart = parseJSON(
                fs.readFileSync(
                    path.join(__dirname, "test_chart.json")
                , 'utf8')
            );
            let [c0, c1] = Array.from(chart.classes.values());
            expect(c0.name[5]).to.equal("\\alpha");
            expect(c1.name[5]).to.equal("\\beta");
            expect(c1.color[0]).to.equal("default");
            expect(c1.color[5]).to.equal("blue");
            expect(c0.max_page).to.equal(2);
        });
    });
});