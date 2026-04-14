#!/usr/bin/env bash

set -e

cargo \
	hack \
	--feature-powerset \
	--at-least-one-of=csr,ssr \
	--at-least-one-of=wasm-js-promise,tokio \
	--mutually-exclusive-features=csr,ssr \
	--mutually-exclusive-features=wasm-js-promise,tokio \
	--mutually-exclusive-features=csr,tokio \
	--mutually-exclusive-features=ssr,wasm-js-promise \
	test
