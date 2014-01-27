import pytest
import re

from hypothesis.testdecorators import given

from util.term import Term


class TestTerm(object):
    @pytest.fixture
    def bash_term(self):
        return Term(False)

    @pytest.fixture
    def zsh_term(self):
        return Term(True)

    def test_bash_escape_codes(self, bash_term):
        assert bash_term._o() == '\['
        assert bash_term._c() == '\]'

    def test_zsh_escape_codes(self, zsh_term):
        assert zsh_term._o() == '%{'
        assert zsh_term._c() == '%}'

    def test_formatColorCode(self, bash_term):
        assert bash_term._formatColorCode([0]) == '\[\033[0m\]'
        assert bash_term._formatColorCode([1, 2]) == '\[\033[1;2m\]'
        assert bash_term._formatColorCode([2, 1]) == '\[\033[1;2m\]'

    @given(int, int)
    def test_getColorCodes(self, bg, fg):
        assert Term._getColorCodes(bg, fg) == [bg + 40, fg + 30]

    @given(bool, bool, bool, bool, bool, bool, bool)
    def test_getOtherFormatCodes(self, a, b, c, d, e, f, g):
        right = []
        if a:
            right.append(1)
        if b:
            right.append(2)
        if c:
            right.append(3)
        if d:
            right.append(4)
        if e:
            right.append(5)
        if f:
            right.append(7)
        if g:
            right.append(8)

        assert Term._getOtherFormatCodes(a, b, c, d, e, f, g) == right

    def test_color(self, bash_term):
        @given(int, int, bool, bool, bool, bool, bool, bool, bool)
        def test_inputs(a, b, c, d, e, f, g, h, i):
            col = bash_term.color(a, b, c, d, e, f, g, h, i)
            # match = re.match(r'\\\[\033\\\[(.+)m\\\]', col)
            match = re.match(r'\\\[\033\[(?P<codes>.*)m\\\]', col)
            assert match is not None
            codes = [int(code) for code in match.group('codes').split(';')]
            assert codes == sorted(codes)
            assert (a + 40) in codes
            assert (b + 30) in codes
            if c:
                assert 1 in codes
            if d:
                assert 2 in codes
            if e:
                assert 3 in codes
            if f:
                assert 4 in codes
            if g:
                assert 5 in codes
            if h:
                assert 7 in codes
            if i:
                assert 8 in codes

        test_inputs()

    def test_reset(self, bash_term):
        match = re.match(r'\\\[\033\[(?P<codes>.*)m\\\]', bash_term.color())

        codes = [int(code) for code in match.group('codes').split(';')]

        assert codes == [0]
