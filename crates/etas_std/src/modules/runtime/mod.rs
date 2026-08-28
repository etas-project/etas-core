pub mod approval;
pub mod budget;
pub mod checkpoint;
pub mod effects;
pub mod error;
pub mod limits;
pub mod time;
pub mod trace;

use crate::StdRegistryBuilder;

pub fn register(builder: &mut StdRegistryBuilder) {
    approval::register(builder);
    error::register(builder);
    limits::register(builder);
    budget::register(builder);
    time::register(builder);
    checkpoint::register(builder);
    trace::register(builder);
}
