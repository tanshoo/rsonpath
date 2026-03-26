use rsonpath_benchmarks::prelude::*;

pub fn depth_3_props_4_len_10_10(c: &mut Criterion) -> Result<(), BenchmarkError> {
    let schema = "./data/schemas/depth_3_props_4_len_10_10_seed_42_schema.json";

    let benchset = Benchset::new(
        "validator::depth_3_props_4_len_10_10",
        dataset::depth_3_props_4_len_10_10_seed_42(),
    )?
    .do_not_measure_file_load_time()
    .add_target(BenchTarget::Rsonschema(schema))?
    .add_target(BenchTarget::Boon(schema))?
    .add_target(BenchTarget::JsonSchema(schema))?
    .add_target(BenchTarget::Dja(schema))?
    .add_target(BenchTarget::SpawnBaseline(schema))?
    .finish();

    benchset.run(c);

    Ok(())
}

pub fn depth_3_props_6_len_8_12(c: &mut Criterion) -> Result<(), BenchmarkError> {
    let schema = "./data/schemas/depth_3_props_6_len_8_12_seed_42_schema.json";

    let benchset = Benchset::new(
        "validator::depth_3_props_6_len_8_12",
        dataset::depth_3_props_6_len_8_12_seed_42(),
    )?
    .do_not_measure_file_load_time()
    .add_target(BenchTarget::Rsonschema(schema))?
    .add_target(BenchTarget::Boon(schema))?
    .add_target(BenchTarget::JsonSchema(schema))?
    .add_target(BenchTarget::Dja(schema))?
    .add_target(BenchTarget::SpawnBaseline(schema))?
    .finish();

    benchset.run(c);

    Ok(())
}

pub fn depth_3_props_8_len_8_12(c: &mut Criterion) -> Result<(), BenchmarkError> {
    let schema = "./data/schemas/depth_3_props_8_len_8_12_seed_42_schema.json";

    let benchset = Benchset::new(
        "validator::depth_3_props_8_len_8_12",
        dataset::depth_3_props_8_len_8_12_seed_42(),
    )?
    .do_not_measure_file_load_time()
    .add_target(BenchTarget::Rsonschema(schema))?
    .add_target(BenchTarget::Boon(schema))?
    .add_target(BenchTarget::JsonSchema(schema))?
    .add_target(BenchTarget::Dja(schema))?
    .add_target(BenchTarget::SpawnBaseline(schema))?
    .finish();

    benchset.run(c);

    Ok(())
}

pub fn baseline_depth_3_props_4_len_10(c: &mut Criterion) -> Result<(), BenchmarkError> {
    let schema = "./data/schemas/baseline_depth_3_props_4_len_10_schema.json";

    let benchset = Benchset::new(
        "validator::baseline_depth_3_props_4_len_10",
        dataset::baseline_depth_3_props_4_len_10(),
    )?
    .do_not_measure_file_load_time()
    .add_target(BenchTarget::Rsonschema(schema))?
    .finish();

    benchset.run(c);

    Ok(())
}

pub fn baseline_depth_3_props_6_len_10(c: &mut Criterion) -> Result<(), BenchmarkError> {
    let schema = "./data/schemas/baseline_depth_3_props_6_len_10_schema.json";

    let benchset = Benchset::new(
        "validator::baseline_depth_3_props_6_len_10",
        dataset::baseline_depth_3_props_6_len_10(),
    )?
    .do_not_measure_file_load_time()
    .add_target(BenchTarget::Rsonschema(schema))?
    .finish();

    benchset.run(c);

    Ok(())
}

pub fn baseline_depth_3_props_8_len_10(c: &mut Criterion) -> Result<(), BenchmarkError> {
    let schema = "./data/schemas/baseline_depth_3_props_8_len_10_schema.json";

    let benchset = Benchset::new(
        "validator::baseline_depth_3_props_8_len_10",
        dataset::baseline_depth_3_props_8_len_10(),
    )?
    .do_not_measure_file_load_time()
    .add_target(BenchTarget::Rsonschema(schema))?
    .finish();

    benchset.run(c);

    Ok(())
}

benchsets!(
    validator_benches,
    depth_3_props_4_len_10_10,
    depth_3_props_6_len_8_12,
    depth_3_props_8_len_8_12,
    baseline_depth_3_props_4_len_10,
    baseline_depth_3_props_6_len_10,
    baseline_depth_3_props_8_len_10
);
