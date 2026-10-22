#!/bin/sh
# A login shell that never answers (a startup file waiting on the network),
# for tests/studio_flows.mjs (Preview 4 S6): Studio gives up after three
# seconds, ends it, and uses its own PATH and the usual folders.
sleep 30
