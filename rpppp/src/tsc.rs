#[cfg(debug_assertions)]
use rand::distributions::{Distribution, Uniform};

#[cfg_attr(debug_assertions, allow(unused_imports))]
use std::cmp::max;
use std::{
    hint::black_box,
    panic,
    time::{Duration, Instant},
};
use tsc_time::has_invariant_tsc;

/// This module is used to execute a loop for a specific amount of TSC cycles.
/// First, run [`calibration`] to get the number needed to run for a
/// certain number of TSC cycles. This result is then used with [`burn`]
/// to run for the original number of TSC cycles.
///
/// # Examples
///
/// ```
/// use rpppp::tsc;
///
/// let cycles = tsc::calibration(200);
/// // print how many TSC cycles that it on average took to run
/// println!("200 TSC cycles average was {}", tsc::cycles_average(cycles));
/// ```

/// This function is used to burn cycles and shouldn't be able to be
/// optimized away by the rust compiler.
pub fn burn(input: u64) {
    for _ in 0..input {
        // unsafe { asm!("nop") }
        black_box(0);
    }
}

/// Calculates the average number of cycles that it will take to perform
/// a certain number of iterations of [`burn`].
pub fn cycles_average(iterations: u64) -> u64 {
    let mut tot = 0;
    const MEASUREMENTS: u64 = if cfg!(debug_assertions) { 3 } else { 50_000 };
    let mut i = 0;

    panic::set_hook(Box::new(|_| {})); // remove panic printout

    // can't use for loop, will be optimized
    while i < MEASUREMENTS {
        let result = panic::catch_unwind(|| {
            // span sometimes panics, so another measurement must be run
            tsc_time::Duration::span(|| burn(iterations)).0.cycles()
        });

        if let Ok(latency) = result {
            tot += latency;
            i += 1;
        }
    }
    let _ = panic::take_hook(); // restore panic printout
    tot / MEASUREMENTS
}

/// Gets the frequency of the processors tsc-counter.
pub fn get_tsc_hz() -> u64 {
    let start = Instant::now();
    let start_tsc = tsc_time::Start::now();
    std::thread::sleep(Duration::from_secs(1));
    let elapsed = start.elapsed();
    let stop_tsc = tsc_time::Stop::now();
    ((stop_tsc - start_tsc).cycles() * 1_000_000) / elapsed.as_micros() as u64
}

#[cfg(debug_assertions)]
/// Finds a number which, when used in [`burn`], will take roughly the
/// provided amount of TSC cycles. The debug version is not very reliable
/// and has a less accurate result than the release version.
pub fn calibration(cycles: u64) -> u64 {
    // Non-invariant TSCs might produce unreliable results:
    assert!(has_invariant_tsc(), "The TSC is not invariant!");

    let mut rng = rand::thread_rng();
    let data = Uniform::from(0i32..(cycles / 20) as i32);

    let mut diff = cycles as i32;
    let mut iterations = 0;
    let mut i = 0;
    let mut resets = 0;
    while diff.abs() > 3 {
        iterations += diff / 2;

        // bad measurement needs restart. Uses random start to avoid always
        // guessing too many iterations and always having to restart
        if iterations < 0 {
            iterations = data.sample(&mut rng);
            resets += 1;
        }

        diff = cycles as i32 - cycles_average(iterations as u64) as i32;
        i += 1;
    }

    println!("iterations={}, i={i}, resets={resets}", iterations);
    assert!(iterations > 0);
    iterations as u64
}

#[cfg(not(debug_assertions))]
/// Finds a number which, when used in [`burn`], will take roughly the
/// provided amount of TSC cycles.
pub fn calibration(ideal_latency: u64) -> u64 {
    // Non-invariant TSCs might produce unreliable results:
    assert!(has_invariant_tsc(), "The TSC is not invariant!");

    let mut candidate_loops = ideal_latency;
    loop {
        let _ = cycles_average(candidate_loops); // warmup
        let actual_latency = cycles_average(candidate_loops) as u64;

        if actual_latency == ideal_latency {
            break;
        }

        candidate_loops = (ideal_latency * candidate_loops) / actual_latency;

        candidate_loops = max(1, candidate_loops);
    }

    candidate_loops
}

/// Calibrates the number of cycles needed to burn for the provided number of
/// TSC cycles.
pub fn calibrate(targets: &[u64]) -> Vec<u64> {
    loop {
        let cycles: Vec<_> =
            targets.iter().map(|target| calibration(*target)).collect();

        println!("tsc={:?}", cycles);

        let mut good_enough = true;
        let accuracy = 50; // higher value means the result must be more accurate

        cycles
            .iter()
            .zip(targets.iter())
            .for_each(|(cycle, target)| {
                let _ = cycles_average(*cycle);
                let average_cycles = cycles_average(*cycle);
                println!("{target} TSC cycles average was {average_cycles}");

                if average_cycles.abs_diff(*target) * accuracy > *target {
                    good_enough = false;
                    println!(
                        "Diff too great: {average_cycles} > {target} +- {}",
                        target / accuracy
                    );
                }
            });

        if good_enough {
            return cycles;
        }
        println!("Rerunning calibration\n");
    }
}
