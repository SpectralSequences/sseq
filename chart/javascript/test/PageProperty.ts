import { expect } from 'chai';
import { PageProperty } from "../src/sseq/PageProperty";
import { parse as parseJSON } from "../src/sseq/json_utils";
import { INFINITY } from "../src/infinity";

function assertSerializeParse(pp : PageProperty<any>){
    expect(parseJSON(JSON.stringify(pp))).to.deep.equal(pp);
}

describe('PageProperty', function () {
  describe('assign', function () {
    it('Test PageProperty slice-assign behavior', function () {
        let pp = new PageProperty(0);
        expect(pp.values).to.deep.equal([[-INFINITY, 0]]);
        pp[":"] = 1;
        expect(pp.values).to.deep.equal([[-INFINITY, 1]]);
        pp[2] = 2;
        expect(pp.values).to.deep.equal([[-INFINITY, 1], [2, 2]]);
        pp[3] = 3;
        expect(pp.values).to.deep.equal([[-INFINITY, 1], [2, 2], [3, 3]]);
        pp[2] = 1;
        expect(pp.values).to.deep.equal([[-INFINITY, 1], [3, 3]]);
        pp["5:10"] = 7;
        expect(pp.values).to.deep.equal([[-INFINITY, 1], [3, 3], [5, 7], [10, 3]]);
        pp["4:8"] = 10;
        expect(pp.values).to.deep.equal([[-INFINITY, 1], [3, 3], [4, 10], [8, 7], [10, 3]]);
        pp[4] = 3;
        expect(pp.values).to.deep.equal([[-INFINITY, 1], [3, 3], [8, 7], [10, 3]]);
        pp["2 : 9"] = 10;
        expect(pp.values).to.deep.equal([[-INFINITY, 1], [2, 10], [9, 7], [10, 3]]);
        pp["4"] = 7;
        expect(pp.values).to.deep.equal([[-INFINITY, 1], [2, 10], [4, 7], [10, 3]]);
        pp["7 : 20"] = 15;
        expect(pp.values).to.deep.equal([[-INFINITY, 1], [2, 10], [4, 7], [7, 15], [20, 3]]);
        pp["6:"] = 9;
        expect(pp.values).to.deep.equal([[-INFINITY, 1], [2, 10], [4, 7], [6, 9]]);
        pp[":3"] = 2;
        expect(pp.values).to.deep.equal([[-INFINITY, 2], [3, 10], [4, 7], [6, 9]]);
        pp[":"] = 77;
        expect(pp.values).to.deep.equal([[-INFINITY, 77]]);
    });
    it('Test PageProperty serialization', function () {
        let pp = new PageProperty(0);
        assertSerializeParse(pp);
        pp[2] = 2
        assertSerializeParse(pp)
        pp[3] = 3
        assertSerializeParse(pp)
        pp[2] = 1
        assertSerializeParse(pp)
        pp["5:10"] = 7
        assertSerializeParse(pp)
        pp["4:8"] = 10
        assertSerializeParse(pp)
        pp["4"] = 3
        assertSerializeParse(pp)
        pp["2 : 9"] = 10
        assertSerializeParse(pp)
        pp["4"] = 7
        assertSerializeParse(pp)
        pp["7 : 20"] = 15
        assertSerializeParse(pp)
        pp["6:"] = 9
        assertSerializeParse(pp)
        pp[":3"] = 2
        assertSerializeParse(pp)
        pp[":"] = 77;
        assertSerializeParse(pp);
    });
  });
});
