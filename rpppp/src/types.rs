use std::time::Instant;

use glommio::channels::channel_mesh::FullMesh;

// These are the IDs that we assume the controller will get. We assert in the
// code that this is correct.
pub const DATA_MESH_CONTROLLER_ID: usize = 0;
pub const CONTROL_MESH_CONTROLLER_ID: usize = 0;

// The type of data that is sent over the channels
pub type ChannelElement<MsgData> = Box<Msg<MsgData>>;
pub type DataMesh<MsgData> = FullMesh<ChannelElement<MsgData>>;

/// Each message has a pipeline of functions that will be run on the message
/// data. This is the type used for these pipelines. Each pipeline will start
/// with the functions to run (at least one) and then end with (at least one)
/// [`None`]. Each pipeline will be [`PIPELINE_SIZE`] elements long.
pub type PipelineElement<MsgData> =
    Option<fn(&mut ChannelElement<MsgData>, usize)>;
/// The maximum length of each pipeline.
pub const PIPELINE_SIZE: usize = 5;
pub const MESH_CHANNEL_SIZE: usize = 8192;

/// This is the message that is sent between the generator, the controller and
/// the workers. It contains the message data, a chain of functions to call on
/// the data, and an index to which function to call.
#[derive(Clone)]
pub struct Msg<MsgData: 'static> {
    pub data: MsgData,
    pub pipeline: &'static [PipelineElement<MsgData>; PIPELINE_SIZE],
    pub pipeline_index: usize,
    pub timestamp: Instant,
}

/// Communicate certain stages of the process between shards.
#[derive(PartialEq, Debug)]
pub enum ControlMessage {
    WorkerInitializationComplete,
    Shutdown,
}
pub type ControlMesh = FullMesh<ControlMessage>;

/// software scheduler or distributed software scheduler
#[derive(Clone, PartialEq, Debug)]
pub enum SchedulingType {
    Sw,
    Dsw,
}
