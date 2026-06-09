use rsonpath_benchmarks::prelude::*;

pub fn arr_5_depth_3_props_4_len_10_10(c: &mut Criterion) -> Result<(), BenchmarkError> {
    let schema = "./data/schemas/arr_5_depth_3_props_4_len_10_10_seed_42_schema.json";

    let benchset = Benchset::new(
        "validator::arr_5_depth_3_props_4_len_10_10",
        dataset::arr_5_depth_3_props_4_len_10_10_seed_42(),
    )?
    .do_not_measure_file_load_time()
    .add_all_validator_targets(schema)?
    .finish();

    benchset.run(c);

    Ok(())
}

pub fn arr_5_depth_3_props_6_len_8_12(c: &mut Criterion) -> Result<(), BenchmarkError> {
    let schema = "./data/schemas/arr_5_depth_3_props_6_len_8_12_seed_42_schema.json";

    let benchset = Benchset::new(
        "validator::arr_5_depth_3_props_6_len_8_12",
        dataset::arr_5_depth_3_props_6_len_8_12_seed_42(),
    )?
    .do_not_measure_file_load_time()
    .add_all_validator_targets(schema)?
    .finish();

    benchset.run(c);

    Ok(())
}

pub fn arr_5_depth_3_props_8_len_8_12(c: &mut Criterion) -> Result<(), BenchmarkError> {
    let schema = "./data/schemas/arr_5_depth_3_props_8_len_8_12_seed_42_schema.json";

    let benchset = Benchset::new(
        "validator::arr_5_depth_3_props_8_len_8_12",
        dataset::arr_5_depth_3_props_8_len_8_12_seed_42(),
    )?
    .do_not_measure_file_load_time()
    .add_all_validator_targets(schema)?
    .finish();

    benchset.run(c);

    Ok(())
}

benchsets!(
    validator_benches,
    arr_5_depth_3_props_4_len_10_10,
    arr_5_depth_3_props_6_len_8_12,
    arr_5_depth_3_props_8_len_8_12
);
