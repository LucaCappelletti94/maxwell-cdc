#![no_main]

use libfuzzer_sys::fuzz_target;
use maxwell_cdc_fuzz::check_bytes;

fuzz_target!(|data: &[u8]| check_bytes(data));
