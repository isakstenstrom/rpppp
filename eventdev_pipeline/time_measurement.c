#include <stdio.h>

#include <rte_atomic.h>
#include <rte_cycles.h>
#include <rte_lcore.h>

#include "pipeline_common.h"

#include "time_measurement.h"

void eventdev_busy_wait_ms(uint32_t ms)
{
    uint64_t deadline;

    deadline = rte_get_timer_cycles() + (rte_get_timer_hz() * ms) / 1000;

    while (deadline > rte_rdtsc_precise())
        ;
}

void eventdev_warm_up(void)
{
    eventdev_busy_wait_ms(100);
}

uint64_t
eventdev_s_to_tsc(double s)
{
    return s * rte_get_timer_hz();
}

double
eventdev_tsc_to_s(uint64_t tsc)
{
    return (double)tsc / rte_get_timer_hz();
}

static void __rte_noinline
__burn_loop(uint64_t num)
{
    uint64_t i;
    for (i = 0; i < num; i++)
        rte_compiler_barrier();
}

#define BENCHMARK_TIME (1)

static uint64_t
__benchmark_loops(uint64_t loops)
{
    uint64_t num_iter;
    uint64_t start;
    uint64_t end;
    uint64_t i;

    num_iter = (BENCHMARK_TIME * rte_get_tsc_hz()) / loops;

    start = rte_get_timer_cycles();

    for (i = 0; i < num_iter; i++)
        __burn_loop(loops);

    end = rte_get_timer_cycles();

    return (end - start) / num_iter;
}

#define MAX_ERROR (15)

uint64_t
eventdev_burn_tsc_to_loops(uint64_t tsc)
{
    uint64_t error;
    uint64_t ideal_latency = tsc;
    uint64_t candidate_loops = tsc;

    eventdev_warm_up();

    for (;;)
    {
        uint64_t actual_latency;

        actual_latency = __benchmark_loops(candidate_loops);

        error = RTE_MAX(actual_latency, ideal_latency) -
                RTE_MIN(actual_latency, ideal_latency);

        if (error < MAX_ERROR)
            break;

        candidate_loops =
            (ideal_latency * candidate_loops) / actual_latency;
    }

    printf("Using %" PRIu64 " loop iterations to burn ~%" PRIu64 " TSC (%.2f TSC/loop).\n", candidate_loops,
           ideal_latency, (double)ideal_latency / candidate_loops);

    return candidate_loops;
}

void eventdev_burn(uint64_t loops)
{
    __burn_loop(loops);
}

void eventdev_worker_tsl_hist(struct worker_data *workers, unsigned num_workers,
                           uint64_t *tsl_hist, uint64_t hist_size, uint32_t stage)
{
    for (unsigned worker = 0; worker < num_workers; worker++)
        for (unsigned i = 0; i < hist_size; i++)
            tsl_hist[i] += workers[worker].tsl[stage][i];
}

void eventdev_worker_tl_hist(struct worker_data *workers, unsigned num_workers,
                          uint64_t *tl_hist, uint64_t hist_size)
{
    for (unsigned worker = 0; worker < num_workers; worker++)
        for (unsigned i = 0; i < hist_size; i++)
            tl_hist[i] += workers[worker].tl[i];
}

void print_hist(uint64_t *arr, bool remove_empty, uint64_t size)
{
    unsigned i;
    if (remove_empty)
    {
        for (i = 0; i < size; i++)
            if (arr[i])
                printf("%u\t%lu\n", i, arr[i]);
    }
    else
        for (i = 0; i < size; i++)
            printf("%lu\n", arr[i]);
}

uint64_t hist_len(uint64_t *arr, uint64_t size)
{
    for (unsigned i = size; i > 0; --i)
        if (arr[i - 1])
            return i - 1;
    return 0;
}
