#include <sourcemeta/blaze/compiler.h>
#include <sourcemeta/blaze/evaluator.h>
#include <sourcemeta/blaze/foundation.h>

#include <sourcemeta/core/io.h>
#include <sourcemeta/core/json.h>

#include <cstdint>
#include <filesystem>
#include <optional>
#include <string>

namespace
{

  thread_local std::string LAST_ERROR;

  auto set_error(const std::string &message) -> void
  {
    LAST_ERROR = message;
  }

  auto clear_error() -> void { LAST_ERROR.clear(); }

  auto parse_json_file(const char *path) -> std::optional<sourcemeta::core::JSON>
  {
    if (path == nullptr)
    {
      set_error("received a null filesystem path");
      return std::nullopt;
    }

    return sourcemeta::core::read_json(std::filesystem::path{path});
  }

  class BlazeSchemaHandle
  {
  public:
    explicit BlazeSchemaHandle(sourcemeta::blaze::Template &&compiled)
        : schema_(std::move(compiled)) {}

    auto validate(const sourcemeta::core::JSON &instance) -> bool { return evaluator_.validate(schema_, instance); }

    sourcemeta::blaze::Template schema_;
    sourcemeta::blaze::Evaluator evaluator_;
  };

  class BlazeInstanceHandle
  {
  public:
    explicit BlazeInstanceHandle(sourcemeta::core::JSON instance)
        : instance_(std::move(instance)) {}

    sourcemeta::core::JSON instance_;
  };

} // namespace

extern "C"
{

  auto blaze_last_error() -> const char * { return LAST_ERROR.c_str(); }
  auto blaze_destroy_instance(void *handle) -> void;

  auto blaze_compile_schema(const char *schema_path) -> void *
  {
    try
    {
      const auto schema = parse_json_file(schema_path);
      if (!schema.has_value())
      {
        return nullptr;
      }

      auto compiled = sourcemeta::blaze::compile(
          *schema,
          sourcemeta::blaze::schema_walker,
          sourcemeta::blaze::schema_resolver,
          sourcemeta::blaze::default_schema_compiler,
          sourcemeta::blaze::Mode::FastValidation);

      clear_error();
      return new BlazeSchemaHandle(std::move(compiled));
    }
    catch (const std::exception &error)
    {
      set_error(error.what());
      return nullptr;
    }
  }

  auto blaze_load_instance(const char *instance_path) -> void *
  {
    try
    {
      const auto instance = parse_json_file(instance_path);
      if (!instance.has_value())
      {
        return nullptr;
      }

      clear_error();
      return new BlazeInstanceHandle(*instance);
    }
    catch (const std::exception &error)
    {
      set_error(error.what());
      return nullptr;
    }
  }

  auto blaze_validate_schema_loaded_instance(void *schema_handle,
                                             void *instance_handle)
      -> std::int32_t
  {
    if (schema_handle == nullptr)
    {
      set_error("received a null Blaze schema handle");
      return -1;
    }

    if (instance_handle == nullptr)
    {
      set_error("received a null Blaze instance handle");
      return -1;
    }

    try
    {
      auto *schema = static_cast<BlazeSchemaHandle *>(schema_handle);
      auto *instance = static_cast<BlazeInstanceHandle *>(instance_handle);
      const auto valid = schema->validate(instance->instance_);
      clear_error();
      return valid ? 1 : 0;
    }
    catch (const std::exception &error)
    {
      set_error(error.what());
      return -1;
    }
  }

  auto blaze_destroy_instance(void *handle) -> void
  {
    if (handle == nullptr)
    {
      return;
    }

    auto *instance = static_cast<BlazeInstanceHandle *>(handle);
    delete instance;
  }

  auto blaze_destroy_schema(void *handle) -> void
  {
    if (handle == nullptr)
    {
      return;
    }

    auto *validator = static_cast<BlazeSchemaHandle *>(handle);
    delete validator;
  }

} // extern "C"
