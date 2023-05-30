# SPDX-License-Identifier: BSD-3-Clause
# Copyright(c) 2017 Intel Corporation
# Copyright(c) 2023 Josef Olsson & Isak Stenström

# meson file, for building this example as part of a main DPDK build.
#
# To build this example as a standalone application with an already-installed
# DPDK instance, use 'make'

allow_experimental_apis = true
deps += 'eventdev'
sources = files(
        'main.c',
        'pipeline_worker_generic.c',
        'pipeline_worker_tx.c',
        'time_measurement.c',
)
