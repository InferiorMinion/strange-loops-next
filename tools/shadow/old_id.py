"""Legacy fact identifiers in a shadow export."""

import re


def valid_old_id(value):
    return isinstance(value, str) and re.fullmatch(r"[!-~]{1,128}", value) is not None
