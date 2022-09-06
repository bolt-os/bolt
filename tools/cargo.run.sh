#!/bin/sh

echo $1 >| .test-boltk
cd $BOLT_ROOT; $MAKE TEST_BOLTK=$1 cargo-test-runner
