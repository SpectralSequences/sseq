import pytest
import random
from fp_linear_algebra import FpVector

primes = [2, 3, 5, 7, 11]
dimensions = [5, 10, 33, 65, 1000]
repeats = 1000

class TestFpVector:
    def setup_class(self):
        pass

    @pytest.mark.parametrize("dimension", dimensions)
    @pytest.mark.parametrize("p", primes)
    def test_freed(self, p, dimension):
        v = FpVector(p, dimension)
        w = FpVector(p, dimension)
        v.free()
        assert str(v) == "FreedVector"
        with pytest.raises(ReferenceError):
            v[0]
        with pytest.raises(ReferenceError):
            v[-1]
        with pytest.raises(ReferenceError):
            v.add(w, 1)
        with pytest.raises(ReferenceError):
            w.add(v, 1)
        with pytest.raises(ReferenceError):
            v.add_basis_element(2, 1)
        with pytest.raises(ReferenceError):
            v.assign(w)
        with pytest.raises(ReferenceError):
            w.assign(v)
        with pytest.raises(ReferenceError):
            v.dimension()
        with pytest.raises(ReferenceError):
            v.free()
        with pytest.raises(ReferenceError):
            v.is_zero()
        with pytest.raises(ReferenceError):
            v.is_zero_pure()
        with pytest.raises(ReferenceError):
            v.set_to_zero()
        with pytest.raises(ReferenceError):
            v.set_to_zero_pure()
        with pytest.raises(ReferenceError):
            v.to_list()

    @pytest.mark.parametrize("dimension", dimensions)
    @pytest.mark.parametrize("p", primes)
    def test_to_from_list(self, p, dimension):
        list = [random.randint(0,p-1) for _ in range(dimension)]
        v = FpVector.from_list(p, list)
        result = v.to_list()
        v.free()
        assert list == result

    @pytest.mark.parametrize("dimension", [10])
    @pytest.mark.parametrize("p", [3])
    def test_pack_get(self, p, dimension):
        list = [random.randint(0,p-1) for x in range(dimension)]
        vector = FpVector.from_list(p, list)
        for i in range(dimension):
            assert vector[i] == list[i]
            assert vector[-i-1] == list[-i-1]
        with pytest.raises(IndexError):
            vector[dimension]
        with pytest.raises(IndexError):
            vector[-dimension-1]
        vector.free()

    @pytest.mark.parametrize("dim", dimensions)
    @pytest.mark.parametrize("p", primes)
    def test_set_get(self, p, dim):
        v = FpVector(p, dim)
        k = [0] * dim
        for i in range(repeats):
            index = random.randint(0, dim-1)
            value = random.randint(0, p-1)
            assert v[index] == k[index]
            k[index] = value
            v[index] = value
            assert v[index] == k[index]
        result = v.to_list()
        v.free()
        assert result == k        

    @pytest.mark.parametrize("dim", dimensions)
    @pytest.mark.parametrize("p", primes)
    def test_assign(self, p, dim):
        k = [random.randint(0,p-1) for x in range(dim)]
        l = [random.randint(0,p-1) for x in range(dim)]
        v = FpVector.from_list(p, k)
        w = FpVector.from_list(p, l)
        v.assign(w)
        result = v.to_list()
        v.free()
        w.free()
        assert result == l

    @pytest.mark.parametrize("dim", dimensions)
    @pytest.mark.parametrize("p", primes)
    def test_self_assign(self, p, dim):
        k = [random.randint(0,p-1) for x in range(dim)]
        v = FpVector.from_list(p, k)
        v.assign(v)
        result = v.to_list()
        assert result == k

    @pytest.mark.parametrize("p", primes)
    def test_zero_dimensional_vec(self, p):
        v = FpVector(p, 0)
        w = FpVector(p, 0)
        v.add(w, 1)
        v.scale(3)
        v.assign(w)
        with pytest.raises(IndexError):
            v[0]
        with pytest.raises(IndexError):
            v[-1]
        with pytest.raises(IndexError):
            v[2]
        v.free()

    @pytest.mark.parametrize("p", primes)
    def test_index_zero_dimensional_vec(self, p):
        pass

    @pytest.mark.parametrize("dim", dimensions)
    @pytest.mark.parametrize("p", primes)
    def test_addBasisElement(self, p, dim):
        v = FpVector(p, dim)
        k = [0] * dim
        for i in range(repeats):
            index = random.randint(0, dim-1)
            value = random.randint(0, p-1)
            k[index] += value
            k[index] = k[index] % p
            v.add_basis_element(index, value)
        result = v.to_list()
        v.free()    
        assert result == k        

    def atest_add(self, p, v, w):
        result = []
        for (a, b) in zip(v,w):
            result.append(a+b % p)
        
        assert self.is_prime(n) == expected        
