#ifndef TIME_MEASUREMENT_H
#define TIME_MEASUREMENT_H

#include <inttypes.h>
#include <stdbool.h>

#include "pipeline_common.h"

#define MAX_LATENCY (100000)

void eventdev_busy_wait_ms(uint32_t ms);

void eventdev_warm_up(void);

uint64_t eventdev_burn_tsc_to_loops(uint64_t tsc);

void eventdev_burn(uint64_t loops);

uint64_t eventdev_s_to_tsc(double s);

double eventdev_tsc_to_s(uint64_t tsc);

void eventdev_worker_tsl_hist(struct worker_data *workers, unsigned num_workers,
                              uint64_t *tsl_hist, uint64_t hist_size, uint32_t stage);

void eventdev_worker_tl_hist(struct worker_data *workers, unsigned num_workers,
                             uint64_t *tl_hist, uint64_t hist_size);

void print_hist(uint64_t *arr, bool remove_empty, uint64_t size);

// finds the position last non zero element
uint64_t hist_len(uint64_t *arr, uint64_t size);

#endif
