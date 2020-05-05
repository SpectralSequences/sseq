import pytest

class TestPrime:
    def is_prime(self, n):
        return n in [2,3,5,7,11,13,17,19,23,29]

    def setup_class(self):
        self.pg = 1    

    @pytest.mark.parametrize("n, expected", [[17, True], [19, True],[21, False]])
    def test_isprime(self, n, expected):
        print(self.pg)
        assert [1,2,3] == [1,2,7,4]
