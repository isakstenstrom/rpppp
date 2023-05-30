use futures::Future;
use glommio::{channels::shared_channel, prelude::*, timer, CpuSet};
use std::time::{Duration, Instant};

use crate::{
    controller,
    types::{ChannelElement, SchedulingType},
    workers,
};

pub use crate::controller::{round_robin_get_next_shard, ShardReturnRequest};

/// Verifies that the provided cores are valid and contains no duplicates.
fn verify_core_layout(
    worker_cores: &Vec<u16>,
    generator_core: Option<u16>,
    controller_core: Option<u16>,
) {
    assert_ne!(worker_cores.len(), 0, "Must have at least one worker core.");

    let mut cores: Vec<_> = worker_cores.clone();

    if let Some(generator) = generator_core {
        cores.push(generator);
    }
    if let Some(controller) = controller_core {
        cores.push(controller);
    }

    let num_cores = cores.len();

    cores.sort();
    Vec::dedup(&mut cores);
    assert_eq!(
        cores.len(),
        num_cores,
        "CPU cores used contains duplicates."
    );

    let num_avail_cores = CpuSet::online().unwrap().len();

    for core in cores {
        assert!(
            (core as usize) < num_avail_cores,
            "Core {core} is not available."
        );
    }
}

/// Starts the RPPPP processes using the DSW scheduler.
/// - `worker_cores` are the cores that will be allocated workers
/// - `generator_core` is the core that will generate data and send it to the
///   workers
/// - `generator` is a function that will generate the data for the test, and
///   send it to the mesh for further processing.
pub fn start_dsw<G, F, MsgData: Send + Clone + 'static>(
    worker_cores: Vec<u16>,
    generator_core: u16,
    generator: G,
    stop_time: Instant,
) -> (Duration, u64)
where
    G: Fn(ShardReturnRequest<MsgData>, Instant) -> F + Send + 'static,
    F: Future<Output = (ShardReturnRequest<MsgData>, u64)>,
{
    verify_core_layout(&worker_cores, Some(generator_core), None);

    let generator_handle =
        LocalExecutorBuilder::new(Placement::Fixed(generator_core as usize))
            .name("generator")
            .spawn(move || async move {
                let (worker_pool, data_mesh, control_mesh) =
                    workers::spawn_workers(
                        SchedulingType::Dsw,
                        &worker_cores,
                        stop_time,
                    );

                let (control_sender, handler, shard) =
                    controller::controller_init(
                        control_mesh,
                        data_mesh,
                        stop_time,
                    )
                    .await;

                let start_timestamp = Instant::now();
                // Send and receive data
                let (shard, num_messages) = generator(shard, stop_time).await;
                let run_duration = start_timestamp.elapsed();

                // wait until all messages have returned
                while **handler.return_counter.borrow() != num_messages {
                    timer::sleep(Duration::from_millis(10)).await;
                }

                let processed_packets = **handler.processed_packets.borrow();

                controller::controller_cleanup(control_sender, shard).await;
                worker_pool.join_all();
                (run_duration, processed_packets)
            })
            .unwrap();

    generator_handle.join().unwrap()
}

/// Starts the RPPPP processes using the SW scheduler.
/// - `worker_cores` are the cores that will be allocated workers
/// - `generator_core` is the core that will generate data
/// - `controller_core` is the core that will receive data and send it to the
///   workers
/// - `generator` is a function that will generate the data for the test
pub fn start_sw<G, F, MsgData: Send + Clone + 'static>(
    worker_cores: Vec<u16>,
    generator_core: u16,
    controller_core: u16,
    generator: G,
    stop_time: Instant,
) -> (Duration, u64)
where
    G: FnOnce(
            shared_channel::SharedSender<ChannelElement<MsgData>>,
            Instant,
        ) -> F
        + Send
        + 'static,
    F: Future<Output = ()> + 'static,
{
    verify_core_layout(
        &worker_cores,
        Some(generator_core),
        Some(controller_core),
    );

    // For sending data from the generator to the controller
    let (task_sender, task_receiver) = shared_channel::new_bounded(10000);

    let controller_handle =
        LocalExecutorBuilder::new(Placement::Fixed(controller_core as usize))
            .name("controller")
            .spawn(move || async move {
                let (worker_pool, data_mesh, control_mesh) =
                    workers::spawn_workers(
                        SchedulingType::Sw,
                        &worker_cores,
                        stop_time,
                    );

                let run_duration_and_processed_packets =
                    controller::run_controller(
                        task_receiver,
                        data_mesh,
                        control_mesh,
                        stop_time,
                    )
                    .await;

                worker_pool.join_all();
                run_duration_and_processed_packets
            })
            .unwrap();

    let generator_handle =
        LocalExecutorBuilder::new(Placement::Fixed(generator_core as usize))
            .name("generator")
            .spawn(move || generator(task_sender, stop_time))
            .unwrap();

    generator_handle.join().unwrap();
    controller_handle.join().unwrap()
}
