use self::implementation::prepare;
use self::{benchmark_options::BenchmarkOptions, implementation::prepare_with_id};
#[cfg(feature = "blaze")]
use crate::implementations::blaze::{Blaze, BlazeError};
use crate::{
    dataset,
    implementations::{
        boon::{Boon, BoonError},
        dja::{Dja, DjaError},
        jsonpath_rust::{JsonpathRust, JsonpathRustError},
        jsonschema::{JsonSchema, JsonSchemaError},
        rsonpath::{Rsonpath, RsonpathCount, RsonpathError, RsonpathMmap, RsonpathMmapCount},
        rsonschema::{
            Rsonschema, RsonschemaError, RsonschemaMmap, RsonschemaMmapWellFormednessOnly,
            RsonschemaMmapWithWellFormedness, RsonschemaWellFormednessOnly, RsonschemaWithWellFormedness,
        },
        rust_jsurfer::{JSurfer, JSurferError},
        serde_json_path::{SerdeJsonPath, SerdeJsonPathError},
        spawn_baseline::{SpawnBaseline, SpawnBaselineError},
    },
};
use criterion::{Criterion, Throughput};
use implementation::{Implementation, PreparedQuery};
use std::{path::PathBuf, time::Duration};
use thiserror::Error;

pub mod benchmark_options;
pub mod implementation;

#[derive(Clone, Copy, Debug)]
pub enum BenchTarget<'q> {
    RsonpathMmap(&'q str, ResultType),
    Rsonpath(&'q str, ResultType),
    JSurfer(&'q str),
    JsonpathRust(&'q str),
    SerdeJsonPath(&'q str),

    Rsonschema(&'q str),
    RsonschemaWithWellFormedness(&'q str),
    RsonschemaWellFormednessOnly(&'q str),
    RsonschemaMmap(&'q str),
    RsonschemaMmapWithWellFormedness(&'q str),
    RsonschemaMmapWellFormednessOnly(&'q str),
    Boon(&'q str),
    JsonSchema(&'q str),
    Dja(&'q str),
    SpawnBaseline(&'q str),
    #[cfg(feature = "blaze")]
    Blaze(&'q str),
}

#[derive(Clone, Copy, Debug)]
pub enum ResultType {
    Full,
    Count,
}

pub struct Benchset {
    id: String,
    options: BenchmarkOptions,
    json_document: dataset::JsonFile,
    implementations: Vec<Box<dyn BenchFn>>,
    measure_file_load: bool,
    measure_compilation_time: bool,
}

pub struct ConfiguredBenchset {
    source: Benchset,
}

impl ConfiguredBenchset {
    pub fn run(&self, c: &mut Criterion) {
        let bench = &self.source;
        let mut group = c.benchmark_group(&bench.id);

        bench.options.apply_to(&mut group);
        group.throughput(Throughput::BytesDecimal(
            u64::try_from(bench.json_document.size_in_bytes).unwrap(),
        ));

        for implementation in bench.implementations.iter() {
            let id = implementation.id();
            group.bench_function(id, |b| b.iter(move || implementation.run()));
        }

        group.finish();
    }
}

impl Benchset {
    pub fn new<S: Into<String>>(id: S, dataset: dataset::Dataset) -> Result<Self, BenchmarkError> {
        let json_file = dataset.file_path().map_err(BenchmarkError::DatasetError)?;

        let warm_up_time = if json_file.size_in_bytes < 10_000_000 {
            None
        } else if json_file.size_in_bytes < 100_000_000 {
            Some(Duration::from_secs(5))
        } else {
            Some(Duration::from_secs(10))
        };

        // We're aiming for over 1GB/s, but some queries run at 100MB/s.
        // Let's say we want to run the query at least 10 times to get significant results.
        const TARGET_NUMBER_OF_QUERIES: f64 = 10.0;
        const TARGET_SPEED_IN_BYTES_PER_SEC: f64 = 100_000_000.0;

        let measurement_secs =
            (json_file.size_in_bytes as f64) * TARGET_NUMBER_OF_QUERIES / TARGET_SPEED_IN_BYTES_PER_SEC;
        let measurement_time = if measurement_secs > 5.0 {
            Some(Duration::from_secs_f64(measurement_secs))
        } else {
            None
        };
        let sample_count = if json_file.size_in_bytes < 1_000_000 {
            None
        } else {
            Some(10)
        };

        Ok(Self {
            id: format!("{}_{}", json_file.file_path, id.into()),
            options: BenchmarkOptions {
                warm_up_time,
                measurement_time,
                sample_count,
            },
            json_document: json_file,
            implementations: vec![],
            measure_file_load: true,
            measure_compilation_time: false,
        })
    }

    pub fn do_not_measure_file_load_time(self) -> Self {
        Self {
            measure_file_load: false,
            ..self
        }
    }

    pub fn measure_compilation_time(self) -> Self {
        Self {
            measure_compilation_time: true,
            ..self
        }
    }

    pub fn add_target(mut self, target: BenchTarget<'_>) -> Result<Self, BenchmarkError> {
        let bench_fn = target.to_bench_fn(
            &self.json_document.file_path,
            !self.measure_file_load,
            !self.measure_compilation_time,
        )?;
        self.implementations.push(bench_fn);
        Ok(self)
    }

    pub fn add_target_with_id(mut self, target: BenchTarget<'_>, id: &'static str) -> Result<Self, BenchmarkError> {
        let bench_fn = target.to_bench_fn_with_id(
            &self.json_document.file_path,
            !self.measure_file_load,
            !self.measure_compilation_time,
            id,
        )?;
        self.implementations.push(bench_fn);
        Ok(self)
    }

    pub fn add_rsonpath_with_all_result_types(self, query: &str) -> Result<Self, BenchmarkError> {
        self.add_target(BenchTarget::Rsonpath(query, ResultType::Full))?
            .add_target(BenchTarget::Rsonpath(query, ResultType::Count))?
            .add_target(BenchTarget::RsonpathMmap(query, ResultType::Full))?
            .add_target(BenchTarget::RsonpathMmap(query, ResultType::Count))
    }

    pub fn add_all_targets_except_jsurfer(self, query: &str) -> Result<Self, BenchmarkError> {
        self.add_target(BenchTarget::RsonpathMmap(query, ResultType::Full))?
            .add_target(BenchTarget::JsonpathRust(query))?
            .add_target(BenchTarget::SerdeJsonPath(query))
    }

    pub fn add_all_targets(self, query: &str) -> Result<Self, BenchmarkError> {
        self.add_target(BenchTarget::RsonpathMmap(query, ResultType::Full))?
            .add_target(BenchTarget::JSurfer(query))?
            .add_target(BenchTarget::JsonpathRust(query))?
            .add_target(BenchTarget::SerdeJsonPath(query))
    }

    pub fn add_all_validator_targets(self, schema: &str) -> Result<Self, BenchmarkError> {
        let this = self
            .add_target(BenchTarget::Rsonschema(schema))?
            .add_target(BenchTarget::Boon(schema))?
            .add_target(BenchTarget::JsonSchema(schema))?
            .add_target(BenchTarget::Dja(schema))?
            .add_target(BenchTarget::SpawnBaseline(schema))?;

        #[cfg(feature = "blaze")]
        let this = this.add_target(BenchTarget::Blaze(schema))?;

        Ok(this)
    }

    pub fn add_rsonschema_with_all_modes(self, schema: &str) -> Result<Self, BenchmarkError> {
        self.add_target(BenchTarget::RsonschemaWellFormednessOnly(schema))?
            .add_target(BenchTarget::Rsonschema(schema))?
            .add_target(BenchTarget::RsonschemaWithWellFormedness(schema))
    }

    pub fn add_rust_native_targets(self, query: &str) -> Result<Self, BenchmarkError> {
        self.add_target(BenchTarget::RsonpathMmap(query, ResultType::Full))?
            .add_target(BenchTarget::JsonpathRust(query))?
            .add_target(BenchTarget::SerdeJsonPath(query))
    }

    pub fn finish(self) -> ConfiguredBenchset {
        ConfiguredBenchset { source: self }
    }
}

trait Target {
    fn to_bench_fn(
        self,
        file_path: &str,
        load_ahead_of_time: bool,
        compile_ahead_of_time: bool,
    ) -> Result<Box<dyn BenchFn>, BenchmarkError>;

    fn to_bench_fn_with_id(
        self,
        file_path: &str,
        load_ahead_of_time: bool,
        compile_ahead_of_time: bool,
        id: &'static str,
    ) -> Result<Box<dyn BenchFn>, BenchmarkError>;
}

impl Target for BenchTarget<'_> {
    fn to_bench_fn(
        self,
        file_path: &str,
        load_ahead_of_time: bool,
        compile_ahead_of_time: bool,
    ) -> Result<Box<dyn BenchFn>, BenchmarkError> {
        match self {
            BenchTarget::Rsonpath(q, ResultType::Full) => {
                let rsonpath = Rsonpath::new()?;
                let prepared = prepare(rsonpath, file_path, q, load_ahead_of_time, compile_ahead_of_time)?;
                Ok(Box::new(prepared))
            }
            BenchTarget::Rsonpath(q, ResultType::Count) => {
                let rsonpath = RsonpathCount::new()?;
                let prepared = prepare(rsonpath, file_path, q, load_ahead_of_time, compile_ahead_of_time)?;
                Ok(Box::new(prepared))
            }
            BenchTarget::RsonpathMmap(q, ResultType::Full) => {
                let rsonpath = RsonpathMmap::new()?;
                let prepared = prepare(rsonpath, file_path, q, load_ahead_of_time, compile_ahead_of_time)?;
                Ok(Box::new(prepared))
            }
            BenchTarget::RsonpathMmap(q, ResultType::Count) => {
                let rsonpath = RsonpathMmapCount::new()?;
                let prepared = prepare(rsonpath, file_path, q, load_ahead_of_time, compile_ahead_of_time)?;
                Ok(Box::new(prepared))
            }
            BenchTarget::JSurfer(q) => {
                let jsurfer = JSurfer::new()?;
                let prepared = prepare(jsurfer, file_path, q, load_ahead_of_time, compile_ahead_of_time)?;
                Ok(Box::new(prepared))
            }
            BenchTarget::JsonpathRust(q) => {
                let jsonpath_rust = JsonpathRust::new()?;
                let prepared = prepare(jsonpath_rust, file_path, q, load_ahead_of_time, compile_ahead_of_time)?;
                Ok(Box::new(prepared))
            }
            BenchTarget::SerdeJsonPath(q) => {
                let serde_json_path = SerdeJsonPath::new()?;
                let prepared = prepare(serde_json_path, file_path, q, load_ahead_of_time, compile_ahead_of_time)?;
                Ok(Box::new(prepared))
            }

            BenchTarget::Rsonschema(q) => {
                let rsonschema = Rsonschema::new()?;
                let prepared = prepare(rsonschema, file_path, q, load_ahead_of_time, compile_ahead_of_time)?;
                Ok(Box::new(prepared))
            }
            BenchTarget::RsonschemaWithWellFormedness(q) => {
                let rsonschema = RsonschemaWithWellFormedness::new()?;
                let prepared = prepare(rsonschema, file_path, q, load_ahead_of_time, compile_ahead_of_time)?;
                Ok(Box::new(prepared))
            }
            BenchTarget::RsonschemaWellFormednessOnly(q) => {
                let rsonschema = RsonschemaWellFormednessOnly::new()?;
                let prepared = prepare(rsonschema, file_path, q, load_ahead_of_time, compile_ahead_of_time)?;
                Ok(Box::new(prepared))
            }
            BenchTarget::RsonschemaMmap(q) => {
                let rsonschema = RsonschemaMmap::new()?;
                let prepared = prepare(rsonschema, file_path, q, load_ahead_of_time, compile_ahead_of_time)?;
                Ok(Box::new(prepared))
            }
            BenchTarget::RsonschemaMmapWithWellFormedness(q) => {
                let rsonschema = RsonschemaMmapWithWellFormedness::new()?;
                let prepared = prepare(rsonschema, file_path, q, load_ahead_of_time, compile_ahead_of_time)?;
                Ok(Box::new(prepared))
            }
            BenchTarget::RsonschemaMmapWellFormednessOnly(q) => {
                let rsonschema = RsonschemaMmapWellFormednessOnly::new()?;
                let prepared = prepare(rsonschema, file_path, q, load_ahead_of_time, compile_ahead_of_time)?;
                Ok(Box::new(prepared))
            }
            BenchTarget::Boon(q) => {
                let boon = Boon::new()?;
                let prepared = prepare(boon, file_path, q, load_ahead_of_time, compile_ahead_of_time)?;
                Ok(Box::new(prepared))
            }
            BenchTarget::JsonSchema(q) => {
                let jsonschema = JsonSchema::new()?;
                let prepared = prepare(jsonschema, file_path, q, load_ahead_of_time, compile_ahead_of_time)?;
                Ok(Box::new(prepared))
            }
            BenchTarget::Dja(q) => {
                let dja = Dja::new()?;
                let prepared = prepare(dja, file_path, q, load_ahead_of_time, compile_ahead_of_time)?;
                Ok(Box::new(prepared))
            }
            BenchTarget::SpawnBaseline(q) => {
                let baseline = SpawnBaseline::new()?;
                let prepared = prepare(baseline, file_path, q, load_ahead_of_time, compile_ahead_of_time)?;
                Ok(Box::new(prepared))
            }
            #[cfg(feature = "blaze")]
            BenchTarget::Blaze(q) => {
                let blaze = Blaze::new()?;
                let prepared = prepare(blaze, file_path, q, load_ahead_of_time, compile_ahead_of_time)?;
                Ok(Box::new(prepared))
            }
        }
    }

    fn to_bench_fn_with_id(
        self,
        file_path: &str,
        load_ahead_of_time: bool,
        compile_ahead_of_time: bool,
        id: &'static str,
    ) -> Result<Box<dyn BenchFn>, BenchmarkError> {
        match self {
            BenchTarget::Rsonpath(q, ResultType::Full) => {
                let rsonpath = Rsonpath::new()?;
                let prepared = prepare_with_id(rsonpath, id, file_path, q, load_ahead_of_time, compile_ahead_of_time)?;
                Ok(Box::new(prepared))
            }
            BenchTarget::Rsonpath(q, ResultType::Count) => {
                let rsonpath = RsonpathCount::new()?;
                let prepared = prepare_with_id(rsonpath, id, file_path, q, load_ahead_of_time, compile_ahead_of_time)?;
                Ok(Box::new(prepared))
            }
            BenchTarget::RsonpathMmap(q, ResultType::Full) => {
                let rsonpath = RsonpathMmap::new()?;
                let prepared = prepare_with_id(rsonpath, id, file_path, q, load_ahead_of_time, compile_ahead_of_time)?;
                Ok(Box::new(prepared))
            }
            BenchTarget::RsonpathMmap(q, ResultType::Count) => {
                let rsonpath = RsonpathMmapCount::new()?;
                let prepared = prepare_with_id(rsonpath, id, file_path, q, load_ahead_of_time, compile_ahead_of_time)?;
                Ok(Box::new(prepared))
            }
            BenchTarget::JSurfer(q) => {
                let jsurfer = JSurfer::new()?;
                let prepared = prepare_with_id(jsurfer, id, file_path, q, load_ahead_of_time, compile_ahead_of_time)?;
                Ok(Box::new(prepared))
            }
            BenchTarget::JsonpathRust(q) => {
                let jsonpath_rust = JsonpathRust::new()?;
                let prepared = prepare_with_id(
                    jsonpath_rust,
                    id,
                    file_path,
                    q,
                    load_ahead_of_time,
                    compile_ahead_of_time,
                )?;
                Ok(Box::new(prepared))
            }
            BenchTarget::SerdeJsonPath(q) => {
                let serde_json_path = SerdeJsonPath::new()?;
                let prepared = prepare_with_id(
                    serde_json_path,
                    id,
                    file_path,
                    q,
                    load_ahead_of_time,
                    compile_ahead_of_time,
                )?;
                Ok(Box::new(prepared))
            }

            BenchTarget::Rsonschema(q) => {
                let rsonschema = Rsonschema::new()?;
                let prepared =
                    prepare_with_id(rsonschema, id, file_path, q, load_ahead_of_time, compile_ahead_of_time)?;
                Ok(Box::new(prepared))
            }
            BenchTarget::RsonschemaWithWellFormedness(q) => {
                let rsonschema = RsonschemaWithWellFormedness::new()?;
                let prepared =
                    prepare_with_id(rsonschema, id, file_path, q, load_ahead_of_time, compile_ahead_of_time)?;
                Ok(Box::new(prepared))
            }
            BenchTarget::RsonschemaWellFormednessOnly(q) => {
                let rsonschema = RsonschemaWellFormednessOnly::new()?;
                let prepared =
                    prepare_with_id(rsonschema, id, file_path, q, load_ahead_of_time, compile_ahead_of_time)?;
                Ok(Box::new(prepared))
            }
            BenchTarget::RsonschemaMmap(q) => {
                let rsonschema = RsonschemaMmap::new()?;
                let prepared =
                    prepare_with_id(rsonschema, id, file_path, q, load_ahead_of_time, compile_ahead_of_time)?;
                Ok(Box::new(prepared))
            }
            BenchTarget::RsonschemaMmapWithWellFormedness(q) => {
                let rsonschema = RsonschemaMmapWithWellFormedness::new()?;
                let prepared =
                    prepare_with_id(rsonschema, id, file_path, q, load_ahead_of_time, compile_ahead_of_time)?;
                Ok(Box::new(prepared))
            }
            BenchTarget::RsonschemaMmapWellFormednessOnly(q) => {
                let rsonschema = RsonschemaMmapWellFormednessOnly::new()?;
                let prepared =
                    prepare_with_id(rsonschema, id, file_path, q, load_ahead_of_time, compile_ahead_of_time)?;
                Ok(Box::new(prepared))
            }
            BenchTarget::Boon(q) => {
                let boon = Boon::new()?;
                let prepared = prepare_with_id(boon, id, file_path, q, load_ahead_of_time, compile_ahead_of_time)?;
                Ok(Box::new(prepared))
            }
            BenchTarget::JsonSchema(q) => {
                let jsonschema = JsonSchema::new()?;
                let prepared =
                    prepare_with_id(jsonschema, id, file_path, q, load_ahead_of_time, compile_ahead_of_time)?;
                Ok(Box::new(prepared))
            }
            BenchTarget::Dja(q) => {
                let dja = Dja::new()?;
                let prepared = prepare_with_id(dja, id, file_path, q, load_ahead_of_time, compile_ahead_of_time)?;
                Ok(Box::new(prepared))
            }
            BenchTarget::SpawnBaseline(q) => {
                let baseline = SpawnBaseline::new()?;
                let prepared = prepare_with_id(baseline, id, file_path, q, load_ahead_of_time, compile_ahead_of_time)?;
                Ok(Box::new(prepared))
            }
            #[cfg(feature = "blaze")]
            BenchTarget::Blaze(q) => {
                let blaze = Blaze::new()?;
                let prepared = prepare_with_id(blaze, id, file_path, q, load_ahead_of_time, compile_ahead_of_time)?;
                Ok(Box::new(prepared))
            }
        }
    }
}

trait BenchFn {
    fn id(&self) -> &str;

    fn run(&self);
}

impl<I: Implementation> BenchFn for PreparedQuery<I> {
    fn id(&self) -> &str {
        self.id
    }

    fn run(&self) {
        let f_storage;
        let q_storage;

        let f = match &self.file {
            implementation::File::NeedToLoad(file_path) => {
                f_storage = self.implementation.load_file(file_path).unwrap();
                &f_storage
            }
            implementation::File::AlreadyLoaded(f) => f,
        };
        let q = match &self.query {
            implementation::Query::NeedToCompile(query_string) => {
                q_storage = self.implementation.compile_query(query_string).unwrap();
                &q_storage
            }
            implementation::Query::AlreadyCompiled(q) => q,
        };

        let result = self.implementation.run(q, f).unwrap();
        std::hint::black_box(result);
    }
}

#[derive(Error, Debug)]
pub enum BenchmarkError {
    #[error("invalid dataset file path, has to be valid UTF-8: '{0}'")]
    InvalidFilePath(PathBuf),
    #[error("error loading dataset: {0}")]
    DatasetError(
        #[source]
        #[from]
        dataset::DatasetError,
    ),
    #[error("error preparing Rsonpath bench: {0}")]
    RsonpathError(
        #[source]
        #[from]
        RsonpathError,
    ),
    #[error("error preparing JSurfer bench: {0}")]
    JSurferError(
        #[source]
        #[from]
        JSurferError,
    ),
    #[error("error preparing JsonpathRust bench: {0}")]
    JsonpathRust(
        #[source]
        #[from]
        JsonpathRustError,
    ),
    #[error("error preparing SerdeJsonPath bench: {0}")]
    SerdeJsonPath(
        #[source]
        #[from]
        SerdeJsonPathError,
    ),

    #[error("error preparing Rsonschema bench: {0}")]
    RsonschemaError(
        #[source]
        #[from]
        RsonschemaError,
    ),
    #[error("error preparing Boon bench: {0}")]
    BoonError(
        #[source]
        #[from]
        BoonError,
    ),
    #[error("error preparing JsonSchema bench: {0}")]
    JsonSchemaError(
        #[source]
        #[from]
        JsonSchemaError,
    ),
    #[error("error preparing DJA bench: {0}")]
    DjaError(
        #[source]
        #[from]
        DjaError,
    ),
    #[error("error preparing spawn baseline bench: {0}")]
    SpawnBaselineError(
        #[source]
        #[from]
        SpawnBaselineError,
    ),
    #[cfg(feature = "blaze")]
    #[error("error preparing Blaze (jsonschema CLI) bench: {0}")]
    BlazeError(
        #[source]
        #[from]
        BlazeError,
    ),
}
