import { expect } from 'chai';
describe('Array', function () {
  describe('#indexOf()', function () {
    it('should return -1 when the value is not present', function () {
      let result = [1, 2, 3].indexOf(4);
      expect(result).to.equal(-1);
    });
  });
});
