use futures_lite::{future::ready, FutureExt};
use glommio::{
    channels::{
        channel_mesh::{self, Receivers},
        sharding::{Handler, HandlerResult, Sharded},
        shared_channel,
    },
    timer::sleep,
};
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

use crate::types::{
    ChannelElement, ControlMesh, ControlMessage, DataMesh,
    CONTROL_MESH_CONTROLLER_ID, DATA_MESH_CONTROLLER_ID,
};

pub type ShardReturnRequest<MsgData> =
    Sharded<ChannelElement<MsgData>, ReturnRequestHandler<MsgData>>;

/// Gets the next shard
pub fn round_robin_get_next_shard(
    nr_shards: usize,
    prev_shard: usize,
) -> usize {
    // Ignores shard 0, which is the scheduler
    (prev_shard % (nr_shards - 1)) + 1
}

/// Increments rr based on nr_shards
pub fn increment_round_robin(rr: &RefCell<usize>, nr_shards: usize) -> usize {
    let mut rr_counter = rr.borrow_mut();
    *rr_counter = round_robin_get_next_shard(nr_shards, *rr_counter);
    *rr_counter
}

#[derive(Clone)]
pub struct ReturnRequestHandler<MsgData: Send + 'static> {
    shard: *mut *mut ShardReturnRequest<MsgData>,
    pub return_counter: RefCell<Rc<u64>>,
    pub processed_packets: RefCell<Rc<u64>>,
    rr_counter: RefCell<usize>,
    stop_time: Instant,
}

impl<MsgData: Send + Clone> Handler<ChannelElement<MsgData>>
    for ReturnRequestHandler<MsgData>
{
    fn handle(
        &self,
        message: ChannelElement<MsgData>,
        _src_shard: usize,
        _cur_shard: usize,
    ) -> HandlerResult {
        if self.stop_time > Instant::now() {
            // Still work to do, so sent it to a worker
            if message.pipeline[message.pipeline_index].is_some() {
                let shard_pointer = self.shard;

                let nr_shards = unsafe {
                    shard_pointer
                        .as_ref()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .nr_shards()
                };

                let next_shard =
                    increment_round_robin(&self.rr_counter, nr_shards);

                return Box::pin(async move {
                    let shard = unsafe {
                        shard_pointer.as_ref().unwrap().as_ref().unwrap()
                    };
                    shard.send_to(next_shard, message).await.unwrap();
                });
            }
            // Counts how many messages have been sent
            unsafe {
                *Rc::get_mut_unchecked(
                    &mut self.processed_packets.borrow_mut(),
                ) += 1
            };
        }
        // Counts how many messages have been sent
        unsafe {
            *Rc::get_mut_unchecked(&mut self.return_counter.borrow_mut()) += 1
        };
        ready(()).boxed_local()
    }
}

impl<MsgData: Send + Clone> ReturnRequestHandler<MsgData> {
    fn new(stop_time: Instant) -> Self {
        ReturnRequestHandler {
            shard: Box::into_raw(Box::new(std::ptr::null_mut())),
            return_counter: RefCell::new(Rc::new(0)),
            processed_packets: RefCell::new(Rc::new(0)),
            rr_counter: RefCell::new(0),
            stop_time,
        }
    }

    fn set_shard(&mut self, shard: &mut ShardReturnRequest<MsgData>) {
        // Handler must be created before the shard, but the shard must be
        // saved in the handler, so therefore the unsafe pointer
        unsafe {
            *self.shard.as_mut().unwrap() = shard;
        }
    }
}

/// Waits for all workers to have finished their initialization
async fn wait_for_worker_init(control_receiver: &Receivers<ControlMessage>) {
    for peer in 0..control_receiver.nr_producers() {
        if peer != control_receiver.peer_id() {
            let message =
                control_receiver.recv_from(peer).await.unwrap().unwrap();
            assert_eq!(message, ControlMessage::WorkerInitializationComplete);
        }
    }
}

/// Initializes the controller and the communication meshes
pub async fn controller_init<MsgData: Send + Clone>(
    control_mesh: ControlMesh,
    data_mesh: DataMesh<MsgData>,
    stop_time: Instant,
) -> (
    channel_mesh::Senders<ControlMessage>,
    ReturnRequestHandler<MsgData>,
    ShardReturnRequest<MsgData>,
) {
    let (control_sender, control_receiver) = control_mesh.join().await.unwrap();
    // We assume a fixed mesh id for the controller.
    assert_eq!(
        control_sender.peer_id(),
        CONTROL_MESH_CONTROLLER_ID,
        "Control mesh controller doesn't have the assumed ID"
    );

    let mut handler = ReturnRequestHandler::new(stop_time);

    let mut shard: ShardReturnRequest<MsgData> =
        Sharded::new(data_mesh, |_, _| 0, handler.clone())
            .await
            .unwrap();
    // We assume a fixed shard id for the controller.
    assert_eq!(
        shard.shard_id(),
        DATA_MESH_CONTROLLER_ID,
        "Data mesh controller doesn't have the assumed ID"
    );

    handler.set_shard(&mut shard);

    wait_for_worker_init(&control_receiver).await;
    (control_sender, handler, shard)
}

/// The main loop of the controller, where data is received from the
/// generator and dispatched to the workers
async fn send_receive<MsgData: Send + Clone>(
    task_receiver: shared_channel::ConnectedReceiver<ChannelElement<MsgData>>,
    shard: &ShardReturnRequest<MsgData>,
    handler: &ReturnRequestHandler<MsgData>,
) -> Duration {
    let mut sent_messages = 0u64;
    let start_timestamp = Instant::now();
    while let Some(task) = task_receiver.recv().await {
        if handler.stop_time > Instant::now() {
            let next_shard =
                increment_round_robin(&handler.rr_counter, shard.nr_shards());

            shard.send_to(next_shard, task).await.unwrap();
            sent_messages += 1;
        }
    }
    let run_duration = start_timestamp.elapsed();

    while **handler.return_counter.borrow() != sent_messages {
        sleep(Duration::from_millis(10)).await;
    }

    run_duration
}

/// Closes all worker channels
pub async fn controller_cleanup<MsgData: Send + Clone>(
    control_sender: channel_mesh::Senders<ControlMessage>,
    mut shard: ShardReturnRequest<MsgData>,
) {
    for i in 1..control_sender.nr_consumers() {
        control_sender
            .send_to(i, ControlMessage::Shutdown)
            .await
            .unwrap();
    }

    shard.close().await;
}

/// Initializes and runs the controller. Returns the time it took for the
/// workers to process all the messages.
pub async fn run_controller<MsgData: Send + Clone>(
    task_receiver: shared_channel::SharedReceiver<ChannelElement<MsgData>>,
    data_mesh: DataMesh<MsgData>,
    control_mesh: ControlMesh,
    stop_time: Instant,
) -> (Duration, u64) {
    let (control_sender, handler, shard) =
        controller_init(control_mesh, data_mesh, stop_time).await;

    let task_receiver = task_receiver.connect().await;
    // Send and receive data
    let run_duration = send_receive(task_receiver, &shard, &handler).await;

    let processed_packets = **handler.processed_packets.borrow();
    controller_cleanup(control_sender, shard).await;

    (run_duration, processed_packets)
}
