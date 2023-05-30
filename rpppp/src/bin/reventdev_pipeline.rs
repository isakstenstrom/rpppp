use glommio::{channels::shared_channel, LocalExecutorBuilder, Placement};
use rand::distributions::{Distribution, Uniform};
use rpppp::{histogram::Histogram, tsc, types::PIPELINE_SIZE};
use std::{
    env,
    time::{Duration, Instant},
};

const TEST_DURATION: Duration = Duration::from_secs(60);
const GENERATOR_CORE: u16 = 7;
const CONTROLLER_CORE: u16 = 5;

const HISTOGRAM_MAX_LATENCY: usize = 100_000;

const TARGET_CYCLES: u64 = 1000;

static mut CYCLES_TO_BURN: u64 = 0;

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

const PROCESS_PIPELINE: [rpppp::types::PipelineElement<DataStruct>;
    rpppp::types::PIPELINE_SIZE] = [
    Some(burn_cycles),
    Some(burn_cycles),
    Some(burn_cycles),
    None,
    None,
];

fn burn_cycles(
    _input: &mut Box<rpppp::types::Msg<DataStruct>>,
    _core_id: usize,
) {
    let msg = _input.as_mut();
    unsafe {
        if let LatencyMeasurement::Switching = LATENCY_MEASUREMENT_TYPE {
            HISTOGRAMS[_core_id][msg.pipeline_index]
                .add_value(msg.timestamp.elapsed().as_micros() as usize);
        }
    }
    tsc::burn(unsafe { CYCLES_TO_BURN });

    unsafe {
        if let LatencyMeasurement::Switching = LATENCY_MEASUREMENT_TYPE {
            _input.as_mut().timestamp = Instant::now();
        } else if let LatencyMeasurement::Total = LATENCY_MEASUREMENT_TYPE {
            if msg.pipeline_index < PIPELINE_SIZE
                || msg.pipeline[msg.pipeline_index].is_none()
            {
                HISTOGRAMS[_core_id][0]
                    .add_value(msg.timestamp.elapsed().as_micros() as usize);
            }
        }
    }
}

async fn generate_traffic(
    task_sender: shared_channel::SharedSender<
        Box<rpppp::types::Msg<DataStruct>>,
    >,
    stop_time: Instant,
) {
    let task_sender = task_sender.connect().await;

    let mut rng = rand::thread_rng();
    let data_distribution = Uniform::from(0_f32..=10_000_f32);

    while stop_time > Instant::now() {
        task_sender
            .send(Box::new(rpppp::types::Msg {
                data: DataStruct {
                    _data: data_distribution.sample(&mut rng),
                },
                pipeline: &PROCESS_PIPELINE,
                pipeline_index: 0,
                timestamp: Instant::now(),
            }))
            .await
            .unwrap();
    }
}

fn main() {
    let starting_time = Instant::now();
    let args: Vec<String> = env::args().collect();

    unsafe {
        LATENCY_MEASUREMENT_TYPE = match &args[1][..] {
            "1" => LatencyMeasurement::Total,
            "2" => LatencyMeasurement::Switching,
            _ => LatencyMeasurement::None,
        };
    }

    // Ensure that no interrupts during calibration. They only happen on core 0
    LocalExecutorBuilder::new(Placement::Fixed(GENERATOR_CORE as usize))
        .name("calibrator")
        .spawn(move || async move {
            unsafe {
                CYCLES_TO_BURN = rpppp::tsc::calibrate(&[TARGET_CYCLES])[0]
            };
        })
        .unwrap()
        .join()
        .unwrap();

    let worker_cores: Vec<_> = args[2]
        .split(',')
        .map(|x| x.parse::<u16>().unwrap())
        .collect();
    let num_workers = worker_cores.len();
    let num_cores = num_workers + 2; // Generator and scheduler

    let num_histograms = num_workers;
    let num_stages = PROCESS_PIPELINE.iter().filter(|p| p.is_some()).count();

    unsafe {
        HISTOGRAMS.clear();
        HISTOGRAMS.resize_with(num_histograms, Default::default);
        for h in HISTOGRAMS.iter_mut() {
            h.resize_with(num_stages, Histogram::new);
        }
    }

    println!("Using worker cores: {:?}", worker_cores);

    let (run_duration, packets_processed) = rpppp::core::start_sw(
        worker_cores,
        GENERATOR_CORE,
        CONTROLLER_CORE,
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

    match unsafe { &LATENCY_MEASUREMENT_TYPE } {
        LatencyMeasurement::None => {
            let done_work =
                packets_processed * TARGET_CYCLES * num_stages as u64;

            let ideal_work = ((num_cores as f64)
                * run_duration.as_secs_f64()
                * (tsc::get_tsc_hz() as f64))
                as usize;

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
