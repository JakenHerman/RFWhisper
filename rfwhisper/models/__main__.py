"""Entry point for `python -m rfwhisper.models` — dispatches to fetch.main()."""

import sys

from rfwhisper.models import fetch

if __name__ == "__main__":
    sys.exit(fetch.main(sys.argv[1:]))
