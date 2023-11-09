#include <unordered_map>
#include <vector>
#include <string>

struct Edge {
  std::string dest;
  int weight;
  int type;
};

using Graph = std::unordered_map<std::string, std::vector<Edge>>; 
using DistanceMatrix = std::unordered_map<std::string, std::unordered_map<std::string, int>>;
using PredecessorMatrix = std::unordered_map<std::string, std::unordered_map<std::string, std::string>>;

Graph createSchoolGraph(const std::string& file_path);

// Currently can't be bothered to write the rest of the function declarations :^).
