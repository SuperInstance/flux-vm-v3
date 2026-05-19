//! Real-world benchmarks for FLUX VM v3 using Criterion.
//!
//! Scenarios:
//! 1. Aviation TCAS: 100 aircraft × 8 constraints × 10 updates
//! 2. AV Sensor Fusion: 50 sensors × 4 constraints × 100Hz
//! 3. Nuclear Reactor: 200 sensors × 8 constraints × 1000Hz
//! 4. Maritime Fleet: 1000 vessels × 4 constraints × 1Hz
//! 5. Energy Grid: 10K points × 4 constraints × 50Hz
//! 6. ICU Monitoring: 50 patients × 8 vitals × 100Hz

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use flux_vm_v3::*;

// ── Scenario generators ──

fn aviation_constraints() -> Vec<check::Constraint> {
    vec![
        check::Constraint::new(0, 45000, "altitude"),
        check::Constraint::new(100, 600, "airspeed"),
        check::Constraint::new(0, 360, "heading"),
        check::Constraint::new(-6000, 6000, "vert_rate"),
        check::Constraint::new(0, 300, "distance"),
        check::Constraint::new(0, 1200, "closure_rate"),
        check::Constraint::new(0, 360, "bearing"),
        check::Constraint::new(-1, 1, "intent"),
    ]
}

fn nuclear_constraints() -> Vec<check::Constraint> {
    let tags = ["neutron_flux", "coolant_temp", "pressure", "flow_rate",
                "valve_pos", "steam_pressure", "containment_temp", "radiation"];
    let mut cs = Vec::new();
    for (i, tag) in tags.iter().enumerate() {
        cs.push(check::Constraint::new(0, (i as i32 + 1) * 1000, tag));
    }
    cs
}

fn grid_constraints() -> Vec<check::Constraint> {
    vec![
        check::Constraint::new(95, 105, "voltage"),    // 0.95-1.05 × 100
        check::Constraint::new(4900, 5100, "frequency"), // 49-51 Hz × 100
        check::Constraint::new(0, 50000, "load"),
        check::Constraint::new(-95, 95, "phase"),
    ]
}

// ── Benchmarks ──

fn bench_aviation_single(c: &mut Criterion) {
    let constraints = aviation_constraints();
    let values: Vec<i32> = (0..8).map(|i| {
        let c = &constraints[i];
        (c.lo + c.hi) / 2
    }).collect();

    c.bench_function("aviation_single_aircraft", |b| {
        b.iter(|| {
            for (i, c) in constraints.iter().enumerate() {
                black_box(c.check(black_box(values[i])));
            }
        })
    });
}

fn bench_aviation_batch(c: &mut Criterion) {
    let constraints = aviation_constraints();
    let n_aircraft = 100;
    let values: Vec<Vec<i32>> = (0..n_aircraft)
        .map(|_| constraints.iter().map(|c| (c.lo + c.hi) / 2).collect())
        .collect();

    c.bench_function("aviation_100_aircraft", |b| {
        b.iter(|| {
            for vals in &values {
                for (i, c) in constraints.iter().enumerate() {
                    black_box(c.check(black_box(vals[i])));
                }
            }
        })
    });
}

fn bench_nuclear_parallel(c: &mut Criterion) {
    let constraints = nuclear_constraints();
    let n_sensors = 200u64;
    let values: Vec<i32> = (0..n_sensors * 8)
        .map(|i| {
            let c = &constraints[i as usize % constraints.len()];
            (c.lo + c.hi) / 2
        })
        .collect();

    c.bench_function("nuclear_200_sensors_serial", |b| {
        b.iter(|| {
            for chunk in values.chunks(8) {
                for (i, &v) in chunk.iter().enumerate() {
                    black_box(constraints[i].check(black_box(v)));
                }
            }
        })
    });
}

fn bench_grid_throughput(c: &mut Criterion) {
    let constraints = grid_constraints();
    let n_points = 10000u64;

    let mut group = c.benchmark_group("energy_grid");
    group.throughput(Throughput::Elements(n_points * 4));

    group.bench_function("10k_points", |b| {
        b.iter(|| {
            for _ in 0..n_points {
                for c in &constraints {
                    black_box(c.check(black_box((c.lo + c.hi) / 2)));
                }
            }
        })
    });

    group.finish();
}

fn bench_parallel_batch(c: &mut Criterion) {
    let mut batch = parallel::ParallelBatch::new();
    for _ in 0..100_000 {
        let v = rand_value();
        batch.add(parallel::ConstraintCheck::new(v, 0, 1000));
    }

    c.bench_function("parallel_100k_checks", |b| {
        b.iter(|| {
            black_box(batch.dispatch())
        })
    });
}

fn bench_vm_constraint_check(c: &mut Criterion) {
    let mut vm = FluxVM::new();
    vm.load_constraints(aviation_preset());

    c.bench_function("vm_aviation_check", |b| {
        b.iter(|| {
            black_box(vm.benchmark(black_box(1000)))
        })
    });
}

fn bench_icu_monitoring(c: &mut Criterion) {
    let constraints = vec![
        check::Constraint::new(40, 200, "hr"),
        check::Constraint::new(80, 100, "spo2"),
        check::Constraint::new(60, 180, "bp_sys"),
        check::Constraint::new(40, 100, "bp_dia"),
        check::Constraint::new(34, 40, "temp"),
        check::Constraint::new(8, 30, "resp"),
        check::Constraint::new(20, 50, "etco2"),
        check::Constraint::new(3, 15, "gcs"),
    ];

    let n_patients = 50u64;
    let values: Vec<[i32; 8]> = (0..n_patients)
        .map(|_| {
            let mut arr = [0i32; 8];
            for (i, c) in constraints.iter().enumerate() {
                arr[i] = (c.lo + c.hi) / 2;
            }
            arr
        })
        .collect();

    let mut group = c.benchmark_group("icu_monitoring");
    group.throughput(Throughput::Elements(n_patients * 8));

    group.bench_function("50_patients", |b| {
        b.iter(|| {
            for vals in &values {
                for (i, c) in constraints.iter().enumerate() {
                    black_box(c.check(black_box(vals[i])));
                }
            }
        })
    });

    group.finish();
}

fn bench_maritime_fleet(c: &mut Criterion) {
    let constraints = vec![
        check::Constraint::new(-9000, 9000, "lat"),
        check::Constraint::new(-18000, 18000, "lon"),
        check::Constraint::new(0, 50, "speed"),
        check::Constraint::new(0, 3600, "heading"),
    ];

    let mut group = c.benchmark_group("maritime");
    for size in [100u64, 1000, 10000] {
        group.throughput(Throughput::Elements(size * 4));
        group.bench_with_input(BenchmarkId::new("fleet", size), &size, |b, &sz| {
            b.iter(|| {
                for _ in 0..sz {
                    for c in &constraints {
                        black_box(c.check(black_box((c.lo + c.hi) / 2)));
                    }
                }
            })
        });
    }
    group.finish();
}

fn rand_value() -> i32 {
    use std::time::SystemTime;
    let ns = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    (ns as i32) % 1000
}

criterion_group!(
    benches,
    bench_aviation_single,
    bench_aviation_batch,
    bench_nuclear_parallel,
    bench_grid_throughput,
    bench_parallel_batch,
    bench_vm_constraint_check,
    bench_icu_monitoring,
    bench_maritime_fleet,
);

criterion_main!(benches);
