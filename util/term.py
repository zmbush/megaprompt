BLACK, RED, GREEN, YELLOW, BLUE, MAGENTA, CYAN, WHITE = range(8)


class Term(object):
    def __init__(self, is_zsh):
        self.is_zsh = is_zsh

    def _o(self):
        if self.is_zsh:
            return '%{'
        else:
            return '\['

    def _c(self):
        if self.is_zsh:
            return '%}'
        else:
            return '\]'

    def _formatColorCode(self, codes):
        return "%s\033[%sm%s" % (self._o(),
                                 ";".join([str(p) for p in sorted(codes)]),
                                 self._c())

    @staticmethod
    def _getColorCodes(bg, fg):
        params = []

        if bg is not False:
            params.append(bg + 40)
        if fg is not False:
            params.append(fg + 30)

        return params

    @staticmethod
    def _getOtherFormatCodes(bold, faint, standout, underscore,
                             blink, reverse, concealed):
        params = []

        if bold:
            params.append(1)
        if faint:
            params.append(2)
        if standout:
            params.append(3)
        if underscore:
            params.append(4)
        if blink:
            params.append(5)
        if reverse:
            params.append(7)
        if concealed:
            params.append(8)

        return params

    def color(self,
              bg=False,
              fg=False,
              bold=False,
              faint=False,
              standout=False,
              underscore=False,
              blink=False,
              reverse=False,
              concealed=False):

        params = []
        params.extend(Term._getColorCodes(bg, fg))
        params.extend(Term._getOtherFormatCodes(bold, faint, standout,
                                                underscore, blink, reverse,
                                                concealed))

        if len(params) == 0:
            params = [0]

        return self._formatColorCode(params)
