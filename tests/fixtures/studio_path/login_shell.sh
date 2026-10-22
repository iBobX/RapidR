#!/bin/sh
# A stand-in login shell for tests/studio_flows.mjs (Preview 4 S6): Studio
# runs `$SHELL -l -i -c SCRIPT`; this one says hello first, as startup files
# do, sets a PATH of its own and runs SCRIPT ($4) with it.
echo "Welcome to the test shell"
PATH=/opt/rapidr-test/bin:/usr/bin:/bin:/usr/sbin:/sbin
export PATH
eval "$4"
echo "bye"
