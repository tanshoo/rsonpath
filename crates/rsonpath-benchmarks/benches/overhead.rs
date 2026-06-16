use rsonpath_benchmarks::prelude::*;

pub fn arr_5_depth_3_props_4_len_10_10(c: &mut Criterion) -> Result<(), BenchmarkError> {
    let schema = "./data/schemas/generated/arr_5_depth_3_props_4_len_10_10_seed_42_schema.json";

    let benchset = Benchset::new(
        "overhead::arr_5_depth_3_props_4_len_10_10",
        dataset::arr_5_depth_3_props_4_len_10_10_seed_42(),
    )?
    .add_rsonschema_with_all_modes(schema)?
    .finish();

    benchset.run(c);

    Ok(())
}

pub fn arr_5_depth_3_props_6_len_8_12(c: &mut Criterion) -> Result<(), BenchmarkError> {
    let schema = "./data/schemas/generated/arr_5_depth_3_props_6_len_8_12_seed_42_schema.json";

    let benchset = Benchset::new(
        "overhead::arr_5_depth_3_props_6_len_8_12",
        dataset::arr_5_depth_3_props_6_len_8_12_seed_42(),
    )?
    .add_rsonschema_with_all_modes(schema)?
    .finish();

    benchset.run(c);

    Ok(())
}

pub fn arr_5_depth_3_props_8_len_8_12(c: &mut Criterion) -> Result<(), BenchmarkError> {
    let schema = "./data/schemas/generated/arr_5_depth_3_props_8_len_8_12_seed_42_schema.json";

    let benchset = Benchset::new(
        "overhead::arr_5_depth_3_props_8_len_8_12",
        dataset::arr_5_depth_3_props_8_len_8_12_seed_42(),
    )?
    .add_rsonschema_with_all_modes(schema)?
    .finish();

    benchset.run(c);

    Ok(())
}
pub fn corporations_10k(c: &mut Criterion) -> Result<(), BenchmarkError> {
    let schema = "./data/schemas/corporations_schema.json";

    let benchset = Benchset::new(
        "overhead::corporations_10k",
        dataset::corporations_10k(),
    )?
    .add_rsonschema_with_all_modes(schema)?
    .finish();

    benchset.run(c);

    Ok(())
}

pub fn corporations_50k(c: &mut Criterion) -> Result<(), BenchmarkError> {
    let schema = "./data/schemas/corporations_schema.json";

    let benchset = Benchset::new(
        "overhead::corporations_50k",
        dataset::corporations_50k(),
    )?
    .add_rsonschema_with_all_modes(schema)?
    .finish();

    benchset.run(c);

    Ok(())
}

pub fn corporations_100k(c: &mut Criterion) -> Result<(), BenchmarkError> {
    let schema = "./data/schemas/corporations_schema.json";

    let benchset = Benchset::new(
        "overhead::corporations_100k",
        dataset::corporations_100k(),
    )?
    .add_rsonschema_with_all_modes(schema)?
    .finish();

    benchset.run(c);

    Ok(())
}

pub fn corporations_200k(c: &mut Criterion) -> Result<(), BenchmarkError> {
    let schema = "./data/schemas/corporations_schema.json";

    let benchset = Benchset::new(
        "overhead::corporations_200k",
        dataset::corporations_200k(),
    )?
    .add_rsonschema_with_all_modes(schema)?
    .finish();

    benchset.run(c);

    Ok(())
}

pub fn clang_10(c: &mut Criterion) -> Result<(), BenchmarkError> {
    let schema = "./data/schemas/clang_schema.json";

    let benchset = Benchset::new(
        "overhead::clang_10",
        dataset::clang_10(),
    )?
    .add_rsonschema_with_all_modes(schema)?
    .finish();

    benchset.run(c);

    Ok(())
}

pub fn clang_50(c: &mut Criterion) -> Result<(), BenchmarkError> {
    let schema = "./data/schemas/clang_schema.json";

    let benchset = Benchset::new(
        "overhead::clang_50",
        dataset::clang_50(),
    )?
    .add_rsonschema_with_all_modes(schema)?
    .finish();

    benchset.run(c);

    Ok(())
}

pub fn clang_100(c: &mut Criterion) -> Result<(), BenchmarkError> {
    let schema = "./data/schemas/clang_schema.json";

    let benchset = Benchset::new(
        "overhead::clang_100",
        dataset::clang_100(),
    )?
    .add_rsonschema_with_all_modes(schema)?
    .finish();

    benchset.run(c);

    Ok(())
}

benchsets!(
    overhead_benches,
    arr_5_depth_3_props_4_len_10_10,
    arr_5_depth_3_props_6_len_8_12,
    arr_5_depth_3_props_8_len_8_12,
    corporations_10k,
    corporations_50k,
    corporations_100k,
    corporations_200k,
    clang_10,
    clang_50,
    clang_100
);
