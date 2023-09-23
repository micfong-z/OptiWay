#include <iostream>
#include <fstream>
#include <unordered_map>
#include <vector>
#include <string>
#include <sstream>
#include <limits>
#include "json.hpp"
using json = nlohmann::json;

using namespace std;

struct Edge {
    string dest;  // destination node
    int weight;   // edge weight (in this case, distance)
    int type;     // edge type
};

using Graph = unordered_map<string, vector<Edge> >;
using DistanceMatrix = unordered_map<string, unordered_map<string, int> >;
using PredecessorMatrix = unordered_map<string, unordered_map<string, string> >;


Graph createSchoolGraph(const string& file_path) {
    /* Construct the school's layout graph from the path.txt */
    Graph graph;
    ifstream file(file_path);

    if (!file.is_open()) {
        cerr << "Failed to open the file." << endl;
        return graph;
    }

    string line;
    while (getline(file, line)) {
        istringstream iss(line);
        
        string node1, node2;
        int distance, type;

        iss >> node1 >> node2 >> distance >> type;

        Edge edge1 = {node2, distance, type};
        Edge edge2 = {node1, distance, type};

        graph[node1].push_back(edge1);
        graph[node2].push_back(edge2);  // Because it's an undirected graph
    }

    file.close();
    return graph;
}

void initializeDistanceAndPredecessorMatrices(const Graph& graph, DistanceMatrix& dist, PredecessorMatrix& pred) {
    /* Initialize the dist and predecessor's matrices for floyd */
    const int INF = numeric_limits<int>::max();

    for (const auto& [node, _] : graph) {
        for (const auto& [node2, __] : graph) {
            if (node == node2) {
                dist[node][node2] = 0;
            } else {
                dist[node][node2] = INF;
            }
            pred[node][node2] = "";
        }
    }

    for (const auto& [node, edges] : graph) {
        for (const auto& edge : edges) {
            dist[node][edge.dest] = edge.weight;
            pred[node][edge.dest] = node;
        }
    }
}

string concatenate(const std::vector<std::string>& vec) {
    std::string result;
    for (const auto& str : vec) {
        if (!result.empty()) {
            result += " "; // Add space before appending, but not for the first string
        }
        result += str;
    }
    return result;
}

void floydWarshall(const Graph& graph, DistanceMatrix& dist, PredecessorMatrix& pred) {
    /* Main Floyd's program */
    const int INF = numeric_limits<int>::max();

    for (const auto& [k, _] : graph) {
        for (const auto& [i, __] : graph) {
            for (const auto& [j, ___] : graph) {
                if (dist[i][k] != INF && dist[k][j] != INF && dist[i][k] + dist[k][j] < dist[i][j]) {
                    dist[i][j] = dist[i][k] + dist[k][j];
                    pred[i][j] = pred[k][j];
                }
            }
        }
    }
}

vector<string> reconstructPath(const PredecessorMatrix& pred, const string& start, const string& end) {
    /* Reconstruct the path from the predecessor's list */
    vector<string> path;

    if (pred.at(start).at(end) == "") {
        return path;  // Empty path means no path exists
    }

    string at = end;
    while (at != start) {
        path.push_back(at);
        at = pred.at(start).at(at);
    }
    path.push_back(start);
    reverse(path.begin(), path.end());
    return path;
}


json getRoutesfromTimetable(const json& timetables, const Graph& graph) {
    /* Obtain the routes as a json file from the timetables's json*/
    json routes;
    DistanceMatrix dist;
    PredecessorMatrix pred;

    initializeDistanceAndPredecessorMatrices(graph, dist, pred);
    floydWarshall(graph, dist, pred);

    for (const auto& [student, timetable] : timetables.items()) {
        json week;
        for (const auto& [day, classes] : timetable.items()) {
            json today;
            
            vector<string> path = reconstructPath(pred, "G", classes["1"]);
            string path_str = concatenate(path);
            today["0"] = path_str; 

            for (int i = 1; i <= 10; i++) {  // 10 classes in a week, compulsory
                const string& current_period = to_string(i);
                const string& current_room = classes[current_period];

                // cout << current_room << endl;

                if (current_room[0] != 'A' && current_room[0] != 'B' && current_room[0] != 'G') continue; 
                // If not the last class
                if (i != 10) {
                    const string& next_period = to_string(i + 1);
                    const string& next_room = classes[next_period];

                    if (next_room[0] != 'A' && next_room[0] != 'B' && next_room[0] != 'G') continue;

                    if (next_room == current_room) today[current_period] = "";
                    else {
                        vector<string> path = reconstructPath(pred, current_room, next_room);
                        string path_str = concatenate(path);
                        today[current_period] = path_str; 
                    }
                } else {
                        vector<string> path = reconstructPath(pred, current_room, "G");
                        string path_str = concatenate(path);
                        today[current_period] = path_str; 
                }
            }
            week[day] = today;
        }
        routes[student] = week;
    }

    return routes;
}

int main() {
    string timetable_path;

    ios::sync_with_stdio(false);
    cin.tie(0);

    // general input: the path to the time table json
    // cin >> timetable_path;  // "../assets/timetable_0.json"
    getline(cin, timetable_path);

    // parse the JSON file
    std::ifstream input_file(timetable_path);
    if (!input_file.is_open()) {
        std::cerr << "Failed to open the file." << std::endl;
        return 1;
    }

    json timetables;
    input_file >> timetables;
    input_file.close();

    Graph graph = createSchoolGraph("../assets/paths.txt");
    
    /*
    ofstream outFile("output.txt");
    if (!outFile.is_open()) {
        cerr << "Failed to open the output file." << endl;
        return 1; // return with an error code
    }
    */
    json routes = getRoutesfromTimetable(timetables, graph);

    std::ofstream file("routes.json");
    if (file.is_open()) {
        file << routes.dump(4); // 4 spaces for indentation
        file.close();
        std::cout << "JSON data written to routes.json successfully." << std::endl;
    } else {
        std::cerr << "Failed to open the file for writing." << std::endl;
    }
    
    return 0;
}
