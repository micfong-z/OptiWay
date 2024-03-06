#include "json.hpp"
#include "rust/cxx.h"
#include <memory>
#include <vector>

// Create a new json object
std::unique_ptr<nlohmann::json> makeJson(rust::Str contents);
std::unique_ptr<std::string> jsonToString(const nlohmann::json &json);
