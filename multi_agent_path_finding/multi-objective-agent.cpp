#include <climits>
#include <cmath>
#include <fstream>
#include <iostream>
#include <queue>
#include <sstream>
#include <string>
#include <unordered_map>
#include <vector>

#include "json.hpp"
using json = nlohmann::json;

using namespace std;

// Edge for the school's graph
struct Edge {
    string dest;  // destination node
    int weight;   // edge weight (in this case, distance)
    int type;     // edge type
};
// Struct to store the student's path at a period
struct StudentPath {
    string id;
    double rperf;
    vector<string> path;
};
// Override the comparison for the priority queue
struct CompareStudentPath {
    bool operator()(const StudentPath& s1, const StudentPath& s2) {
        return s1.rperf < s2.rperf;
    }
};

using Graph = unordered_map<string, vector<Edge>>;
using PathPQ = priority_queue<StudentPath, vector<StudentPath>, CompareStudentPath>;

int BATCH_SIZE = 10;
const double CONGESTION_PENALTY = 10000.0;
int ITER_NUM = INT_MAX;
int ITER_COUNT = 1;
string ROUTE_FILE_PATH;
int ITER_SAVE_STEPS = 500;

// Code to create the school's layout graph from the path.txt
Graph createSchoolGraph(const string& file_path) {
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

// Codes for getting Floyd's shortest path for all the students
json getRoutesfromTimetable(const json& timetables, const Graph& graph,
                            const json& shortest_paths) {
    /* Obtain the routes as a json file from the timetables's json*/
    json routes;

    for (const auto& [student, timetable] : timetables.items()) {
        json week;
        for (const auto& [day, classes] : timetable.items()) {
            json today;

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
        routes[student] = week;
    }

    return routes;
}

// Code to slice a route string into a vector of strings
vector<string> vectorizeString(const string s) {
    vector<string> tokens;
    int start = 0;
    int end = s.find(' ');

    while (end != string::npos) {
        tokens.push_back(s.substr(start, end - start));
        start = end + 1;
        end = s.find(' ', start);
    }

    tokens.push_back(s.substr(start, end));

    return tokens;
}

// Code to concatenate a vector of strings into a single string
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

// Code to get the shortest path from start to end using Dijkstra's algorithm with
// congestion penalty
vector<string> getDijkstraPenaltiedPath(const string& start, const string& end,
                                        const Graph& graph,
                                        const unordered_map<string, int>& congestion) {
    unordered_map<string, double> distances;
    unordered_map<string, bool> visited;
    unordered_map<string, string> prev;  // to store the path

    vector<string> path;
    if (start == end) return path = {"G", "G"};

    for (const auto& [node, _] : graph) {
        distances[node] = INT_MAX;
        visited[node] = false;
    }
    distances[start] = 0;

    priority_queue<pair<double, string>> pq;
    pq.push({0, start});

    while (!pq.empty()) {
        string current = pq.top().second;
        pq.pop();

        if (visited[current]) continue;
        visited[current] = true;

        string destination;
        try {
            for (const auto& edge : graph.at(current)) {
                destination = edge.dest;
                double edgeWeight = edge.weight + (CONGESTION_PENALTY *
                                                   congestion.at(current + edge.dest));
                if (!visited[edge.dest] &&
                    distances[current] + edgeWeight < distances[edge.dest]) {
                    distances[edge.dest] = distances[current] + edgeWeight;
                    pq.push({-distances[edge.dest], edge.dest});
                    prev[edge.dest] =
                        current;  // save the previous node for path reconstruction
                }
            }
        } catch (const out_of_range& e) {
            cout << current << endl;
            cerr << "Error: " << e.what() << endl;
        }
    }

    // Reconstruct path from 'end' to 'start' using the 'prev' mapping
    for (string at = end; at != ""; at = prev[at]) {
        path.push_back(at);
    }
    reverse(path.begin(), path.end());  // since the path is constructed in reverse

    return path;
}

// Code to calculate r_perf using a whole congestion matrix
double computePerformanceIndex(const vector<string>& route,
                               const unordered_map<string, int>& congestion,
                               const Graph& graph) {
    double r_perf = 0.0;
    int c = 0;

    if (route[0] == "G" && route[1] == "G") return 0.0;  // spare class

    for (int i = 0; i < route.size() - 1; i++) {
        string start = route[i];
        string end = route[i + 1];

        if (start == "G" || end == "G") continue;  // Ignore the paths to and from G_floor

        int w_i;
        for (const auto& edge : graph.at(start)) {
            if (edge.dest == end) {
                w_i = edge.weight;
                break;
            }
        }

        int c_i = congestion.at(start + end);
        r_perf +=
            w_i * (2 + (exp((c_i - 300.0) / 200.0) - exp(-(c_i - 300.0) / 200.0)) /
                           (exp((c_i - 300.0) / 200.0) + exp(-(c_i - 300.0) / 200.0)));
        c += c_i;
    }

    return r_perf;
}

// Code for a single iteration of the code for a single period
void iter(PathPQ& paths, double& sum_rperf, unordered_map<string, int>& congestion,
          const Graph& graph, string& last_start, string& last_end,
          vector<StudentPath>& temp) {
    // A single iteration
    StudentPath worst_path;
    bool flag = false;
    while (!paths.empty()) {
        worst_path = paths.top();  // The student whose route has the greatest r_perf
        paths.pop();
        if (worst_path.path[0] == last_start &&
            worst_path.path[worst_path.path.size() - 1] == last_end) {
            temp.push_back(worst_path);
        } else {
            flag = true;
            break;
        }
    }
    if (!flag) return;
    sum_rperf -= worst_path.rperf;
    vector<string> new_route = getDijkstraPenaltiedPath(
        worst_path.path[0], worst_path.path[worst_path.path.size() - 1], graph,
        congestion);
    // for (const auto v: worst_path.path) cout << v << " ";
    // cout << endl;
    // for (const auto v: new_route) cout << v << " ";
    double new_rperf = computePerformanceIndex(new_route, congestion, graph);
    // cout << worst_path.rperf << " " << new_rperf << endl;
    if (new_rperf < worst_path.rperf) {
        StudentPath new_path = {worst_path.id, new_rperf, new_route};
        paths.push(new_path);
        sum_rperf += new_rperf;
        // cout << "UPDATED PATH " << worst_path.id << endl;
    } else {
        // if the path doesn't change, the student's path cannot be further optimized as
        // the congestion penalty is already too high
        temp.push_back(worst_path);
        sum_rperf += worst_path.rperf;
        last_start = worst_path.path[0],
        last_end = worst_path.path[worst_path.path.size() - 1];
    }
    // TODO： validate last_start and last_end
}

// Code for multiple iterations of the code for a single period
void iterMultiple(PathPQ& paths, double& sum_rperf,
                  unordered_map<string, int>& congestion, const Graph& graph) {
    string last_start, last_end;
    vector<StudentPath> temp;
    PathPQ paths_copy = paths;
    double sum_rperf_copy = sum_rperf;
    for (int i = ITER_COUNT; i < ITER_NUM; i++) {
        // A single iteration
        iter(paths, sum_rperf, congestion, graph, last_start, last_end, temp);

        if (i % BATCH_SIZE | i == 0)
            cout << "ITER " << i << " APPR " << sum_rperf << endl;
        else {  // When the batch size is met
            for (const auto& [node, edges] : graph) {
                for (const auto& edge : edges) {
                    congestion[node + edge.dest] = 0;
                }
            }

            vector<StudentPath>
                all_paths;  // contain all the paths from paths pq and temp vector
            PathPQ new_paths;
            // Get the congestion for the period
            while (!paths.empty()) {
                StudentPath path = paths.top();
                vector<string> route = path.path;
                paths.pop();
                for (int i = 0; i < route.size() - 1; i++)
                    congestion[route[i] + route[i + 1]]++;
                new_paths.push(path);
                all_paths.push_back(path);
            }

            for (const StudentPath& path : temp) {
                vector<string> route = path.path;
                for (int i = 0; i < route.size() - 1; i++)
                    congestion[route[i] + route[i + 1]]++;
                all_paths.push_back(path);
            }

            sum_rperf = 0.0;
            for (const StudentPath& path : all_paths) {
                double rperf = computePerformanceIndex(path.path, congestion, graph);
                sum_rperf += rperf;
            }

            paths = new_paths;

            if (sum_rperf < sum_rperf_copy) {
                sum_rperf_copy = sum_rperf;
                paths_copy = paths;
            } else {
                temp.push_back(paths_copy.top());
                paths_copy.pop();
                paths = paths_copy;
                sum_rperf = sum_rperf_copy;
            }

            cout << "ITER " << i << " ACC" << sum_rperf << endl;
        }
    }
    return;
}

// Code for generating the route for a single period
void iterSinglePeriod(const int day, const int period, json& route_tables,
                      const Graph& graph) {
    // route_tables is the floyd_warshall's routes for all students at all periods
    // generated from gerRoutesfromTimetable() Initialize the congestion matrix
    unordered_map<string, int> congestion;
    for (const auto& [node, edges] : graph) {
        for (const auto& edge : edges) {
            congestion[node + edge.dest] = 0;
        }
    }

    // Get the congestion for the period
    for (const auto& [student, route_table] : route_tables.items()) {
        vector<string> route =
            vectorizeString(route_table[to_string(day)][to_string(period)].get<string>());
        for (int i = 0; i < route.size() - 1; i++) {
            congestion[route[i] + route[i + 1]]++;
            congestion[route[i + 1] + route[i]]++;
        }
    }

    // Update the pq of paths
    PathPQ paths;
    double sum_rperf = 0;
    for (const auto& [student, route_table] : route_tables.items()) {
        vector<string> route =
            vectorizeString(route_table[to_string(day)][to_string(period)].get<string>());
        double rperf = computePerformanceIndex(route, congestion, graph);
        StudentPath path = {student, rperf, route};
        paths.push(path);
        sum_rperf += rperf;
    }
    cout << "INITIAL PERFORMACE: " << sum_rperf << endl;

    // Run the iterations
    iterMultiple(paths, sum_rperf, congestion, graph);

    // Save the routes
    while (!paths.empty()) {
        StudentPath path = paths.top();
        paths.pop();
        route_tables[path.id][to_string(day)][to_string(period)] = concatenate(path.path);
    }

    cout << "FINAL PERFORMANCE: " << sum_rperf << endl;

    return;
}

// Code for generating the route for a single day, with each iter covering all periods
void iterSingleDay(const int day, json& route_tables, const Graph& graph,
                   json& perf_indices) {
    PathPQ paths[15], paths_copy[15];
    double sum_rperf[15] = {0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0},
           sum_rperf_copy[15];
    unordered_map<string, int> congestions[15];
    double prev_best_rperf[15];

    for (int period = 0; period <= 11; period++) {
        if (period == 1 || period == 3 || period == 8 || period == 10)
            continue;  // All the students are having classes right now

        for (const auto& [node, edges] : graph) {
            for (const auto& edge : edges) {
                congestions[period][node + edge.dest] = 0;
            }
        }

        // Get the congestion for the period
        for (const auto& [student, route_table] : route_tables.items()) {
            const string route_str = route_table[to_string(day)][to_string(period)];
            if (route_str == "") continue;
            vector<string> route = vectorizeString(route_str);
            for (int i = 0; i < route.size() - 1; i++) {
                congestions[period][route[i] + route[i + 1]]++;
                congestions[period][route[i + 1] + route[i]]++;
            }
        }

        for (const auto& [student, route_table] : route_tables.items()) {
            const string route_str = route_table[to_string(day)][to_string(period)];
            vector<string> route;
            if (route_str == "")
                route = {"G", "G"};  // if this student is having a spare class
            else
                route = vectorizeString(route_str);
            double rperf = computePerformanceIndex(route, congestions[period], graph);
            StudentPath path = {student, rperf, route};
            paths[period].push(path);
            sum_rperf[period] += rperf;
        }

        paths_copy[period] = paths[period];
        sum_rperf_copy[period] = sum_rperf[period];
        prev_best_rperf[period] = sum_rperf[period];

        // cout << "PERIOD " << period << " INITIAL PERFORMANCE: " << sum_rperf[period] <<
        // endl;
    }

    string last_start[15], last_end[15];
    vector<StudentPath> temp[15];
    for (int i = ITER_COUNT + 1; i <= ITER_NUM; i++) {
        for (int period = 0; period <= 11; period++) {
            if (period == 1 || period == 3 || period == 8 || period == 10)
                continue;  // All the students are having classes right now

            iter(paths[period], sum_rperf[period], congestions[period], graph,
                 last_start[period], last_end[period], temp[period]);
            prev_best_rperf[period] = min(sum_rperf[period], prev_best_rperf[period]);

            if (i % BATCH_SIZE | i == 0) {
                // cout << fixed << setprecision(0) << "0 " << i << ' ' << day << ' ' <<
                // period << ' ' << sum_rperf[period] << ' ' << prev_best_rperf[period] <<
                // endl;
            } else {
                for (const auto& [node, edges] : graph) {
                    for (const auto& edge : edges) {
                        congestions[period][node + edge.dest] = 0;
                    }
                }

                vector<StudentPath>
                    all_paths;  // contain all the paths from paths pq and temp vector
                PathPQ new_paths;
                // Get the congestion for the period
                while (!paths[period].empty()) {
                    StudentPath path = paths[period].top();
                    vector<string> route = path.path;
                    paths[period].pop();
                    for (int i = 0; i < route.size() - 1; i++)
                        congestions[period][route[i] + route[i + 1]]++;
                    new_paths.push(path);
                    all_paths.push_back(path);
                }

                for (const StudentPath& path : temp[period]) {
                    vector<string> route = path.path;
                    for (int i = 0; i < route.size() - 1; i++)
                        congestions[period][route[i] + route[i + 1]]++;
                    all_paths.push_back(path);
                }

                sum_rperf[period] = 0.0;
                for (const StudentPath& path : all_paths) {
                    double rperf =
                        computePerformanceIndex(path.path, congestions[period], graph);
                    sum_rperf[period] += rperf;
                }

                paths[period] = new_paths;

                if (sum_rperf[period] > sum_rperf_copy[period]) {
                    temp[period].push_back(paths_copy[period].top());
                    paths_copy[period].pop();
                    paths[period] = paths_copy[period];
                    sum_rperf[period] = sum_rperf_copy[period];
                } else {
                    paths_copy[period] = paths[period];
                    sum_rperf_copy[period] = sum_rperf[period];
                }
                prev_best_rperf[period] = min(sum_rperf[period], prev_best_rperf[period]);
                perf_indices[to_string(day)][to_string(period)] = int(sum_rperf[period]);
                cout << fixed << setprecision(0) << "0 " << i << ' ' << day << ' '
                     << period << ' ' << sum_rperf[period] << ' '
                     << prev_best_rperf[period] << endl;
            }

            if (!(i % ITER_SAVE_STEPS) && period == 0 &&
                i) {  // for every 500 iterations dump the json file
                vector<pair<string, string>>
                    all_paths;  // contain the students and their related paths

                while (!paths[period].empty()) {
                    StudentPath path = paths[period].top();
                    paths[period].pop();
                    all_paths.push_back({path.id, concatenate(path.path)});
                }

                for (const StudentPath& path : temp[period])
                    all_paths.push_back({path.id, concatenate(path.path)});

                for (const auto& path : all_paths)
                    route_tables[path.first][to_string(day)][to_string(period)] =
                        path.second;

                json iter_output;
                iter_output["iter"] = i;
                iter_output["indices"] = perf_indices;
                iter_output["routes"] = route_tables;

                ofstream iter_output_file(ROUTE_FILE_PATH + "_" + to_string(day) +
                                          ".json");
                if (iter_output_file.is_open()) {
                    iter_output_file << iter_output.dump();  // minimize size
                    iter_output_file.close();
                    // cout << "JSON data written to routes.json successfully." << endl;
                    cout << fixed << setprecision(0) << "1 " << i << ' ' << day << ' '
                         << period << ' ' << sum_rperf[period] << ' '
                         << prev_best_rperf[period] << endl;
                } else {
                    // cerr << "Failed to open the file for writing." << endl;
                    cout << fixed << setprecision(0) << "! " << i << ' ' << day << ' '
                         << period << ' ' << sum_rperf[period] << ' '
                         << prev_best_rperf[period] << endl;
                }
            }
        }
    }
}

int main(int argc, char** argv) {
    // args parsing
    int day;
    for (int i = 0; i < argc; ++i) {
        if (strcmp(argv[i], "-b") == 0 &&
            i + 1 < argc) {  // Batch Size: the amount of iterations before updating the
                             // congestion
            BATCH_SIZE = stoi(argv[i + 1]);
        }
        if (strcmp(argv[i], "-f") == 0 &&
            i + 1 < argc) {  // Route File Path: the path to the routes file
            ROUTE_FILE_PATH = argv[i + 1];
        }
        if (strcmp(argv[i], "-d") == 0 &&
            i + 1 < argc) {  // Day: the day to run the algorithm for
            day = stoi(argv[i + 1]);
        }
        if (strcmp(argv[i], "-s") == 0 &&
            i + 1 < argc) {  // Save Iteration Step: the amount of iterations to before
                             // each save
            ITER_SAVE_STEPS = stoi(argv[i + 1]);
        }
    }

    // parse the shortest paths json file
    ifstream route_tables_file(ROUTE_FILE_PATH);
    if (!route_tables_file.is_open()) {
        cerr << "Failed to open routes file" << endl;
        return 1;
    }

    // save the shortest paths
    json route_tables, perf_indices;
    route_tables_file >> route_tables;
    route_tables_file.close();

    ITER_COUNT = route_tables["iter"][day - 1];
    perf_indices = route_tables["indices"];
    route_tables = route_tables["routes"];

    // create the layout graph for the school
    Graph graph = createSchoolGraph("../assets/paths.txt");

    // run the algorithm for a period
    iterSingleDay(day, route_tables, graph, perf_indices);

    return 0;
}
