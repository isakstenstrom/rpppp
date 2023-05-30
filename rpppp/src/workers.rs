use std::{cell::RefCell, time::Instant};

use futures_lite::{future::ready, FutureExt};
use glommio::{
    channels::{
        channel_mesh::MeshBuilder,
        sharding::{Handler, HandlerResult, Sharded},
    },
    enclose,
    prelude::*,
    CpuSet,
};

use crate::controller::increment_round_robin;
use crate::types::{
    ChannelElement, ControlMesh, ControlMessage, DataMesh, SchedulingType,
    CONTROL_MESH_CONTROLLER_ID, DATA_MESH_CONTROLLER_ID, MESH_CHANNEL_SIZE,
};

type ShardRequest<MsgData> =
    Sharded<ChannelElement<MsgData>, RequestHandler<MsgData>>;

/// Handles the messages within the main sharding mesh.
#[derive(Clone)]
struct RequestHandler<MsgData: Send + 'static> {
    _nr_shards: usize,
    scheduling_type: SchedulingType,
    shard: *mut *mut ShardRequest<MsgData>,
    rr_counter: RefCell<usize>,
    stop_time: Instant,
}

impl<MsgData: Send + Clone> Handler<ChannelElement<MsgData>>
    for RequestHandler<MsgData>
{
    fn handle(
        &self,
        msg: ChannelElement<MsgData>,
        _src_shard: usize,
        _cur_shard: usize,
    ) -> HandlerResult {
        let shard_pointer = self.shard;
        let scheduling_type = self.scheduling_type.clone();
        let stop_time = self.stop_time;

        let next_shard =
            increment_round_robin(&self.rr_counter, self._nr_shards);

        // must detach or else deadlock is possible
        glommio::executor()
            .spawn_local(async move {
                let shard = unsafe {
                    shard_pointer.as_ref().unwrap().as_ref().unwrap()
                };
                if stop_time > Instant::now() {
                    RequestHandler::worker_function(
                        scheduling_type,
                        msg,
                        shard,
                        next_shard,
                        stop_time,
                    )
                    .await;
                } else {
                    // Send result back to the controller
                    shard.send_to(DATA_MESH_CONTROLLER_ID, msg).await.unwrap();
                }
            })
            .detach();

        ready(()).boxed_local()
    }
}

impl<MsgData: Send + Clone> RequestHandler<MsgData> {
    fn new(
        scheduling_type: SchedulingType,
        num_shards: usize,
        stop_time: Instant,
    ) -> Self {
        RequestHandler {
            _nr_shards: num_shards,
            scheduling_type,
            shard: Box::into_raw(Box::new(std::ptr::null_mut())),
            rr_counter: RefCell::from(0),
            stop_time,
        }
    }

    fn set_shard(&self, shard: &mut ShardRequest<MsgData>) {
        // Handler must be created before the shard, but the shard must be
        // saved in the handler, so therefore the unsafe pointer
        unsafe {
            *self.shard.as_mut().unwrap() = shard;
        }
    }

    /// Performs a function in the message pipeline before sending it on if
    /// more functions are left in the pipeline.
    async fn worker_function(
        scheduling_type: SchedulingType,
        mut message: ChannelElement<MsgData>,
        shard: &ShardRequest<MsgData>,
        next_shard: usize,
        _stop_time: Instant,
    ) {
        // Assume that there is work in the pipeline
        let func = message.pipeline[message.pipeline_index].unwrap();
        func(&mut message, shard.shard_id() - 1);
        message.pipeline_index += 1;

        if scheduling_type == SchedulingType::Sw
            || message.pipeline[message.pipeline_index].is_none()
        {
            // Send result back to the controller
            shard
                .send_to(DATA_MESH_CONTROLLER_ID, message)
                .await
                .unwrap();
            return;
        }

        // More work to do and uses DSW, so is sent to next shard
        shard.send_to(next_shard, message).await.unwrap();
    }
}

/// Finds the CPUs provided in `worker_cpus`
fn get_cpuset(worker_cpus: &[u16]) -> (usize, Vec<CpuSet>) {
    // Finds the specific CPUs
    let mut cpu_vec = Vec::new();
    for cpu in worker_cpus {
        cpu_vec
            .push(CpuSet::online().unwrap().filter(|l| l.cpu == *cpu as usize))
    }

    assert! {cpu_vec.iter().all(|set| !set.is_empty())};
    assert_eq!(worker_cpus.len(), cpu_vec.len());

    (cpu_vec.len(), cpu_vec)
}

/// Joins the shard mesh and sends and receives the required messages to the
/// controller
async fn worker_main<MsgData: Send + Clone>(
    scheduling_type: SchedulingType,
    control_mesh: &ControlMesh,
    nr_cores: usize,
    data_mesh: &DataMesh<MsgData>,
    stop_time: Instant,
) {
    let (control_sender, control_receiver) =
        control_mesh.clone().join().await.unwrap();

    let handler = RequestHandler::new(scheduling_type, nr_cores + 1, stop_time);

    // ignore the shard function
    let mut shard = Sharded::new(data_mesh.clone(), |_, _| 0, handler.clone())
        .await
        .unwrap();

    handler.set_shard(&mut shard);

    // Send to the mesh that this shard has initialized and will wait for the
    // signal to close
    control_sender
        .send_to(
            CONTROL_MESH_CONTROLLER_ID,
            ControlMessage::WorkerInitializationComplete,
        )
        .await
        .unwrap();

    // Wait for the controller to determine that execution is done
    control_receiver
        .recv_from(CONTROL_MESH_CONTROLLER_ID)
        .await
        .unwrap()
        .unwrap();

    shard.close().await;
}

/// Spawns workers in a mesh for the CPUs specified in `worker_cores`
pub fn spawn_workers<MsgData: Send + Clone>(
    scheduling_type: SchedulingType,
    worker_cores: &[u16],
    stop_time: Instant,
) -> (
    glommio::PoolThreadHandles<()>,
    DataMesh<MsgData>,
    ControlMesh,
) {
    let (nr_cores, cpu_vec) = get_cpuset(worker_cores);

    // Sends the regular messages
    let data_mesh =
        MeshBuilder::full(nr_cores + 1, MESH_CHANNEL_SIZE / nr_cores);
    // Used for sending messages about the execution
    let control_mesh = MeshBuilder::full(nr_cores + 1, 1);

    let pool = LocalExecutorPoolBuilder::new(PoolPlacement::Custom(cpu_vec))
        .name("Workers")
        .on_all_shards(enclose!((data_mesh, control_mesh) move || async move {
            worker_main(scheduling_type, &control_mesh, nr_cores, &data_mesh, stop_time).await;
        }))
        .unwrap();

    (pool, data_mesh, control_mesh)
}
