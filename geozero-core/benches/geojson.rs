use criterion::{criterion_group, criterion_main, Criterion};
use geozero_api::DebugReader;
use geozero_core::read_json;
use std::fs::File;
use std::io::BufReader;

fn read_geojson_file() -> std::result::Result<(), std::io::Error> {
    // Comparison: time ogrinfo -al -so canada.json canada >/dev/null
    // ogrinfo: 150ms <-> geozero: 43ms
    let fin = File::open("canada.json")?;
    let filein = BufReader::new(fin);
    let reader = DebugReader {};
    read_json(filein, reader).unwrap();
    Ok(())
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("read_geojson_file", |b| b.iter(|| read_geojson_file()));
}

criterion_group!(name=benches; config=Criterion::default().sample_size(10); targets=criterion_benchmark);
criterion_main!(benches);
