# Eventdev_pipeline

This is the version of Eventdev_pipeline used in the thesis.

## Installation

This application is compiled together with DPDK. The steps to this are:

1. Copy the file `rte_eth_null.c` into the DPDK repository to
   `drivers/net/null/`. This file is needed for the latency measurements to
   function, and the only difference to the original file is the addition of
   line 102.
1. Copy the entire `eventdev_pipeline` folder to `examples/eventdev_pipeline`
   in the DPDK repository to overwrite the old application.
1. Follow the [DPDK Quick Start Guide](https://core.dpdk.org/doc/quick-start/)
   to compile the application. Be sure to include the examples in the build.
1. Eventdev_pipeline can then be run with
   `build/examples/dpdk-eventdev_pipeline` from the DPDK repo.