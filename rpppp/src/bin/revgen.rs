use glommio::{LocalExecutorBuilder, Placement};
use rand::distributions::{Distribution, Uniform};
use rpppp::core::ShardReturnRequest;
use rpppp::histogram::Histogram;
use rpppp::tsc::{self, get_tsc_hz};
use std::time::Duration;
use std::{env, time::Instant};

const TEST_DURATION: Duration = Duration::from_secs(60);
const GENERATOR_CORE: u16 = 7;

const HISTOGRAM_MAX_LATENCY: usize = 100_000;

const TARGET_CYCLES: [u64; 3] = [1000, 1000, 1000];

static mut CYCLES_TO_BURN: Vec<u64> = Vec::new();

static mut HISTOGRAMS: Vec<Vec<Histogram<HISTOGRAM_MAX_LATENCY>>> = Vec::new();
static mut LATENCY_MEASUREMENT_TYPE: LatencyMeasurement =
    LatencyMeasurement::None;

#[derive(Clone)]
struct DataStruct {
    _data: f32,
}

enum LatencyMeasurement {
    None,
    Total,
    Switching,
}

type MsgData = DataStruct;
type PipeElem = rpppp::types::PipelineElement<MsgData>;

const PROCESS_PIPELINE: [PipeElem; rpppp::types::PIPELINE_SIZE] = [
    Some(burn_cycles),
    Some(burn_cycles),
    Some(burn_cycles),
    None,
    None,
];

fn burn_cycles(input: &mut Box<rpppp::types::Msg<MsgData>>, core_id: usize) {
    let msg = input.as_mut();
    let idx = msg.pipeline_index;
    unsafe {
        match LATENCY_MEASUREMENT_TYPE {
            LatencyMeasurement::Switching => {
                HISTOGRAMS[core_id][idx]
                    .add_value(msg.timestamp.elapsed().as_micros() as usize);
                tsc::burn(CYCLES_TO_BURN[idx]);
                msg.timestamp = Instant::now();
            }
            LatencyMeasurement::Total => {
                tsc::burn(CYCLES_TO_BURN[idx]);
                HISTOGRAMS[core_id][0]
                    .add_value(msg.timestamp.elapsed().as_micros() as usize);
            }
            LatencyMeasurement::None => {
                tsc::burn(CYCLES_TO_BURN[idx]);
            }
        }
    }
}

/// Generates the traffic that will be handled by rpppp
async fn generate_traffic(
    shard: ShardReturnRequest<MsgData>,
    stop_time: Instant,
) -> (rpppp::core::ShardReturnRequest<MsgData>, u64) {
    // Somehow it is faster to generate random data than to use 0, even though
    // the data isn't used
    let mut rng = rand::thread_rng();
    let data_distribution = Uniform::from(0_f32..=10_000_f32);

    let mut packets_sent = 0_u64;

    while stop_time > Instant::now() {
        shard
            .send_to(
                rpppp::core::round_robin_get_next_shard(
                    shard.nr_shards(),
                    packets_sent as usize,
                ),
                Box::new(rpppp::types::Msg {
                    data: DataStruct {
                        _data: data_distribution.sample(&mut rng),
                    },
                    pipeline: &PROCESS_PIPELINE,
                    pipeline_index: 0,
                    timestamp: Instant::now(),
                }),
            )
            .await
            .unwrap();
        packets_sent += 1;
    }

    (shard, packets_sent)
}

fn get_total_work_per_packet() -> u64 {
    let mut tot_work = 0;
    for work in TARGET_CYCLES {
        tot_work += work;
    }
    tot_work
}

fn main() {
    let starting_time = Instant::now();

    let (worker_cores, num_workers, num_cores, num_stages) = setup();

    // Run the simulation
    let (run_duration, packets_processed) = rpppp::core::start_dsw(
        worker_cores,
        GENERATOR_CORE,
        generate_traffic,
        Instant::now() + TEST_DURATION,
    );

    let s = format!(
        "Run duration {:.2}\tending time {:.2}\tdiff {:.2}",
        run_duration.as_secs_f32(),
        starting_time.elapsed().as_secs_f32(),
        starting_time.elapsed().as_secs_f32() - run_duration.as_secs_f32(),
    );
    eprintln!("\x1b[93m{s}\x1b[0m");
    println!("{s}");

    post_print(
        packets_processed,
        num_cores,
        run_duration,
        num_workers,
        num_stages,
    );
}

/// Set up and calibrate before run
fn setup() -> (Vec<u16>, usize, usize, usize) {
    let args: Vec<String> = env::args().collect();

    unsafe {
        LATENCY_MEASUREMENT_TYPE = match &args[1][..] {
            "0" => LatencyMeasurement::None,
            "1" => LatencyMeasurement::Total,
            "2" => LatencyMeasurement::Switching,
            x => {
                panic!("Latency measurement type {} not supported!", x);
            }
        };
    }

    // Ensure that no interrupts during calibration. They only happen on core 0
    LocalExecutorBuilder::new(Placement::Fixed(GENERATOR_CORE as usize))
        .name("calibrator")
        .spawn(move || async move {
            unsafe { CYCLES_TO_BURN = rpppp::tsc::calibrate(&TARGET_CYCLES) };
        })
        .unwrap()
        .join()
        .unwrap();

    let worker_cores: Vec<_> = args[2]
        .split(",")
        .map(|x| x.parse::<u16>().unwrap())
        .collect();
    let num_workers = worker_cores.len();
    let num_cores = num_workers + 1; // +1 from generator

    let num_histograms = num_workers;
    let num_stages = PROCESS_PIPELINE.iter().filter(|p| p.is_some()).count();

    unsafe {
        HISTOGRAMS.clear();
        for i in 0..num_histograms {
            HISTOGRAMS.push(Vec::new());
            for _ in 0..num_stages {
                HISTOGRAMS[i].push(Histogram::new())
            }
        }
    }

    println!("Using worker cores: {:?}", worker_cores);
    (worker_cores, num_workers, num_cores, num_stages)
}

/// Print the collected data
fn post_print(
    packets_processed: u64,
    num_cores: usize,
    run_duration: Duration,
    num_workers: usize,
    num_stages: usize,
) {
    match unsafe { &LATENCY_MEASUREMENT_TYPE } {
        LatencyMeasurement::None => {
            let done_work = packets_processed * get_total_work_per_packet();
            let ideal_work = (num_cores as f64
                * run_duration.as_secs_f64()
                * get_tsc_hz() as f64) as u64;

            println!(
                "# AVG\n{}\t{}\t{}\t{}",
                num_workers,
                packets_processed as f64 / run_duration.as_micros() as f64,
                done_work,
                ideal_work
            );
        }
        LatencyMeasurement::Total => {
            println!("# TL");
            let mut total_hist: Histogram<HISTOGRAM_MAX_LATENCY> =
                Histogram::new();
            unsafe {
                HISTOGRAMS
                    .iter()
                    .for_each(|hist| total_hist.add_data_from(&hist[0]));
            }
            total_hist.print(false);
            println!();
        }
        LatencyMeasurement::Switching => {
            for stage in 0..num_stages {
                println!("# TSL-{stage}");
                let mut total_hist: Histogram<HISTOGRAM_MAX_LATENCY> =
                    Histogram::new();
                unsafe {
                    HISTOGRAMS.iter().for_each(|hist| {
                        total_hist.add_data_from(&hist[stage])
                    });
                }
                total_hist.print(false);
                println!();
            }
        }
    }
}
