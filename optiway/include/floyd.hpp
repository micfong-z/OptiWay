#include <memory>
#include <string>
#include <unordered_map>
#include <vector>

#include "json.hpp"
#include "rust/cxx.h"

using Json = nlohmann::json;

struct Edge {
	std::string dest;
	int weight;
	int type;
};

using Graph = std::unordered_map<std::string, std::vector<Edge>>;
using DistanceMatrix =
		std::unordered_map<std::string, std::unordered_map<std::string, int>>;
using PredecessorMatrix =
		std::unordered_map<std::string,
				std::unordered_map<std::string, std::string>>;

std::unique_ptr<Graph> createSchoolGraph(rust::Str file_path);
std::unique_ptr<Json> getRoutesFromTimetable(const Json &timetables,
											 const Graph &graph,
											 const Json &shortest_paths);
