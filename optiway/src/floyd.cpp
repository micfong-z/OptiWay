#include <fstream>
#include <iostream>
#include <limits>
#include <sstream>
#include <string>
#include <unordered_map>
#include <vector>

#include "floyd.hpp"
#include "json.hpp"
#include "rust/cxx.h"
#include "utilities.hpp"

using namespace std;

std::unique_ptr<Graph> createSchoolGraph(rust::Str file_path) {
    /* Construct the school's layout graph from the path.txt */
    std::unique_ptr<Graph> graph = std::make_unique<Graph>();
    ifstream file(file_path.data());

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

		(*graph)[node1].push_back(edge1);
		(*graph)[node2].push_back(edge2);   // Because it's an undirected graph
    }

    file.close();
    return graph;
}

void initializeDistanceAndPredecessorMatrices(const Graph& graph, DistanceMatrix& dist,
                                              PredecessorMatrix& pred) {
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

string concatenate(const vector<string>& vec) {
    string result;
    for (const auto& str : vec) {
        if (!result.empty()) {
            result += " ";  // Add space before appending, but not for the first string
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
                if (dist[i][k] != INF && dist[k][j] != INF &&
                    dist[i][k] + dist[k][j] < dist[i][j]) {
                    dist[i][j] = dist[i][k] + dist[k][j];
                    pred[i][j] = pred[k][j];
                }
            }
        }
    }
}

vector<string> reconstructPath(const PredecessorMatrix& pred, const string& start,
                               const string& end) {
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

std::unique_ptr<Json> getRoutesfromTimetable(const Json& timetables, const Graph& graph,
                            const Json& shortest_paths) {
    /* Obtain the routes as a json file from the timetables's json*/
	std::unique_ptr<Json> routes = std::make_unique<Json>();

    for (const auto& [student, timetable] : timetables.items()) {
        Json week;
        for (const auto& [day, classes] : timetable.items()) {
            Json today;

            // Start of the day, going from G_floor to the first class
            string path_str = shortest_paths["G" + classes["1"].get<string>()];
            today["0"] = path_str;

            // Lunch time is denoted by period "6", and periods after are delayed by one
            // index
            path_str =
                shortest_paths["G" +
                               classes["7"].get<string>()];  // Originally, P7 is the
                                                             // first afternoon class
            today["7"] = path_str;

            int offset = 0;
            for (int period = 1; period <= 11; period++) {
                // Skip P7 as it's already been handled
                if (period == 7) {
                    offset = 1;  // Add offset to be minused because P7 is now P6
                    continue;
                }

                const string& current_period = to_string(period - offset);
                const string& current_room = classes[current_period];

                // cout << current_room << endl;

                if (current_room[0] != 'A' && current_room[0] != 'B' &&
                    current_room[0] != 'G')
                    continue;
                // If not the last class
                if (period != 11) {
                    // If AS/AL students have P6s, the should go to G_floor in the end
                    if (period == 6 && stoi(student) < 22000) {
                        if (current_room == "G") {
                            today["6"] = "";
                            continue;
                        }
                        string path_str = shortest_paths[current_room + "G"];
                        today["6"] = path_str;
                        continue;
                    }

                    if (period == 6 && stoi(student) >= 22000) {
                        today["6"] = "";
                        continue;
                    }

                    const string& next_period = to_string(period - offset + 1);
                    const string& next_room = classes[next_period];

                    if (next_room[0] != 'A' && next_room[0] != 'B' && next_room[0] != 'G')
                        continue;

                    if (next_room == current_room)
                        today[to_string(period)] = "";
                    else {
                        string path_str = shortest_paths[current_room + next_room];
                        today[to_string(period)] = path_str;
                    }
                } else {
                    string path_str = shortest_paths[current_room + "G"];
                    today[to_string(period)] = path_str;
                }
            }
            week[day] = today;
        }
		(*routes)[student] = week;
    }

    return routes;
}

// pre-calculated all the shortest paths in advance to avoid run-time calculations
void generate_all_paths(const Graph& graph) {
    DistanceMatrix dist;
    PredecessorMatrix pred;

    initializeDistanceAndPredecessorMatrices(graph, dist, pred);
    floydWarshall(graph, dist, pred);

    Json routes;
    for (const auto [room1, _] : graph) {
        for (const auto [room2, __] : graph) {
            if (room1[0] != 'A' && room1[0] != 'B' && room1[0] != 'G') continue;
            if (room2[0] != 'A' && room2[0] != 'B' && room2[0] != 'G') continue;
            if (room1 == room2) continue;
            vector<string> path = reconstructPath(pred, room1, room2);
            string path_str = concatenate(path);
            routes[room1 + room2] = path_str;
        }
    }

    std::ofstream file("shortest_paths.json");
    if (file.is_open()) {
        file << routes.dump(4);  // 4 spaces for indentation
        file.close();
        std::cout << "JSON data written to routes.json successfully." << endl;
    } else {
        std::cerr << "Failed to open the file for writing." << endl;
    }
}

void generate_all_floyd_distances(const Graph& graph) {
    DistanceMatrix dist;
    PredecessorMatrix pred;

    initializeDistanceAndPredecessorMatrices(graph, dist, pred);
    floydWarshall(graph, dist, pred);

    Json distances;
    for (const auto [room1, _] : graph) {
        for (const auto [room2, __] : graph) {
            if (room1[0] != 'A' && room1[0] != 'B' && room1[0] != 'G') continue;
            if (room2[0] != 'A' && room2[0] != 'B' && room2[0] != 'G') continue;
            if (room1 == room2) continue;
            distances[room1 + room2] = dist[room1][room2];
        }
    }

    std::ofstream file("../assets/distances.json");
    if (file.is_open()) {
        file << distances.dump(4);  // 4 spaces for indentation
        file.close();
        std::cout << "JSON data written to distances.json successfully." << endl;
    } else {
        std::cerr << "Failed to open the file for writing." << endl;
    }
}

// int main() {
// 	std::unique_ptr<Json> hello = makeJson("hello");

//     string timetable_path;

//     ios::sync_with_stdio(false);
//     cin.tie(0);

//     // general input: the path to the time table json
//     // cin >> timetable_path;  // "../assets/timetable_0.json"
//     getline(cin, timetable_path);

//     // parse the JSON file
//     ifstream input_file(timetable_path);
//     if (!input_file.is_open()) {
//         cerr << "Failed to open the timetable file." << endl;
//         return 1;
//     }

//     Json timetables;
//     input_file >> timetables;
//     input_file.close();

// 	rust::String filename{"../assets/paths.txt"};
//     std::unique_ptr<Graph> graph = createSchoolGraph(filename);
//     // generate_all_paths(graph);

//     ifstream paths_file("../assets/shortest_paths.json");
//     if (!paths_file.is_open()) {
//         cerr << "Failed to open shortest_paths.json" << endl;
//         return 1;
//     }

//     Json shortest_paths;
//     paths_file >> shortest_paths;
//     paths_file.close();

//     std::unique_ptr<Json> routes = getRoutesfromTimetable(timetables, *graph,
//     shortest_paths);

//     ofstream file("routes.json");
//     if (file.is_open()) {
//         file << (*routes).dump();  // faster i/o
//         file.close();
//         cout << "JSON data written to routes.json successfully." << endl;
//     } else {
//         cerr << "Failed to open the file for writing." << endl;
//     }

//     return 0;
// }
