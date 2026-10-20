#!/usr/bin/env bash

set -e

cargo \
	hack \
	--feature-powerset \
	"$@"
