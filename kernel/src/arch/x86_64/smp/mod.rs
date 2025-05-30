use crate::{arch::limine::MP_REQUEST, trace};

pub fn init() {
    let mp_response = MP_REQUEST.get_response().unwrap();
    let cpu_count = mp_response.cpus().len();
    trace!("CPU Count: {}", cpu_count);
}
