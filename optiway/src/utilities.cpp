#include "utilities.hpp"
#include "json.hpp"
#include "rust/cxx.h"
#include <memory>

std::unique_ptr<nlohmann::json> makeJson(rust::Str contents) {
  std::unique_ptr<nlohmann::json> json = std::make_unique<nlohmann::json>();
  *json = nlohmann::json::parse(contents);
  return json;
}

std::unique_ptr<std::string> jsonToString(const nlohmann::json &json) {
  return std::make_unique<std::string>(json);
}
